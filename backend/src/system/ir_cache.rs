//! IR Cache - кеширование промежуточного представления (SemanticProgram)
//!
//! Оптимизирует LSP hover запросы, избегая повторного парсинга файлов.
//!
//! # Характеристики
//! - LRU eviction стратегия
//! - Потокобезопасный async API
//! - Статистика hits/misses/evictions
//! - Capacity: 100 файлов (~10 MB RAM)
//!
//! # Целевые метрики
//! - Первый hover: 50-100ms (парсинг + кеш)
//! - Последующие hover: <5ms (кеш попадание)
//! - Hit rate: >90%

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use bsl_shared::ir::SemanticProgram;
use tracing::debug;

/// IR Cache для кеширования промежуточного представления
///
/// Использует LRU eviction для управления памятью.
/// Потокобезопасный через `Arc<RwLock<...>>`.
///
/// # Примеры
/// ```
/// use bsl_backend::system::IrCache;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let cache = IrCache::new(100);
///
///     // Проверяем кеш перед парсингом
///     let hash = 12345u64;
///     if let Some(ir) = cache.get(hash).await {
///         println!("Cache HIT!");
///     } else {
///         println!("Cache MISS, парсим файл...");
///         // let ir = parse_file(...);
///         // cache.put(hash, Arc::new(ir)).await;
///     }
///
///     let stats = cache.get_stats().await;
///     println!("Hit rate: {:.2}%", cache.get_hit_rate().await);
/// }
/// ```
pub struct IrCache {
    /// LRU storage для SemanticProgram
    ///
    /// Используем RwLock вместо Mutex потому что:
    /// - LRU.get() модифицирует внутреннее состояние (перемещает элемент в начало)
    /// - Требуется write lock даже для чтения
    storage: Arc<RwLock<LruCache<u64, Arc<SemanticProgram>>>>,

    /// Статистика использования кеша
    stats: Arc<RwLock<IrCacheStats>>,
}

/// Статистика IR Cache
#[derive(Debug, Clone, Default)]
pub struct IrCacheStats {
    /// Количество успешных попаданий в кеш
    pub hits: usize,

    /// Количество промахов (cache miss)
    pub misses: usize,

    /// Количество вытесненных элементов при переполнении
    pub evictions: usize,
}

impl IrCache {
    /// Создать новый IR Cache с заданной вместимостью
    ///
    /// # Параметры
    /// - `capacity`: максимальное количество закешированных файлов
    ///
    /// # Примеры
    /// ```
    /// use bsl_backend::system::IrCache;
    /// let cache = IrCache::new(100); // 100 файлов (~10 MB RAM)
    /// ```
    pub fn new(capacity: usize) -> Self {
        debug!("Creating IrCache with capacity: {}", capacity);

        Self {
            storage: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(capacity).expect("Capacity must be > 0"))
            )),
            stats: Arc::new(RwLock::new(IrCacheStats::default())),
        }
    }

    /// Получить SemanticProgram из кеша
    ///
    /// # Примечание о write lock
    /// Используем write lock вместо read lock потому что LRU.get()
    /// модифицирует внутреннее состояние кеша (перемещает элемент в начало списка).
    ///
    /// # Параметры
    /// - `hash`: u64 хеш содержимого файла
    ///
    /// # Возвращает
    /// - `Some(Arc<SemanticProgram>)` если найдено в кеше
    /// - `None` если не найдено (cache miss)
    pub async fn get(&self, hash: u64) -> Option<Arc<SemanticProgram>> {
        // Write lock нужен потому что LRU обновляет порядок элементов
        let mut storage = self.storage.write().await;
        let result = storage.get(&hash).cloned();

        // Обновляем статистику
        let mut stats = self.stats.write().await;
        if result.is_some() {
            stats.hits += 1;
            debug!("✅ IrCache HIT for hash {}", hash);
        } else {
            stats.misses += 1;
            debug!("❌ IrCache MISS for hash {}", hash);
        }

        result
    }

    /// Сохранить SemanticProgram в кеш
    ///
    /// Если кеш заполнен, автоматически вытесняет наименее используемый элемент (LRU).
    ///
    /// # Параметры
    /// - `hash`: u64 хеш содержимого файла
    /// - `ir`: Arc<SemanticProgram> для избежания клонирования
    pub async fn put(&self, hash: u64, ir: Arc<SemanticProgram>) {
        let mut storage = self.storage.write().await;

        // Проверяем, нужно ли вытеснить элемент
        let will_evict = storage.len() >= storage.cap().get() && !storage.contains(&hash);

        if will_evict {
            let mut stats = self.stats.write().await;
            stats.evictions += 1;
            debug!("⚠️ IrCache eviction (capacity reached)");
        }

        storage.put(hash, ir);
        debug!("💾 Cached IR for hash {}", hash);
    }

    /// Получить статистику использования кеша
    ///
    /// # Возвращает
    /// - `IrCacheStats` с количеством hits, misses, evictions
    pub async fn get_stats(&self) -> IrCacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Получить hit rate кеша в процентах
    ///
    /// # Возвращает
    /// - `f64` от 0.0 до 100.0 — процент попаданий в кеш
    /// - Возвращает 0.0 если ещё не было обращений к кешу
    ///
    /// # Примеры
    /// ```no_run
    /// # use bsl_backend::system::IrCache;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let cache = IrCache::new(100);
    /// let hit_rate = cache.get_hit_rate().await;
    /// println!("Hit rate: {:.2}%", hit_rate); // "Hit rate: 92.15%"
    /// # }
    /// ```
    pub async fn get_hit_rate(&self) -> f64 {
        let stats = self.stats.read().await;
        let total = stats.hits + stats.misses;

        if total == 0 {
            return 0.0;
        }

        (stats.hits as f64 / total as f64) * 100.0
    }

    /// Очистить весь кеш (используется в тестах)
    ///
    /// # Примечание
    /// Сбрасывает статистику hits/misses/evictions.
    #[allow(dead_code)]
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        storage.clear();

        let mut stats = self.stats.write().await;
        *stats = IrCacheStats::default();

        debug!("🗑️ IrCache cleared");
    }

    /// Получить текущий размер кеша
    ///
    /// # Возвращает
    /// - `usize` количество элементов в кеше
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        let storage = self.storage.read().await;
        storage.len()
    }

    /// Проверить, пуст ли кеш
    #[allow(dead_code)]
    pub async fn is_empty(&self) -> bool {
        let storage = self.storage.read().await;
        storage.is_empty()
    }

    /// Получить вместимость кеша
    #[allow(dead_code)]
    pub async fn capacity(&self) -> usize {
        let storage = self.storage.read().await;
        storage.cap().get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::ir::{SemanticProgram, SemanticNode, SymbolTable, Span};

    fn create_test_ir(name: &str) -> Arc<SemanticProgram> {
        use bsl_shared::ir::{SemanticNodeKind, SourceInfo};

        let mut symbols = SymbolTable::new();
        let root_scope = symbols.root_scope;

        Arc::new(SemanticProgram {
            symbols,
            nodes: vec![SemanticNode {
                kind: SemanticNodeKind::VariableDeclaration {
                    name: "test".to_string(),
                    type_hint: None,
                    is_export: false,
                    initial_value_type: None,
                },
                span: Span::stub(),
                scope_id: root_scope,
            }],
            source_info: SourceInfo {
                path: name.to_string(),
                content_hash: 0,
            },
            cfg: None,
        })
    }

    #[tokio::test]
    async fn test_cache_hit_miss() {
        let cache = IrCache::new(10);

        let hash1 = 12345u64;
        let ir1 = create_test_ir("file1.bsl");

        // Первый запрос - miss
        assert!(cache.get(hash1).await.is_none());

        // Сохраняем в кеш
        cache.put(hash1, ir1.clone()).await;

        // Второй запрос - hit
        let cached = cache.get(hash1).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().source_info.path, "file1.bsl");
    }

    #[tokio::test]
    async fn test_stats() {
        let cache = IrCache::new(10);

        let hash1 = 11111u64;
        let hash2 = 22222u64;
        let ir1 = create_test_ir("file1.bsl");

        // 1 miss
        cache.get(hash1).await;

        // Put
        cache.put(hash1, ir1.clone()).await;

        // 1 hit
        cache.get(hash1).await;

        // 1 miss
        cache.get(hash2).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
    }

    #[tokio::test]
    async fn test_hit_rate() {
        let cache = IrCache::new(10);

        let hash1 = 111u64;
        let ir1 = create_test_ir("file1.bsl");

        // Пустой кеш - 0%
        assert_eq!(cache.get_hit_rate().await, 0.0);

        // 1 miss
        cache.get(hash1).await;
        assert_eq!(cache.get_hit_rate().await, 0.0); // 0/1 = 0%

        // Put
        cache.put(hash1, ir1).await;

        // 1 hit
        cache.get(hash1).await;
        assert_eq!(cache.get_hit_rate().await, 50.0); // 1/2 = 50%

        // Ещё 1 hit
        cache.get(hash1).await;
        let rate = cache.get_hit_rate().await;
        assert!((rate - 66.66).abs() < 0.1); // 2/3 ≈ 66.66%
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = IrCache::new(2); // Маленький кеш для теста

        let hash1 = 111u64;
        let hash2 = 222u64;
        let hash3 = 333u64;

        let ir1 = create_test_ir("file1.bsl");
        let ir2 = create_test_ir("file2.bsl");
        let ir3 = create_test_ir("file3.bsl");

        // Заполняем кеш
        cache.put(hash1, ir1).await;
        cache.put(hash2, ir2).await;

        assert_eq!(cache.len().await, 2);

        // Добавляем третий элемент - должен вытеснить первый
        cache.put(hash3, ir3).await;

        assert_eq!(cache.len().await, 2);

        let stats = cache.get_stats().await;
        assert_eq!(stats.evictions, 1);

        // Первый элемент вытеснен
        assert!(cache.get(hash1).await.is_none());

        // Второй и третий остались
        assert!(cache.get(hash2).await.is_some());
        assert!(cache.get(hash3).await.is_some());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = IrCache::new(10);

        let hash1 = 111u64;
        let ir1 = create_test_ir("file1.bsl");

        cache.put(hash1, ir1).await;
        cache.get(hash1).await; // hit

        assert_eq!(cache.len().await, 1);

        cache.clear().await;

        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }
}
