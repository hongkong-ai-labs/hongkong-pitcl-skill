//! Provenance and freshness metadata for the static data in this crate.
//!
//! The live feeds carry their own timestamps; published fares, timetables and
//! policies do not. This registry records where each static dataset came from,
//! its printed effective date where one exists, and when this project last
//! verified it.

use serde::Serialize;

pub const LAST_VERIFIED: &str = "2026-11-22";

#[derive(Debug, Clone, Serialize)]
pub struct DatasetInfo {
    pub name: String,
    pub source: String,
    /// Effective date printed by the publisher, if any.
    pub effective: Option<String>,
    /// Date this project last checked the source.
    pub last_verified: String,
    pub note: String,
}

fn dataset(name: &str, source: &str, effective: Option<&str>, note: &str) -> DatasetInfo {
    DatasetInfo {
        name: name.to_string(),
        source: source.to_string(),
        effective: effective.map(str::to_string),
        last_verified: LAST_VERIFIED.to_string(),
        note: note.to_string(),
    }
}

pub fn datasets() -> Vec<DatasetInfo> {
    vec![
        dataset(
            "PITCL live bus ETA and weekday timetable",
            "https://360-api.socif.co (the backend behind eta.pitcl.com.hk)",
            None,
            "Live. NR330=249, NR332=250, NR334=252, NR338=253. Weekend headways come from the same metadata.",
        ),
        dataset(
            "KMB 230R live ETA",
            "https://data.etabus.gov.hk/v1/transport/kmb",
            None,
            "Live. 230R is operated by KMB, not PITCL.",
        ),
        dataset(
            "PITCL fares and timetables (NR330, NR332, NR338)",
            "https://www.pitcl.com.hk/en/bus_service.php",
            Some("2025-04-01"),
            "Fares and published service information. NR334 is dated separately because its leaflet changed later.",
        ),
        dataset(
            "PITCL NR334 fare and timetable",
            "https://www.pitcl.com.hk/attachment/bus/pdf/1758554831o21IM.pdf",
            Some("2025-09-23"),
            "Airport service. Selected departures also call at the HZMB Passenger Clearance Building.",
        ),
        dataset(
            "PITCL NR338S timetable and fare",
            "https://www.pitcl.com.hk/attachment/bus/pdf/1777455439nJNoi.pdf",
            Some("2025-09-01"),
            "Daytime Central service via Hong Kong Station. Resident offer fare is stated until 31 Dec 2026.",
        ),
        dataset(
            "Sun Bus NR331/NR331S timetable and fares",
            "https://www.td.gov.hk/filemanager/en/content_4796/NR331en.pdf",
            None,
            "Transport Department Schedule of Services; no effective date is printed on the sheet. Fares checked 2026-11-22: NR331 $15.6/$9.6, NR331S $16.3/$10.0.",
        ),
        dataset(
            "KMB 230R timetable",
            "https://www.td.gov.hk/en/transport_in_hong_kong/public_transport/non_franchised/list_of_approved_rs_nt/index.html",
            None,
            "Published KMB/TD frequency table. The route API does not carry a fare field; fare checked 2026-11-22 at HK$15.50.",
        ),
        dataset(
            "Park Island ferry timetable and fares",
            "https://www.pitcl.com.hk/en/ferry_service.php",
            Some("2025-08-17"),
            "Central and Tsuen Wan sailings. Times are pier gate-closing times; there is no public real-time ferry feed.",
        ),
        dataset(
            "PITCL high-wind and typhoon contingency plan",
            "https://www.pitcl.com.hk/attachment/contingency_traffic_arrangement_eng.pdf",
            Some("2018-08-14"),
            "PAX Notice 2018.08. Actual arrangements depend on conditions and Transport Department approval.",
        ),
        dataset(
            "PITCL ferry passenger rules",
            "https://www.pitcl.com.hk/attachment/notes_to_passengers_ferry.pdf",
            None,
            "Luggage, pets, fares and on-board rules. The bus notes PDF is image-only.",
        ),
        dataset(
            "HKO weather, warnings and regional wind",
            "https://data.weather.gov.hk/weatherAPI/opendata/weather.php and /hko_data/regional-weather/latest_10min_wind.csv",
            None,
            "Live. The regional wind file does not include a Tsing Ma Bridge anemometer.",
        ),
        dataset(
            "TD special traffic news",
            "https://resource.data.one.gov.hk/td/en/specialtrafficnews.xml",
            None,
            "Live territory-wide XML; Ma Wan relevance is keyword-based.",
        ),
        dataset(
            "PITCL notices",
            "https://www.pitcl.com.hk/en/news.php",
            None,
            "Scraped HTML; the site does not expose publication dates in the list.",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dataset_has_provenance_and_a_verification_date() {
        let datasets = datasets();
        assert!(datasets.len() >= 10);
        for dataset in datasets {
            assert!(!dataset.name.is_empty());
            assert!(dataset.source.starts_with("https://"));
            assert_eq!(dataset.last_verified, LAST_VERIFIED);
        }
    }
}
