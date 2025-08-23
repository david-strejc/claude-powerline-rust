use anyhow::Result;
use dashmap::DashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Generic cache with TTL support
pub struct Cache<K, V> {
    data: Arc<DashMap<K, CacheEntry<V>>>,
    default_ttl: Duration,
}

struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

impl<K, V> Cache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            data: Arc::new(DashMap::new()),
            default_ttl,
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let entry = self.data.get(key)?;
        if Instant::now() > entry.expires_at {
            self.data.remove(key);
            None
        } else {
            Some(entry.value.clone())
        }
    }

    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let expires_at = Instant::now() + ttl;
        self.data.insert(key, CacheEntry { value, expires_at });
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        self.data.remove(key).map(|(_, entry)| entry.value)
    }

    pub fn clear_expired(&self) {
        let now = Instant::now();
        self.data.retain(|_, entry| now <= entry.expires_at);
    }

    pub fn clear(&self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<K, V> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            default_ttl: self.default_ttl,
        }
    }
}