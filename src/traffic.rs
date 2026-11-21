//! Transport Department special traffic news.
//!
//! Official real-time XML:
//! `https://resource.data.one.gov.hk/td/en/specialtrafficnews.xml`
//! (and `/tc/` for Traditional Chinese). The feed covers territory-wide
//! incidents; `relevant` flags messages that mention the Ma Wan corridor,
//! ferries, the airport or connecting districts.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Serialize;

use crate::cache::ResponseCache;

const BASE: &str = "https://resource.data.one.gov.hk/td";
const USER_AGENT: &str = "mawan/0.1 (Ma Wan transport CLI; +https://github.com/)";

#[derive(Debug, Clone, Serialize)]
pub struct TrafficAlert {
    pub id: String,
    /// `new` or `updated` (the feed's republished status).
    pub status: String,
    pub headline: String,
    pub detail: String,
    pub districts: Vec<String>,
    /// Normalised reference time, e.g. `2026-11-21 22:58`.
    pub reference: String,
    /// Mentions Ma Wan, the ferry piers, the airport corridor or a connecting district.
    pub relevant: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrafficReport {
    pub source: String,
    pub total: usize,
    pub relevant: usize,
    /// True when the report includes non-Ma-Wan messages.
    pub all: bool,
    pub alerts: Vec<TrafficAlert>,
}

pub struct TrafficClient {
    http: reqwest::Client,
    cache: ResponseCache,
}

impl TrafficClient {
    pub fn new(ttl: Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(20))
            .build()
            .context("building traffic client")?;
        Ok(Self {
            http,
            cache: ResponseCache::new(ttl),
        })
    }

    pub async fn report(&self, lang: &str, all: bool, limit: usize) -> Result<TrafficReport> {
        let url = format!("{BASE}/{lang}/specialtrafficnews.xml");
        let body = if let Some(body) = self.cache.get(&url) {
            body
        } else {
            let response = self
                .http
                .get(&url)
                .header(reqwest::header::ACCEPT, "application/xml")
                .send()
                .await
                .with_context(|| format!("GET {url}"))?;
            let status = response.status();
            let body = response
                .text()
                .await
                .with_context(|| format!("reading {url}"))?;
            if !status.is_success() {
                bail!(
                    "TD traffic feed returned HTTP {status}: {}",
                    body.chars().take(160).collect::<String>()
                );
            }
            self.cache.insert(&url, body.clone());
            body
        };
        let mut alerts = parse_xml(&body, lang)?;
        let total = alerts.len();
        let relevant = alerts.iter().filter(|alert| alert.relevant).count();
        if !all {
            alerts.retain(|alert| alert.relevant);
        }
        if limit > 0 {
            alerts.truncate(limit);
        }
        Ok(TrafficReport {
            source: url,
            total,
            relevant,
            all,
            alerts,
        })
    }
}

#[derive(Default)]
struct Builder {
    id: String,
    status: String,
    chin_text: String,
    chin_short: String,
    eng_text: String,
    eng_short: String,
    reference: String,
    districts: Vec<String>,
    district: String,
}

impl Builder {
    fn finish(self, lang: &str) -> TrafficAlert {
        let (headline_src, detail_src) = if lang == "tc" {
            (&self.chin_short, &self.chin_text)
        } else {
            (&self.eng_short, &self.eng_text)
        };
        let fallback = if lang == "tc" {
            &self.eng_short
        } else {
            &self.chin_short
        };
        let headline = if headline_src.trim().is_empty() {
            fallback.trim().to_string()
        } else {
            headline_src.trim().to_string()
        };
        let detail = if detail_src.trim().is_empty() {
            headline.clone()
        } else {
            detail_src.trim().to_string()
        };
        let status = match self.status.trim() {
            "2" => "new".to_string(),
            "3" => "updated".to_string(),
            other => other.to_string(),
        };
        let relevant = is_relevant(&headline, &detail, &self.districts);
        TrafficAlert {
            id: self.id.trim().to_string(),
            status,
            headline: headline.trim().to_string(),
            detail: detail.trim().to_string(),
            districts: self.districts,
            reference: normalize_reference(&self.reference),
            relevant,
        }
    }
}

fn parse_xml(xml: &str, lang: &str) -> Result<Vec<TrafficAlert>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut alerts = Vec::new();
    let mut current: Option<Builder> = None;
    let mut field = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name());
                if name == "message" {
                    current = Some(Builder::default());
                    field.clear();
                } else if current.is_some() {
                    if name == "District"
                        && let Some(builder) = current.as_mut()
                    {
                        builder.district.clear();
                    }
                    field = name;
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(builder) = current.as_mut() {
                    let raw = text.xml_content(quick_xml::XmlVersion::Explicit1_0);
                    let value = quick_xml::escape::unescape(&raw)
                        .map(|value| value.into_owned())
                        .unwrap_or_else(|_| raw.into_owned());
                    // A text node can be split at entity boundaries (for
                    // example `&#034;` in the Chinese feed), so append rather
                    // than replace.
                    match field.as_str() {
                        "msgID" => builder.id.push_str(&value),
                        "CurrentStatus" => builder.status.push_str(&value),
                        "ChinText" => builder.chin_text.push_str(&value),
                        "ChinShort" => builder.chin_short.push_str(&value),
                        "EngText" => builder.eng_text.push_str(&value),
                        "EngShort" => builder.eng_short.push_str(&value),
                        "ReferenceDate" => builder.reference.push_str(&value),
                        "District" => builder.district.push_str(&value),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let name = local_name(event.name());
                if name == "message"
                    && let Some(builder) = current.take()
                {
                    alerts.push(builder.finish(lang));
                } else if name == "District"
                    && let Some(builder) = current.as_mut()
                {
                    let district = builder.district.trim();
                    if !district.is_empty() {
                        builder.districts.push(district.to_string());
                    }
                    builder.district.clear();
                }
                field.clear();
            }
            Ok(Event::Eof) => break,
            Err(error) => bail!("parsing TD traffic XML: {error}"),
            _ => {}
        }
    }
    Ok(alerts)
}

fn local_name(name: quick_xml::name::QName<'_>) -> String {
    let name = name.as_ref();
    name.rsplit(':').next().unwrap_or(name).to_string()
}

fn normalize_reference(value: &str) -> String {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() >= 3 {
        let date = parts[0].replace('/', "-");
        let ampm = parts[1];
        if matches!(ampm, "上午" | "下午" | "AM" | "PM") {
            let time = parts[2];
            let mut time = time.split(':');
            if let (Some(hour), Some(minute)) = (time.next(), time.next())
                && let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>())
            {
                let mut hour = hour % 12;
                if ampm == "下午" || ampm == "PM" {
                    hour += 12;
                }
                return format!("{date} {hour:02}:{minute:02}");
            }
        }
    }
    value.trim().to_string()
}

fn is_relevant(headline: &str, detail: &str, districts: &[String]) -> bool {
    let haystack = format!("{headline} {detail} {}", districts.join(" ")).to_lowercase();
    if is_rail_only(&haystack) {
        return false;
    }
    const KEYWORDS: &[&str] = &[
        "ma wan",
        "park island",
        "tsing ma bridge",
        "tsing yi",
        "kwai fong",
        "tsuen wan",
        "lantau link",
        "north lantau highway",
        "airport station",
        "airport terminal",
        "hong kong international airport",
        "chek lap kok",
        "hzmb",
        "hong kong port",
        "western harbour crossing",
        "west kowloon highway",
        "central ferry",
        "ferry pier",
        "ferry service",
        "ferry services",
        "typhoon",
        "rainstorm",
        "visibility",
        "strong wind",
        "馬灣",
        "珀麗灣",
        "青馬大橋",
        "青衣",
        "葵芳",
        "荃灣",
        "青嶼幹線",
        "北大嶼山公路",
        "機場站",
        "機場客運大樓",
        "香港國際機場",
        "赤鱲角",
        "港珠澳",
        "香港口岸",
        "西區海底隧道",
        "西九龍公路",
        "中環渡輪",
        "渡輪碼頭",
        "渡輪服務",
        "颱風",
        "暴雨",
        "能見度",
        "強風",
    ];
    KEYWORDS.iter().any(|keyword| haystack.contains(keyword))
}

/// Railway and high-speed-rail service notices are not road traffic and do
/// not affect the Ma Wan buses or ferries, even when they mention a station
/// in a connecting district.
fn is_rail_only(text: &str) -> bool {
    const RAIL: &[&str] = &[
        "high speed rail",
        "mtr",
        "train service",
        "railway",
        "airport express",
        "tung chung line",
        "港鐵",
        "高鐵",
        "列車",
        "東涌綫",
        "機場快綫",
    ];
    const ROAD_OR_FERRY: &[&str] = &[
        "road",
        "highway",
        "lane",
        "traffic",
        "bridge",
        "tunnel",
        "ferry",
        "道路",
        "行車線",
        "交通",
        "橋",
        "隧道",
        "渡輪",
    ];
    RAIL.iter().any(|keyword| text.contains(keyword))
        && !ROAD_OR_FERRY.iter().any(|keyword| text.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<body xmlns='http://data.one.gov.hk/td'>
<message>
<msgID>1</msgID>
<CurrentStatus>3</CurrentStatus>
<ChinText>因颱風，馬灣渡輪服務暫停。</ChinText>
<ChinShort>馬灣渡輪暫停</ChinShort>
<EngText>Due to typhoon, ferry services to Ma Wan are suspended.</EngText>
<EngShort>Ma Wan ferry suspended</EngShort>
<ReferenceDate> 2026/11/21 下午 10:58:14 </ReferenceDate>
<IncidentRefNo></IncidentRefNo>
<CountofDistricts>1</CountofDistricts>
<ListOfDistrict><District>Tsuen Wan</District></ListOfDistrict>
</message>
<message>
<msgID>2</msgID>
<CurrentStatus>2</CurrentStatus>
<ChinText>北區水管工程。</ChinText>
<ChinShort>北區水管工程</ChinShort>
<EngText>Watermain works in North District.</EngText>
<EngShort>North District watermain works</EngShort>
<ReferenceDate> 2026/11/21 下午 09:00:00 </ReferenceDate>
<IncidentRefNo></IncidentRefNo>
<CountofDistricts>1</CountofDistricts>
<ListOfDistrict><District>North</District></ListOfDistrict>
</message>
</body>"#;

    #[test]
    fn parses_and_flags_relevant_traffic() {
        let alerts = parse_xml(SAMPLE, "en").unwrap();
        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].id, "1");
        assert_eq!(alerts[0].status, "updated");
        assert_eq!(alerts[0].reference, "2026-11-21 22:58");
        assert!(alerts[0].relevant);
        assert!(!alerts[1].relevant);
        assert_eq!(alerts[0].districts, vec!["Tsuen Wan"]);
    }

    #[test]
    fn selects_chinese_text_when_requested() {
        let alerts = parse_xml(SAMPLE, "tc").unwrap();
        assert_eq!(alerts[0].headline, "馬灣渡輪暫停");
        assert!(alerts[0].relevant);
    }
}
