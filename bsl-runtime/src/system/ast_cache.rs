//! AST cache for parsed BSL modules.
//!
//! Provides an in-memory LRU cache for ParseResult to avoid repeated parsing
//! when opening or reusing the same content.

use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

use lru::LruCache;

use crate::parsing::ParseResult;
use crate::system::runtime_config::{global_runtime_config, RuntimeKey};

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct AstCacheStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub entries: usize,
    pub capacity: usize,
}

pub struct AstCache {
    storage: Arc<RwLock<LruCache<[u8; 32], Arc<ParseResult>>>>,
    stats: Arc<RwLock<AstCacheStats>>,
}

impl AstCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::MIN),
            ))),
            stats: Arc::new(RwLock::new(AstCacheStats::default())),
        }
    }

    pub fn new_from_env() -> Self {
        let capacity = global_runtime_config()
            .get_usize(RuntimeKey::AstCacheCapacity)
            .unwrap_or(64);
        Self::new(capacity)
    }

    pub fn get(&self, key: [u8; 32]) -> Option<Arc<ParseResult>> {
        let mut storage = self
            .storage
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = storage.get(&key).cloned();

        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if result.is_some() {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }

        result
    }

    pub fn take_if_unique(&self, key: [u8; 32]) -> Option<ParseResult> {
        let mut storage = self
            .storage
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = storage.pop(&key);

        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = entry {
            stats.hits += 1;
            match Arc::try_unwrap(entry) {
                Ok(parse_result) => Some(parse_result),
                Err(shared_entry) => {
                    storage.put(key, shared_entry);
                    None
                }
            }
        } else {
            stats.misses += 1;
            None
        }
    }

    pub fn put(&self, key: [u8; 32], value: Arc<ParseResult>) {
        let mut storage = self
            .storage
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let will_evict = storage.len() >= storage.cap().get() && !storage.contains(&key);

        if will_evict {
            let mut stats = self
                .stats
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            stats.evictions += 1;
        }

        storage.put(key, value);
    }

    pub fn clear(&self) {
        let mut storage = self
            .storage
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        storage.clear();
    }

    pub fn stats(&self) -> AstCacheStats {
        let stats = self
            .stats
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let storage = self
            .storage
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut snapshot = stats.clone();
        snapshot.entries = storage.len();
        snapshot.capacity = storage.cap().get();
        snapshot
    }
}

#[cfg(test)]
#[path = "ast_cache/tests.rs"]
mod tests;
