//! Timetables that the live feeds do not carry.
//!
//! Sources:
//! - NR338S: PITCL leaflet "Park Island – Central (Via Hong Kong Stn.)",
//!   Ver. 20250901, effective 1 September 2025.
//! - NR331 / NR331S: Transport Department Schedule of Services (NR331en.pdf)
//!   plus Sun Bus / Ma Wan Park service hours for the return leg.
//! - 230R: KMB timetable as published by KMB and the Transport Department.
//!
//! PITCL NR330/NR332/NR334/NR338 timetables are fetched live from the
//! operator's ETA backend instead; see `socif.rs`.

use crate::model::{
    DayType, DirectionInfo, Fare, Frequency, OfficialStop, OfficialStops, RouteInfo, Schedule,
};
use crate::data::LAST_VERIFIED;

pub const ROUTES: &[&str] = &[
    "NR330", "NR332", "NR334", "NR338", "NR338S", "NR331", "NR331S", "230R",
];

pub fn is_known(code: &str) -> bool {
    ROUTES.iter().any(|route| route.eq_ignore_ascii_case(code))
}

pub fn route(code: &str) -> Option<RouteInfo> {
    let code = canonical(code)?;
    let info = match code {
        "NR330" => RouteInfo {
            code: code.into(),
            operator: "Park Island Transport (PITCL)".into(),
            name_en: "Park Island ↔ MTR Tsing Yi Station".into(),
            name_tc: "珀麗灣 ↔ 港鐵青衣站".into(),
            realtime: true,
            source: "PITCL / Socif official ETA".into(),
            directions: vec![
                dir(1, "Park Island", "珀麗灣", "MTR Tsing Yi Station", "港鐵青衣站"),
                dir(2, "MTR Tsing Yi Station", "港鐵青衣站", "Park Island", "珀麗灣"),
            ],
            fare: Some(Fare::adult(13.80, Some(8.50)).with_note(
                "Resident fare requires a registered Octopus card.",
            )),
            effective: Some("2025-04-01".into()),
            last_verified: LAST_VERIFIED.into(),
            official_stops: vec![
                official(1, "PITCL NR330 leaflet, effective 2025-04-01", vec![
                    live("Park Island", "珀麗灣"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Tsing Yi Station", "港鐵青衣站"),
                ]),
                official(2, "PITCL NR330 leaflet, effective 2025-04-01", vec![
                    live("Tsing Yi Station", "港鐵青衣站"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Tin Liu New Village", "田寮新村"),
                    live("Park Island", "珀麗灣"),
                ]),
            ],
            notes: vec![
                "Peak supplement NR330A is operated by Sun Bus under the same route number in journey planners.".into(),
                "Inbound live predictions include Tin Liu New Village; outbound uses the four-stop official routing.".into(),
            ],
        },
        "NR332" => RouteInfo {
            code: code.into(),
            operator: "Park Island Transport (PITCL)".into(),
            name_en: "Park Island ↔ MTR Kwai Fong Station".into(),
            name_tc: "珀麗灣 ↔ 港鐵葵芳站".into(),
            realtime: true,
            source: "PITCL / Socif official ETA".into(),
            directions: vec![
                dir(1, "Park Island", "珀麗灣", "MTR Kwai Fong Station", "港鐵葵芳站"),
                dir(2, "MTR Kwai Fong Station", "港鐵葵芳站", "Park Island", "珀麗灣"),
            ],
            fare: Some(Fare::adult(14.50, Some(9.80)).with_note(
                "Resident fare requires a registered Octopus card.",
            )),
            effective: Some("2025-04-01".into()),
            last_verified: LAST_VERIFIED.into(),
            official_stops: vec![
                official(1, "PITCL NR332 leaflet, effective 2025-04-01", vec![
                    live("Park Island", "珀麗灣"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("MTR Kwai Fong Station", "港鐵葵芳站"),
                ]),
                official(2, "PITCL NR332 leaflet, effective 2025-04-01", vec![
                    live("MTR Kwai Fong Station", "港鐵葵芳站"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Tin Liu New Village", "田寮新村"),
                    live("Park Island", "珀麗灣"),
                ]),
            ],
            notes: vec![
                "Overnight departures from Park Island at 01:15, 02:00, 02:45, 03:30, 04:15, 05:00 and 05:45 also call at Tsing Yi Station.".into(),
                "Inbound live predictions include Tin Liu New Village; outbound uses the four-stop official routing.".into(),
            ],
        },
        "NR334" => RouteInfo {
            code: code.into(),
            operator: "Park Island Transport (PITCL)".into(),
            name_en: "Park Island ↺ Hong Kong International Airport".into(),
            name_tc: "珀麗灣 ↺ 香港國際機場".into(),
            realtime: true,
            source: "PITCL / Socif official ETA".into(),
            directions: vec![
                dir(1, "Park Island", "珀麗灣", "Airport (Terminal 2)", "機場（二號客運大樓）"),
                dir(2, "Airport (Terminal 2)", "機場（二號客運大樓）", "Park Island", "珀麗灣"),
            ],
            fare: Some(Fare::adult(32.50, None).with_note(
                "No resident fare. Octopus or cash; the $2 concession does not apply.",
            )),
            effective: Some("2025-09-23".into()),
            last_verified: LAST_VERIFIED.into(),
            official_stops: vec![
                official(1, "PITCL NR334 leaflet, effective 2025-09-23", vec![
                    live("Park Island", "珀麗灣"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Cathay City", "國泰城"),
                    airport_terminal("Airport Terminal 1", "機場一號客運大樓", "Alighting only. The live feed labels this stop simply Airport."),
                    airport_terminal("Airport Terminal 2", "機場二號客運大樓", "Coach Bay No. 7. The live feed labels this stop simply Airport."),
                    hzmb(),
                ]),
                official(2, "PITCL NR334 leaflet, effective 2025-09-23", vec![
                    airport_terminal("Airport Terminal 2", "機場二號客運大樓", "Coach Bay No. 7. This is the published departure point."),
                    airport_terminal("Airport Terminal 1", "機場一號客運大樓", "The live feed labels this stop simply Airport."),
                    hzmb(),
                    live("Tin Liu New Village", "田寮新村"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Park Island", "珀麗灣"),
                ]),
            ],
            notes: vec![
                "Hourly departures at :00 from Park Island and :30 from the Airport call at HZMB Hong Kong Port on selected trips.".into(),
                "This is a circular route; the departure time shown at the Airport is indicative.".into(),
                "The live feed collapses Terminal 1 and Terminal 2 into one Airport stop and does not list the HZMB calls; see official_stops.".into(),
            ],
        },
        "NR338" => RouteInfo {
            code: code.into(),
            operator: "Park Island Transport (PITCL)".into(),
            name_en: "Park Island ↔ Central (Pier 2), overnight".into(),
            name_tc: "珀麗灣 ↔ 中環（二號碼頭），通宵".into(),
            realtime: true,
            source: "PITCL / Socif official ETA".into(),
            directions: vec![
                dir(1, "Park Island", "珀麗灣", "Central (Pier 2)", "中環（二號碼頭）"),
                dir(2, "Central (Pier 2)", "中環（二號碼頭）", "Park Island", "珀麗灣"),
            ],
            fare: Some(Fare::adult(32.00, Some(24.50)).with_note(
                "Resident fare requires a registered Octopus card.",
            )),
            effective: Some("2025-04-01".into()),
            last_verified: LAST_VERIFIED.into(),
            official_stops: vec![
                official(1, "PITCL/TD NR338 route, checked 2026-11-22", vec![
                    live("Park Island", "珀麗灣"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Luk Kwok Centre", "六國中心"),
                    live("22-23 Connaught Road Central", "干諾道中22-23號"),
                    live("Central (Pier 2)", "中環（二號碼頭）"),
                ]),
                official(2, "PITCL/TD NR338 route, checked 2026-11-22", vec![
                    live("Central (Pier 2)", "中環（二號碼頭）"),
                    live("Harbour Centre", "灣仔海港中心"),
                    live("Beach Commercial Complex", "沙灘廣場"),
                    live("Kei Wai Primary School", "基慧小學"),
                    live("Tin Liu New Village", "田寮新村"),
                    live("Park Island", "珀麗灣"),
                ]),
            ],
            notes: vec![
                "Overnight only; six departures in each direction per night.".into(),
                "Inbound live predictions include Harbour Centre and Tin Liu New Village.".into(),
            ],
        },
        "NR338S" => RouteInfo {
            code: code.into(),
            operator: "Park Island Transport (PITCL)".into(),
            name_en: "Ma Wan ↔ Central (via Hong Kong Station)".into(),
            name_tc: "馬灣 ↔ 中環（經香港站）".into(),
            realtime: false,
            source: "PITCL leaflet Ver. 20250901".into(),
            directions: vec![
                dir(1, "Park Island", "珀麗灣", "Central (Pier 2)", "中環（二號碼頭）"),
                dir(2, "Central (Pier 2)", "中環（二號碼頭）", "Park Island", "珀麗灣"),
            ],
            fare: Some(Fare {
                adult_visitor: Some(39.00),
                adult_resident: Some(24.10),
                child_visitor: Some(19.50),
                child_resident: Some(12.10),
                elderly_visitor: Some(12.10),
                elderly_resident: Some(12.10),
                note: Some(
                    "Resident offer until 31 Dec 2026: $22.00 adult, $11.00 child/elderly.".into(),
                ),
            }),
            notes: vec![
                "Daytime ferry-replacement service. No public real-time feed; times are the published timetable.".into(),
            ],
        },
        "NR331" => RouteInfo {
            code: code.into(),
            operator: "Sun Bus".into(),
            name_en: "Ma Wan (Pak Yan Road) ↔ Tsuen Wan Station".into(),
            name_tc: "馬灣（珀欣路）↔ 荃灣站".into(),
            realtime: false,
            source: "Transport Department schedule; Sun Bus".into(),
            directions: vec![
                dir(1, "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）", "Tsuen Wan Station", "荃灣站"),
                dir(2, "Tsuen Wan Station", "荃灣站", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
            ],
            fare: None,
            notes: vec![
                "Circular service. The return leg is shown as a frequency because Sun Bus does not publish exact return times.".into(),
            ],
        },
        "NR331S" => RouteInfo {
            code: code.into(),
            operator: "Sun Bus".into(),
            name_en: "Ma Wan (Pak Yan Road) ↔ Tsuen Wan West (Nina Tower)".into(),
            name_tc: "馬灣（珀欣路）↔ 荃灣西（如心廣場）".into(),
            realtime: false,
            source: "Transport Department schedule; Sun Bus".into(),
            directions: vec![
                dir(1, "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）", "Tsuen Wan West", "荃灣西"),
                dir(2, "Tsuen Wan West", "荃灣西", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
            ],
            fare: None,
            notes: vec![
                "Circular service. The return leg is shown as a frequency because Sun Bus does not publish exact return times.".into(),
            ],
        },
        "230R" => RouteInfo {
            code: code.into(),
            operator: "Kowloon Motor Bus (KMB)".into(),
            name_en: "Kowloon Station ↔ Ma Wan (Pak Yan Road)".into(),
            name_tc: "九龍站 ↔ 馬灣（珀欣路）".into(),
            realtime: true,
            source: "KMB open data; KMB/TD timetable".into(),
            directions: vec![
                dir(1, "Kowloon Station", "九龍站", "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）"),
                dir(2, "Ma Wan (Pak Yan Road)", "馬灣（珀欣路）", "Kowloon Station", "九龍站"),
            ],
            fare: Some(Fare::adult(15.50, None).with_note(
                "Same-day return and interchange discounts apply.",
            )),
            notes: vec![
                "KMB direction 1 is the operator's outbound direction (Kowloon Station → Ma Wan).".into(),
            ],
        },
        _ => return None,
    };
    Some(info)
}

fn dir(seq: u8, orig_en: &str, orig_tc: &str, dest_en: &str, dest_tc: &str) -> DirectionInfo {
    DirectionInfo {
        seq,
        orig_en: orig_en.into(),
        orig_tc: orig_tc.into(),
        dest_en: dest_en.into(),
        dest_tc: dest_tc.into(),
    }
}

fn canonical(code: &str) -> Option<&'static str> {
    ROUTES
        .iter()
        .copied()
        .find(|route| route.eq_ignore_ascii_case(code))
}

fn exact(route: &str, seq: u8, day: DayType, times: &[&str], note: impl Into<String>) -> Schedule {
    let info = self::route(route).expect("known route");
    let direction = info
        .directions
        .iter()
        .find(|direction| direction.seq == seq)
        .expect("known direction");
    Schedule {
        route: route.to_string(),
        direction: seq,
        direction_en: direction.name_en(),
        direction_tc: direction.name_tc(),
        day,
        source: "official-exact".into(),
        first: times.first().map(|value| (*value).to_string()),
        last: times.last().map(|value| (*value).to_string()),
        departures: times.iter().map(|value| (*value).to_string()).collect(),
        frequencies: vec![],
        note: Some(note.into()),
    }
}

fn frequency(
    route: &str,
    seq: u8,
    day: DayType,
    rows: &[(&str, Option<&str>, u32, u32)],
    note: impl Into<String>,
) -> Schedule {
    let info = self::route(route).expect("known route");
    let direction = info
        .directions
        .iter()
        .find(|direction| direction.seq == seq)
        .expect("known direction");
    let frequencies: Vec<Frequency> = rows
        .iter()
        .map(|(start, end, min, max)| Frequency {
            start: (*start).to_string(),
            end: end.map(|value| value.to_string()),
            min_minutes: Some(*min),
            max_minutes: Some(*max),
        })
        .collect();
    Schedule {
        route: route.to_string(),
        direction: seq,
        direction_en: direction.name_en(),
        direction_tc: direction.name_tc(),
        day,
        source: "official-frequency".into(),
        first: rows.first().map(|row| row.0.to_string()),
        last: rows.last().and_then(|row| row.1).map(str::to_string),
        departures: vec![],
        frequencies,
        note: Some(note.into()),
    }
}

const NR338S_WEEKDAY_PI: &[&str] = &[
    "07:35", "07:40", "08:35", "08:40", "11:00", "13:00", "15:00", "15:30", "16:30", "20:30",
    "22:00", "23:00",
];
const NR338S_WEEKDAY_CENTRAL: &[&str] = &[
    "08:05", "08:10", "09:05", "09:10", "11:30", "13:30", "15:30", "16:00", "17:00", "21:00",
    "22:30", "23:30",
];
const NR338S_WEEKEND_PI: &[&str] = &[
    "09:00", "11:00", "13:00", "15:00", "17:00", "19:00", "21:00", "22:00", "23:00",
];
const NR338S_WEEKEND_CENTRAL: &[&str] = &[
    "09:30", "11:30", "13:30", "15:30", "17:30", "19:30", "21:30", "22:30", "23:30",
];

const NR331_DAILY_PI: &[&str] = &[
    "06:00", "06:30", "07:00", "07:20", "07:40", "08:00", "08:20", "08:40", "09:00", "09:20",
    "09:40", "10:00", "10:20", "10:40", "11:00", "11:20", "11:40", "12:00", "12:20", "12:40",
    "13:00", "13:20", "13:40", "14:00", "14:20", "14:40", "15:00", "15:20", "15:40", "16:00",
    "16:20", "16:40", "17:00", "17:20", "17:40", "18:00", "18:20", "18:40", "19:00", "19:20",
    "19:40", "20:00", "20:20", "20:40", "21:00", "21:30", "22:00", "22:30", "23:00", "23:30",
    "00:00",
];

const NR331S_DAILY_PI: &[&str] = &[
    "06:45", "07:15", "07:45", "08:15", "08:45", "09:15", "09:45", "10:15", "10:45", "11:15",
    "11:45", "12:15", "12:45", "13:15", "13:45", "14:15", "14:45", "15:15", "15:45", "16:15",
    "16:45", "17:15", "17:45", "18:15", "18:45", "19:15", "19:45", "20:15", "20:45", "21:15",
    "21:45", "22:15", "22:45", "23:15",
];

pub fn schedule(code: &str, seq: u8, day: DayType) -> Option<Schedule> {
    let code = canonical(code)?;
    if seq != 1 && seq != 2 {
        return None;
    }
    let weekday = day == DayType::Weekday;
    Some(match code {
        "NR338S" => {
            let times = match (weekday, seq) {
                (true, 1) => NR338S_WEEKDAY_PI,
                (true, _) => NR338S_WEEKDAY_CENTRAL,
                (false, 1) => NR338S_WEEKEND_PI,
                (false, _) => NR338S_WEEKEND_CENTRAL,
            };
            exact(
                code,
                seq,
                day,
                times,
                "Published PITCL timetable (Ver. 20250901, effective 2025-09-01). No public real-time feed.",
            )
        }
        "NR331" => {
            if seq == 1 {
                exact(
                    code,
                    seq,
                    day,
                    NR331_DAILY_PI,
                    "Daily times from Ma Wan (Pak Yan Road), Transport Department schedule.",
                )
            } else {
                frequency(
                    code,
                    seq,
                    day,
                    &[("06:20", Some("00:20"), 20, 30)],
                    "Circular return leg: Sun Bus publishes service hours rather than exact times.",
                )
            }
        }
        "NR331S" => {
            if seq == 1 {
                exact(
                    code,
                    seq,
                    day,
                    NR331S_DAILY_PI,
                    "Daily times from Ma Wan (Pak Yan Road), Transport Department schedule.",
                )
            } else {
                frequency(
                    code,
                    seq,
                    day,
                    &[("07:10", Some("23:40"), 30, 60)],
                    "Circular return leg: Sun Bus publishes service hours rather than exact times.",
                )
            }
        }
        "230R" => {
            if weekday {
                if seq == 1 {
                    frequency(
                        code,
                        seq,
                        day,
                        &[("10:00", Some("21:00"), 60, 60)],
                        "KMB weekday timetable: one bus per hour.",
                    )
                } else {
                    frequency(
                        code,
                        seq,
                        day,
                        &[("11:00", Some("22:00"), 60, 60)],
                        "KMB weekday timetable: one bus per hour.",
                    )
                }
            } else if seq == 1 {
                frequency(
                    code,
                    seq,
                    day,
                    &[
                        ("09:00", Some("11:00"), 30, 30),
                        ("11:00", Some("12:00"), 20, 20),
                        ("12:00", Some("16:30"), 30, 30),
                        ("16:30", Some("21:30"), 60, 60),
                    ],
                    "KMB Saturday/Sunday/public-holiday timetable.",
                )
            } else {
                frequency(
                    code,
                    seq,
                    day,
                    &[
                        ("11:00", Some("12:00"), 60, 60),
                        ("12:00", Some("16:00"), 30, 30),
                        ("16:00", Some("17:00"), 20, 20),
                        ("17:00", Some("18:30"), 30, 30),
                        ("18:30", Some("22:30"), 60, 60),
                    ],
                    "KMB Saturday/Sunday/public-holiday timetable.",
                )
            }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_advertised_route_has_a_registry_entry() {
        for code in ROUTES {
            let info = route(code).unwrap_or_else(|| panic!("missing {code}"));
            assert_eq!(info.directions.len(), 2);
        }
    }

    #[test]
    fn published_bus_fares_are_attached() {
        let nr330 = route("NR330").unwrap().fare.expect("NR330 fare");
        assert_eq!(nr330.adult_visitor, Some(13.80));
        assert_eq!(nr330.adult_resident, Some(8.50));
        let nr334 = route("NR334").unwrap().fare.expect("NR334 fare");
        assert_eq!(nr334.adult_visitor, Some(32.50));
        assert_eq!(nr334.adult_resident, None);
    }

    #[test]
    fn nr338s_has_exact_published_times() {
        let schedule = schedule("NR338S", 1, DayType::Weekday).unwrap();
        assert_eq!(schedule.source, "official-exact");
        assert_eq!(schedule.first.as_deref(), Some("07:35"));
        assert_eq!(schedule.last.as_deref(), Some("23:00"));
    }

    #[test]
    fn kmb_230r_weekend_is_frequency_based() {
        let schedule = schedule("230R", 1, DayType::Saturday).unwrap();
        assert_eq!(schedule.source, "official-frequency");
        assert_eq!(schedule.frequencies.len(), 4);
        assert_eq!(schedule.frequencies[1].min_minutes, Some(20));
    }
}
