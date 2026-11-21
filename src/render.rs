use chrono::DateTime;

use crate::ferry::{FerryFare, FerryRoute, FerryTimetable};
use crate::model::{Eta, Fare, RouteInfo, Schedule, Stop, hkt};
use crate::news::{NewsDetail, NewsItem};
use crate::pitcl::Guide;
use crate::traffic::TrafficReport;
use crate::weather::WeatherReport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Tc,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Tc => "tc",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" => Some(Lang::En),
            "tc" | "zh" | "zh-hk" | "zh_hk" | "chinese" => Some(Lang::Tc),
            _ => None,
        }
    }
}

fn pick<'a>(lang: Lang, en: &'a str, tc: &'a str) -> &'a str {
    match lang {
        Lang::En => en,
        Lang::Tc => tc,
    }
}

fn width(text: &str) -> usize {
    text.chars().count()
}

fn pad(text: &str, target: usize) -> String {
    let mut out = text.to_string();
    let len = width(text);
    if len < target {
        out.push_str(&" ".repeat(target - len));
    }
    out
}

fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|h| width(h)).collect();
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            if index < widths.len() {
                widths[index] = widths[index].max(width(cell));
            }
        }
    }
    let mut out = String::new();
    out.push_str(
        &headers
            .iter()
            .enumerate()
            .map(|(index, header)| pad(header, widths[index]))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    out.push_str(
        &widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    for row in rows {
        out.push_str(
            &row.iter()
                .enumerate()
                .map(|(index, cell)| pad(cell, widths[index]))
                .collect::<Vec<_>>()
                .join("  "),
        );
        out.push('\n');
    }
    out
}

pub fn routes_table(routes: &[RouteInfo], lang: Lang) -> String {
    let rows = routes
        .iter()
        .map(|route| {
            vec![
                route.code.clone(),
                route.operator.clone(),
                match lang {
                    Lang::En => route.name_en.clone(),
                    Lang::Tc => route.name_tc.clone(),
                },
                if route.realtime { "live" } else { "timetable" }.to_string(),
            ]
        })
        .collect();
    table(&["ROUTE", "OPERATOR", "SERVICE", "DATA"], rows)
}

pub fn stops_table(stops: &[Stop], lang: Lang) -> String {
    let rows = stops
        .iter()
        .map(|stop| {
            vec![
                stop.seq.to_string(),
                stop.id.clone(),
                pick(lang, &stop.name_en, &stop.name_tc).to_string(),
            ]
        })
        .collect();
    table(&["SEQ", "STOP ID", "NAME"], rows)
}

pub fn etas_table(etas: &[Eta], lang: Lang) -> String {
    let rows = etas
        .iter()
        .map(|eta| {
            let when = eta
                .at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&hkt()).format("%H:%M").to_string())
                .unwrap_or_else(|| "-".to_string());
            let minutes = match eta.minutes {
                Some(value) if value <= 0 => "due".to_string(),
                Some(value) => format!("{value} min"),
                None => "-".to_string(),
            };
            let remark = match lang {
                Lang::En => eta.remark_en.clone(),
                Lang::Tc => eta.remark_tc.clone(),
            };
            let base = if eta.realtime { "live" } else { "schedule" };
            let source = if remark.is_empty() || remark.eq_ignore_ascii_case("scheduled") {
                base.to_string()
            } else {
                format!("{base} ({remark})")
            };
            vec![
                eta.stop_seq.to_string(),
                pick(lang, &eta.stop_en, &eta.stop_tc).to_string(),
                when,
                minutes,
                source,
            ]
        })
        .collect();
    table(&["SEQ", "STOP", "ETA", "IN", "SOURCE"], rows)
}

pub fn fare_summary(fare: &Fare) -> String {
    let mut parts = Vec::new();
    if let Some(part) = fare_pair("Adult", fare.adult_visitor, fare.adult_resident) {
        parts.push(part);
    }
    if let Some(part) = fare_pair("Child", fare.child_visitor, fare.child_resident) {
        parts.push(part);
    }
    if let Some(part) = fare_pair("Elderly", fare.elderly_visitor, fare.elderly_resident) {
        parts.push(part);
    }
    parts.join(" · ")
}

fn fare_pair(label: &str, visitor: Option<f64>, resident: Option<f64>) -> Option<String> {
    match (visitor, resident) {
        (Some(visitor), Some(resident)) => Some(format!(
            "{label}: HK${visitor:.2} visitor / HK${resident:.2} resident"
        )),
        (Some(visitor), None) => Some(format!("{label}: HK${visitor:.2}")),
        (None, Some(resident)) => Some(format!("{label}: HK${resident:.2} resident")),
        (None, None) => None,
    }
}

fn fare_cell(visitor: f64, resident: f64) -> String {
    format!("${visitor:.2} / ${resident:.2}")
}

pub fn ferry_fare_summary(fare: &FerryFare) -> String {
    [
        fare_pair("Adult", Some(fare.adult_visitor), Some(fare.adult_resident)),
        fare_pair("Child", Some(fare.child_visitor), Some(fare.child_resident)),
        fare_pair(
            "Elderly",
            Some(fare.elderly_visitor),
            Some(fare.elderly_resident),
        ),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

pub fn ferry_routes_table(routes: &[FerryRoute], lang: Lang) -> String {
    let rows = routes
        .iter()
        .map(|route| {
            vec![
                route.code.clone(),
                pick(lang, &route.name_en, &route.name_tc).to_string(),
                pick(lang, &route.other_pier_en, &route.other_pier_tc).to_string(),
                fare_cell(route.fare.adult_visitor, route.fare.adult_resident),
                fare_cell(route.fare.child_visitor, route.fare.child_resident),
                fare_cell(route.fare.elderly_visitor, route.fare.elderly_resident),
            ]
        })
        .collect();
    table(
        &[
            "ROUTE",
            "SERVICE",
            "OTHER PIER",
            "ADULT V/R",
            "CHILD V/R",
            "ELDERLY V/R",
        ],
        rows,
    )
}

pub fn ferry_schedule_text(table: &FerryTimetable, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}  {}  [{}]\n",
        table.route,
        pick(lang, &table.route_en, &table.route_tc),
        table.day.label()
    ));
    out.push_str(&format!(
        "\n{}\n",
        pick(lang, "From Park Island Pier", "珀麗灣碼頭開")
    ));
    push_times(&mut out, &table.from_park_island);
    let other = match lang {
        Lang::En => format!("From {}", table.other_pier_en),
        Lang::Tc => format!("{}開", table.other_pier_tc),
    };
    out.push_str(&format!("\n{other}\n"));
    push_times(&mut out, &table.from_other_pier);
    out.push_str(&format!("\nFare: {}\n", ferry_fare_summary(&table.fare)));
    if !table.fare.note.is_empty() {
        out.push_str(&format!("{}\n", table.fare.note));
    }
    out.push_str(&format!("\n{}\n", table.note));
    out
}

fn push_times(out: &mut String, times: &[String]) {
    for chunk in times.chunks(8) {
        out.push_str(&chunk.join("  "));
        out.push('\n');
    }
}

pub fn news_list_text(items: &[NewsItem], lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(match lang {
        Lang::En => "PITCL notices (newest first)\n",
        Lang::Tc => "珀麗灣客運最新通告（最新在前）\n",
    });
    for (index, item) in items.iter().enumerate() {
        out.push_str(&format!(
            "\n{:>2}. {}\n    {}\n",
            index + 1,
            item.title,
            item.url
        ));
    }
    out
}

pub fn weather_text(report: &WeatherReport, lang: Lang) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}  {}\n",
        pick(lang, "Hong Kong Observatory weather", "香港天文台天氣"),
        report.updated
    ));

    if !report.temperature.is_empty() {
        out.push_str(&format!("\n{}\n", pick(lang, "Temperature", "氣溫")));
        let rows = report
            .temperature
            .iter()
            .map(|observation| {
                vec![
                    observation.place.clone(),
                    format!("{:.0} {}", observation.value, observation.unit),
                ]
            })
            .collect();
        out.push_str(&table(&["PLACE", "TEMP"], rows));
    }

    if !report.humidity.is_empty() {
        out.push_str(&format!("\n{}\n", pick(lang, "Humidity", "濕度")));
        let rows = report
            .humidity
            .iter()
            .map(|observation| {
                vec![
                    observation.place.clone(),
                    format!("{:.0} {}", observation.value, observation.unit),
                ]
            })
            .collect();
        out.push_str(&table(&["PLACE", "VALUE"], rows));
    }

    if !report.rainfall.is_empty() {
        out.push_str(&format!("\n{}\n", pick(lang, "Rainfall", "雨量")));
        let rows = report
            .rainfall
            .iter()
            .map(|rainfall| vec![rainfall.place.clone(), format!("{:.0} mm", rainfall.max_mm)])
            .collect();
        out.push_str(&table(&["PLACE", "MAX"], rows));
    }

    if let Some(uv) = &report.uv {
        out.push_str(&format!(
            "\n{}: {} {:.1} ({})\n",
            pick(lang, "UV index", "紫外線指數"),
            uv.place,
            uv.value,
            uv.description
        ));
    }

    out.push_str(&format!("\n{}\n", pick(lang, "Warnings", "警告")));
    if report.warnings.is_empty() {
        out.push_str(&format!(
            "{}\n",
            pick(lang, "No active warnings.", "現時沒有生效警告。")
        ));
    } else {
        for warning in &report.warnings {
            let kind = if warning.kind.is_empty() {
                String::new()
            } else {
                format!(" ({})", warning.kind)
            };
            out.push_str(&format!(
                "- {}{} — {} {}\n",
                warning.name, kind, warning.action, warning.updated
            ));
        }
    }

    out.push_str(&format!(
        "\n{}\n",
        pick(lang, "PITCL impact", "對珀麗灣客運的影響")
    ));
    out.push_str(&format!(
        "{}: {}\n{}\n{}: {}\n{}: {}\n",
        pick(lang, "Level", "程度"),
        report.impact.level,
        report.impact.summary,
        pick(lang, "Bus", "巴士"),
        report.impact.bus,
        pick(lang, "Ferry", "渡輪"),
        report.impact.ferry
    ));

    if let Some(forecast) = &report.forecast {
        out.push_str(&format!("\n{}\n", forecast.period));
        out.push_str(&format!("{}\n", forecast.description));
        if !forecast.outlook.is_empty() {
            out.push_str(&format!("Outlook: {}\n", forecast.outlook));
        }
    }
    out
}

pub fn traffic_text(report: &TrafficReport, lang: Lang) -> String {
    let mut out = String::new();
    let scope = if report.all {
        pick(lang, "all messages", "所有消息")
    } else {
        pick(lang, "Ma Wan-related", "與馬灣相關")
    };
    out.push_str(&format!(
        "{} — {} ({} {}/{})\n",
        pick(lang, "TD special traffic news", "運輸署特別交通消息"),
        scope,
        report.alerts.len(),
        pick(lang, "shown", "顯示"),
        report.total
    ));
    if report.alerts.is_empty() {
        out.push_str(&format!(
            "\n{}\n",
            if report.all {
                pick(lang, "No special traffic news.", "現時沒有特別交通消息。")
            } else {
                pick(
                    lang,
                    "No Ma Wan-related special traffic news.",
                    "現時沒有與馬灣相關的特別交通消息。",
                )
            }
        ));
        return out;
    }
    for (index, alert) in report.alerts.iter().enumerate() {
        let headline = alert
            .headline
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "\n{:>2}. [{}] {}\n",
            index + 1,
            alert.status,
            preview(&headline, 180)
        ));
        if alert.detail != alert.headline {
            let detail = preview(&alert.detail, 500);
            for line in detail.lines() {
                if !line.trim().is_empty() {
                    out.push_str(&format!("    {}\n", line.trim()));
                }
            }
            if detail.ends_with('…') {
                out.push_str("    (use --json for the full message)\n");
            }
        }
        if !alert.districts.is_empty() {
            out.push_str(&format!(
                "    {}: {}\n",
                pick(lang, "Districts", "地區"),
                alert.districts.join(", ")
            ));
        }
        out.push_str(&format!(
            "    {}: {}\n",
            pick(lang, "Reference", "參考時間"),
            alert.reference
        ));
    }
    out
}

fn preview(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push('…');
    out
}

pub fn guide_text(guide: &Guide) -> String {
    let mut out = format!(
        "{}\nSource: {}\n\n{}\n",
        guide.title, guide.source, guide.note
    );
    for section in &guide.sections {
        out.push_str(&format!("\n{}\n", section.heading));
        for item in &section.items {
            out.push_str(&format!("- {item}\n"));
        }
    }
    out
}

pub fn news_detail_text(detail: &NewsDetail) -> String {
    let mut out = format!("{}\n{}\n", detail.title, detail.url);
    if !detail.content.is_empty() {
        out.push_str(&format!("\n{}\n", detail.content));
    }
    out
}

pub fn schedule_text(schedule: &Schedule, lang: Lang) -> String {
    let mut out = String::new();
    let direction = pick(lang, &schedule.direction_en, &schedule.direction_tc);
    out.push_str(&format!(
        "{}  dir {}  {}  [{} / {}]\n",
        schedule.route,
        schedule.direction,
        direction,
        schedule.day.label(),
        schedule.source
    ));
    if let Some(first) = &schedule.first {
        out.push_str(&format!("First {}", first));
        if let Some(last) = &schedule.last {
            out.push_str(&format!("  ·  Last {last}"));
        }
        out.push('\n');
    }
    out.push('\n');

    if !schedule.departures.is_empty() {
        for chunk in schedule.departures.chunks(8) {
            out.push_str(&chunk.join("  "));
            out.push('\n');
        }
    } else {
        for frequency in &schedule.frequencies {
            let end = frequency.end.as_deref().unwrap_or("-");
            let span = match (frequency.min_minutes, frequency.max_minutes) {
                (Some(min), Some(max)) if min == max => format!("every {min} min"),
                (Some(min), Some(max)) => format!("every {min}-{max} min"),
                _ => "fixed departure".to_string(),
            };
            out.push_str(&format!("{}–{}  {}\n", frequency.start, end, span));
        }
    }
    if let Some(note) = &schedule.note {
        out.push_str(&format!("\n{note}\n"));
    }
    out
}
