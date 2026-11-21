//! Hong Kong Observatory open-data weather for the Ma Wan area.
//!
//! Uses the HKO Open Data API:
//! `https://data.weather.gov.hk/weatherAPI/opendata/weather.php`.
//! No key is required.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::cache::ResponseCache;

const BASE: &str = "https://data.weather.gov.hk/weatherAPI/opendata/weather.php";
const WIND_URL: &str =
    "https://data.weather.gov.hk/weatherAPI/hko_data/regional-weather/latest_10min_wind.csv";
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
pub struct WindObservation {
    pub place: String,
    pub direction: String,
    pub speed_kmh: Option<f64>,
    pub gust_kmh: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeWind {
    /// False because no public feed includes a Tsing Ma Bridge anemometer.
    pub available: bool,
    /// Nearest regional station used as a proxy, currently Tsing Yi.
    pub station: Option<String>,
    pub speed_kmh: Option<f64>,
    pub gust_kmh: Option<f64>,
    /// `unknown` because the published bridge thresholds cannot be applied.
    pub stage: String,
    pub note: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeatherReport {
    pub updated: String,
    pub temperature: Vec<Observation>,
    pub humidity: Vec<Observation>,
    pub rainfall: Vec<Rainfall>,
    pub uv: Option<UvIndex>,
    pub wind: Vec<WindObservation>,
    pub bridge_wind: BridgeWind,
    pub warnings: Vec<WeatherWarning>,
    pub impact: ServiceImpact,
    pub forecast: Option<Forecast>,
}

pub struct WeatherClient {
    http: reqwest::Client,
    cache: ResponseCache,
}

impl WeatherClient {
    pub fn new(ttl: Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .build()
            .context("building weather client")?;
        Ok(Self {
            http,
            cache: ResponseCache::new(ttl),
        })
    }

    pub async fn report(&self, lang: &str) -> Result<WeatherReport> {
        let (current, warnings, forecast, wind) = tokio::join!(
            self.get("rhrread", lang),
            self.get("warnsum", lang),
            self.get("flw", lang),
            self.wind(lang)
        );
        let current = current.context("fetching current weather")?;
        // Warnings, the forecast and regional wind are useful but not
        // essential; do not fail the whole report when one is unavailable.
        let warnings = warnings.unwrap_or(Value::Null);
        let forecast = forecast.unwrap_or(Value::Null);
        let wind = wind.unwrap_or_default();
        Ok(summarise(&current, &warnings, &forecast, &wind, lang))
    }

    async fn get(&self, data_type: &str, lang: &str) -> Result<Value> {
        let url = format!("{BASE}?dataType={data_type}&lang={lang}");
        self.get_json(&url, data_type).await
    }

    async fn wind(&self, lang: &str) -> Result<Vec<WindObservation>> {
        let body = self.get_text(WIND_URL, "regional wind").await?;
        Ok(parse_wind(&body, lang))
    }

    async fn get_json(&self, url: &str, label: &str) -> Result<Value> {
        let body = self.get_text(url, label).await?;
        serde_json::from_str(&body).with_context(|| format!("parsing HKO {label}"))
    }

    async fn get_text(&self, url: &str, label: &str) -> Result<String> {
        if let Some(body) = self.cache.get(url) {
            return Ok(body);
        }
        let response = self
            .http
            .get(url)
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
                "HKO API returned HTTP {status} for {label}: {}",
                body.chars().take(160).collect::<String>()
            );
        }
        self.cache.insert(url, body.clone());
        Ok(body)
    }
}

pub fn summarise(
    current: &Value,
    warnings: &Value,
    forecast: &Value,
    wind: &[WindObservation],
    lang: &str,
) -> WeatherReport {
    let warnings = active_warnings(warnings);
    let impact = service_impact(&warnings, lang);
    let bridge_wind = bridge_wind(wind, lang);
    WeatherReport {
        updated: string_at(current, &["updateTime"]),
        temperature: observations(&current["temperature"], relevant_temperature_place),
        humidity: observations(&current["humidity"], |_| true)
            .into_iter()
            .take(3)
            .collect(),
        rainfall: rainfall(&current["rainfall"]),
        uv: uv_index(&current["uvindex"]),
        wind: wind.to_vec(),
        bridge_wind,
        warnings,
        impact,
        forecast: forecast_summary(forecast),
    }
}

fn text(lang: &str, en: &str, tc: &str) -> String {
    if lang == "tc" { tc } else { en }.to_string()
}

pub fn parse_wind(csv: &str, lang: &str) -> Vec<WindObservation> {
    const PLACES: &[(&str, &str)] = &[
        ("Tsing Yi", "青衣"),
        ("Chek Lap Kok", "赤鱲角"),
        ("Cheung Chau", "長洲"),
        ("Central Pier", "中環碼頭"),
        ("Waglan Island", "橫瀾島"),
    ];
    let mut out = Vec::new();
    for line in csv.lines().skip(1) {
        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() < 5 {
            continue;
        }
        let place = columns[1].trim();
        let Some((_, place_tc)) = PLACES.iter().find(|(en, _)| *en == place) else {
            continue;
        };
        let direction = match columns[2].trim() {
            "N/A" | "" => String::new(),
            value if lang == "tc" => wind_direction_tc(value).to_string(),
            value => value.to_string(),
        };
        out.push(WindObservation {
            place: if lang == "tc" {
                (*place_tc).to_string()
            } else {
                place.to_string()
            },
            direction,
            speed_kmh: speed(columns[3].trim()),
            gust_kmh: speed(columns[4].trim()),
        });
    }
    out
}

fn speed(value: &str) -> Option<f64> {
    match value {
        "" => Some(0.0),
        "N/A" => None,
        value => value.parse().ok(),
    }
}

fn wind_direction_tc(value: &str) -> &'static str {
    match value {
        "North" => "北",
        "Northeast" => "東北",
        "East" => "東",
        "Southeast" => "東南",
        "South" => "南",
        "Southwest" => "西南",
        "West" => "西",
        "Northwest" => "西北",
        "Calm" => "無風",
        "Variable" => "風向不定",
        _ => "",
    }
}

fn bridge_wind(wind: &[WindObservation], lang: &str) -> BridgeWind {
    let station = if lang == "tc" { "青衣" } else { "Tsing Yi" };
    let observation = wind.iter().find(|observation| observation.place == station);
    BridgeWind {
        available: false,
        station: observation.map(|observation| observation.place.clone()),
        speed_kmh: observation.and_then(|observation| observation.speed_kmh),
        gust_kmh: observation.and_then(|observation| observation.gust_kmh),
        stage: "unknown".into(),
        note: text(
            lang,
            "HKO's open regional wind feed has no Tsing Ma Bridge anemometer. The Tsing Yi station is shown as a regional proxy only; PITCL's 40/60/75 km/h bridge thresholds cannot be applied automatically.",
            "香港天文台公開的分區風速資料沒有青馬大橋測風站。青衣站只可作為區域參考，不能自動套用珀麗灣客運40／60／75公里每小時的青馬大橋門檻。",
        ),
        source: WIND_URL.into(),
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

fn service_impact(warnings: &[WeatherWarning], lang: &str) -> ServiceImpact {
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
    let rainstorm_signal = rainstorm.and_then(|warning| rainstorm_name(warning, lang));

    let (mut level, mut summary, mut bus, mut ferry) = match typhoon_signal.as_deref() {
        Some("9") | Some("10") => (
            "severe",
            text(
                lang,
                &format!(
                    "Signal No. {}: treat ferry service as suspended; buses may be suspended under high-wind arrangements.",
                    typhoon_signal.as_deref().unwrap_or("?")
                ),
                &format!(
                    "{}號風球：渡輪服務應視為暫停；巴士可能因強風安排而暫停。",
                    typhoon_signal.as_deref().unwrap_or("?")
                ),
            ),
            text(
                lang,
                "Expect suspension; check PITCL notices and TD traffic news.",
                "預計暫停；請查看珀麗灣客運通告及運輸署交通消息。",
            ),
            text(
                lang,
                "Suspended; do not rely on a ferry crossing.",
                "暫停；不要依賴渡輪過海。",
            ),
        ),
        Some("8") => (
            "disruption",
            text(
                lang,
                "T8: ferries suspend about 2 hours after the signal is hoisted, not necessarily immediately. Buses may continue until Stage 2 high-wind management on Tsing Ma Bridge (>75 km/h) suspends them.",
                "八號風球：渡輪一般在信號發出約兩小時後暫停，並非即時停航。巴士可能繼續服務，直至青馬大橋實施第二階段強風交通管理（風速超過75公里每小時）才停駛。",
            ),
            text(
                lang,
                "May continue at first; suspended if Tsing Ma Bridge Stage 2 (>75 km/h) is in force. Check TD traffic news.",
                "初期可能繼續服務；如青馬大橋實施第二階段（超過75公里每小時）則停駛。請查看運輸署交通消息。",
            ),
            text(
                lang,
                "Suspends about 2 hours after T8; Tsuen Wan ferry may be strengthened until crowds clear.",
                "八號風球後約兩小時暫停；荃灣航線可能在人群疏導前加強服務。",
            ),
        ),
        Some("3") => (
            "watch",
            text(
                lang,
                "T3: ferries may be limited or suspended; buses normally continue but can be delayed by high wind.",
                "三號風球：渡輪可能有限度服務或暫停；巴士一般繼續行駛，但可能受強風延誤。",
            ),
            text(
                lang,
                "Normally continues; delays possible.",
                "一般繼續服務；可能延誤。",
            ),
            text(
                lang,
                "May be limited or suspended, especially before or shortly after T8.",
                "可能有限度服務或暫停，尤其在八號風球前後。",
            ),
        ),
        Some("1") => (
            "watch",
            text(
                lang,
                "T1: ferries may be delayed; buses normally continue.",
                "一號風球：渡輪可能延誤；巴士一般繼續服務。",
            ),
            text(lang, "Normally continues.", "一般繼續服務。"),
            text(
                lang,
                "May be delayed by sea conditions.",
                "可能受海面情況延誤。",
            ),
        ),
        _ if pre8 => (
            "watch",
            text(
                lang,
                "T8 is expected within 2 hours: prepare for ferry suspension and possible bus suspension.",
                "預計兩小時內發出八號風球：請準備渡輪暫停及巴士可能停駛。",
            ),
            text(
                lang,
                "Prepare for possible suspension if high wind reaches Stage 2.",
                "如強風達到第二階段，請準備可能停駛。",
            ),
            text(
                lang,
                "Will suspend about 2 hours after T8 is hoisted.",
                "八號風球發出後約兩小時暫停。",
            ),
        ),
        _ => (
            "normal",
            text(
                lang,
                "No tropical cyclone warning signal is in force.",
                "現時沒有熱帶氣旋警告信號生效。",
            ),
            text(
                lang,
                "No typhoon-related suspension is published.",
                "沒有公布因颱風而暫停的服務。",
            ),
            text(
                lang,
                "No typhoon-related suspension is published.",
                "沒有公布因颱風而暫停的服務。",
            ),
        ),
    };

    if let Some(signal) = &rainstorm_signal {
        if level == "normal" {
            level = "watch";
        }
        summary.push_str(&text(
            lang,
            &format!(
                " {} rainstorm signal: expect delays; PITCL's published plan has no automatic suspension.",
                signal
            ),
            &format!(
                "{}暴雨警告：可能延誤；珀麗灣客運公布的措施沒有自動停駛。",
                signal
            ),
        ));
        bus.push_str(&text(
            lang,
            " Rainstorm: expect delays.",
            " 暴雨：可能延誤。",
        ));
        ferry.push_str(&text(
            lang,
            " Rainstorm: expect delays.",
            " 暴雨：可能延誤。",
        ));
    }

    if warnings
        .iter()
        .any(|warning| warning.code.eq_ignore_ascii_case("WMSGNL"))
    {
        if level == "normal" {
            level = "watch";
        }
        summary.push_str(&text(
            lang,
            " Strong monsoon signal: ferry sailings may be delayed.",
            " 強烈季候風信號：渡輪航班可能延誤。",
        ));
        ferry.push_str(&text(
            lang,
            " Strong monsoon: may be delayed.",
            " 強烈季候風：可能延誤。",
        ));
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

fn rainstorm_name(warning: &WeatherWarning, lang: &str) -> Option<String> {
    let subtype = warning.subtype.to_ascii_uppercase();
    if subtype.ends_with('B') {
        return Some(text(lang, "Black", "黑色"));
    }
    if subtype.ends_with('R') {
        return Some(text(lang, "Red", "紅色"));
    }
    if subtype.ends_with('A') {
        return Some(text(lang, "Amber", "黃色"));
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
        let report = summarise(&current, &warnings, &Value::Null, &[], "en");
        assert_eq!(report.impact.typhoon_signal.as_deref(), Some("8"));
        assert!(report.impact.summary.contains("2 hours"));
        assert!(report.impact.bus.contains("75 km/h"));
        assert!(report.impact.ferry.contains("2 hours"));
    }

    #[test]
    fn t8_impact_is_localised() {
        let warnings = json!({
            "WTCSGNL": {
                "name": "熱帶氣旋警告信號",
                "code": "TC8NE",
                "type": "八號東北烈風或暴風信號",
                "actionCode": "ISSUE"
            }
        });
        let report = summarise(&json!({}), &warnings, &Value::Null, &[], "tc");
        assert!(report.impact.summary.contains("兩小時"));
        assert!(report.impact.bus.contains("75公里"));
    }

    #[test]
    fn regional_wind_is_parsed_but_bridge_stage_is_unknown() {
        let csv = "Date time,Automatic Weather Station,10-Minute Mean Wind Direction(Compass points),10-Minute Mean Speed(km/hour),10-Minute Maximum Gust(km/hour)\n202611220020,Tsing Yi,East,42,55\n202611220020,Chek Lap Kok,Northeast,14,18\n";
        let wind = parse_wind(csv, "en");
        assert_eq!(wind.len(), 2);
        assert_eq!(wind[0].place, "Tsing Yi");
        assert_eq!(wind[0].speed_kmh, Some(42.0));
        let bridge = bridge_wind(&wind, "en");
        assert!(!bridge.available);
        assert_eq!(bridge.stage, "unknown");
        assert_eq!(bridge.speed_kmh, Some(42.0));
        assert!(bridge.note.contains("no Tsing Ma Bridge"));
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
        let report = summarise(&current, &warnings, &forecast, &[], "en");
        assert_eq!(report.temperature.len(), 2);
        assert_eq!(report.temperature[1].place, "Tsing Yi");
        assert_eq!(report.rainfall.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "WRAIN");
        assert_eq!(report.forecast.unwrap().description, "Cloudy");
    }
}
