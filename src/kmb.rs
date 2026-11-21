//! Client for the KMB/LWB open-data ETA API, which carries route 230R.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::Deserialize;
use serde_json::Value;

use crate::model::{Eta, Stop, hkt};

pub const BASE: &str = "https://data.etabus.gov.hk/v1/transport/kmb";
pub const USER_AGENT: &str = "mawan/0.1 (Ma Wan bus CLI; +https://github.com/)";

struct CacheEntry {
    at: Instant,
    body: String,
}

pub struct KmbClient {
    http: reqwest::Client,
    cache: Mutex<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl KmbClient {
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
                "KMB API returned HTTP {status} for {path}: {}",
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

    pub async fn stops(&self, route: &str, direction: u8) -> Result<Vec<Stop>> {
        let bound = bound_name(direction)?;
        let value = self.get(&format!("/route-stop/{route}/{bound}/1")).await?;
        let rows: Vec<RawRouteStop> = serde_json::from_value(
            value
                .get("data")
                .cloned()
                .unwrap_or_else(|| Value::Array(vec![])),
        )
        .context("KMB route-stop: unexpected response")?;
        if rows.is_empty() {
            return Err(anyhow!(
                "KMB has no stop data for route {route} ({bound}); it may not be in the open-data feed"
            ));
        }

        let lookups = rows.iter().map(|row| {
            let stop = row.stop.clone();
            async move {
                let value = self.get(&format!("/stop/{stop}")).await?;
                let data = value
                    .get("data")
                    .ok_or_else(|| anyhow!("stop {stop}: missing data"))?;
                let name_en = data
                    .get("name_en")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let name_tc = data
                    .get("name_tc")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                Ok::<_, anyhow::Error>((stop, name_en, name_tc))
            }
        });
        let names: HashMap<String, (String, String)> = join_all(lookups)
            .await
            .into_iter()
            .filter_map(|result| result.ok())
            .map(|(id, en, tc)| (id, (en, tc)))
            .collect();

        Ok(rows
            .into_iter()
            .map(|row| {
                let seq = row.seq.parse::<u16>().unwrap_or_default();
                let (name_en, name_tc) = names
                    .get(&row.stop)
                    .cloned()
                    .unwrap_or_else(|| (row.stop.clone(), row.stop.clone()));
                Stop {
                    seq,
                    id: row.stop,
                    name_en,
                    name_tc,
                }
            })
            .collect())
    }

    pub async fn etas(&self, route: &str, direction: u8) -> Result<Vec<Eta>> {
        let stops = self.stops(route, direction).await?;
        let expected = if direction == 1 { "O" } else { "I" };
        let lookups = stops.iter().map(|stop| {
            let stop = stop.clone();
            async move {
                let path = format!("/eta/{}/{route}/1", stop.id);
                let value = self.get(&path).await?;
                let rows: Vec<RawEta> = serde_json::from_value(
                    value
                        .get("data")
                        .cloned()
                        .unwrap_or_else(|| Value::Array(vec![])),
                )
                .with_context(|| format!("ETA {path}: unexpected response"))?;
                Ok::<_, anyhow::Error>((stop, rows))
            }
        });
        let results = join_all(lookups).await;

        let now = Utc::now();
        let mut out = Vec::new();
        for result in results {
            let (stop, rows) = result?;
            for row in rows {
                if row.dir.as_deref() != Some(expected) {
                    continue;
                }
                // After the last bus, KMB still emits one row per stop with a
                // null ETA. Those are not upcoming arrivals.
                if row.eta.is_none() {
                    continue;
                }
                let at = row
                    .eta
                    .as_deref()
                    .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                    .map(|value| {
                        value
                            .with_timezone(&hkt())
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
                    });
                let minutes = row
                    .eta
                    .as_deref()
                    .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                    .map(|value| {
                        let seconds = (value.with_timezone(&Utc) - now).num_seconds();
                        (seconds as f64 / 60.0).round() as i64
                    });
                let remark_en = row.rmk_en.unwrap_or_default();
                let remark_tc = row.rmk_tc.unwrap_or_default();
                out.push(Eta {
                    stop_seq: stop.seq,
                    stop_id: stop.id.clone(),
                    stop_en: stop.name_en.clone(),
                    stop_tc: stop.name_tc.clone(),
                    direction,
                    eta_seq: row.eta_seq,
                    minutes,
                    at,
                    remark_en: remark_en.clone(),
                    remark_tc,
                    realtime: row.eta.is_some(),
                });
            }
        }
        out.sort_by_key(|eta| {
            (
                eta.stop_seq,
                eta.at.clone().unwrap_or_default(),
                eta.eta_seq,
            )
        });
        Ok(out)
    }
}

pub fn bound_name(direction: u8) -> Result<&'static str> {
    match direction {
        1 => Ok("outbound"),
        2 => Ok("inbound"),
        other => Err(anyhow!("direction must be 1 or 2 (got {other})")),
    }
}

#[derive(Debug, Deserialize)]
struct RawRouteStop {
    seq: String,
    stop: String,
}

#[derive(Debug, Deserialize)]
struct RawEta {
    dir: Option<String>,
    eta_seq: u32,
    eta: Option<String>,
    #[serde(default)]
    rmk_en: Option<String>,
    #[serde(default)]
    rmk_tc: Option<String>,
}
