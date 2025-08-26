//! Система кеширования результатов анализа типов
//!
//! Этот модуль предоставляет эффективное кеширование результатов
//! межпроцедурного анализа и других дорогих операций.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// Менеджер кеширования анализа
pub struct AnalysisCacheManager {
    /// Путь к директории кеша
    cache_dir: PathBuf,
    /// In-memory кеш для быстрого доступа
    memory_cache: HashMap<CacheKey, CachedInterproceduralResults>,
    /// Максимальный размер memory кеша
    max_memory_entries: usize,
    /// Версия анализатора для кеша
    #[allow(dead_code)]
    analyzer_version: String,
    /// Статистика использования кеша
    stats: CacheStats,
}

/// Статистика кеширования
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub invalidations: usize,
    pub disk_reads: usize,
    pub disk_writes: usize,
}

impl CacheStats {
    /// Получить hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

impl AnalysisCacheManager {
    /// Создать новый менеджер кеширования
    pub fn new<P: AsRef<Path>>(cache_dir: P, analyzer_version: &str) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();

        // Создаем директорию кеша если не существует
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir)?;
        }

        Ok(Self {
            cache_dir,
            memory_cache: HashMap::new(),
            max_memory_entries: 100, // Ограничение memory кеша
            analyzer_version: analyzer_version.to_string(),
            stats: CacheStats::default(),
        })
    }

    /// Получить результаты из кеша
    pub fn get(&mut self, key: &CacheKey) -> Option<CachedInterproceduralResults> {
        // Сначала проверяем memory кеш
        if let Some(cached) = self.memory_cache.get(key) {
            if cached.is_valid() {
                self.stats.hits += 1;
                return Some(cached.clone());
            } else {
                // Кеш устарел, удаляем
                self.memory_cache.remove(key);
                self.stats.invalidations += 1;
            }
        }

        // Проверяем disk кеш
        if let Ok(cached) = self.load_from_disk(key) {
            if cached.is_valid() {
                // Добавляем в memory кеш
                self.ensure_memory_cache_size();
                self.memory_cache.insert(key.clone(), cached.clone());

                self.stats.hits += 1;
                self.stats.disk_reads += 1;
                return Some(cached);
            } else {
                // Удаляем устаревший файл
                let _ = self.remove_from_disk(key);
                self.stats.invalidations += 1;
            }
        }

        self.stats.misses += 1;
        None
    }

    /// Сохранить результаты в кеш
    pub fn put(&mut self, key: CacheKey, results: CachedInterproceduralResults) -> Result<()> {
        // Добавляем в memory кеш
        self.ensure_memory_cache_size();
        self.memory_cache.insert(key.clone(), results.clone());

        // Сохраняем на диск асинхронно
        self.save_to_disk(&key, &results)?;
        self.stats.disk_writes += 1;

        Ok(())
    }

    /// Инвалидировать кеш для ключа
    pub fn invalidate(&mut self, key: &CacheKey) {
        self.memory_cache.remove(key);
        let _ = self.remove_from_disk(key);
        self.stats.invalidations += 1;
    }

    /// Очистить весь кеш
    pub fn clear(&mut self) -> Result<()> {
        self.memory_cache.clear();

        // Удаляем все файлы кеша
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
            std::fs::create_dir_all(&self.cache_dir)?;
        }

        self.stats = CacheStats::default();
        Ok(())
    }

    /// Получить статистику кеша
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Убедиться что memory кеш не превышает лимит
    fn ensure_memory_cache_size(&mut self) {
        while self.memory_cache.len() >= self.max_memory_entries {
            // Удаляем самый старый элемент (простая стратегия)
            if let Some(oldest_key) = self.memory_cache.keys().next().cloned() {
                self.memory_cache.remove(&oldest_key);
            }
        }
    }

    /// Загрузить из диска
    fn load_from_disk(&self, key: &CacheKey) -> Result<CachedInterproceduralResults> {
        let file_path = self.get_cache_file_path(key);
        let data = std::fs::read(&file_path)?;
        let cached: CachedInterproceduralResults = bincode::deserialize(&data)?;
        Ok(cached)
    }

    /// Сохранить на диск
    fn save_to_disk(&self, key: &CacheKey, results: &CachedInterproceduralResults) -> Result<()> {
        let file_path = self.get_cache_file_path(key);
        let data = bincode::serialize(results)?;
        std::fs::write(&file_path, data)?;
        Ok(())
    }

    /// Удалить с диска
    fn remove_from_disk(&self, key: &CacheKey) -> Result<()> {
        let file_path = self.get_cache_file_path(key);
        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
        }
        Ok(())
    }

    /// Получить путь к файлу кеша
    fn get_cache_file_path(&self, key: &CacheKey) -> PathBuf {
        let filename = format!(
            "{}_{}.cache",
            &key.content_hash[..16], // Первые 16 символов хеша
            key.analyzer_version.replace('.', "_")
        );
        self.cache_dir.join(filename)
    }

    /// Очистить устаревшие записи кеша
    pub fn cleanup_expired(&mut self) -> Result<usize> {
        let mut removed_count = 0;

        // Очищаем memory кеш
        let expired_keys: Vec<_> = self
            .memory_cache
            .iter()
            .filter(|(_, cached)| !cached.is_valid())
            .map(|(key, _)| key.clone())
            .collect();

        for key in &expired_keys {
            self.memory_cache.remove(key);
            removed_count += 1;
        }

        // Очищаем disk кеш
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(data) = std::fs::read(entry.path()) {
                    if let Ok(cached) = bincode::deserialize::<CachedInterproceduralResults>(&data)
                    {
                        if !cached.is_valid() {
                            let _ = std::fs::remove_file(entry.path());
                            removed_count += 1;
                        }
                    }
                }
            }
        }

        self.stats.invalidations += removed_count;
        Ok(removed_count)
    }

    /// Получить размер кеша на диске
    pub fn get_disk_cache_size(&self) -> Result<u64> {
        let mut total_size = 0;

        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }
}

/// Интегрированный кеширующий межпроцедурный анализатор
pub struct CachedInterproceduralAnalyzer {
    /// Базовый анализатор
    base_analyzer: crate::domain::analysis::interprocedural::InterproceduralAnalyzer,
    /// Менеджер кеширования
    cache_manager: AnalysisCacheManager,
    /// Версия анализатора
    analyzer_version: String,
}

impl CachedInterproceduralAnalyzer {
    /// Создать новый кеширующий анализатор
    pub fn new<P: AsRef<Path>>(
        call_graph: CallGraph,
        context: TypeContext,
        cache_dir: P,
    ) -> Result<Self> {
        let analyzer_version = env!("CARGO_PKG_VERSION").to_string();

        Ok(Self {
            base_analyzer: crate::domain::analysis::interprocedural::InterproceduralAnalyzer::new(
                call_graph, context,
            ),
            cache_manager: AnalysisCacheManager::new(cache_dir, &analyzer_version)?,
            analyzer_version,
        })
    }

    /// Проанализировать с кешированием
    pub fn analyze_with_cache(&mut self, file_content: &str) -> Result<TypeContext> {
        let cache_key = CacheKey::from_content(file_content, &self.analyzer_version);

        // Проверяем кеш
        if let Some(cached) = self.cache_manager.get(&cache_key) {
            tracing::info!("Используем кешированные результаты межпроцедурного анализа");

            // Восстанавливаем контекст из кеша
            let context = TypeContext {
                variables: HashMap::new(),
                functions: cached.function_signatures,
                current_scope: crate::domain::analysis::dependency_graph::Scope::Global,
                scope_stack: vec![],
            };

            return Ok(context);
        }

        tracing::info!("Кеш не найден, выполняем полный межпроцедурный анализ");

        // Выполняем полный анализ
        self.base_analyzer.analyze_all_functions();

        // Получаем результаты
        let function_results = self.base_analyzer.get_analyzed_functions().clone();
        let mut context = TypeContext {
            variables: HashMap::new(),
            functions: HashMap::new(),
            current_scope: crate::domain::analysis::dependency_graph::Scope::Global,
            scope_stack: vec![],
        };

        // Обновляем контекст
        for func_name in function_results.keys() {
            if let Some(signature) = self.base_analyzer.get_function_signature(func_name) {
                context.functions.insert(func_name.clone(), signature);
            }
        }

        // Создаем кешируемые результаты
        let cached_results = CachedInterproceduralResults::from_analysis(
            function_results,
            &context,
            &self.base_analyzer.call_graph,
            Duration::from_secs(3600), // 1 час TTL
        );

        // Сохраняем в кеш
        if let Err(e) = self.cache_manager.put(cache_key, cached_results) {
            tracing::warn!("Не удалось сохранить в кеш: {}", e);
        }

        Ok(context)
    }

    /// Получить статистику кеширования
    pub fn get_cache_stats(&self) -> &CacheStats {
        self.cache_manager.get_stats()
    }

    /// Очистить кеш
    pub fn clear_cache(&mut self) -> Result<()> {
        self.cache_manager.clear()
    }

    /// Получить информацию о кеше
    pub fn get_cache_info(&self) -> Result<CacheInfo> {
        Ok(CacheInfo {
            memory_entries: self.cache_manager.memory_cache.len(),
            disk_size_bytes: self.cache_manager.get_disk_cache_size()?,
            hit_rate: self.cache_manager.stats.hit_rate(),
            stats: self.cache_manager.stats.clone(),
        })
    }
}

/// Информация о состоянии кеша
#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub memory_entries: usize,
    pub disk_size_bytes: u64,
    pub hit_rate: f64,
    pub stats: CacheStats,
}

impl CacheInfo {
    /// Форматировать информацию о кеше
    pub fn format_human_readable(&self) -> String {
        format!(
            "🗄️ Кеш межпроцедурного анализа:\n\
             📦 Memory entries: {}\n\
             💾 Disk size: {:.2} MB\n\
             🎯 Hit rate: {:.1}%\n\
             📊 Hits: {}, Misses: {}, Invalidations: {}\n\
             💿 Disk: {} reads, {} writes",
            self.memory_entries,
            self.disk_size_bytes as f64 / (1024.0 * 1024.0),
            self.hit_rate * 100.0,
            self.stats.hits,
            self.stats.misses,
            self.stats.invalidations,
            self.stats.disk_reads,
            self.stats.disk_writes
        )
    }
}

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
    use tempfile::TempDir;

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

    #[test]
    fn test_analysis_cache_manager() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut manager = AnalysisCacheManager::new(temp_dir.path(), "test-1.0.0")?;

        let key = CacheKey::from_content("test content", "test-1.0.0");
        let results = CachedInterproceduralResults {
            function_results: HashMap::new(),
            function_signatures: HashMap::new(),
            call_graph_summary: CallGraphSummary {
                function_call_counts: HashMap::new(),
                topological_order: vec!["TestFunc".to_string()],
                recursive_functions: vec![],
            },
            created_at: SystemTime::now(),
            ttl: Duration::from_secs(3600),
        };

        // Тест put/get
        manager.put(key.clone(), results.clone())?;
        let retrieved = manager.get(&key);

        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().call_graph_summary.topological_order,
            vec!["TestFunc".to_string()]
        );

        // Проверяем статистику
        let stats = manager.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.disk_writes, 1);

        Ok(())
    }

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
