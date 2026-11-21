//! Static PITCL guidance that is not available from the live feeds.
//!
//! Sources:
//! - Contact page: https://www.pitcl.com.hk/en/customer_service.php?id=1
//! - Adverse weather: https://www.pitcl.com.hk/attachment/contingency_traffic_arrangement_eng.pdf
//!   (PAX Notice 2018.08, 14 August 2018)
//! - Ferry rules: https://www.pitcl.com.hk/attachment/notes_to_passengers_ferry.pdf
//!
//! These are published policies, not live predictions. Always tell the user
//! that actual arrangements depend on conditions and Transport Department
//! approval, and point them to PITCL notices for the current situation.

use serde::Serialize;

pub const TOPICS: &[&str] = &["contact", "weather", "policies"];

#[derive(Debug, Clone, Serialize)]
pub struct Guide {
    pub topic: String,
    pub title: String,
    pub source: String,
    pub note: String,
    pub sections: Vec<GuideSection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuideSection {
    pub heading: String,
    pub items: Vec<String>,
}

pub fn guide(topic: &str) -> Option<Guide> {
    match canonical(topic)? {
        "contact" => Some(contact()),
        "weather" => Some(weather()),
        "policies" => Some(policies()),
        _ => None,
    }
}

pub fn canonical(topic: &str) -> Option<&'static str> {
    match topic.trim().to_ascii_lowercase().as_str() {
        "contact" | "contacts" | "hotline" => Some("contact"),
        "weather" | "typhoon" | "contingency" | "adverse-weather" => Some("weather"),
        "policies" | "policy" | "rules" | "passengers" => Some("policies"),
        _ => None,
    }
}

fn contact() -> Guide {
    Guide {
        topic: "contact".into(),
        title: "PITCL contact and resident fare registration".into(),
        source: "https://www.pitcl.com.hk/en/customer_service.php?id=1".into(),
        note: "Contact details can change; check the official page before relying on them.".into(),
        sections: vec![
            GuideSection {
                heading: "General enquiry".into(),
                items: vec![
                    "Hotline: 2946 8888".into(),
                    "Fax: 2525 5556".into(),
                    "Email: enquiry@pitcl.com.hk".into(),
                    "Address: 1/F, Park Island Ferry Pier, 8 Pak Lai Road, Ma Wan, N.T., Hong Kong".into(),
                    "Office hours: Monday to Friday 09:00-18:15 (lunch 13:00-14:20)".into(),
                ],
            },
            GuideSection {
                heading: "Resident fares and feedback".into(),
                items: vec![
                    "Resident bus and ferry fares require a registered Octopus card.".into(),
                    "Registration and change-of-data forms: https://www.pitcl.com.hk/en/customer_service.php?id=2".into(),
                    "Customer feedback form: https://www.pitcl.com.hk/en/customer_service.php?id=3".into(),
                    "Private hire enquiries: 2946 8899 or enquiry@pitcl.com.hk.".into(),
                ],
            },
            GuideSection {
                heading: "Lost property".into(),
                items: vec![
                    "Ferry lost property: ask the pier staff.".into(),
                    "Other enquiries: call 2946 8888.".into(),
                ],
            },
        ],
    }
}

fn weather() -> Guide {
    Guide {
        topic: "weather".into(),
        title: "PITCL high wind and typhoon arrangements".into(),
        source:
            "https://www.pitcl.com.hk/attachment/contingency_traffic_arrangement_eng.pdf".into(),
        note: "Published contingency plan (PAX Notice 2018.08). Actual arrangements depend on conditions and Transport Department approval; check PITCL notices, pier/bus-stop boards and HKO warnings.".into(),
        sections: vec![
            GuideSection {
                heading: "High wind on Tsing Ma Bridge".into(),
                items: vec![
                    "40-60 km/h: bus services may be delayed.".into(),
                    "60-75 km/h with Stage 1 high-wind management: PITCL uses approved super-low-floor buses on the upper deck; services may be strengthened until 75 km/h; delays are possible and KMB Y41 contingency support may be arranged.".into(),
                    "Above 75 km/h with Stage 2: bus services are suspended, including KMB Y41. Tsuen Wan ferry is strengthened; a 400-seat ferry or extra sailings may be used. Free shuttle buses link Tsing Yi Station, Tsuen Wan Station, Kwai Fong Metroplaza and Tsuen Wan Ferry Pier until ferry service stops.".into(),
                    "Below 75 km/h: bus services resume; KMB Y41 support may be used if needed.".into(),
                    "Below 60 km/h: normal bus service resumes.".into(),
                ],
            },
            GuideSection {
                heading: "Typhoon signals".into(),
                items: vec![
                    "Signal 1: ferry services may be delayed by sea conditions.".into(),
                    "Signal 3: limited service or suspension is possible before or shortly after Signal 8.".into(),
                    "Signal 8 expected within 2 hours: PITCL monitors sea state, positions a spare vessel at Tsuen Wan Pier and prepares a suspension schedule.".into(),
                    "Signal 8 hoisted: suspension notices are posted at the piers; Tsuen Wan ferry is strengthened until crowds clear; the Central route may use a 400-seat ferry. Under Marine Department rules, ferries suspend about 2 hours after Signal 8.".into(),
                    "Signal 3 lowered: services resume as soon as possible, with extra sailings if needed.".into(),
                    "Signal 1 lowered: ferries resume but may still be delayed.".into(),
                ],
            },
            GuideSection {
                heading: "Where to check".into(),
                items: vec![
                    "HKO warnings and forecast: `mawan weather`.".into(),
                    "PITCL notices: `mawan news`.".into(),
                    "Road incidents: `mawan traffic`.".into(),
                    "PITCL hotline: 2946 8888.".into(),
                ],
            },
        ],
    }
}

fn policies() -> Guide {
    Guide {
        topic: "policies".into(),
        title: "PITCL passenger policies".into(),
        source: "https://www.pitcl.com.hk/attachment/notes_to_passengers_ferry.pdf".into(),
        note: "Ferry rules below are from the official Notes to Ferry Passengers. The bus notes are published as an image PDF; use the link for the official bus version.".into(),
        sections: vec![
            GuideSection {
                heading: "Ferry fares".into(),
                items: vec![
                    "Every passenger must pay the appropriate fare before boarding.".into(),
                    "Children under 1 travelling with an adult, without occupying a seat, travel free.".into(),
                    "Children under 12 and passengers aged 65 or above pay half fare; proof of age may be requested.".into(),
                    "Fares are not refunded in cash.".into(),
                ],
            },
            GuideSection {
                heading: "Luggage and pets".into(),
                items: vec![
                    "Each passenger may carry two pieces of hand luggage, each up to 30 x 50 x 81 cm.".into(),
                    "Luggage must not block seats, aisles or emergency exits; if more than 20 pieces are on board, items may be carried on the next ferry.".into(),
                    "Pets need a form from pier staff and must be restrained and muzzled or carried in a secure container; guide dogs are exempt.".into(),
                ],
            },
            GuideSection {
                heading: "On board".into(),
                items: vec![
                    "Remain seated until the ferry has berthed and the gangplank is lowered.".into(),
                    "No smoking, dangerous goods, gambling or behaviour that obstructs or offends others.".into(),
                    "Follow crew instructions and note emergency exits and lifejackets.".into(),
                ],
            },
            GuideSection {
                heading: "Bus passengers".into(),
                items: vec![
                    "Official bus notes: https://www.pitcl.com.hk/attachment/notes_to_passengers.pdf".into(),
                    "Resident bus fares require a registered Octopus card; otherwise the visitor fare applies.".into(),
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guide_topic_is_available() {
        for topic in TOPICS {
            let guide = guide(topic).unwrap_or_else(|| panic!("missing {topic}"));
            assert!(!guide.sections.is_empty());
            assert!(!guide.source.is_empty());
        }
    }

    #[test]
    fn weather_guide_covers_the_suspension_threshold() {
        let guide = guide("typhoon").unwrap();
        let text = guide
            .sections
            .iter()
            .flat_map(|section| section.items.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("75 km/h"));
        assert!(text.contains("2 hours"));
    }
}
