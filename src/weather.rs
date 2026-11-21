//! Hong Kong Observatory open-data weather for the Ma Wan area.
//!
//! Uses the HKO Open Data API:
//! `https://data.weather.gov.hk/weatherAPI/opendata/weather.php`.
//! No key is required.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

const BASE: &str = "https://data.weather.gov.hk/weatherAPI/opendata/weather.php";
const USER_AGENT: &str = "mawan/0.1 (Ma Wan transport CLI; +https://github.com/)";

#[derive(Debug, Clone, Serialize)]
pub struct Observation {
    pub place: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rainfall {
    pub place: String,
    pub max_mm: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UvIndex {
    pub place: String,
    pub value: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeatherWarning {
    /// Warning statement code, e.g. `WTCSGNL`.
    pub code: String,
    /// Signal/subtype code, e.g. `TC8NE`, `WRAINR`.
    pub subtype: String,
    pub name: String,
    pub kind: String,
    pub action: String,
    pub issued: String,
    pub updated: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceImpact {
    /// `normal`, `watch`, `disruption` or `severe`.
    pub level: String,
    pub summary: String,
    pub bus: String,
    pub ferry: String,
    pub typhoon_signal: Option<String>,
    pub rainstorm_signal: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Forecast {
    pub period: String,
    pub description: String,
    pub outlook: String,
    pub situation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeatherReport {
    pub updated: String,
    pub temperature: Vec<Observation>,
    pub humidity: Vec<Observation>,
    pub rainfall: Vec<Rainfall>,
    pub uv: Option<UvIndex>,
    pub warnings: Vec<WeatherWarning>,
    pub impact: ServiceImpact,
    pub forecast: Option<Forecast>,
}

pub struct WeatherClient {
    http: reqwest::Client,
}

impl WeatherClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .build()
            .context("building weather client")?;
        Ok(Self { http })
    }

    pub async fn report(&self, lang: &str) -> Result<WeatherReport> {
        let (current, warnings, forecast) = tokio::join!(
            self.get("rhrread", lang),
            self.get("warnsum", lang),
            self.get("flw", lang)
        );
        let current = current.context("fetching current weather")?;
        // Warnings and the forecast are useful but not essential; do not fail
        // the whole report when one of them is unavailable.
        let warnings = warnings.unwrap_or(Value::Null);
        let forecast = forecast.unwrap_or(Value::Null);
        Ok(summarise(&current, &warnings, &forecast))
    }

    async fn get(&self, data_type: &str, lang: &str) -> Result<Value> {
        let url = format!("{BASE}?dataType={data_type}&lang={lang}");
        let response = self
            .http
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| format!("reading {url}"))?;
        if !status.is_success() {
            bail!(
                "HKO API returned HTTP {status} for {data_type}: {}",
                body.chars().take(160).collect::<String>()
            );
        }
        serde_json::from_str(&body).with_context(|| format!("parsing HKO {data_type}"))
    }
}

pub fn summarise(current: &Value, warnings: &Value, forecast: &Value) -> WeatherReport {
    let warnings = active_warnings(warnings);
    let impact = service_impact(&warnings);
    WeatherReport {
        updated: string_at(current, &["updateTime"]),
        temperature: observations(&current["temperature"], relevant_temperature_place),
        humidity: observations(&current["humidity"], |_| true)
            .into_iter()
            .take(3)
            .collect(),
        rainfall: rainfall(&current["rainfall"]),
        uv: uv_index(&current["uvindex"]),
        warnings,
        impact,
        forecast: forecast_summary(forecast),
    }
}

fn string_at(value: &Value, path: &[&str]) -> String {
    let mut node = value;
    for key in path {
        node = &node[*key];
    }
    node.as_str().unwrap_or_default().to_string()
}

fn observations(value: &Value, keep: fn(&str) -> bool) -> Vec<Observation> {
    value["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let place = row["place"].as_str()?;
            if !keep(place) {
                return None;
            }
            Some(Observation {
                place: place.to_string(),
                value: row["value"].as_f64()?,
                unit: row["unit"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

fn rainfall(value: &Value) -> Vec<Rainfall> {
    value["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let place = row["place"].as_str()?;
            if !relevant_rain_place(place) {
                return None;
            }
            Some(Rainfall {
                place: place.to_string(),
                max_mm: row["max"].as_f64()?,
            })
        })
        .collect()
}

fn uv_index(value: &Value) -> Option<UvIndex> {
    let row = value["data"].as_array()?.first()?;
    Some(UvIndex {
        place: row["place"].as_str().unwrap_or_default().to_string(),
        value: row["value"].as_f64()?,
        description: row["desc"].as_str().unwrap_or_default().to_string(),
    })
}

fn active_warnings(value: &Value) -> Vec<WeatherWarning> {
    let Some(map) = value.as_object() else {
        return Vec::new();
    };
    let mut warnings: Vec<WeatherWarning> = map
        .iter()
        .filter_map(|(code, row)| {
            let action = row["actionCode"].as_str().unwrap_or_default();
            if action.eq_ignore_ascii_case("cancel") {
                return None;
            }
            Some(WeatherWarning {
                code: code.clone(),
                subtype: row["code"].as_str().unwrap_or_default().to_string(),
                name: row["name"].as_str().unwrap_or_default().to_string(),
                kind: row["type"].as_str().unwrap_or_default().to_string(),
                action: action.to_string(),
                issued: row["issueTime"].as_str().unwrap_or_default().to_string(),
                updated: row["updateTime"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect();
    warnings.sort_by(|a, b| a.name.cmp(&b.name).then(a.code.cmp(&b.code)));
    warnings
}

fn service_impact(warnings: &[WeatherWarning]) -> ServiceImpact {
    let typhoon = warnings
        .iter()
        .find(|warning| warning.code.eq_ignore_ascii_case("WTCSGNL"));
    let rainstorm = warnings
        .iter()
        .find(|warning| warning.code.eq_ignore_ascii_case("WRAIN"));
    let pre8 = warnings.iter().any(|warning| {
        warning.code.eq_ignore_ascii_case("WTCPRE8")
            || warning.subtype.eq_ignore_ascii_case("WTCPRE8")
    });

    let typhoon_signal = typhoon.and_then(typhoon_number);
    let rainstorm_signal = rainstorm.and_then(rainstorm_name);

    let (mut level, mut summary, mut bus, mut ferry) = match typhoon_signal.as_deref() {
        Some("9") | Some("10") => (
            "severe",
            format!(
                "Signal No. {}: treat ferry service as suspended; buses may be suspended under high-wind arrangements.",
                typhoon_signal.as_deref().unwrap_or("?")
            ),
            "Expect suspension; check PITCL notices and TD traffic news.".to_string(),
            "Suspended; do not rely on a ferry crossing.".to_string(),
        ),
        Some("8") => (
            "disruption",
            "T8: ferries suspend about 2 hours after the signal is hoisted, not necessarily immediately. Buses may continue until Stage 2 high-wind management on Tsing Ma Bridge (>75 km/h) suspends them.".to_string(),
            "May continue at first; suspended if Tsing Ma Bridge Stage 2 (>75 km/h) is in force. Check TD traffic news.".to_string(),
            "Suspends about 2 hours after T8; Tsuen Wan ferry may be strengthened until crowds clear.".to_string(),
        ),
        Some("3") => (
            "watch",
            "T3: ferries may be limited or suspended; buses normally continue but can be delayed by high wind.".to_string(),
            "Normally continues; delays possible.".to_string(),
            "May be limited or suspended, especially before or shortly after T8.".to_string(),
        ),
        Some("1") => (
            "watch",
            "T1: ferries may be delayed; buses normally continue.".to_string(),
            "Normally continues.".to_string(),
            "May be delayed by sea conditions.".to_string(),
        ),
        _ if pre8 => (
            "watch",
            "T8 is expected within 2 hours: prepare for ferry suspension and possible bus suspension.".to_string(),
            "Prepare for possible suspension if high wind reaches Stage 2.".to_string(),
            "Will suspend about 2 hours after T8 is hoisted.".to_string(),
        ),
        _ => (
            "normal",
            "No tropical cyclone warning signal is in force.".to_string(),
            "No typhoon-related suspension is published.".to_string(),
            "No typhoon-related suspension is published.".to_string(),
        ),
    };

    if let Some(signal) = &rainstorm_signal {
        if level == "normal" {
            level = "watch";
        }
        summary.push_str(&format!(
            " {} rainstorm signal: expect delays; PITCL's published plan has no automatic suspension.",
            signal
        ));
        bus.push_str(" Rainstorm: expect delays.");
        ferry.push_str(" Rainstorm: expect delays.");
    }

    if warnings
        .iter()
        .any(|warning| warning.code.eq_ignore_ascii_case("WMSGNL"))
    {
        if level == "normal" {
            level = "watch";
        }
        summary.push_str(" Strong monsoon signal: ferry sailings may be delayed.");
        ferry.push_str(" Strong monsoon: may be delayed.");
    }

    ServiceImpact {
        level: level.to_string(),
        summary,
        bus,
        ferry,
        typhoon_signal,
        rainstorm_signal,
    }
}

fn typhoon_number(warning: &WeatherWarning) -> Option<String> {
    let subtype = warning.subtype.to_ascii_lowercase();
    for number in ["10", "9", "8", "3", "1"] {
        if subtype == format!("tc{number}") || subtype.starts_with("tc8") && number == "8" {
            return Some(number.to_string());
        }
    }
    let kind = warning.kind.to_ascii_lowercase();
    for number in ["10", "9", "8", "3", "1"] {
        if kind.contains(&format!("no. {number}")) || kind.contains(&format!("number {number}")) {
            return Some(number.to_string());
        }
    }
    None
}

fn rainstorm_name(warning: &WeatherWarning) -> Option<String> {
    let subtype = warning.subtype.to_ascii_uppercase();
    if subtype.ends_with('B') {
        return Some("Black".to_string());
    }
    if subtype.ends_with('R') {
        return Some("Red".to_string());
    }
    if subtype.ends_with('A') {
        return Some("Amber".to_string());
    }
    let kind = warning.kind.trim();
    if kind.is_empty() {
        None
    } else {
        Some(kind.to_string())
    }
}

fn forecast_summary(value: &Value) -> Option<Forecast> {
    let period = value["forecastPeriod"].as_str()?;
    Some(Forecast {
        period: period.to_string(),
        description: value["forecastDesc"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        outlook: value["outlook"].as_str().unwrap_or_default().to_string(),
        situation: value["generalSituation"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    })
}

fn relevant_temperature_place(place: &str) -> bool {
    matches!(
        place,
        "Hong Kong Observatory"
            | "香港天文台"
            | "Tsing Yi"
            | "青衣"
            | "Tsuen Wan Ho Koon"
            | "荃灣可觀"
            | "Tsuen Wan Shing Mun Valley"
            | "荃灣城門谷"
            | "Chek Lap Kok"
            | "赤鱲角"
            | "Cheung Chau"
            | "長洲"
    )
}

fn relevant_rain_place(place: &str) -> bool {
    matches!(
        place,
        "Tsuen Wan" | "荃灣" | "Kwai Tsing" | "葵青" | "Islands District" | "離島區"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn t8_is_not_an_automatic_bus_cancellation() {
        let current = json!({"updateTime": "2026-11-21T23:51:00+08:00"});
        let warnings = json!({
            "WTCSGNL": {
                "name": "Tropical Cyclone Warning Signal",
                "code": "TC8NE",
                "type": "No. 8 North East",
                "actionCode": "ISSUE"
            }
        });
        let report = summarise(&current, &warnings, &Value::Null);
        assert_eq!(report.impact.typhoon_signal.as_deref(), Some("8"));
        assert!(report.impact.summary.contains("2 hours"));
        assert!(report.impact.bus.contains("75 km/h"));
        assert!(report.impact.ferry.contains("2 hours"));
    }

    #[test]
    fn selects_ma_wan_area_observations_and_active_warnings() {
        let current = json!({
            "updateTime": "2026-11-21T23:51:00+08:00",
            "temperature": {"data": [
                {"place": "Hong Kong Observatory", "value": 21, "unit": "C"},
                {"place": "Tsing Yi", "value": 20, "unit": "C"},
                {"place": "Ta Kwu Ling", "value": 14, "unit": "C"}
            ]},
            "humidity": {"data": [{"place": "Hong Kong Observatory", "value": 53, "unit": "percent"}]},
            "rainfall": {"data": [
                {"place": "Tsuen Wan", "max": 0, "unit": "mm"},
                {"place": "Yuen Long", "max": 0, "unit": "mm"}
            ]},
            "uvindex": {"data": [{"place": "King's Park", "value": 0.1, "desc": "low"}]}
        });
        let warnings = json!({
            "WFIRE": {"name": "Fire Danger Warning", "actionCode": "CANCEL"},
            "WRAIN": {"name": "Rainstorm Warning Signal", "type": "Amber", "actionCode": "ISSUE"}
        });
        let forecast = json!({"forecastPeriod": "Sunday", "forecastDesc": "Cloudy", "outlook": "Cooler", "generalSituation": "Monsoon"});
        let report = summarise(&current, &warnings, &forecast);
        assert_eq!(report.temperature.len(), 2);
        assert_eq!(report.temperature[1].place, "Tsing Yi");
        assert_eq!(report.rainfall.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "WRAIN");
        assert_eq!(report.forecast.unwrap().description, "Cloudy");
    }
}
