//! Система кеширования для документации

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use super::hierarchy::TypeDocumentationFull;

/// Система кеширования документации
pub struct DocumentationCache {
    /// Кеш деталей типов
    type_details_cache: Arc<RwLock<HashMap<String, CacheEntry<TypeDocumentationFull>>>>,
    
    /// Кеш результатов поиска
    search_results_cache: Arc<RwLock<HashMap<String, CacheEntry<Vec<String>>>>>,
    
    /// Кеш иерархий
    hierarchy_cache: Arc<RwLock<HashMap<String, CacheEntry<String>>>>,
    
    /// Конфигурация кеша
    config: CacheConfig,
    
    /// Статистика кеша
    statistics: Arc<RwLock<CacheStatistics>>,
}

/// Запись в кеше с TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    /// Данные
    data: T,
    
    /// Время создания
    created_at: DateTime<Utc>,
    
    /// Время истечения
    expires_at: DateTime<Utc>,
    
    /// Количество обращений
    access_count: usize,
    
    /// Время последнего доступа
    last_accessed: DateTime<Utc>,
}

/// Конфигурация кеша
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Максимальный размер кеша типов
    pub max_type_cache_size: usize,
    
    /// Максимальный размер кеша поиска
    pub max_search_cache_size: usize,
    
    /// TTL для деталей типов (секунды)
    pub type_details_ttl: u64,
    
    /// TTL для результатов поиска (секунды)
    pub search_results_ttl: u64,
    
    /// Стратегия вытеснения
    pub eviction_strategy: EvictionStrategy,
    
    /// Интервал очистки (секунды)
    pub cleanup_interval_seconds: u64,
}

/// Стратегия вытеснения
#[derive(Debug, Clone)]
pub enum EvictionStrategy {
    LRU,   // Least Recently Used
    LFU,   // Least Frequently Used  
    TTL,   // Time To Live only
    FIFO,  // First In First Out
}

/// Статистика работы кеша
#[derive(Debug, Clone, Serialize)]
pub struct CacheStatistics {
    /// Общее количество обращений
    pub total_requests: usize,
    
    /// Количество попаданий
    pub cache_hits: usize,
    
    /// Количество промахов
    pub cache_misses: usize,
    
    /// Процент попаданий
    pub hit_rate: f64,
    
    /// Текущий размер кеша типов
    pub type_cache_size: usize,
    
    /// Текущий размер кеша поиска
    pub search_cache_size: usize,
    
    /// Количество вытеснений
    pub evictions: usize,
    
    /// Использование памяти (примерное, MB)
    pub estimated_memory_mb: f64,
}

impl DocumentationCache {
    /// Создать новый кеш
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }
    
    /// Создать кеш с конфигурацией
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            type_details_cache: Arc::new(RwLock::new(HashMap::new())),
            search_results_cache: Arc::new(RwLock::new(HashMap::new())),
            hierarchy_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            statistics: Arc::new(RwLock::new(CacheStatistics::default())),
        }
    }
    
    /// Получить детали типа из кеша
    pub async fn get_type_details(&self, type_id: &str) -> Option<TypeDocumentationFull> {
        self.update_statistics_request().await;
        
        let cache = self.type_details_cache.read().await;
        
        if let Some(entry) = cache.get(type_id) {
            if !self.is_expired(&entry.expires_at) {
                self.update_statistics_hit().await;
                let result = entry.data.clone();
                // Обновляем время доступа
                drop(cache);
                self.update_access_time(type_id).await;
                return Some(result);
            }
        }
        
        self.update_statistics_miss().await;
        None
    }
    
    /// Сохранить детали типа в кеш
    pub async fn store_type_details(&self, type_id: &str, details: &TypeDocumentationFull) {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.config.type_details_ttl as i64);
        
        let entry = CacheEntry {
            data: details.clone(),
            created_at: now,
            expires_at,
            access_count: 1,
            last_accessed: now,
        };
        
        let mut cache = self.type_details_cache.write().await;
        
        // Проверяем размер кеша и вытесняем при необходимости
        if cache.len() >= self.config.max_type_cache_size {
            self.evict_entries(&mut cache).await;
        }
        
        cache.insert(type_id.to_string(), entry);
        
        // Обновляем статистику
        self.update_cache_size_stats().await;
    }
    
    /// Получить процент попаданий в кеш
    pub async fn get_hit_rate(&self) -> f64 {
        let stats = self.statistics.read().await;
        stats.hit_rate
    }
    
    /// Получить полную статистику кеша
    pub async fn get_statistics(&self) -> CacheStatistics {
        self.statistics.read().await.clone()
    }
    
    /// Очистить весь кеш
    pub async fn clear_all(&self) {
        {
            let mut type_cache = self.type_details_cache.write().await;
            type_cache.clear();
        }
        
        {
            let mut search_cache = self.search_results_cache.write().await;
            search_cache.clear();
        }
        
        {
            let mut hierarchy_cache = self.hierarchy_cache.write().await;
            hierarchy_cache.clear();
        }
        
        // Сбрасываем статистику
        {
            let mut stats = self.statistics.write().await;
            *stats = CacheStatistics::default();
        }
    }
    
    /// Очистить просроченные записи
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        
        // Очищаем кеш типов
        {
            let mut cache = self.type_details_cache.write().await;
            cache.retain(|_, entry| !self.is_expired(&entry.expires_at));
        }
        
        // Очищаем кеш поиска
        {
            let mut cache = self.search_results_cache.write().await;
            cache.retain(|_, entry| !self.is_expired(&entry.expires_at));
        }
        
        self.update_cache_size_stats().await;
    }
    
    // Приватные методы
    
    fn is_expired(&self, expires_at: &DateTime<Utc>) -> bool {
        Utc::now() > *expires_at
    }
    
    async fn update_access_time(&self, type_id: &str) {
        let mut cache = self.type_details_cache.write().await;
        if let Some(entry) = cache.get_mut(type_id) {
            entry.last_accessed = Utc::now();
            entry.access_count += 1;
        }
    }
    
    async fn evict_entries(&self, cache: &mut HashMap<String, CacheEntry<TypeDocumentationFull>>) {
        match self.config.eviction_strategy {
            EvictionStrategy::LRU => {
                // Находим запись с самым старым временем доступа
                if let Some((oldest_key, _)) = cache.iter()
                    .min_by_key(|(_, entry)| entry.last_accessed) {
                    let oldest_key = oldest_key.clone();
                    cache.remove(&oldest_key);
                }
            }
            
            EvictionStrategy::LFU => {
                // Находим запись с наименьшим количеством обращений
                if let Some((least_used_key, _)) = cache.iter()
                    .min_by_key(|(_, entry)| entry.access_count) {
                    let least_used_key = least_used_key.clone();
                    cache.remove(&least_used_key);
                }
            }
            
            EvictionStrategy::TTL => {
                // Удаляем просроченные записи
                let now = Utc::now();
                cache.retain(|_, entry| entry.expires_at > now);
            }
            
            EvictionStrategy::FIFO => {
                // Находим самую старую запись
                if let Some((oldest_key, _)) = cache.iter()
                    .min_by_key(|(_, entry)| entry.created_at) {
                    let oldest_key = oldest_key.clone();
                    cache.remove(&oldest_key);
                }
            }
        }
        
        // Обновляем статистику вытеснений
        {
            let mut stats = self.statistics.write().await;
            stats.evictions += 1;
        }
    }
    
    async fn update_statistics_request(&self) {
        let mut stats = self.statistics.write().await;
        stats.total_requests += 1;
    }
    
    async fn update_statistics_hit(&self) {
        let mut stats = self.statistics.write().await;
        stats.cache_hits += 1;
        stats.hit_rate = if stats.total_requests > 0 {
            (stats.cache_hits as f64) / (stats.total_requests as f64) * 100.0
        } else {
            0.0
        };
    }
    
    async fn update_statistics_miss(&self) {
        let mut stats = self.statistics.write().await;
        stats.cache_misses += 1;
        stats.hit_rate = if stats.total_requests > 0 {
            (stats.cache_hits as f64) / (stats.total_requests as f64) * 100.0
        } else {
            0.0
        };
    }
    
    async fn update_cache_size_stats(&self) {
        let type_cache_size = self.type_details_cache.read().await.len();
        let search_cache_size = self.search_results_cache.read().await.len();
        
        let mut stats = self.statistics.write().await;
        stats.type_cache_size = type_cache_size;
        stats.search_cache_size = search_cache_size;
        
        // Примерная оценка использования памяти
        stats.estimated_memory_mb = (type_cache_size * 1024 + search_cache_size * 512) as f64 / (1024.0 * 1024.0);
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_type_cache_size: 5000,
            max_search_cache_size: 1000,
            type_details_ttl: 3600,      // 1 час
            search_results_ttl: 1800,    // 30 минут
            eviction_strategy: EvictionStrategy::LRU,
            cleanup_interval_seconds: 300, // 5 минут
        }
    }
}

impl Default for CacheStatistics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            hit_rate: 0.0,
            type_cache_size: 0,
            search_cache_size: 0,
            evictions: 0,
            estimated_memory_mb: 0.0,
        }
    }
}