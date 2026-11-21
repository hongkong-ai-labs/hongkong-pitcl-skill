//! Client for the official Park Island (PITCL) ETA backend.
//!
//! The Park Island ETA web app at `eta.pitcl.com.hk` is a Socif deployment.
//! Its public JSON API lives at `360-api.socif.co` and is the feed behind the
//! app. PITCL's own route IDs on that platform are stable:
//! NR330=249, NR332=250, NR334=252, NR338=253.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use serde_json::Value;

use crate::model::{DayType, DirectionInfo, Eta, Frequency, Schedule, Stop, hkt};

pub const BASE: &str = "https://360-api.socif.co";
pub const USER_AGENT: &str = "mawan/0.1 (Ma Wan bus CLI; +https://github.com/)";

/// Route code, Socif route id.
pub const SOCIF_ROUTES: &[(&str, u64)] = &[
    ("NR330", 249),
    ("NR332", 250),
    ("NR334", 252),
    ("NR338", 253),
];

pub fn route_id(code: &str) -> Option<u64> {
    SOCIF_ROUTES
        .iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(code))
        .map(|(_, id)| *id)
}

struct CacheEntry {
    at: Instant,
    body: String,
}

pub struct SocifClient {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl SocifClient {
    pub fn new(ttl: Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(15))
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            http,
            cache: Mutex::new(HashMap::new()),
            ttl,
        })
    }

    async fn get(&self, path: &str) -> Result<Value> {
        if self.ttl > Duration::ZERO
            && let Ok(cache) = self.cache.lock()
            && let Some(entry) = cache.get(path)
            && entry.at.elapsed() < self.ttl
        {
            return serde_json::from_str(&entry.body)
                .with_context(|| format!("cached JSON for {path}"));
        }

        let url = format!("{BASE}{path}");
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
            return Err(anyhow!(
                "PITCL ETA API returned HTTP {status} for {path}: {}",
                body.chars().take(200).collect::<String>()
            ));
        }
        let value: Value =
            serde_json::from_str(&body).with_context(|| format!("parsing JSON from {url}"))?;
        if self.ttl > Duration::ZERO
            && let Ok(mut cache) = self.cache.lock()
        {
            cache.insert(
                path.to_string(),
                CacheEntry {
                    at: Instant::now(),
                    body,
                },
            );
        }
        Ok(value)
    }

    pub async fn metadata(&self, id: u64) -> Result<RouteMeta> {
        let value = self.get(&format!("/api/route/{id}")).await?;
        let mut rows: Vec<RouteMeta> = serde_json::from_value(
            value
                .get("data")
                .cloned()
                .ok_or_else(|| anyhow!("route {id}: missing data"))?,
        )
        .with_context(|| format!("route {id}: unexpected response"))?;
        rows.pop()
            .ok_or_else(|| anyhow!("route {id}: no metadata returned"))
    }

    pub async fn stops(&self, id: u64, seq: u8) -> Result<Vec<Stop>> {
        let value = self.get(&format!("/api/route-stop/{id}/{seq}")).await?;
        let data = value
            .get("data")
            .ok_or_else(|| anyhow!("route {id}/{seq}: missing data"))?;
        let rows: Vec<RawStop> = serde_json::from_value(
            data.get("route_stops")
                .cloned()
                .unwrap_or_else(|| Value::Array(vec![])),
        )
        .context("route-stop: unexpected response")?;
        Ok(rows
            .into_iter()
            .map(|row| Stop {
                seq: row.stop_seq,
                id: row.stop_id.to_string(),
                name_en: row.name_en,
                name_tc: row.name_tc,
            })
            .collect())
    }

    /// Estimated arrivals for every stop on a route direction.
    pub async fn etas(&self, id: u64, seq: u8) -> Result<Vec<Eta>> {
        let stops = self.stops(id, seq).await?;
        let value = self.get(&format!("/api/eta/route-stop/{id}/{seq}")).await?;
        let data = value
            .get("data")
            .ok_or_else(|| anyhow!("ETA {id}/{seq}: missing data"))?;
        let rows: Vec<RawStopEta> = serde_json::from_value(
            data.get("eta")
                .cloned()
                .unwrap_or_else(|| Value::Array(vec![])),
        )
        .context("ETA: unexpected response")?;

        let mut out = Vec::new();
        for row in rows {
            let Some(stop) = stops.iter().find(|s| s.seq == row.stop_seq) else {
                continue;
            };
            for entry in row.eta {
                let remark_en = entry.remarks_en.unwrap_or_default();
                out.push(Eta {
                    stop_seq: stop.seq,
                    stop_id: stop.id.clone(),
                    stop_en: stop.name_en.clone(),
                    stop_tc: stop.name_tc.clone(),
                    direction: seq,
                    eta_seq: entry.eta_seq,
                    minutes: entry.diff,
                    at: entry
                        .timestamp
                        .as_deref()
                        .and_then(|value| normalize_timestamp(value).ok()),
                    realtime: !remark_en.eq_ignore_ascii_case("scheduled"),
                    remark_en,
                    remark_tc: entry.remarks_tc.unwrap_or_default(),
                });
            }
        }
        Ok(out)
    }

    /// Exact weekday departure times as HH:MM in Hong Kong time.
    pub async fn weekday_departures(&self, id: u64, seq: u8) -> Result<Vec<String>> {
        let value = self
            .get(&format!("/api/weekdaySchedule/{id}/{seq}"))
            .await?;
        let rows: Vec<RawSchedule> = serde_json::from_value(
            value
                .get("data")
                .cloned()
                .unwrap_or_else(|| Value::Array(vec![])),
        )
        .context("weekday schedule: unexpected response")?;
        let row = rows
            .into_iter()
            .find(|row| row.route_seq == seq)
            .ok_or_else(|| anyhow!("no weekday schedule for route {id} direction {seq}"))?;

        let mut seen = HashSet::new();
        let mut departures = Vec::new();
        for raw in row.schedules {
            let Ok(time) = normalize_hhmm(&raw) else {
                continue;
            };
            if seen.insert(time.clone()) {
                departures.push(time);
            }
        }
        // The feed can begin after midnight (NR332's 01:00-05:30 extras).
        // Rotate so the daytime service starts the timetable and the
        // after-midnight departures sit at the end, as printed by the operator.
        if let Some(index) = departures.iter().position(|time| time.as_str() >= "05:00")
            && index > 0
        {
            departures.rotate_left(index);
        }
        Ok(departures)
    }

    pub fn frequencies(meta: &RouteMeta, seq: u8, day: DayType) -> Vec<Frequency> {
        let Some(direction) = meta.directions.iter().find(|d| d.route_seq == seq) else {
            return Vec::new();
        };
        let mut rows: Vec<&Headway> = direction
            .headways
            .iter()
            .filter(|headway| matches_day(headway, day))
            .collect();
        if rows.is_empty() && day == DayType::Holiday {
            rows = direction
                .headways
                .iter()
                .filter(|headway| matches_day(headway, DayType::Sunday))
                .collect();
        }
        rows.into_iter()
            .map(|headway| Frequency {
                start: hhmm_from_seconds(&headway.start_time),
                end: headway
                    .end_time
                    .as_deref()
                    .map(hhmm_from_seconds)
                    .filter(|end| !end.is_empty()),
                min_minutes: headway.frequency,
                max_minutes: headway.frequency_upper.or(headway.frequency),
            })
            .collect()
    }
}

pub fn build_schedule(
    code: &str,
    meta: &RouteMeta,
    seq: u8,
    day: DayType,
    weekday_exact: Vec<String>,
) -> Schedule {
    let direction = meta.directions.iter().find(|d| d.route_seq == seq);
    let (direction_en, direction_tc) = direction
        .map(|d| (d.name_en(), d.name_tc()))
        .unwrap_or_else(|| (String::new(), String::new()));

    let frequencies = SocifClient::frequencies(meta, seq, day);
    let fixed_departures =
        !frequencies.is_empty() && frequencies.iter().all(|f| f.min_minutes.is_none());
    let (departures, source) = if fixed_departures {
        // Fixed clock departures (NR338): every headway row is one departure.
        (
            frequencies.iter().map(|f| f.start.clone()).collect(),
            "official-fixed",
        )
    } else if day == DayType::Weekday && !weekday_exact.is_empty() {
        (weekday_exact, "official-exact")
    } else {
        (Vec::new(), "official-frequency")
    };

    let first = departures
        .first()
        .cloned()
        .or_else(|| frequencies.first().map(|f| f.start.clone()));
    let last = departures
        .last()
        .cloned()
        .or_else(|| frequencies.last().and_then(|f| f.end.clone()));

    Schedule {
        route: code.to_string(),
        direction: seq,
        direction_en,
        direction_tc,
        day,
        source: source.to_string(),
        first,
        last,
        departures,
        frequencies,
        note: None,
    }
}

pub fn normalize_timestamp(raw: &str) -> Result<String> {
    let parsed =
        DateTime::parse_from_rfc3339(raw).with_context(|| format!("invalid timestamp {raw}"))?;
    Ok(parsed
        .with_timezone(&hkt())
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
}

fn normalize_hhmm(raw: &str) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(raw)
        .with_context(|| format!("invalid schedule timestamp {raw}"))?;
    Ok(parsed.with_timezone(&hkt()).format("%H:%M").to_string())
}

fn hhmm_from_seconds(raw: &str) -> String {
    let mut parts = raw.split(':');
    let hour = parts.next().and_then(|v| v.parse::<u32>().ok());
    let minute = parts.next().and_then(|v| v.parse::<u32>().ok());
    match (hour, minute) {
        (Some(hour), Some(minute)) => format!("{hour:02}:{minute:02}"),
        _ => raw.to_string(),
    }
}

fn matches_day(headway: &Headway, day: DayType) -> bool {
    let weekday = |index: usize| headway.weekdays.get(index).copied().unwrap_or(false);
    match day {
        DayType::Weekday => weekday(0),
        DayType::Saturday => weekday(5),
        DayType::Sunday => weekday(6),
        DayType::Holiday => headway.public_holiday,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteMeta {
    #[serde(rename = "route_id")]
    pub route_id: u64,
    #[serde(rename = "route_code")]
    pub route_code: String,
    #[serde(default)]
    pub description_en: Option<String>,
    #[serde(default)]
    pub directions: Vec<MetaDirection>,
}

impl RouteMeta {
    pub fn direction(&self, seq: u8) -> Option<&MetaDirection> {
        self.directions.iter().find(|d| d.route_seq == seq)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetaDirection {
    pub route_seq: u8,
    #[serde(default)]
    pub orig_en: String,
    #[serde(default)]
    pub orig_tc: String,
    #[serde(default)]
    pub dest_en: String,
    #[serde(default)]
    pub dest_tc: String,
    #[serde(default)]
    pub headways: Vec<Headway>,
}

impl MetaDirection {
    pub fn name_en(&self) -> String {
        format!("{} → {}", self.orig_en, self.dest_en)
    }

    pub fn name_tc(&self) -> String {
        format!("{} → {}", self.orig_tc, self.dest_tc)
    }

    pub fn info(&self) -> DirectionInfo {
        DirectionInfo {
            seq: self.route_seq,
            orig_en: self.orig_en.clone(),
            orig_tc: self.orig_tc.clone(),
            dest_en: self.dest_en.clone(),
            dest_tc: self.dest_tc.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Headway {
    #[serde(default)]
    pub weekdays: Vec<bool>,
    #[serde(default)]
    pub public_holiday: bool,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub frequency: Option<u32>,
    #[serde(default)]
    pub frequency_upper: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawStop {
    stop_seq: u16,
    stop_id: u64,
    #[serde(default)]
    name_en: String,
    #[serde(default)]
    name_tc: String,
}

#[derive(Debug, Deserialize)]
struct RawStopEta {
    #[serde(rename = "stopSeq")]
    stop_seq: u16,
    #[serde(default)]
    eta: Vec<RawEta>,
}

#[derive(Debug, Deserialize)]
struct RawEta {
    eta_seq: u32,
    #[serde(default)]
    diff: Option<i64>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    remarks_en: Option<String>,
    #[serde(default)]
    remarks_tc: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSchedule {
    route_seq: u8,
    #[serde(default)]
    schedules: Vec<String>,
}

pub fn fixed_offset() -> FixedOffset {
    hkt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_utc_schedule_to_hkt() {
        assert_eq!(
            normalize_hhmm("2026-11-20T22:00:00.000+00:00").unwrap(),
            "06:00"
        );
        assert_eq!(
            normalize_hhmm("2026-11-20T16:30:00.000+00:00").unwrap(),
            "00:30"
        );
    }

    #[test]
    fn day_matching_uses_the_operator_arrays() {
        let headway = Headway {
            weekdays: vec![true, true, true, true, true, false, false],
            public_holiday: false,
            start_time: "06:00:00".to_string(),
            end_time: Some("07:00:00".to_string()),
            frequency: Some(600),
            frequency_upper: None,
        };
        assert!(matches_day(&headway, DayType::Weekday));
        assert!(!matches_day(&headway, DayType::Saturday));
    }
}
