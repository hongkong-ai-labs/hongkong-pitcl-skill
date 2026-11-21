//! Unified access to the Ma Wan bus routes, across PITCL, KMB and timetables.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};

use crate::ferry::{self, FerryRoute, FerryTimetable};
use crate::kmb::KmbClient;
use crate::model::{DayType, Eta, RouteInfo, Schedule, Stop};
use crate::news::{NewsClient, NewsDetail, NewsItem};
use crate::pitcl::{self, Guide};
use crate::socif::{self, SocifClient};
use crate::timetable;
use crate::traffic::{TrafficClient, TrafficReport};
use crate::weather::{WeatherClient, WeatherReport};

pub struct Service {
    socif: Arc<SocifClient>,
    kmb: Arc<KmbClient>,
    news: Arc<NewsClient>,
    weather: Arc<WeatherClient>,
    traffic: Arc<TrafficClient>,
}

impl Service {
    pub fn new(cache_ttl: Duration) -> Result<Self> {
        Ok(Self {
            socif: Arc::new(SocifClient::new(cache_ttl)?),
            kmb: Arc::new(KmbClient::new(cache_ttl)?),
            news: Arc::new(NewsClient::new()?),
            weather: Arc::new(WeatherClient::new()?),
            traffic: Arc::new(TrafficClient::new()?),
        })
    }

    pub fn guide(&self, topic: &str) -> Result<Guide> {
        pitcl::guide(topic).ok_or_else(|| {
            anyhow!(
                "unknown guide topic '{topic}'; use {}",
                pitcl::TOPICS.join(", ")
            )
        })
    }

    pub async fn weather(&self, lang: &str) -> Result<WeatherReport> {
        self.weather.report(lang).await
    }

    pub async fn traffic(&self, lang: &str, all: bool, limit: usize) -> Result<TrafficReport> {
        self.traffic.report(lang, all, limit).await
    }

    pub fn ferry_routes(&self) -> Vec<FerryRoute> {
        ferry::routes()
    }

    pub fn ferry_schedule(&self, code: &str, day: DayType) -> Result<FerryTimetable> {
        ferry::schedule(code, day).ok_or_else(|| {
            anyhow!(
                "unknown ferry route '{code}'; known routes: {}",
                ferry::ROUTES.join(", ")
            )
        })
    }

    pub async fn news(&self, lang: &str, limit: usize) -> Result<Vec<NewsItem>> {
        self.news.list(lang, limit).await
    }

    pub async fn news_detail(&self, lang: &str, id: u32) -> Result<NewsDetail> {
        self.news.detail(lang, id).await
    }

    /// Static registry, no network needed.
    pub fn routes(&self) -> Vec<RouteInfo> {
        timetable::ROUTES
            .iter()
            .filter_map(|code| timetable::route(code))
            .collect()
    }

    pub fn route(&self, code: &str) -> Result<RouteInfo> {
        timetable::route(code).ok_or_else(|| {
            anyhow!(
                "unknown route '{code}'; known routes: {}",
                timetable::ROUTES.join(", ")
            )
        })
    }

    pub async fn stops(&self, code: &str, direction: u8) -> Result<Vec<Stop>> {
        let info = self.route(code)?;
        ensure_direction(&info, direction)?;
        if let Some(id) = socif::route_id(code) {
            return self.socif.stops(id, direction).await;
        }
        if code.eq_ignore_ascii_case("230R") {
            return self.kmb.stops("230R", direction).await;
        }
        Ok(static_stops(code, direction))
    }

    pub async fn etas(&self, code: &str, direction: u8) -> Result<Vec<Eta>> {
        let info = self.route(code)?;
        ensure_direction(&info, direction)?;
        if !info.realtime {
            bail!(
                "{} has no public real-time feed; use `mawan schedule {code}` for its timetable",
                info.code
            );
        }
        if let Some(id) = socif::route_id(code) {
            return self.socif.etas(id, direction).await;
        }
        if code.eq_ignore_ascii_case("230R") {
            return self.kmb.etas("230R", direction).await;
        }
        bail!("no ETA source configured for {code}")
    }

    pub async fn schedule(&self, code: &str, direction: u8, day: DayType) -> Result<Schedule> {
        let info = self.route(code)?;
        ensure_direction(&info, direction)?;
        if let Some(id) = socif::route_id(code) {
            let meta = self.socif.metadata(id).await?;
            let exact = if day == DayType::Weekday {
                self.socif
                    .weekday_departures(id, direction)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            return Ok(socif::build_schedule(
                &info.code, &meta, direction, day, exact,
            ));
        }
        timetable::schedule(&info.code, direction, day)
            .ok_or_else(|| anyhow!("no timetable configured for {code}"))
    }
}

pub fn parse_direction(value: Option<&str>) -> Result<u8> {
    match value.map(|v| v.trim().to_ascii_lowercase()) {
        None => Ok(1),
        Some(value) if value == "1" || value == "o" || value == "outbound" => Ok(1),
        Some(value) if value == "2" || value == "i" || value == "inbound" => Ok(2),
        Some(value) => Err(anyhow!(
            "direction must be 1/outbound or 2/inbound (got '{value}')"
        )),
    }
}

fn ensure_direction(info: &RouteInfo, direction: u8) -> Result<()> {
    if info.directions.iter().any(|d| d.seq == direction) {
        Ok(())
    } else {
        Err(anyhow!(
            "{} has no direction {}; use 1 or 2",
            info.code,
            direction
        ))
    }
}

pub fn limit_per_stop(etas: Vec<Eta>, limit: usize) -> Vec<Eta> {
    if limit == 0 {
        return etas;
    }
    let mut counts: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
    let mut out = Vec::new();
    for eta in etas {
        let count = counts.entry(eta.stop_seq).or_insert(0);
        if *count < limit {
            *count += 1;
            out.push(eta);
        }
    }
    out
}

/// Filter arrivals to one stop by sequence number or a case-insensitive name
/// fragment in English or Traditional Chinese.
pub fn filter_stop(mut etas: Vec<Eta>, filter: Option<&str>) -> Result<Vec<Eta>> {
    let Some(filter) = filter.map(str::trim).filter(|f| !f.is_empty()) else {
        return Ok(etas);
    };
    if let Ok(seq) = filter.parse::<u16>() {
        etas.retain(|eta| eta.stop_seq == seq);
        if etas.is_empty() {
            bail!("no stop with sequence {seq} in these arrivals");
        }
        return Ok(etas);
    }
    let needle = filter.to_lowercase();
    etas.retain(|eta| {
        eta.stop_en.to_lowercase().contains(&needle) || eta.stop_tc.to_lowercase().contains(&needle)
    });
    if etas.is_empty() {
        bail!("no stop matching '{filter}' in these arrivals");
    }
    Ok(etas)
}

fn stop(seq: u16, id: &str, en: &str, tc: &str) -> Stop {
    Stop {
        seq,
        id: id.to_string(),
        name_en: en.to_string(),
        name_tc: tc.to_string(),
    }
}

pub fn static_stops(code: &str, direction: u8) -> Vec<Stop> {
    if code.eq_ignore_ascii_case("NR338S") {
        return if direction == 1 {
            vec![
                stop(1, "NR338S-1", "Park Island", "珀麗灣"),
                stop(
                    2,
                    "NR338S-2",
                    "Ma Wan 1868 / Noah's Ark / Beach Commercial Complex",
                    "馬灣1868／挪亞方舟／臨海廣場",
                ),
                stop(3, "NR338S-3", "Kei Wai Primary School", "基慧小學"),
                stop(4, "NR338S-4", "Tin Liu New Village", "田寮新村"),
                stop(5, "NR338S-5", "MTR Hong Kong Station", "港鐵香港站"),
                stop(6, "NR338S-6", "Central (Pier 2)", "中環（二號碼頭）"),
            ]
        } else {
            vec![
                stop(1, "NR338S-6", "Central (Pier 2)", "中環（二號碼頭）"),
                stop(2, "NR338S-5", "MTR Hong Kong Station", "港鐵香港站"),
                stop(3, "NR338S-4", "Tin Liu New Village", "田寮新村"),
                stop(4, "NR338S-3", "Kei Wai Primary School", "基慧小學"),
                stop(5, "NR338S-1", "Park Island", "珀麗灣"),
            ]
        };
    }

    if code.eq_ignore_ascii_case("NR331") {
        return if direction == 1 {
            vec![
                stop(1, "NR331-1", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
                stop(
                    2,
                    "NR331-2",
                    "Ma Wan Rural Committee Road (South)",
                    "馬灣鄉事會路（南）",
                ),
                stop(
                    3,
                    "NR331-3",
                    "Ma Wan Rural Committee Road (North)",
                    "馬灣鄉事會路（北）",
                ),
                stop(4, "NR331-4", "Kei Wai Primary School", "基慧小學"),
                stop(5, "NR331-5", "Tsuen Wan Station", "荃灣站"),
            ]
        } else {
            vec![
                stop(1, "NR331-5", "Tsuen Wan Station", "荃灣站"),
                stop(2, "NR331-4", "Kei Wai Primary School", "基慧小學"),
                stop(3, "NR331-6", "Park Island Block 21", "珀麗灣第21座"),
                stop(
                    4,
                    "NR331-2",
                    "Ma Wan Rural Committee Road (South)",
                    "馬灣鄉事會路（南）",
                ),
                stop(5, "NR331-1", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
            ]
        };
    }

    if code.eq_ignore_ascii_case("NR331S") {
        return if direction == 1 {
            vec![
                stop(1, "NR331S-1", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
                stop(
                    2,
                    "NR331S-2",
                    "Ma Wan Rural Committee Road (South)",
                    "馬灣鄉事會路（南）",
                ),
                stop(
                    3,
                    "NR331S-3",
                    "Ma Wan Rural Committee Road (North)",
                    "馬灣鄉事會路（北）",
                ),
                stop(4, "NR331S-4", "Kei Wai Primary School", "基慧小學"),
                stop(5, "NR331S-5", "Tsuen Wan West Station", "荃灣西站"),
                stop(6, "NR331S-6", "Nina Tower", "如心廣場"),
            ]
        } else {
            vec![
                stop(1, "NR331S-6", "Nina Tower", "如心廣場"),
                stop(2, "NR331S-5", "Tsuen Wan West Station", "荃灣西站"),
                stop(3, "NR331S-4", "Kei Wai Primary School", "基慧小學"),
                stop(4, "NR331S-7", "Park Island Block 21", "珀麗灣第21座"),
                stop(
                    5,
                    "NR331S-2",
                    "Ma Wan Rural Committee Road (South)",
                    "馬灣鄉事會路（南）",
                ),
                stop(6, "NR331S-1", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
            ]
        };
    }

    Vec::new()
}
