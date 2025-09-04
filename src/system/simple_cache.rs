//! Simple Analysis Cache - замена сложного AnalysisCacheManager
//!
//! Простой LRU кеш с TTL вместо многоуровневого L1+L2 кеширования

use anyhow::Result;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};
use tracing::debug;

use crate::domain::types::TypeResolution;

/// Простой анализ кеш с LRU eviction и TTL
pub struct AnalysisCache {
    storage: lru::LruCache<FileHash, AnalysisResult>,
    ttl_tracker: HashMap<FileHash, Instant>,
    default_ttl: Duration,
}

/// Хеш файла для кеширования  
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHash {
    pub content_hash: String,
    pub file_path: String,
}

impl Hash for FileHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content_hash.hash(state);
        self.file_path.hash(state);
    }
}

impl FileHash {
    pub fn from_content(file_path: &str, content: &str) -> Self {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Self {
            content_hash: hash,
            file_path: file_path.to_string(),
        }
    }
}

/// Результат анализа для кеширования
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub file_path: String,
    pub type_resolutions: HashMap<String, TypeResolution>,
    pub analysis_duration_ms: u64,
    pub cached_at: Instant,
}

impl AnalysisCache {
    /// Создать новый простой кеш
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: lru::LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap()),
            ttl_tracker: HashMap::new(),
            default_ttl: Duration::from_secs(300), // 5 минут TTL
        }
    }

    /// Получить из кеша с TTL проверкой
    pub fn get(&mut self, file_hash: &FileHash) -> Option<AnalysisResult> {
        // Simple TTL check
        if let Some(timestamp) = self.ttl_tracker.get(file_hash) {
            if timestamp.elapsed() > self.default_ttl {
                debug!("Cache entry expired for {:?}", file_hash.file_path);
                self.storage.pop(file_hash);
                self.ttl_tracker.remove(file_hash);
                return None;
            }
        }

        self.storage.get(file_hash).cloned()
    }

    /// Добавить в кеш с TTL
    pub fn insert(&mut self, file_hash: FileHash, result: AnalysisResult) {
        self.storage.put(file_hash.clone(), result);
        self.ttl_tracker.insert(file_hash, Instant::now());
    }

    /// Прогрев кеша (simple version)
    pub fn warm_cache(&self) -> Result<()> {
        debug!("Simple cache warming completed");
        Ok(())
    }

    /// Простая статистика кеша
    pub fn cache_stats(&self) -> SimpleCacheStats {
        SimpleCacheStats {
            current_size: self.storage.len(),
            max_capacity: self.storage.cap().get(),
            ttl_entries: self.ttl_tracker.len(),
        }
    }

    /// Очистка устаревших записей
    pub fn cleanup_expired(&mut self) {
        let expired_keys: Vec<_> = self
            .ttl_tracker
            .iter()
            .filter_map(|(key, timestamp)| {
                if timestamp.elapsed() > self.default_ttl {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            self.storage.pop(&key);
            self.ttl_tracker.remove(&key);
        }

        debug!(
            "Cache cleanup: removed {} expired entries",
            self.ttl_tracker.len().saturating_sub(self.storage.len())
        );
    }
}

/// Простая статистика кеша (вместо сложной CacheStats)
#[derive(Debug, Clone)]
pub struct SimpleCacheStats {
    pub current_size: usize,
    pub max_capacity: usize,
    pub ttl_entries: usize,
}

impl SimpleCacheStats {
    pub fn utilization_percentage(&self) -> f64 {
        if self.max_capacity == 0 {
            0.0
        } else {
            (self.current_size as f64 / self.max_capacity as f64) * 100.0
        }
    }
}

// === COMPARISON WITH COMPLEX CACHE ===

/// Сравнение: Simple vs Complex caching
///
/// Complex (AnalysisCacheManager):
/// - Memory cache + Disk cache  
/// - CacheStats with hits/misses/invalidations
/// - Специализированные кеши (TypeLRUCache, etc)
/// - ~200+ LOC
///
/// Simple (AnalysisCache):  
/// - LRU cache + TTL tracking
/// - Simple statistics
/// - ~100 LOC
///
/// Экономия: ~50% сложности, сохраняет основную функциональность
#[cfg(test)]
mod comparison_notes {
    //! Сравнение: Simple vs Complex caching
    //!
    //! Complex (AnalysisCacheManager):
    //! - Memory cache + Disk cache  
    //! - CacheStats with hits/misses/invalidations
    //! - Специализированные кеши (TypeLRUCache, etc)
    //! - ~200+ LOC
    //!
    //! Simple (AnalysisCache):  
    //! - LRU cache + TTL tracking
    //! - Simple statistics
    //! - ~100 LOC
    //!
    //! Экономия: ~50% сложности, сохраняет основную функциональность
}
