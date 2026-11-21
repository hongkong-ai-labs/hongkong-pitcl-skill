use chrono::{Datelike, FixedOffset, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

/// Hong Kong is UTC+8 all year; the ETA feeds publish either +08:00 or UTC.
pub const HKT_OFFSET_SECS: i32 = 8 * 3600;

pub fn hkt() -> FixedOffset {
    FixedOffset::east_opt(HKT_OFFSET_SECS).expect("valid UTC+8 offset")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum DayType {
    Weekday,
    Saturday,
    Sunday,
    Holiday,
}

impl DayType {
    pub fn label(self) -> &'static str {
        match self {
            DayType::Weekday => "weekday",
            DayType::Saturday => "saturday",
            DayType::Sunday => "sunday",
            DayType::Holiday => "holiday",
        }
    }

    /// Day type for a Hong Kong date. Public-holiday dates are the ones the
    /// hkbus dataset publishes for 2026-2027.
    pub fn for_date(date: NaiveDate) -> Self {
        if is_public_holiday(date) {
            return DayType::Holiday;
        }
        match date.weekday() {
            Weekday::Sat => DayType::Saturday,
            Weekday::Sun => DayType::Sunday,
            _ => DayType::Weekday,
        }
    }
}

/// Parse a day selector: weekday, saturday, sunday, holiday, today,
/// tomorrow, or a YYYY-MM-DD date.
pub fn parse_day(value: &str) -> Result<DayType, String> {
    parse_day_with_date(value).map(|(day, _)| day)
}

/// Parse a day selector together with the calendar date it refers to.
/// Named day types without a date use today's date, which lets callers
/// compute "next departure" windows without inventing a date.
pub fn parse_day_with_date(value: &str) -> Result<(DayType, NaiveDate), String> {
    let today = chrono::Utc::now().with_timezone(&hkt()).date_naive();
    match value.trim().to_ascii_lowercase().as_str() {
        "weekday" | "weekdays" => Ok((DayType::Weekday, today)),
        "saturday" | "sat" => Ok((DayType::Saturday, today)),
        "sunday" | "sun" => Ok((DayType::Sunday, today)),
        "holiday" | "holidays" | "public-holiday" => Ok((DayType::Holiday, today)),
        "today" => Ok((DayType::for_date(today), today)),
        "tomorrow" => {
            let date = today + chrono::Days::new(1);
            Ok((DayType::for_date(date), date))
        }
        other => NaiveDate::parse_from_str(other, "%Y-%m-%d")
            .map(|date| (DayType::for_date(date), date))
            .map_err(|_| {
                format!(
                    "unknown day '{value}'; use weekday, saturday, sunday, holiday, today, tomorrow or YYYY-MM-DD"
                )
            }),
    }
}

fn is_public_holiday(date: NaiveDate) -> bool {
    const HOLIDAYS: &[&str] = &[
        "20260101", "20260217", "20260218", "20260219", "20260403", "20260404", "20260406",
        "20260407", "20260501", "20260525", "20260619", "20260701", "20260926", "20261001",
        "20261019", "20261225", "20261226", "20270101", "20270206", "20270208", "20270209",
        "20270326", "20270327", "20270329", "20270405", "20270501", "20270513", "20270609",
        "20270701", "20270916", "20271001", "20271008", "20271225", "20271227",
    ];
    let key = date.format("%Y%m%d").to_string();
    HOLIDAYS.binary_search(&key.as_str()).is_ok()
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectionInfo {
    pub seq: u8,
    pub orig_en: String,
    pub orig_tc: String,
    pub dest_en: String,
    pub dest_tc: String,
}

impl DirectionInfo {
    pub fn name_en(&self) -> String {
        format!("{} → {}", self.orig_en, self.dest_en)
    }

    pub fn name_tc(&self) -> String {
        format!("{} → {}", self.orig_tc, self.dest_tc)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Fare {
    pub adult_visitor: Option<f64>,
    pub adult_resident: Option<f64>,
    pub child_visitor: Option<f64>,
    pub child_resident: Option<f64>,
    pub elderly_visitor: Option<f64>,
    pub elderly_resident: Option<f64>,
    pub note: Option<String>,
}

impl Fare {
    pub fn adult(visitor: f64, resident: Option<f64>) -> Self {
        Self {
            adult_visitor: Some(visitor),
            adult_resident: resident,
            child_visitor: None,
            child_resident: None,
            elderly_visitor: None,
            elderly_resident: None,
            note: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OfficialStop {
    pub name_en: String,
    pub name_tc: String,
    /// The matching name in the live stop list, when the live feed covers it.
    /// `None` means the stop is published by the operator but absent from live
    /// predictions.
    pub live_name_en: Option<String>,
    pub live_name_tc: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OfficialStops {
    pub direction: u8,
    pub source: String,
    pub stops: Vec<OfficialStop>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteInfo {
    pub code: String,
    pub operator: String,
    pub name_en: String,
    pub name_tc: String,
    pub realtime: bool,
    /// Where the live data or timetable comes from.
    pub source: String,
    pub directions: Vec<DirectionInfo>,
    pub fare: Option<Fare>,
    /// Printed effective date of the fare/timetable source, where one exists.
    pub effective: Option<String>,
    pub last_verified: String,
    pub official_stops: Vec<OfficialStops>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stop {
    pub seq: u16,
    pub id: String,
    pub name_en: String,
    pub name_tc: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Eta {
    pub stop_seq: u16,
    pub stop_id: String,
    pub stop_en: String,
    pub stop_tc: String,
    pub direction: u8,
    pub eta_seq: u32,
    /// Minutes from now as published by the operator; may be absent.
    pub minutes: Option<i64>,
    /// Scheduled or estimated arrival, RFC 3339 in Hong Kong time.
    pub at: Option<String>,
    pub remark_en: String,
    pub remark_tc: String,
    /// False when the operator explicitly labels the value as scheduled.
    pub realtime: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Frequency {
    pub start: String,
    /// `None` for a single fixed departure.
    pub end: Option<String>,
    pub min_minutes: Option<u32>,
    pub max_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Schedule {
    pub route: String,
    pub direction: u8,
    pub direction_en: String,
    pub direction_tc: String,
    pub day: DayType,
    /// `official-exact`, `official-frequency`, or `official-fixed`.
    pub source: String,
    pub first: Option<String>,
    pub last: Option<String>,
    pub departures: Vec<String>,
    pub frequencies: Vec<Frequency>,
    /// Printed effective date of the published timetable, if any.
    pub effective: Option<String>,
    pub last_verified: String,
    pub note: Option<String>,
}
