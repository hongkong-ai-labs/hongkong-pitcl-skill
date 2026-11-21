//! Small in-process response cache shared by the weather, traffic and news
//! clients. Live transport clients keep their own caches because their keys
//! are structured; this one stores raw response bodies.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct Entry {
    at: Instant,
    body: String,
}

pub struct ResponseCache {
    ttl: Duration,
    entries: Mutex<HashMap<String, Entry>>,
}

impl ResponseCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        if self.ttl == Duration::ZERO {
            return None;
        }
        let entries = self.entries.lock().ok()?;
        let entry = entries.get(key)?;
        (entry.at.elapsed() < self.ttl).then(|| entry.body.clone())
    }

    pub fn insert(&self, key: impl Into<String>, body: String) {
        if self.ttl == Duration::ZERO {
            return;
        }
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(
                key.into(),
                Entry {
                    at: Instant::now(),
                    body,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_until_disabled() {
        let cache = ResponseCache::new(Duration::from_secs(60));
        cache.insert("k", "v".into());
        assert_eq!(cache.get("k").as_deref(), Some("v"));

        let disabled = ResponseCache::new(Duration::ZERO);
        disabled.insert("k", "v".into());
        assert!(disabled.get("k").is_none());
    }
}
