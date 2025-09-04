//! Система кеширования результатов анализа типов
//!
//! ⚠️ LEGACY код - AnalysisCacheManager удален, используется простой кеш в SystemCoordinator
//! Сохранены только структуры для совместимости

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use crate::domain::analysis::interprocedural::CallGraph;
use crate::domain::analysis::type_checker::{FunctionSignature, TypeContext};
use crate::domain::types::TypeResolution;

/// Ключ кеша на основе хеша содержимого файла
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    /// SHA256 хеш содержимого файла
    pub content_hash: String,
    /// Версия анализатора
    pub analyzer_version: String,
    /// Дополнительные параметры (отсортированный вектор для Hash)
    pub params: Vec<(String, String)>,
}

impl std::hash::Hash for CacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.content_hash.hash(state);
        self.analyzer_version.hash(state);
        for (k, v) in &self.params {
            k.hash(state);
            v.hash(state);
        }
    }
}

impl CacheKey {
    /// Создать ключ из содержимого файла
    pub fn from_content(content: &str, analyzer_version: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Self {
            content_hash: hash,
            analyzer_version: analyzer_version.to_string(),
            params: vec![],
        }
    }

    /// Добавить параметр к ключу
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.params.push((key.to_string(), value.to_string()));
        self.params.sort(); // Сортируем для консистентности
        self
    }
}

/// Кешированные результаты межпроцедурного анализа
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedInterproceduralResults {
    /// Результаты анализа функций
    pub function_results: HashMap<String, TypeResolution>,
    /// Сигнатуры функций
    pub function_signatures: HashMap<String, FunctionSignature>,
    /// Граф вызовов (упрощенная версия)
    pub call_graph_summary: CallGraphSummary,
    /// Время создания кеша
    pub created_at: SystemTime,
    /// TTL (время жизни) кеша
    pub ttl: Duration,
}

impl CachedInterproceduralResults {
    /// Проверить валидность кеша
    pub fn is_valid(&self) -> bool {
        if let Ok(elapsed) = self.created_at.elapsed() {
            elapsed < self.ttl
        } else {
            false
        }
    }

    /// Создать из результатов анализа
    pub fn from_analysis(
        function_results: HashMap<String, TypeResolution>,
        context: &TypeContext,
        call_graph: &CallGraph,
        ttl: Duration,
    ) -> Self {
        Self {
            function_results,
            function_signatures: context.functions.clone(),
            call_graph_summary: CallGraphSummary::from_call_graph(call_graph),
            created_at: SystemTime::now(),
            ttl,
        }
    }
}

/// Упрощенная версия графа вызовов для кеширования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphSummary {
    /// Функции и количество их вызовов
    pub function_call_counts: HashMap<String, usize>,
    /// Топологический порядок функций
    pub topological_order: Vec<String>,
    /// Рекурсивные функции
    pub recursive_functions: Vec<String>,
}

impl CallGraphSummary {
    pub fn from_call_graph(call_graph: &CallGraph) -> Self {
        let function_call_counts = HashMap::new();

        // Подсчитываем вызовы (заглушка - CallGraph пока не экспортирует нужные методы)
        // TODO: Добавить методы в CallGraph для получения статистики

        Self {
            function_call_counts,
            topological_order: call_graph.topological_sort(),
            recursive_functions: vec![], // TODO: Определение рекурсивных функций
        }
    }
}

// ===== LEGACY CODE REMOVED =====
// AnalysisCacheManager и связанные структуры удалены
// Теперь используется упрощенный AnalysisCache из simple_cache.rs

/// LRU кеш для быстрого доступа к часто используемым типам
pub struct TypeLRUCache {
    cache: lru::LruCache<String, TypeResolution>,
    hits: usize,
    misses: usize,
}

impl TypeLRUCache {
    /// Создать новый LRU кеш
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap()),
            hits: 0,
            misses: 0,
        }
    }

    /// Получить тип из кеша
    pub fn get(&mut self, key: &str) -> Option<&TypeResolution> {
        if let Some(type_res) = self.cache.get(key) {
            self.hits += 1;
            Some(type_res)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Добавить тип в кеш
    pub fn put(&mut self, key: String, type_res: TypeResolution) {
        self.cache.put(key, type_res);
    }

    /// Получить статистику
    pub fn get_hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }

    /// Очистить кеш
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    // use tempfile::TempDir; // Убрано - не нужно без AnalysisCacheManager

    #[test]
    fn test_cache_key_creation() {
        let content = "Функция Тест() КонецФункции";
        let key1 = CacheKey::from_content(content, "1.0.0");
        let key2 = CacheKey::from_content(content, "1.0.0");
        let key3 = CacheKey::from_content("другой контент", "1.0.0");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_with_params() {
        let key = CacheKey::from_content("test", "1.0.0")
            .with_param("debug", "true")
            .with_param("optimization", "fast");

        assert_eq!(key.params.len(), 2);
        assert!(key
            .params
            .contains(&("debug".to_string(), "true".to_string())));
    }

    #[test]
    fn test_cached_results_validity() {
        let results = CachedInterproceduralResults {
            function_results: HashMap::new(),
            function_signatures: HashMap::new(),
            call_graph_summary: CallGraphSummary {
                function_call_counts: HashMap::new(),
                topological_order: vec![],
                recursive_functions: vec![],
            },
            created_at: SystemTime::now(),
            ttl: Duration::from_secs(60),
        };

        assert!(results.is_valid());

        let expired_results = CachedInterproceduralResults {
            created_at: SystemTime::now() - Duration::from_secs(120),
            ttl: Duration::from_secs(60),
            ..results
        };

        assert!(!expired_results.is_valid());
    }

    // REMOVED: test_analysis_cache_manager - AnalysisCacheManager удален
    
    #[test]
    fn test_type_lru_cache() {
        let mut cache = TypeLRUCache::new(2);

        let string_type =
            crate::domain::standard_types::primitive_type(crate::domain::types::PrimitiveType::String);
        let number_type =
            crate::domain::standard_types::primitive_type(crate::domain::types::PrimitiveType::Number);

        // Добавляем типы
        cache.put("var1".to_string(), string_type.clone());
        cache.put("var2".to_string(), number_type.clone());

        // Проверяем что типы найдены
        assert!(cache.get("var1").is_some());
        assert!(cache.get("var2").is_some());

        // Добавляем третий тип (должен вытеснить первый)
        let bool_type =
            crate::domain::standard_types::primitive_type(crate::domain::types::PrimitiveType::Boolean);
        cache.put("var3".to_string(), bool_type);

        // var1 должен быть вытеснен
        assert!(cache.get("var1").is_none());
        assert!(cache.get("var2").is_some());
        assert!(cache.get("var3").is_some());

        // Проверяем статистику
        assert!(cache.get_hit_rate() > 0.0);
    }
}
