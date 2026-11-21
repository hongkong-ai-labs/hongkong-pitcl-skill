//! Park Island ferry routes.
//!
//! Source: PITCL ferry timetables effective 17 August 2025
//! (`ferry_service.php`, PDFs 1755353991cflXU and 17553540181Q6ca).
//! There is no public real-time ferry feed; these are published times.

use serde::Serialize;

use crate::data::LAST_VERIFIED;
use crate::model::DayType;

#[derive(Debug, Clone, Serialize)]
pub struct FerryFare {
    pub adult_resident: f64,
    pub adult_visitor: f64,
    pub child_resident: f64,
    pub child_visitor: f64,
    pub elderly_resident: f64,
    pub elderly_visitor: f64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FerryRoute {
    pub code: String,
    pub name_en: String,
    pub name_tc: String,
    pub park_island_pier_en: String,
    pub park_island_pier_tc: String,
    pub other_pier_en: String,
    pub other_pier_tc: String,
    pub other_pier_address_en: String,
    pub other_pier_address_tc: String,
    /// Published crossing time in minutes (Transport Department).
    pub journey_minutes: u32,
    pub fare: FerryFare,
    pub effective: Option<String>,
    pub last_verified: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FerryTimetable {
    pub route: String,
    pub route_en: String,
    pub route_tc: String,
    pub day: DayType,
    pub from_park_island: Vec<String>,
    pub from_other_pier: Vec<String>,
    pub park_island_pier_en: String,
    pub park_island_pier_tc: String,
    pub other_pier_en: String,
    pub other_pier_tc: String,
    pub other_pier_address_en: String,
    pub other_pier_address_tc: String,
    pub journey_minutes: u32,
    pub first_from_park_island: Option<String>,
    pub last_from_park_island: Option<String>,
    pub first_from_other_pier: Option<String>,
    pub last_from_other_pier: Option<String>,
    pub fare: FerryFare,
    pub effective: Option<String>,
    pub last_verified: String,
    pub note: String,
}

pub const ROUTES: &[&str] = &["central", "tsuen-wan"];

const EFFECTIVE: &str = "2025-08-17";
const NOTE: &str = "Published timetable effective 17 August 2025. Times are pier gate-closing times. No public real-time ferry feed. The $2 concession applies to eligible Octopus cardholders.";
const PARK_ISLAND_PIER_EN: &str = "Park Island Ferry Pier, 8 Pak Lai Road, Ma Wan";
const PARK_ISLAND_PIER_TC: &str = "珀麗灣碼頭（馬灣珀萊路8號）";

const CENTRAL_WEEKDAY_PI: &[&str] = &[
    "06:30", "07:00", "07:20", "08:00", "08:20", "09:00", "09:30", "10:00", "12:00", "14:00",
    "16:00", "17:00", "17:30", "18:00", "18:30", "19:00", "19:30", "20:00", "21:00",
];
const CENTRAL_WEEKDAY_OTHER: &[&str] = &[
    "07:00", "07:30", "07:50", "08:30", "08:50", "09:30", "10:00", "10:30", "12:30", "14:30",
    "16:30", "17:30", "18:00", "18:30", "19:00", "19:30", "20:00", "20:30", "21:30",
];
const CENTRAL_WEEKEND_PI: &[&str] = &[
    "08:00", "10:00", "12:00", "14:00", "16:00", "18:00", "20:00",
];
const CENTRAL_WEEKEND_OTHER: &[&str] = &[
    "08:30", "10:30", "12:30", "14:30", "16:30", "18:30", "20:30",
];

const TSUEN_WAN_WEEKDAY_PI: &[&str] = &["08:10", "09:10", "16:10"];
const TSUEN_WAN_WEEKDAY_OTHER: &[&str] = &["08:30", "09:30", "16:30"];
const TSUEN_WAN_WEEKEND_PI: &[&str] = &["10:10", "13:10", "16:10"];
const TSUEN_WAN_WEEKEND_OTHER: &[&str] = &["10:30", "13:30", "16:30"];

fn central_fare() -> FerryFare {
    FerryFare {
        adult_resident: 24.10,
        adult_visitor: 36.00,
        child_resident: 12.10,
        child_visitor: 18.00,
        elderly_resident: 12.10,
        elderly_visitor: 18.00,
        note: "Child fare is for ages 1-11; elderly fare is for ages 65 or above.".into(),
    }
}

fn tsuen_wan_fare() -> FerryFare {
    FerryFare {
        adult_resident: 10.20,
        adult_visitor: 12.00,
        child_resident: 5.10,
        child_visitor: 6.00,
        elderly_resident: 5.10,
        elderly_visitor: 6.00,
        note: "Child fare is for ages 1-11; elderly fare is for ages 65 or above.".into(),
    }
}

pub fn routes() -> Vec<FerryRoute> {
    ROUTES.iter().filter_map(|code| route(code)).collect()
}

pub fn route(code: &str) -> Option<FerryRoute> {
    let code = canonical(code)?;
    Some(match code {
        "central" => FerryRoute {
            code: code.into(),
            name_en: "Park Island ↔ Central".into(),
            name_tc: "珀麗灣 ↔ 中環".into(),
            park_island_pier_en: PARK_ISLAND_PIER_EN.into(),
            park_island_pier_tc: PARK_ISLAND_PIER_TC.into(),
            other_pier_en: "Central Pier No. 2".into(),
            other_pier_tc: "中環二號碼頭".into(),
            other_pier_address_en: "Man Kwong Street, Central".into(),
            other_pier_address_tc: "中環民光街".into(),
            journey_minutes: 22,
            fare: central_fare(),
            effective: Some(EFFECTIVE.into()),
            last_verified: LAST_VERIFIED.into(),
        },
        "tsuen-wan" => FerryRoute {
            code: code.into(),
            name_en: "Park Island ↔ Tsuen Wan".into(),
            name_tc: "珀麗灣 ↔ 荃灣".into(),
            park_island_pier_en: PARK_ISLAND_PIER_EN.into(),
            park_island_pier_tc: PARK_ISLAND_PIER_TC.into(),
            other_pier_en: "Tsuen Wan Pier".into(),
            other_pier_tc: "荃灣碼頭".into(),
            other_pier_address_en: "Tsuen Wan Ferry Pier, Tsuen Wan".into(),
            other_pier_address_tc: "荃灣渡輪碼頭".into(),
            journey_minutes: 12,
            fare: tsuen_wan_fare(),
            effective: Some(EFFECTIVE.into()),
            last_verified: LAST_VERIFIED.into(),
        },
        _ => return None,
    })
}

pub fn schedule(code: &str, day: DayType) -> Option<FerryTimetable> {
    let info = route(code)?;
    let weekday = day == DayType::Weekday;
    let (from_park_island, from_other_pier) = match info.code.as_str() {
        "central" if weekday => (CENTRAL_WEEKDAY_PI, CENTRAL_WEEKDAY_OTHER),
        "central" => (CENTRAL_WEEKEND_PI, CENTRAL_WEEKEND_OTHER),
        "tsuen-wan" if weekday => (TSUEN_WAN_WEEKDAY_PI, TSUEN_WAN_WEEKDAY_OTHER),
        "tsuen-wan" => (TSUEN_WAN_WEEKEND_PI, TSUEN_WAN_WEEKEND_OTHER),
        _ => return None,
    };
    let from_park_island = strings(from_park_island);
    let from_other_pier = strings(from_other_pier);
    Some(FerryTimetable {
        route: info.code,
        route_en: info.name_en,
        route_tc: info.name_tc,
        day,
        first_from_park_island: from_park_island.first().cloned(),
        last_from_park_island: from_park_island.last().cloned(),
        first_from_other_pier: from_other_pier.first().cloned(),
        last_from_other_pier: from_other_pier.last().cloned(),
        from_park_island,
        from_other_pier,
        park_island_pier_en: info.park_island_pier_en,
        park_island_pier_tc: info.park_island_pier_tc,
        other_pier_en: info.other_pier_en,
        other_pier_tc: info.other_pier_tc,
        other_pier_address_en: info.other_pier_address_en,
        other_pier_address_tc: info.other_pier_address_tc,
        journey_minutes: info.journey_minutes,
        fare: info.fare,
        effective: info.effective,
        last_verified: info.last_verified,
        note: NOTE.into(),
    })
}

pub fn canonical(code: &str) -> Option<&'static str> {
    let normalised = code.trim().to_ascii_lowercase().replace('_', "-");
    let normalised = normalised.replace(' ', "-");
    ROUTES.iter().copied().find(|route| {
        *route == normalised
            || match *route {
                "central" => normalised == "central-pier" || normalised == "pier-2",
                "tsuen-wan" => normalised == "tw" || normalised == "tsuenwan",
                _ => false,
            }
    })
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn central_weekday_has_published_sailings() {
        let table = schedule("central", DayType::Weekday).unwrap();
        assert_eq!(table.from_park_island.len(), 19);
        assert_eq!(table.from_park_island.first().unwrap(), "06:30");
        assert_eq!(table.from_other_pier.first().unwrap(), "07:00");
    }

    #[test]
    fn weekend_uses_the_reduced_ferry_timetable() {
        let table = schedule("tsuen-wan", DayType::Sunday).unwrap();
        assert_eq!(table.from_park_island, vec!["10:10", "13:10", "16:10"]);
        assert_eq!(table.fare.adult_visitor, 12.00);
    }

    #[test]
    fn ferry_routes_carry_journey_times_and_piers() {
        let central = route("central").unwrap();
        assert_eq!(central.journey_minutes, 22);
        assert!(central.other_pier_address_en.contains("Man Kwong Street"));
        assert!(central.park_island_pier_en.contains("Pak Lai Road"));
        let tsuen_wan = route("tsuen-wan").unwrap();
        assert_eq!(tsuen_wan.journey_minutes, 12);
    }

    #[test]
    fn timetable_exposes_first_and_last_sailings() {
        let table = schedule("central", DayType::Weekday).unwrap();
        assert_eq!(table.first_from_park_island.as_deref(), Some("06:30"));
        assert_eq!(table.last_from_park_island.as_deref(), Some("21:00"));
        assert_eq!(table.first_from_other_pier.as_deref(), Some("07:00"));
        assert_eq!(table.last_from_other_pier.as_deref(), Some("21:30"));
    }
}
