//! Next departures from a chosen Ma Wan origin, combining live bus ETAs,
//! published timetables and ferry sailings.

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::model::{DayType, Eta, hkt};
use crate::service::Service;

pub const ORIGINS: &[&str] = &[
    "park-island",
    "ma-wan",
    "central",
    "tsuen-wan",
    "tsuen-wan-west",
    "tsing-yi",
    "kwai-fong",
    "airport",
    "kowloon-station",
];

#[derive(Debug, Clone, Serialize)]
pub struct Departure {
    pub mode: String,
    pub service: String,
    pub destination_en: String,
    pub destination_tc: String,
    pub time: Option<String>,
    pub at: Option<String>,
    pub minutes: Option<i64>,
    /// `live`, `scheduled` or `timetable`.
    pub source: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NextReport {
    pub origin: String,
    pub origin_en: String,
    pub origin_tc: String,
    pub day: DayType,
    pub date: String,
    pub generated_at: String,
    pub departures: Vec<Departure>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextTime {
    pub time: String,
    pub at: String,
    pub minutes: i64,
}

fn canonical(origin: &str) -> Option<&'static str> {
    let normalised = origin.trim().to_ascii_lowercase().replace('_', "-");
    let normalised = normalised.replace(' ', "-");
    ORIGINS.iter().copied().find(|candidate| {
        *candidate == normalised
            || match *candidate {
                "park-island" => normalised == "pi" || normalised == "park",
                "ma-wan" => normalised == "pak-yan-road",
                "central" => normalised == "central-pier" || normalised == "pier-2",
                "tsuen-wan" => normalised == "tw",
                "tsuen-wan-west" => normalised == "nina-tower",
                "tsing-yi" => normalised == "tsing-yi-station",
                "kwai-fong" => normalised == "kwai-fong-station",
                "airport" => normalised == "hkia",
                "kowloon-station" => normalised == "kowloon" || normalised == "elements",
                _ => false,
            }
    })
}

fn origin_names(origin: &'static str) -> (&'static str, &'static str) {
    match origin {
        "park-island" => ("Park Island", "珀麗灣"),
        "ma-wan" => ("Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
        "central" => ("Central", "中環"),
        "tsuen-wan" => ("Tsuen Wan", "荃灣"),
        "tsuen-wan-west" => ("Tsuen Wan West", "荃灣西"),
        "tsing-yi" => ("MTR Tsing Yi Station", "港鐵青衣站"),
        "kwai-fong" => ("MTR Kwai Fong Station", "港鐵葵芳站"),
        "airport" => ("Hong Kong International Airport", "香港國際機場"),
        "kowloon-station" => ("Kowloon Station", "九龍站"),
        _ => (origin, origin),
    }
}

pub async fn gather(
    service: &Service,
    origin: &str,
    day: DayType,
    date: NaiveDate,
    limit: usize,
    _lang: &str,
) -> Result<NextReport> {
    let origin = canonical(origin)
        .ok_or_else(|| anyhow!("unknown origin '{origin}'; use {}", ORIGINS.join(", ")))?;
    let limit = limit.max(1);
    let now = Utc::now().with_timezone(&hkt());
    let today = date == now.date_naive();
    let mut departures = Vec::new();
    let mut notes = Vec::new();

    match origin {
        "park-island" => {
            add_ferry(
                service,
                "central",
                true,
                day,
                date,
                now,
                limit,
                &mut departures,
                &mut notes,
            )?;
            add_ferry(
                service,
                "tsuen-wan",
                true,
                day,
                date,
                now,
                limit,
                &mut departures,
                &mut notes,
            )?;
            for route in ["NR330", "NR332", "NR334", "NR338"] {
                add_live_or_schedule(
                    service,
                    route,
                    1,
                    day,
                    date,
                    today,
                    now,
                    limit,
                    &mut departures,
                    &mut notes,
                )
                .await?;
            }
        }
        "ma-wan" => {
            add_live_or_schedule(
                service,
                "230R",
                2,
                day,
                date,
                today,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
            for route in ["NR331", "NR331S", "NR338S"] {
                add_schedule(
                    service,
                    route,
                    1,
                    day,
                    date,
                    now,
                    limit,
                    &mut departures,
                    &mut notes,
                )
                .await?;
            }
        }
        "central" => {
            add_ferry(
                service,
                "central",
                false,
                day,
                date,
                now,
                limit,
                &mut departures,
                &mut notes,
            )?;
            add_live_or_schedule(
                service,
                "NR338",
                2,
                day,
                date,
                today,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
            add_schedule(
                service,
                "NR338S",
                2,
                day,
                date,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
        }
        "tsuen-wan" => {
            add_ferry(
                service,
                "tsuen-wan",
                false,
                day,
                date,
                now,
                limit,
                &mut departures,
                &mut notes,
            )?;
            notes.push(
                "NR331's return leg from Tsuen Wan is frequency-based (every 20-30 min); see `mawan schedule NR331 -d 2`."
                    .into(),
            );
        }
        "tsuen-wan-west" => {
            notes.push(
                "NR331S's return leg from Tsuen Wan West is frequency-based (every 30-60 min); see `mawan schedule NR331S -d 2`."
                    .into(),
            );
        }
        "tsing-yi" => {
            add_live_or_schedule(
                service,
                "NR330",
                2,
                day,
                date,
                today,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
        }
        "kwai-fong" => {
            add_live_or_schedule(
                service,
                "NR332",
                2,
                day,
                date,
                today,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
        }
        "airport" => {
            add_live_or_schedule(
                service,
                "NR334",
                2,
                day,
                date,
                today,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
        }
        "kowloon-station" => {
            add_live_or_schedule(
                service,
                "230R",
                1,
                day,
                date,
                today,
                now,
                limit,
                &mut departures,
                &mut notes,
            )
            .await?;
        }
        _ => bail!("origin {origin} is not configured"),
    }

    departures.sort_by_key(|departure| departure.at.clone().unwrap_or_default());
    let (origin_en, origin_tc) = origin_names(origin);
    Ok(NextReport {
        origin: origin.into(),
        origin_en: origin_en.into(),
        origin_tc: origin_tc.into(),
        day,
        date: date.format("%Y-%m-%d").to_string(),
        generated_at: now.to_rfc3339_opts(SecondsFormat::Secs, false),
        departures,
        notes,
    })
}

#[allow(clippy::too_many_arguments)]
async fn add_live_or_schedule(
    service: &Service,
    route: &str,
    direction: u8,
    day: DayType,
    date: NaiveDate,
    today: bool,
    now: DateTime<FixedOffset>,
    limit: usize,
    departures: &mut Vec<Departure>,
    notes: &mut Vec<String>,
) -> Result<()> {
    if today {
        add_live(service, route, direction, now, limit, departures, notes).await
    } else {
        add_schedule(
            service, route, direction, day, date, now, limit, departures, notes,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
async fn add_live(
    service: &Service,
    route: &str,
    direction: u8,
    now: DateTime<FixedOffset>,
    limit: usize,
    departures: &mut Vec<Departure>,
    notes: &mut Vec<String>,
) -> Result<()> {
    let info = service.route(route)?;
    let destination = info
        .directions
        .iter()
        .find(|candidate| candidate.seq == direction);
    let (destination_en, destination_tc) = destination
        .map(|direction| (direction.dest_en.clone(), direction.dest_tc.clone()))
        .unwrap_or_default();
    match service.etas(route, direction).await {
        Ok(etas) => {
            let Some(first) = etas.iter().map(|eta| eta.stop_seq).min() else {
                notes.push(format!("{route}: no upcoming arrivals reported."));
                return Ok(());
            };
            let mut rows: Vec<&Eta> = etas.iter().filter(|eta| eta.stop_seq == first).collect();
            rows.sort_by_key(|eta| (eta.at.clone().unwrap_or_default(), eta.eta_seq));
            for eta in rows.into_iter().take(limit) {
                departures.push(from_eta(
                    &info.code,
                    eta,
                    &destination_en,
                    &destination_tc,
                    now,
                ));
            }
        }
        Err(error) => notes.push(format!("{route}: {error}")),
    }
    Ok(())
}

fn from_eta(
    route: &str,
    eta: &Eta,
    destination_en: &str,
    destination_tc: &str,
    now: DateTime<FixedOffset>,
) -> Departure {
    let at = eta.at.clone();
    let time = at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&hkt()).format("%H:%M").to_string());
    let minutes = eta.minutes.or_else(|| {
        at.as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| (value.with_timezone(&hkt()) - now).num_minutes())
    });
    Departure {
        mode: "bus".into(),
        service: route.into(),
        destination_en: destination_en.into(),
        destination_tc: destination_tc.into(),
        time,
        at,
        minutes,
        source: if eta.realtime { "live" } else { "scheduled" }.into(),
        note: Some(eta.remark_en.clone()).filter(|remark| !remark.is_empty()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn add_schedule(
    service: &Service,
    route: &str,
    direction: u8,
    day: DayType,
    date: NaiveDate,
    now: DateTime<FixedOffset>,
    limit: usize,
    departures: &mut Vec<Departure>,
    notes: &mut Vec<String>,
) -> Result<()> {
    let info = service.route(route)?;
    let destination = info
        .directions
        .iter()
        .find(|candidate| candidate.seq == direction);
    let (destination_en, destination_tc) = destination
        .map(|direction| (direction.dest_en.clone(), direction.dest_tc.clone()))
        .unwrap_or_default();
    let schedule = service.schedule(route, direction, day).await?;
    if schedule.departures.is_empty() {
        notes.push(format!(
            "{route}: frequency-based service; see `mawan schedule {route} -d {direction}` for the windows."
        ));
        return Ok(());
    }
    let filter = date == now.date_naive();
    for next in next_times(&schedule.departures, date, now, limit, filter) {
        departures.push(Departure {
            mode: "bus".into(),
            service: route.into(),
            destination_en: destination_en.clone(),
            destination_tc: destination_tc.clone(),
            time: Some(next.time),
            at: Some(next.at),
            minutes: Some(next.minutes),
            source: "timetable".into(),
            note: schedule.note.clone(),
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_ferry(
    service: &Service,
    route: &str,
    from_park_island: bool,
    day: DayType,
    date: NaiveDate,
    now: DateTime<FixedOffset>,
    limit: usize,
    departures: &mut Vec<Departure>,
    notes: &mut Vec<String>,
) -> Result<()> {
    let table = service.ferry_schedule(route, day)?;
    let times = if from_park_island {
        &table.from_park_island
    } else {
        &table.from_other_pier
    };
    let (destination_en, destination_tc) = if from_park_island {
        (table.other_pier_en.clone(), table.other_pier_tc.clone())
    } else {
        ("Park Island".to_string(), "珀麗灣".to_string())
    };
    let filter = date == now.date_naive();
    let next = next_times(times, date, now, limit, filter);
    if next.is_empty() {
        notes.push(format!(
            "{}: no more sailings today; see the timetable for tomorrow.",
            table.route
        ));
    }
    for next in next {
        departures.push(Departure {
            mode: "ferry".into(),
            service: table.route.clone(),
            destination_en: destination_en.clone(),
            destination_tc: destination_tc.clone(),
            time: Some(next.time),
            at: Some(next.at),
            minutes: Some(next.minutes),
            source: "timetable".into(),
            note: Some(format!(
                "About {} min crossing; times are pier gate-closing times.",
                table.journey_minutes
            )),
        });
    }
    Ok(())
}

/// Pick the next clock times from a published HH:MM list. When `filter` is
/// true, times already passed are skipped; after-midnight times are rolled to
/// the next day when they are more than six hours behind the current time.
pub fn next_times(
    times: &[String],
    date: NaiveDate,
    now: DateTime<FixedOffset>,
    limit: usize,
    filter: bool,
) -> Vec<NextTime> {
    let mut out = Vec::new();
    for time in times {
        let Ok(naive) = NaiveTime::parse_from_str(time, "%H:%M") else {
            continue;
        };
        let Some(mut candidate) = date.and_time(naive).and_local_timezone(hkt()).single() else {
            continue;
        };
        if filter && candidate < now {
            let behind = now - candidate;
            if behind.num_hours() >= 6 {
                candidate += Duration::days(1);
            } else {
                continue;
            }
        }
        if filter && candidate < now {
            continue;
        }
        out.push(NextTime {
            time: time.clone(),
            at: candidate.to_rfc3339_opts(SecondsFormat::Secs, false),
            minutes: (candidate - now).num_minutes(),
        });
        if out.len() == limit {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn skips_past_ferries_and_rolls_after_midnight_times() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let now = hkt().with_ymd_and_hms(2026, 11, 22, 23, 50, 0).unwrap();
        let times = vec!["20:00".into(), "00:00".into(), "06:00".into()];
        let next = next_times(&times, date, now, 2, true);
        assert_eq!(next[0].time, "00:00");
        assert_eq!(next[0].minutes, 10);
        assert_eq!(next[1].time, "06:00");
    }

    #[test]
    fn keeps_early_morning_times_that_just_passed() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let now = hkt().with_ymd_and_hms(2026, 11, 22, 0, 10, 0).unwrap();
        let times = vec!["00:00".into(), "00:30".into()];
        let next = next_times(&times, date, now, 2, true);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].time, "00:30");
    }

    #[test]
    fn future_date_returns_the_first_sailings() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        let now = hkt().with_ymd_and_hms(2026, 11, 22, 23, 50, 0).unwrap();
        let times = vec!["06:30".into(), "07:00".into()];
        let next = next_times(&times, date, now, 2, false);
        assert_eq!(next[0].time, "06:30");
        assert_eq!(next[0].minutes, 400);
    }
}
