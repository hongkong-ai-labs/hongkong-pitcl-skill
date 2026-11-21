//! GeoJSON helpers for route shapes published by the PITCL/Socif feed.

use serde::Serialize;

/// A GeoJSON LineString. Coordinates are `[longitude, latitude]` in WGS84,
/// as required by RFC 7946.
#[derive(Debug, Clone, Serialize)]
pub struct LineString {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub coordinates: Vec<[f64; 2]>,
}

impl LineString {
    pub fn from_encoded(shape: &str) -> Option<Self> {
        let coordinates = decode_polyline(shape)?;
        (coordinates.len() >= 2).then_some(Self {
            kind: "LineString",
            coordinates,
        })
    }
}

/// Decode a Google encoded polyline into `[longitude, latitude]` pairs.
/// Returns `None` for malformed input rather than emitting a partial path.
pub fn decode_polyline(encoded: &str) -> Option<Vec<[f64; 2]>> {
    let bytes = encoded.as_bytes();
    let mut index = 0;
    let mut latitude = 0i64;
    let mut longitude = 0i64;
    let mut coordinates = Vec::new();

    while index < bytes.len() {
        let (lat_delta, next) = decode_value(bytes, index)?;
        index = next;
        let (lon_delta, next) = decode_value(bytes, index)?;
        index = next;
        latitude = latitude.checked_add(lat_delta)?;
        longitude = longitude.checked_add(lon_delta)?;
        coordinates.push([longitude as f64 / 100_000.0, latitude as f64 / 100_000.0]);
    }

    Some(coordinates)
}

fn decode_value(bytes: &[u8], mut index: usize) -> Option<(i64, usize)> {
    let mut result = 0i64;
    let mut shift = 0u32;
    loop {
        let byte = *bytes.get(index)?;
        index += 1;
        let chunk = byte.checked_sub(63)?;
        if chunk > 63 {
            return None;
        }
        result |= i64::from(chunk & 0x1f) << shift;
        shift += 5;
        if chunk < 0x20 {
            break;
        }
        if shift > 60 {
            return None;
        }
    }
    let delta = if result & 1 == 1 {
        !(result >> 1)
    } else {
        result >> 1
    };
    Some((delta, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_standard_polyline_example() {
        // https://developers.google.com/maps/documentation/utilities/polylinealgorithm
        let decoded = decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@").unwrap();
        assert_eq!(decoded.len(), 3);
        assert!((decoded[0][0] - -120.2).abs() < 0.00001);
        assert!((decoded[0][1] - 38.5).abs() < 0.00001);
        assert!((decoded[2][0] - -126.453).abs() < 0.00001);
        assert!((decoded[2][1] - 43.252).abs() < 0.00001);
    }

    #[test]
    fn rejects_truncated_shapes() {
        assert!(decode_polyline("_p~iF").is_none());
        assert!(decode_polyline("").unwrap().is_empty());
    }

    #[test]
    fn builds_a_geojson_linestring() {
        let line = LineString::from_encoded("_p~iF~ps|U_ulLnnqC").unwrap();
        assert_eq!(line.kind, "LineString");
        assert_eq!(line.coordinates.len(), 2);
    }
}
