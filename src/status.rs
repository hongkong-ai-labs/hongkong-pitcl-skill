//! One-call Ma Wan status: weather impact, relevant traffic and PITCL notices.

use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::model::hkt;
use crate::news::NewsItem;
use crate::service::Service;
use crate::traffic::TrafficReport;
use crate::weather::WeatherReport;

#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub generated_at: String,
    pub level: String,
    pub summary: String,
    pub bus: String,
    pub ferry: String,
    pub weather: Option<WeatherReport>,
    pub traffic: Option<TrafficReport>,
    pub notices: Vec<NewsItem>,
    /// Individual feeds that could not be fetched.
    pub errors: Vec<String>,
    pub caveats: Vec<String>,
}

fn pick<'a>(lang: &str, en: &'a str, tc: &'a str) -> &'a str {
    if lang == "tc" { tc } else { en }
}

pub async fn gather(service: &Service, lang: &str) -> StatusReport {
    let (weather, traffic, notices) = tokio::join!(
        service.weather(lang),
        service.traffic(lang, false, 0),
        service.news(lang, 5)
    );

    let mut errors = Vec::new();
    let mut weather = match weather {
        Ok(report) => Some(report),
        Err(error) => {
            errors.push(format!("weather: {error:#}"));
            None
        }
    };
    let traffic = match traffic {
        Ok(report) => Some(report),
        Err(error) => {
            errors.push(format!("traffic: {error:#}"));
            None
        }
    };
    let notices = match notices {
        Ok(items) => items,
        Err(error) => {
            errors.push(format!("notices: {error:#}"));
            Vec::new()
        }
    };

    if let (Some(weather), Some(traffic)) = (weather.as_mut(), traffic.as_ref()) {
        crate::weather::apply_traffic_bridge_stage(weather, &traffic.alerts, lang);
    }

    let (level, summary, bus, ferry) = match weather.as_ref() {
        Some(report) => (
            report.impact.level.clone(),
            report.impact.summary.clone(),
            report.impact.bus.clone(),
            report.impact.ferry.clone(),
        ),
        None => (
            "unknown".into(),
            pick(
                lang,
                "Weather impact is unavailable; check HKO warnings and PITCL notices directly.",
                "未能取得天氣影響資料；請直接查看天文台警告及珀麗灣客運通告。",
            )
            .into(),
            pick(
                lang,
                "Unknown; check PITCL notices.",
                "未知；請查看珀麗灣客運通告。",
            )
            .into(),
            pick(
                lang,
                "Unknown; check PITCL notices.",
                "未知；請查看珀麗灣客運通告。",
            )
            .into(),
        ),
    };

    let mut caveats = vec![
        pick(
            lang,
            "There is no public PITCL real-time service-status or ferry ETA feed. This combines HKO warnings, TD traffic news and PITCL notices.",
            "珀麗灣客運沒有公開的實時服務狀態或渡輪預計到站資料。本狀態綜合天文台警告、運輸署交通消息及珀麗灣客運通告。",
        )
        .into(),
        pick(
            lang,
            "A warning or traffic incident is not confirmation that a specific sailing or bus is cancelled.",
            "警告或交通事故並不代表某一班渡輪或巴士必然取消。",
        )
        .into(),
    ];
    if let Some(report) = weather.as_ref()
        && !report.bridge_wind.available
    {
        caveats.push(report.bridge_wind.note.clone());
    }

    StatusReport {
        generated_at: Utc::now()
            .with_timezone(&hkt())
            .to_rfc3339_opts(SecondsFormat::Secs, false),
        level,
        summary,
        bus,
        ferry,
        weather,
        traffic,
        notices,
        errors,
        caveats,
    }
}
