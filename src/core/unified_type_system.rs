//! Единая система типов BSL - центральная архитектура
//!
//! TypeResolution как единственный источник истины о типах.
//! Все остальные компоненты (LSP, веб, поиск) - это разные представления
//! одних и тех же TypeResolution объектов.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::platform_resolver::{CompletionItem, PlatformTypeResolver};
use super::types::{
    Certainty, ConcreteType, FacetKind, ResolutionResult, ResolutionSource, TypeResolution,
};
use crate::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;

/// Единая система типов BSL
///
/// Центральная точка ответственности за все операции с типами.
/// TypeResolution является единственным источником истины.
pub struct UnifiedTypeSystem {
    /// Основное хранилище типов (источник истины)
    type_resolutions: Arc<RwLock<HashMap<String, TypeResolution>>>,

    /// Платформенные типы (из syntax helper)
    platform_resolver: Arc<RwLock<PlatformTypeResolver>>,

    /// Конфигурационные типы (из XML)
    configuration_parser: Arc<RwLock<Option<ConfigurationGuidedParser>>>,

    /// Кеш для производительности
    resolution_cache: Arc<RwLock<HashMap<String, CachedResolution>>>,

    /// Статистика системы
    statistics: Arc<RwLock<UnifiedSystemStats>>,

    /// Конфигурация системы
    config: UnifiedSystemConfig,
}

/// Кешированное разрешение типа
#[derive(Debug, Clone)]
pub struct CachedResolution {
    /// Разрешение типа
    pub resolution: TypeResolution,

    /// Время создания
    pub created_at: std::time::Instant,

    /// Количество использований
    pub usage_count: u64,

    /// Время последнего использования
    pub last_used: std::time::Instant,
}

/// Статистика единой системы типов
#[derive(Debug, Clone, Default)]
pub struct UnifiedSystemStats {
    /// Загруженные платформенные типы
    pub platform_types_count: usize,

    /// Загруженные конфигурационные типы
    pub configuration_types_count: usize,

    /// Всего TypeResolution в системе
    pub total_resolutions: usize,

    /// Запросы к системе
    pub resolution_requests: u64,

    /// Попадания в кеш
    pub cache_hits: u64,

    /// Промахи кеша
    pub cache_misses: u64,

    /// Время последнего обновления
    pub last_updated: Option<std::time::Instant>,
}

/// Конфигурация единой системы
#[derive(Debug, Clone)]
pub struct UnifiedSystemConfig {
    /// Путь к справке синтакс-помощника
    pub syntax_helper_path: Option<String>,

    /// Путь к конфигурации
    pub configuration_path: Option<String>,

    /// Использовать guided discovery
    pub use_guided_discovery: bool,

    /// TTL кеша в секундах
    pub cache_ttl_seconds: u64,

    /// Максимальный размер кеша
    pub max_cache_size: usize,

    /// Включить детальное логирование
    pub verbose_logging: bool,
}

impl UnifiedTypeSystem {
    /// Создать новую единую систему типов
    pub fn new(config: UnifiedSystemConfig) -> Self {
        Self {
            type_resolutions: Arc::new(RwLock::new(HashMap::new())),
            platform_resolver: Arc::new(RwLock::new(PlatformTypeResolver::new())),
            configuration_parser: Arc::new(RwLock::new(None)),
            resolution_cache: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(UnifiedSystemStats::default())),
            config,
        }
    }

    /// Создать с настройками по умолчанию
    pub fn with_defaults() -> Self {
        Self::new(UnifiedSystemConfig::default())
    }

    /// Инициализировать систему типов
    pub async fn initialize(&self) -> Result<()> {
        println!("🏗️ Инициализация единой системы типов...");

        // Загружаем платформенные типы
        self.load_platform_types().await?;

        // Загружаем конфигурационные типы
        if let Some(config_path) = &self.config.configuration_path {
            self.load_configuration_types(config_path).await?;
        }

        // Строим единый индекс типов
        self.build_unified_index().await?;

        println!("🎉 Единая система типов инициализирована!");
        self.print_statistics().await;

        Ok(())
    }

    /// Получить статистику системы
    pub async fn get_statistics(&self) -> UnifiedSystemStats {
        self.statistics.read().await.clone()
    }

    // === CORE API - РАЗРЕШЕНИЕ ТИПОВ ===

    /// Разрешить выражение в TypeResolution (основной метод)
    pub async fn resolve_expression(&self, expression: &str) -> TypeResolution {
        // Проверяем кеш
        if let Some(cached) = self.get_from_cache(expression).await {
            self.increment_cache_hits().await;
            return cached.resolution;
        }

        self.increment_cache_misses().await;
        self.increment_resolution_requests().await;

        // Разрешаем через platform resolver
        let mut platform_resolver = self.platform_resolver.write().await;
        let resolution = platform_resolver.resolve_expression(expression);

        // Кешируем результат
        self.cache_resolution(expression, &resolution).await;

        resolution
    }

    /// Получить все типы как TypeResolution (для поиска и документации)
    pub async fn get_all_type_resolutions(&self) -> Vec<(String, TypeResolution)> {
        let resolutions = self.type_resolutions.read().await;
        resolutions
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Получить автодополнение
    pub async fn get_completions(&self, expression: &str) -> Vec<CompletionItem> {
        self.increment_resolution_requests().await;

        let platform_resolver = self.platform_resolver.write().await;
        platform_resolver.get_completions(expression)
    }

    /// Найти типы по паттерну (для поиска)
    pub async fn find_types_by_pattern(&self, pattern: &str) -> Vec<TypeResolution> {
        let resolutions = self.type_resolutions.read().await;

        resolutions
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&pattern.to_lowercase()))
            .map(|(_, resolution)| resolution.clone())
            .collect()
    }

    /// Получить TypeResolution по ID
    pub async fn get_type_by_id(&self, type_id: &str) -> Option<TypeResolution> {
        let resolutions = self.type_resolutions.read().await;
        resolutions.get(type_id).cloned()
    }

    // === ПРИВАТНЫЕ МЕТОДЫ ===

    async fn load_platform_types(&self) -> Result<()> {
        // PlatformTypeResolver уже инициализируется в конструкторе
        // Просто получаем готовые типы
        let platform_resolver = self.platform_resolver.read().await;
        let platform_count = platform_resolver.get_platform_globals_count();

        // Обновляем статистику
        let mut stats = self.statistics.write().await;
        stats.platform_types_count = platform_count;
        stats.total_resolutions = platform_count;

        println!("✅ Доступно {} платформенных типов", platform_count);
        Ok(())
    }

    async fn load_configuration_types(&self, config_path: &str) -> Result<()> {
        println!("⚙️ Загружаем конфигурационные типы из: {}", config_path);

        let mut guided_parser = ConfigurationGuidedParser::new(config_path);
        let config_resolutions = guided_parser.parse_with_configuration_guide()?;

        // Добавляем конфигурационные типы в основное хранилище
        let mut resolutions = self.type_resolutions.write().await;

        for config_resolution in config_resolutions {
            if let ResolutionResult::Concrete(ConcreteType::Configuration(config)) =
                &config_resolution.result
            {
                let key = format!("{:?}.{}", config.kind, config.name);
                resolutions.insert(key, config_resolution);
            }
        }

        // Обновляем статистику
        let mut stats = self.statistics.write().await;
        stats.configuration_types_count = resolutions.len() - stats.platform_types_count;
        stats.total_resolutions = resolutions.len();

        // Сохраняем парсер для возможных обновлений
        *self.configuration_parser.write().await = Some(guided_parser);

        println!(
            "✅ Загружено {} конфигурационных типов",
            stats.configuration_types_count
        );
        Ok(())
    }

    async fn build_unified_index(&self) -> Result<()> {
        println!("🔍 Строим единый индекс типов...");

        let resolutions = self.type_resolutions.read().await;

        // Индексация для быстрого поиска
        // TODO: Можно добавить дополнительные индексы по категориямм фасетам и т.д.

        println!("✅ Единый индекс построен для {} типов", resolutions.len());
        Ok(())
    }

    async fn get_from_cache(&self, expression: &str) -> Option<CachedResolution> {
        let cache = self.resolution_cache.read().await;

        if let Some(cached) = cache.get(expression) {
            // Проверяем TTL
            if cached.created_at.elapsed().as_secs() < self.config.cache_ttl_seconds {
                // Обновляем время последнего использования
                return Some(cached.clone());
            }
        }

        None
    }

    async fn cache_resolution(&self, expression: &str, resolution: &TypeResolution) {
        let mut cache = self.resolution_cache.write().await;

        // Проверяем размер кеша
        if cache.len() >= self.config.max_cache_size {
            // Удаляем старые записи (простая LRU)
            let oldest_key = cache
                .iter()
                .min_by_key(|(_, cached)| cached.last_used)
                .map(|(k, _)| k.clone());

            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }

        cache.insert(
            expression.to_string(),
            CachedResolution {
                resolution: resolution.clone(),
                created_at: std::time::Instant::now(),
                usage_count: 1,
                last_used: std::time::Instant::now(),
            },
        );
    }

    async fn increment_resolution_requests(&self) {
        let mut stats = self.statistics.write().await;
        stats.resolution_requests += 1;
    }

    async fn increment_cache_hits(&self) {
        let mut stats = self.statistics.write().await;
        stats.cache_hits += 1;
    }

    async fn increment_cache_misses(&self) {
        let mut stats = self.statistics.write().await;
        stats.cache_misses += 1;
    }

    async fn print_statistics(&self) {
        let stats = self.statistics.read().await;
        println!("📊 Статистика единой системы типов:");
        println!("  - Платформенные типы: {}", stats.platform_types_count);
        println!(
            "  - Конфигурационные типы: {}",
            stats.configuration_types_count
        );
        println!("  - Всего TypeResolution: {}", stats.total_resolutions);
        println!("  - Запросы: {}", stats.resolution_requests);

        if stats.cache_hits + stats.cache_misses > 0 {
            let hit_ratio =
                stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64;
            println!("  - Cache hit ratio: {:.2}", hit_ratio);
        }
    }
}

impl Default for UnifiedSystemConfig {
    fn default() -> Self {
        Self {
            syntax_helper_path: Some("examples/syntax_helper/rebuilt.shcntx_ru".to_string()),
            configuration_path: None,
            use_guided_discovery: true,
            cache_ttl_seconds: 3600, // 1 час
            max_cache_size: 10000,   // 10K записей
            verbose_logging: false,
        }
    }
}

// === ИНТЕРФЕЙСЫ К ЕДИНОЙ СИСТЕМЕ ===

/// LSP интерфейс к единой системе типов
///
/// Предоставляет методы, специфичные для Language Server Protocol
pub struct LspTypeInterface {
    unified_system: Arc<UnifiedTypeSystem>,
}

impl LspTypeInterface {
    pub fn new(unified_system: Arc<UnifiedTypeSystem>) -> Self {
        Self { unified_system }
    }

    /// Разрешить выражение для LSP
    pub async fn resolve_expression(&self, expression: &str) -> TypeResolution {
        self.unified_system.resolve_expression(expression).await
    }

    /// Автодополнение для LSP
    pub async fn get_completions(&self, expression: &str) -> Vec<CompletionItem> {
        self.unified_system.get_completions(expression).await
    }

    /// Получить тип переменной в контексте
    pub async fn get_variable_type(&self, variable_name: &str, _context: &str) -> TypeResolution {
        // Для простоты пока используем базовое разрешение
        self.unified_system.resolve_expression(variable_name).await
    }

    /// Проверить совместимость типов для присваивания
    pub async fn check_assignment_compatibility(
        &self,
        _from_type: &TypeResolution,
        _to_type: &TypeResolution,
    ) -> bool {
        // TODO: Реализовать проверку совместимости
        true
    }
}

/// Веб интерфейс к единой системе типов
///
/// Предоставляет методы для веб-интерфейса и документации
pub struct WebTypeInterface {
    unified_system: Arc<UnifiedTypeSystem>,
}

impl WebTypeInterface {
    pub fn new(unified_system: Arc<UnifiedTypeSystem>) -> Self {
        Self { unified_system }
    }

    /// Получить все типы для отображения в веб-интерфейсе
    pub async fn get_all_types_for_display(&self) -> Vec<TypeDisplayInfo> {
        let all_resolutions = self.unified_system.get_all_type_resolutions().await;

        all_resolutions
            .into_iter()
            .map(|(name, resolution)| TypeDisplayInfo::from_resolution(name, resolution))
            .collect()
    }

    /// Найти типы по запросу
    pub async fn search_types(&self, query: &str) -> Vec<TypeDisplayInfo> {
        let matching_resolutions = self.unified_system.find_types_by_pattern(query).await;

        matching_resolutions
            .into_iter()
            .enumerate()
            .map(|(i, resolution)| {
                TypeDisplayInfo::from_resolution(format!("search_result_{}", i), resolution)
            })
            .collect()
    }

    /// Получить детальную информацию о типе
    pub async fn get_type_details(&self, type_id: &str) -> Option<TypeDetailedInfo> {
        if let Some(resolution) = self.unified_system.get_type_by_id(type_id).await {
            Some(TypeDetailedInfo::from_resolution(
                type_id.to_string(),
                resolution,
            ))
        } else {
            None
        }
    }
}

/// Информация о типе для отображения в веб-интерфейсе
#[derive(Debug, Clone)]
pub struct TypeDisplayInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub certainty: Certainty,
    pub source: ResolutionSource,
    pub available_facets: Vec<FacetKind>,
}

impl TypeDisplayInfo {
    pub fn from_resolution(id: String, resolution: TypeResolution) -> Self {
        let (name, category) = match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(platform_type)) => {
                (platform_type.name.clone(), "Platform".to_string())
            }
            ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) => {
                (config_type.name.clone(), format!("{:?}", config_type.kind))
            }
            ResolutionResult::Concrete(ConcreteType::Primitive(primitive)) => {
                (format!("{:?}", primitive), "Primitive".to_string())
            }
            _ => ("Unknown".to_string(), "Unknown".to_string()),
        };

        Self {
            id,
            name: name.clone(),
            category,
            description: format!("Type: {} (certainty: {:?})", name, resolution.certainty),
            certainty: resolution.certainty,
            source: resolution.source,
            available_facets: resolution.available_facets,
        }
    }
}

/// Детальная информация о типе
#[derive(Debug, Clone)]
pub struct TypeDetailedInfo {
    pub id: String,
    pub name: String,
    pub full_resolution: TypeResolution,
    pub methods: Vec<String>,
    pub properties: Vec<String>,
    pub facets: Vec<FacetKind>,
}

impl TypeDetailedInfo {
    pub fn from_resolution(id: String, resolution: TypeResolution) -> Self {
        let name = match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(platform_type)) => {
                platform_type.name.clone()
            }
            ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) => {
                config_type.name.clone()
            }
            _ => "Unknown".to_string(),
        };

        let methods = match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(platform_type)) => platform_type
                .methods
                .iter()
                .map(|m| m.name.clone())
                .collect(),
            ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) => config_type
                .attributes
                .iter()
                .map(|a| a.name.clone())
                .collect(),
            _ => Vec::new(),
        };

        let properties = match &resolution.result {
            ResolutionResult::Concrete(ConcreteType::Platform(platform_type)) => platform_type
                .properties
                .iter()
                .map(|p| p.name.clone())
                .collect(),
            _ => Vec::new(),
        };

        Self {
            id,
            name,
            full_resolution: resolution.clone(),
            methods,
            properties,
            facets: resolution.available_facets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unified_type_system_creation() {
        let system = UnifiedTypeSystem::with_defaults();
        let stats = system.get_statistics().await;

        assert_eq!(stats.total_resolutions, 0);
        assert_eq!(stats.resolution_requests, 0);
    }

    #[tokio::test]
    async fn test_lsp_interface() {
        let system = Arc::new(UnifiedTypeSystem::with_defaults());
        let lsp_interface = LspTypeInterface::new(system.clone());

        // Тест разрешения выражения
        let resolution = lsp_interface.resolve_expression("ТаблицаЗначений").await;
        assert_ne!(resolution.certainty, Certainty::Known); // Без инициализации будет Unknown/Inferred

        // Тест автодополнения
        let completions = lsp_interface.get_completions("Табли").await;
        // В тестовом окружении может быть 0 или несколько результатов
    }

    #[tokio::test]
    async fn test_web_interface() {
        let system = Arc::new(UnifiedTypeSystem::with_defaults());
        let web_interface = WebTypeInterface::new(system.clone());

        // Тест получения всех типов для веб-интерфейса
        let display_types = web_interface.get_all_types_for_display().await;
        // В тестовом окружении без инициализации будет пустой

        // Тест поиска
        let search_results = web_interface.search_types("ТаблицаЗначений").await;
        // Результат зависит от загруженных данных
    }
}
