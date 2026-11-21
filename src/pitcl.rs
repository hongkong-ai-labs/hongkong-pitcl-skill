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

use crate::data::LAST_VERIFIED;

pub const TOPICS: &[&str] = &["contact", "weather", "policies"];

#[derive(Debug, Clone, Serialize)]
pub struct Guide {
    pub topic: String,
    pub title: String,
    pub source: String,
    pub effective: Option<String>,
    pub last_verified: String,
    pub note: String,
    pub sections: Vec<GuideSection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuideSection {
    pub heading: String,
    pub items: Vec<String>,
}

pub fn guide(topic: &str, lang: &str) -> Option<Guide> {
    match canonical(topic)? {
        "contact" => Some(contact(lang)),
        "weather" => Some(weather(lang)),
        "policies" => Some(policies(lang)),
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

fn pick<'a>(lang: &str, en: &'a str, tc: &'a str) -> &'a str {
    if lang == "tc" { tc } else { en }
}

fn section(lang: &str, heading_en: &str, heading_tc: &str, items: &[(&str, &str)]) -> GuideSection {
    GuideSection {
        heading: pick(lang, heading_en, heading_tc).to_string(),
        items: items
            .iter()
            .map(|(en, tc)| pick(lang, en, tc).to_string())
            .collect(),
    }
}

fn contact(lang: &str) -> Guide {
    Guide {
        topic: "contact".into(),
        title: pick(
            lang,
            "PITCL contact and resident fare registration",
            "珀麗灣客運聯絡資料及居民票價登記",
        )
        .into(),
        source: "https://www.pitcl.com.hk/en/customer_service.php?id=1".into(),
        effective: None,
        last_verified: LAST_VERIFIED.into(),
        note: pick(
            lang,
            "Contact details can change; check the official page before relying on them.",
            "聯絡資料可能更改；使用前請查看官方網頁。",
        )
        .into(),
        sections: vec![
            section(
                lang,
                "General enquiry",
                "一般查詢",
                &[
                    ("Hotline: 2946 8888", "熱線：2946 8888"),
                    ("Fax: 2525 5556", "傳真：2525 5556"),
                    ("Email: enquiry@pitcl.com.hk", "電郵：enquiry@pitcl.com.hk"),
                    (
                        "Address: 1/F, Park Island Ferry Pier, 8 Pak Lai Road, Ma Wan, N.T., Hong Kong",
                        "地址：香港新界馬灣珀萊路8號珀麗灣碼頭1樓",
                    ),
                    (
                        "Office hours: Monday to Friday 09:00-18:15 (lunch 13:00-14:20)",
                        "辦公時間：星期一至五 09:00-18:15（午膳 13:00-14:20）",
                    ),
                ],
            ),
            section(
                lang,
                "Resident fares and feedback",
                "居民票價及意見",
                &[
                    (
                        "Resident bus and ferry fares require a registered Octopus card.",
                        "居民巴士及渡輪票價須使用已登記八達通卡。",
                    ),
                    (
                        "Registration and change-of-data forms: https://www.pitcl.com.hk/en/customer_service.php?id=2",
                        "登記及更改資料表格：https://www.pitcl.com.hk/en/customer_service.php?id=2",
                    ),
                    (
                        "Customer feedback form: https://www.pitcl.com.hk/en/customer_service.php?id=3",
                        "客戶意見表格：https://www.pitcl.com.hk/en/customer_service.php?id=3",
                    ),
                    (
                        "Private hire enquiries: 2946 8899 or enquiry@pitcl.com.hk.",
                        "租車查詢：2946 8899 或 enquiry@pitcl.com.hk。",
                    ),
                ],
            ),
            section(
                lang,
                "Lost property",
                "失物",
                &[
                    (
                        "Ferry lost property: ask the pier staff.",
                        "渡輪失物：請向碼頭職員查詢。",
                    ),
                    (
                        "Other enquiries: call 2946 8888.",
                        "其他查詢：請致電 2946 8888。",
                    ),
                ],
            ),
        ],
    }
}

fn weather(lang: &str) -> Guide {
    Guide {
        topic: "weather".into(),
        title: pick(
            lang,
            "PITCL high wind and typhoon arrangements",
            "珀麗灣客運強風及颱風安排",
        )
        .into(),
        source:
            "https://www.pitcl.com.hk/attachment/contingency_traffic_arrangement_eng.pdf".into(),
        effective: Some("2018-08-14".into()),
        last_verified: LAST_VERIFIED.into(),
        note: pick(
            lang,
            "Published contingency plan (PAX Notice 2018.08). Actual arrangements depend on conditions and Transport Department approval; check PITCL notices, pier/bus-stop boards and HKO warnings.",
            "公布的應變計劃（PAX Notice 2018.08）。實際安排取決於當時情況及運輸署批准；請查看珀麗灣客運通告、碼頭／巴士站告示及天文台警告。",
        )
        .into(),
        sections: vec![
            section(
                lang,
                "High wind on Tsing Ma Bridge",
                "青馬大橋強風",
                &[
                    (
                        "40-60 km/h: bus services may be delayed.",
                        "40至60公里每小時：巴士服務可能延誤。",
                    ),
                    (
                        "60-75 km/h with Stage 1 high-wind management: PITCL uses approved super-low-floor buses on the upper deck; services may be strengthened until 75 km/h; delays are possible and KMB Y41 contingency support may be arranged.",
                        "60至75公里每小時並實施第一階段強風交通管理：珀麗灣客運使用獲准行走上層橋面的超低地台巴士；服務可能加強至75公里每小時；可能延誤，並可能安排九巴Y41應急支援。",
                    ),
                    (
                        "Above 75 km/h with Stage 2: bus services are suspended, including KMB Y41. Tsuen Wan ferry is strengthened; a 400-seat ferry or extra sailings may be used. Free shuttle buses link Tsing Yi Station, Tsuen Wan Station, Kwai Fong Metroplaza and Tsuen Wan Ferry Pier until ferry service stops.",
                        "超過75公里每小時並實施第二階段：巴士服務暫停，包括九巴Y41。荃灣渡輪會加強服務，可能使用400座位渡輪或加開班次。渡輪服務停止前，設免費穿梭巴士連接青衣站、荃灣站、葵芳新都會廣場及荃灣渡輪碼頭。",
                    ),
                    (
                        "Below 75 km/h: bus services resume; KMB Y41 support may be used if needed.",
                        "低於75公里每小時：巴士服務恢復；如有需要可能使用九巴Y41支援。",
                    ),
                    (
                        "Below 60 km/h: normal bus service resumes.",
                        "低於60公里每小時：巴士服務回復正常。",
                    ),
                ],
            ),
            section(
                lang,
                "Typhoon signals",
                "颱風信號",
                &[
                    (
                        "Signal 1: ferry services may be delayed by sea conditions.",
                        "一號風球：渡輪服務可能受海面情況延誤。",
                    ),
                    (
                        "Signal 3: limited service or suspension is possible before or shortly after Signal 8.",
                        "三號風球：八號風球前後可能有限度服務或暫停。",
                    ),
                    (
                        "Signal 8 expected within 2 hours: PITCL monitors sea state, positions a spare vessel at Tsuen Wan Pier and prepares a suspension schedule.",
                        "預計兩小時內發出八號風球：珀麗灣客運監察海面情況，在荃灣碼頭安排後備船隻，並準備暫停航班時間表。",
                    ),
                    (
                        "Signal 8 hoisted: suspension notices are posted at the piers; Tsuen Wan ferry is strengthened until crowds clear; the Central route may use a 400-seat ferry. Under Marine Department rules, ferries suspend about 2 hours after Signal 8.",
                        "八號風球發出：碼頭張貼暫停服務通告；荃灣渡輪加強服務直至人群疏導；中環航線可能使用400座位渡輪。根據海事處規定，渡輪在八號風球發出約兩小時後暫停。",
                    ),
                    (
                        "Signal 3 lowered: services resume as soon as possible, with extra sailings if needed.",
                        "三號風球除下：服務盡快恢復，如有需要加開班次。",
                    ),
                    (
                        "Signal 1 lowered: ferries resume but may still be delayed.",
                        "一號風球除下：渡輪恢復，但仍可能延誤。",
                    ),
                ],
            ),
            section(
                lang,
                "Where to check",
                "查閱途徑",
                &[
                    (
                        "HKO warnings and forecast: `mawan weather`.",
                        "天文台警告及預報：`mawan weather`。",
                    ),
                    ("PITCL notices: `mawan news`.", "珀麗灣客運通告：`mawan news`。"),
                    ("Road incidents: `mawan traffic`.", "道路事故：`mawan traffic`。"),
                    ("PITCL hotline: 2946 8888.", "珀麗灣客運熱線：2946 8888。"),
                ],
            ),
        ],
    }
}

fn policies(lang: &str) -> Guide {
    Guide {
        topic: "policies".into(),
        title: pick(lang, "PITCL passenger policies", "珀麗灣客運乘客守則").into(),
        source: "https://www.pitcl.com.hk/attachment/notes_to_passengers_ferry.pdf".into(),
        effective: None,
        last_verified: LAST_VERIFIED.into(),
        note: pick(
            lang,
            "Ferry rules below are from the official Notes to Ferry Passengers. The bus notes are published as an image PDF; use the link for the official bus version.",
            "以下渡輪規則來自官方《渡輪乘客須知》。巴士乘客須知以圖像PDF公布；請使用連結查看官方巴士版本。",
        )
        .into(),
        sections: vec![
            section(
                lang,
                "Ferry fares",
                "渡輪票價",
                &[
                    (
                        "Every passenger must pay the appropriate fare before boarding.",
                        "每位乘客登船前須繳付適當票價。",
                    ),
                    (
                        "Children under 1 travelling with an adult, without occupying a seat, travel free.",
                        "一歲以下由成人陪同而不佔座位的兒童可免費乘船。",
                    ),
                    (
                        "Children under 12 and passengers aged 65 or above pay half fare; proof of age may be requested.",
                        "十二歲以下兒童及六十五歲或以上乘客可享半價；可能需要出示年齡證明。",
                    ),
                    ("Fares are not refunded in cash.", "票價不以現金退還。"),
                ],
            ),
            section(
                lang,
                "Luggage and pets",
                "行李及寵物",
                &[
                    (
                        "Each passenger may carry two pieces of hand luggage, each up to 30 x 50 x 81 cm.",
                        "每位乘客可攜帶兩件手提行李，每件尺寸上限為30 x 50 x 81厘米。",
                    ),
                    (
                        "Luggage must not block seats, aisles or emergency exits; if more than 20 pieces are on board, items may be carried on the next ferry.",
                        "行李不得阻塞座位、通道或緊急出口；如船上行李超過20件，物品可能須改由下一班渡輪運載。",
                    ),
                    (
                        "Pets need a form from pier staff and must be restrained and muzzled or carried in a secure container; guide dogs are exempt.",
                        "寵物須向碼頭職員索取表格，並須受約束及戴上口罩，或放置於穩固容器內；導盲犬除外。",
                    ),
                ],
            ),
            section(
                lang,
                "On board",
                "船上",
                &[
                    (
                        "Remain seated until the ferry has berthed and the gangplank is lowered.",
                        "渡輪泊岸及跳板放下前，請留在座位。",
                    ),
                    (
                        "No smoking, dangerous goods, gambling or behaviour that obstructs or offends others.",
                        "船上禁止吸煙、攜帶危險品、賭博，以及阻礙或冒犯他人的行為。",
                    ),
                    (
                        "Follow crew instructions and note emergency exits and lifejackets.",
                        "遵從船員指示，並留意緊急出口及救生衣位置。",
                    ),
                ],
            ),
            section(
                lang,
                "Bus passengers",
                "巴士乘客",
                &[
                    (
                        "Official bus notes: https://www.pitcl.com.hk/attachment/notes_to_passengers.pdf",
                        "官方巴士乘客須知：https://www.pitcl.com.hk/attachment/notes_to_passengers.pdf",
                    ),
                    (
                        "Resident bus fares require a registered Octopus card; otherwise the visitor fare applies.",
                        "居民巴士票價須使用已登記八達通卡；否則適用訪客票價。",
                    ),
                ],
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guide_topic_is_available() {
        for topic in TOPICS {
            for lang in ["en", "tc"] {
                let guide = guide(topic, lang).unwrap_or_else(|| panic!("missing {topic}/{lang}"));
                assert!(!guide.sections.is_empty());
                assert!(!guide.source.is_empty());
            }
        }
    }

    #[test]
    fn weather_guide_covers_the_suspension_threshold() {
        let guide = guide("typhoon", "en").unwrap();
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

    #[test]
    fn chinese_guide_is_translated() {
        let guide = guide("weather", "tc").unwrap();
        assert!(guide.title.contains("颱風"));
        assert!(guide.sections[0].items.join(" ").contains("75公里"));
    }
}
