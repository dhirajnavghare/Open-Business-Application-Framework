use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Thread-safe in-memory Cache-Aside store with TTL expiration (System Design Primer pattern).
///
/// # Example
/// ```
/// use obaf_core::cache::TtlCache;
/// use std::time::Duration;
///
/// let cache = TtlCache::new(Duration::from_secs(60), 100);
/// cache.insert("user:1".to_string(), "Alice".to_string());
/// assert_eq!(cache.get(&"user:1".to_string()), Some("Alice".to_string()));
/// ```
#[derive(Clone)]
pub struct TtlCache<K, V> {
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Duration,
    max_capacity: usize,
}

impl<K: Eq + Hash + Clone, V: Clone> TtlCache<K, V> {
    /// Create a new TTL cache with default TTL and maximum entry capacity.
    pub fn new(default_ttl: Duration, max_capacity: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_capacity,
        }
    }

    /// Retrieve value from cache if present and not expired.
    pub fn get(&self, key: &K) -> Option<V> {
        let lock = self.entries.read().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = lock.get(key) {
            if Instant::now() < entry.expires_at {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Insert or update an entry using default TTL.
    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    /// Insert or update an entry with a specific custom TTL.
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut lock = self.entries.write().unwrap_or_else(|e| e.into_inner());

        // Capacity protection (DoS): purge expired items if over capacity
        if lock.len() >= self.max_capacity {
            let now = Instant::now();
            lock.retain(|_, entry| entry.expires_at > now);

            // If still at capacity after purging expired items, evict the entry closest to expiration
            if lock.len() >= self.max_capacity {
                if let Some(earliest_key) = lock
                    .iter()
                    .min_by_key(|(_, entry)| entry.expires_at)
                    .map(|(k, _)| k.clone())
                {
                    lock.remove(&earliest_key);
                }
            }
        }

        lock.insert(
            key,
            CacheEntry {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    /// Remove a specific entry by key.
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut lock = self.entries.write().unwrap_or_else(|e| e.into_inner());
        lock.remove(key).map(|e| e.value)
    }

    /// Total active (including potentially expired) entries count.
    pub fn len(&self) -> usize {
        let lock = self.entries.read().unwrap_or_else(|e| e.into_inner());
        lock.len()
    }

    /// Whether the cache is currently empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all entries from cache.
    pub fn clear(&self) {
        let mut lock = self.entries.write().unwrap_or_else(|e| e.into_inner());
        lock.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_ttl_cache_insertion_and_expiration() {
        let cache = TtlCache::new(Duration::from_millis(50), 10);
        cache.insert("preset:restaurant", "italian_bistro");

        assert_eq!(cache.get(&"preset:restaurant"), Some("italian_bistro"));

        // Wait for TTL expiration
        sleep(Duration::from_millis(60));
        assert_eq!(cache.get(&"preset:restaurant"), None);
    }

    #[test]
    fn test_ttl_cache_capacity_purge() {
        let cache = TtlCache::new(Duration::from_millis(20), 2);
        cache.insert("k1", 100);
        cache.insert("k2", 200);

        sleep(Duration::from_millis(30));
        // Over capacity insert triggers expired cleanup
        cache.insert("k3", 300);

        assert_eq!(cache.get(&"k1"), None);
        assert_eq!(cache.get(&"k2"), None);
        assert_eq!(cache.get(&"k3"), Some(300));
    }
}
