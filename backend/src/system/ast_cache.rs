//! AST cache for parsed BSL modules.
//!
//! Provides an in-memory LRU cache for ParseResult to avoid repeated parsing
//! when opening or reusing the same content.

use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

use lru::LruCache;

use crate::parsing::ParseResult;

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
        let capacity = std::env::var("BSL_AST_CACHE_CAPACITY")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(64);
        Self::new(capacity)
    }

    pub fn get(&self, key: [u8; 32]) -> Option<Arc<ParseResult>> {
        let mut storage = self.storage.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        let result = storage.get(&key).cloned();

        let mut stats = self.stats.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if result.is_some() {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }

        result
    }

    pub fn put(&self, key: [u8; 32], value: Arc<ParseResult>) {
        let mut storage = self.storage.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        let will_evict = storage.len() >= storage.cap().get() && !storage.contains(&key);

        if will_evict {
            let mut stats = self.stats.write().unwrap_or_else(|poisoned| poisoned.into_inner());
            stats.evictions += 1;
        }

        storage.put(key, value);
    }

    pub fn clear(&self) {
        let mut storage = self.storage.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        storage.clear();
    }

    pub fn stats(&self) -> AstCacheStats {
        let stats = self.stats.read().unwrap_or_else(|poisoned| poisoned.into_inner());
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
mod tests {
    use super::*;
    use crate::parsing::bsl::ast::Program;
    use crate::parsing::ParseResult;

    #[test]
    fn ast_cache_hits_misses_and_evictions() {
        let cache = AstCache::new(1);
        let parse = ParseResult::success(Program { statements: vec![] });

        assert!(cache.get([1; 32]).is_none());

        cache.put([1; 32], Arc::new(parse.clone()));
        assert!(cache.get([1; 32]).is_some());

        cache.put([2; 32], Arc::new(parse));
        assert!(cache.get([1; 32]).is_none());

        let stats = cache.stats();
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.evictions, 1);
    }
}
