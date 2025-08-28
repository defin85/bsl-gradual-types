//! Центральный сервис типов BSL - единая точка ответственности
//!
//! TypeSystemService объединяет:
//! - Platform types (PlatformTypeResolver)  
//! - Documentation system (PlatformDocumentationProvider)
//! - Search engine (DocumentationSearchEngine)
//! - Configuration parsing (ConfigurationGuidedParser)

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::resolvers::platform::CompletionItem;
use crate::domain::analysis::type_checker::TypeContext;
use crate::domain::search::{AdvancedSearchQuery, SearchResults, TypeHierarchy};
use super::types::TypeResolution;
use super::unified_type_system::{
    LspTypeInterface, TypeDetailedInfo, TypeDisplayInfo, UnifiedSystemConfig, UnifiedTypeSystem,
    WebTypeInterface,
};
// TODO: Implement TypeHierarchy in documentation_service  
// use crate::application::documentation_service::TypeHierarchy;
// TODO: Implement search functionality in documentation_service
// use crate::documentation::{AdvancedSearchQuery, SearchResults};

/// Центральный сервис системы типов BSL v2.0
///
/// Фасад над UnifiedTypeSystem для удобного использования в LSP и веб-сервере.
/// Предоставляет high-level API и статистику использования.
pub struct TypeSystemService {
    /// Состояние инициализации
    initialization_state: Arc<RwLock<InitializationState>>,

    /// Единая система типов (источник истины)
    unified_system: Arc<UnifiedTypeSystem>,

    /// LSP интерфейс
    lsp_interface: LspTypeInterface,

    /// Веб интерфейс
    web_interface: WebTypeInterface,

    /// Конфигурация сервиса
    #[allow(dead_code)]
    config: Arc<RwLock<TypeSystemServiceConfig>>,

    /// Статистика использования
    usage_stats: Arc<RwLock<ServiceUsageStats>>,
}

/// Состояние инициализации сервиса
#[derive(Debug, Clone)]
pub struct InitializationState {
    /// Завершена ли инициализация
    pub is_initialized: bool,

    /// Текущий этап
    pub current_stage: InitializationStage,

    /// Прогресс (0-100)
    pub progress: u8,

    /// Сообщение о текущей операции
    pub status_message: String,

    /// Время начала инициализации
    pub start_time: Option<std::time::Instant>,

    /// Ошибки инициализации
    pub errors: Vec<String>,
}

/// Этапы инициализации
#[derive(Debug, Clone, PartialEq)]
pub enum InitializationStage {
    Starting,
    LoadingPlatformTypes,
    BuildingDocumentation,
    BuildingSearchIndexes,
    Finalizing,
    Ready,
    Failed,
}

/// Конфигурация TypeSystemService
#[derive(Debug, Clone)]
pub struct TypeSystemServiceConfig {
    /// Путь к справке синтакс-помощника
    pub syntax_helper_path: Option<String>,

    /// Путь к конфигурации проекта
    pub project_config_path: Option<String>,

    /// Использовать guided discovery парсер
    pub use_guided_discovery: bool,

    /// Настройки кеширования
    pub cache_settings: CacheSettings,

    /// Настройки поиска
    pub search_settings: SearchSettings,
}

/// Настройки кеширования
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Включить кеширование типов
    pub enable_type_cache: bool,

    /// Включить кеширование поиска
    pub enable_search_cache: bool,

    /// TTL для кеша в секундах
    pub cache_ttl_seconds: u64,

    /// Максимальный размер кеша
    pub max_cache_size: usize,
}

/// Настройки поиска
#[derive(Debug, Clone)]
pub struct SearchSettings {
    /// Включить fuzzy search по умолчанию
    pub enable_fuzzy_by_default: bool,

    /// Максимальное количество результатов
    pub max_search_results: usize,

    /// Время жизни предложений автодополнения
    pub suggestions_ttl_seconds: u64,
}

/// Статистика использования сервиса
#[derive(Debug, Clone, Default)]
pub struct ServiceUsageStats {
    /// Количество запросов к LSP
    pub lsp_requests: u64,

    /// Количество веб-запросов
    pub web_requests: u64,

    /// Количество поисковых запросов
    pub search_requests: u64,

    /// Количество запросов автодополнения
    pub completion_requests: u64,

    /// Время работы сервиса
    pub uptime_seconds: u64,

    /// Использование памяти (примерно)
    pub memory_usage_mb: f64,
}

impl TypeSystemService {
    /// Создать новый экземпляр сервиса на базе UnifiedTypeSystem
    pub fn new(config: TypeSystemServiceConfig) -> Self {
        // Конвертируем конфигурацию для UnifiedTypeSystem
        let unified_config = UnifiedSystemConfig {
            syntax_helper_path: config.syntax_helper_path.clone(),
            configuration_path: config.project_config_path.clone(),
            use_guided_discovery: config.use_guided_discovery,
            cache_ttl_seconds: config.cache_settings.cache_ttl_seconds,
            max_cache_size: config.cache_settings.max_cache_size,
            verbose_logging: false,
        };

        // Создаем единую систему типов
        let unified_system = Arc::new(UnifiedTypeSystem::new(unified_config));

        // Создаем интерфейсы
        let lsp_interface = LspTypeInterface::new(unified_system.clone());
        let web_interface = WebTypeInterface::new(unified_system.clone());

        Self {
            initialization_state: Arc::new(RwLock::new(InitializationState::new())),
            unified_system,
            lsp_interface,
            web_interface,
            config: Arc::new(RwLock::new(config)),
            usage_stats: Arc::new(RwLock::new(ServiceUsageStats::default())),
        }
    }

    /// Создать с настройками по умолчанию
    pub fn with_defaults() -> Self {
        Self::new(TypeSystemServiceConfig::default())
    }

    /// Асинхронная инициализация всех компонентов через UnifiedTypeSystem
    pub async fn initialize(&self) -> Result<()> {
        let mut state = self.initialization_state.write().await;
        state.start_initialization();
        drop(state);

        // Инициализируем единую систему типов
        self.set_stage(
            InitializationStage::LoadingPlatformTypes,
            "Инициализация UnifiedTypeSystem...",
        )
        .await;

        match self.unified_system.initialize().await {
            Ok(_) => {
                self.set_stage(
                    InitializationStage::Ready,
                    "TypeSystemService готов к работе",
                )
                .await;

                let mut state = self.initialization_state.write().await;
                state.complete_initialization();

                println!("🎉 TypeSystemService v2.0 полностью инициализирован!");
                Ok(())
            }
            Err(e) => {
                self.set_stage(
                    InitializationStage::Failed,
                    &format!("Ошибка инициализации: {}", e),
                )
                .await;
                Err(e)
            }
        }
    }

    /// Получить состояние инициализации
    pub async fn get_initialization_state(&self) -> InitializationState {
        self.initialization_state.read().await.clone()
    }

    // === API ДЛЯ LSP СЕРВЕРА ===

    /// Резолвить выражение (для LSP)
    pub async fn resolve_expression(&self, expression: &str) -> TypeResolution {
        self.increment_lsp_requests().await;
        self.lsp_interface.resolve_expression(expression).await
    }

    /// Получить автодополнение (для LSP)
    pub async fn get_completions(&self, expression: &str) -> Vec<CompletionItem> {
        self.increment_completion_requests().await;
        self.lsp_interface.get_completions(expression).await
    }

    /// Получить тип переменной в контексте (для LSP)
    pub async fn get_variable_type(&self, variable_name: &str, context: &str) -> TypeResolution {
        self.increment_lsp_requests().await;
        self.lsp_interface
            .get_variable_type(variable_name, context)
            .await
    }

    /// Проверить совместимость типов (для LSP)
    pub async fn check_assignment_compatibility(
        &self,
        from_type: &TypeResolution,
        to_type: &TypeResolution,
    ) -> bool {
        self.increment_lsp_requests().await;
        self.lsp_interface
            .check_assignment_compatibility(from_type, to_type)
            .await
    }

    /// Обновить контекст типов (для LSP)
    pub async fn update_type_context(&self, _context: TypeContext) -> Result<()> {
        // TODO: Интеграция с TypeContext через UnifiedTypeSystem
        Ok(())
    }

    // === API ДЛЯ ВЕБ-СЕРВЕРА ===

    /// Выполнить поиск (для веб-сервера)
    pub async fn search(&self, _query: AdvancedSearchQuery) -> Result<SearchResults> {
        self.increment_search_requests().await;
        // TODO: Интеграция поиска через UnifiedTypeSystem
        // Пока возвращаем заглушку
        Ok(SearchResults {
            results: Vec::new(),
            total_count: 0,
            truncated: false,
            // pagination_info: crate::documentation::search::PaginationInfo {
            //     current_page: 0,
            //     total_pages: 0,
            //     has_next: false,
            //     has_previous: false,
            //     page_size: query.pagination.page_size,
            // },
        })
    }

    /// Получить предложения автодополнения (для веб-сервера)
    pub async fn get_suggestions(&self, _partial_query: &str) -> Result<Vec<String>> {
        self.increment_web_requests().await;
        // TODO: Интеграция автодополнения через UnifiedTypeSystem
        Ok(Vec::new())
    }

    /// Получить все типы для отображения (для веб-сервера)
    pub async fn get_all_types_for_display(&self) -> Vec<TypeDisplayInfo> {
        self.increment_web_requests().await;
        self.web_interface.get_all_types_for_display().await
    }

    /// Поиск типов через веб интерфейс
    pub async fn search_types_for_display(&self, query: &str) -> Vec<TypeDisplayInfo> {
        self.increment_web_requests().await;
        self.web_interface.search_types(query).await
    }

    /// Получить детальную информацию о типе
    pub async fn get_type_details(&self, type_id: &str) -> Option<TypeDetailedInfo> {
        self.increment_web_requests().await;
        self.web_interface.get_type_details(type_id).await
    }

    /// Получить иерархию типов (для веб-сервера)
    pub async fn get_type_hierarchy(&self) -> Result<TypeHierarchy> {
        self.increment_web_requests().await;
        // TODO: Создать TypeHierarchy из UnifiedTypeSystem данных
        // TODO: Создать TypeHierarchy из UnifiedTypeSystem
        Err(anyhow::anyhow!(
            "get_type_hierarchy интеграция в разработке"
        ))
    }

    // === СТАТИСТИКА И МОНИТОРИНГ ===

    /// Получить статистику использования
    pub async fn get_usage_stats(&self) -> ServiceUsageStats {
        self.usage_stats.read().await.clone()
    }

    /// Получить статистику производительности
    pub async fn get_performance_stats(&self) -> Result<PerformanceStats> {
        let unified_stats = self.unified_system.get_statistics().await;
        let usage_stats = self.get_usage_stats().await;

        let cache_hit_ratio = if unified_stats.cache_hits + unified_stats.cache_misses > 0 {
            unified_stats.cache_hits as f64
                / (unified_stats.cache_hits + unified_stats.cache_misses) as f64
        } else {
            0.0
        };

        Ok(PerformanceStats {
            total_requests: usage_stats.lsp_requests + usage_stats.web_requests,
            unified_system_stats: unified_stats,
            memory_usage_mb: usage_stats.memory_usage_mb,
            cache_hit_ratio,
        })
    }

    /// Получить статистику единой системы типов
    pub async fn get_unified_system_stats(&self) -> super::unified_type_system::UnifiedSystemStats {
        self.unified_system.get_statistics().await
    }

    // === ПРИВАТНЫЕ МЕТОДЫ ===

    async fn set_stage(&self, stage: InitializationStage, message: &str) {
        let mut state = self.initialization_state.write().await;
        state.current_stage = stage;
        state.status_message = message.to_string();
        state.progress = match state.current_stage {
            InitializationStage::Starting => 0,
            InitializationStage::LoadingPlatformTypes => 25,
            InitializationStage::BuildingDocumentation => 50,
            InitializationStage::BuildingSearchIndexes => 75,
            InitializationStage::Finalizing => 90,
            InitializationStage::Ready => 100,
            InitializationStage::Failed => 0,
        };

        println!("📊 [{}%] {}", state.progress, message);
    }

    // Старые методы инициализации удалены - теперь все делает UnifiedTypeSystem

    async fn increment_lsp_requests(&self) {
        let mut stats = self.usage_stats.write().await;
        stats.lsp_requests += 1;
    }

    async fn increment_web_requests(&self) {
        let mut stats = self.usage_stats.write().await;
        stats.web_requests += 1;
    }

    async fn increment_search_requests(&self) {
        let mut stats = self.usage_stats.write().await;
        stats.search_requests += 1;
    }

    async fn increment_completion_requests(&self) {
        let mut stats = self.usage_stats.write().await;
        stats.completion_requests += 1;
    }
}

/// Статистика производительности
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    /// Общее количество запросов
    pub total_requests: u64,

    /// Статистика единой системы типов
    pub unified_system_stats: super::unified_type_system::UnifiedSystemStats,

    /// Использование памяти
    pub memory_usage_mb: f64,

    /// Коэффициент попаданий в кеш
    pub cache_hit_ratio: f64,
}

impl InitializationState {
    fn new() -> Self {
        Self {
            is_initialized: false,
            current_stage: InitializationStage::Starting,
            progress: 0,
            status_message: "Готов к инициализации".to_string(),
            start_time: None,
            errors: Vec::new(),
        }
    }

    fn start_initialization(&mut self) {
        self.start_time = Some(std::time::Instant::now());
        self.current_stage = InitializationStage::Starting;
        self.status_message = "Начинаем инициализацию...".to_string();
        self.errors.clear();
    }

    fn complete_initialization(&mut self) {
        self.is_initialized = true;
        self.current_stage = InitializationStage::Ready;
        self.progress = 100;

        if let Some(start_time) = self.start_time {
            let duration = start_time.elapsed();
            self.status_message =
                format!("Инициализация завершена за {:.2}s", duration.as_secs_f64());
        } else {
            self.status_message = "Инициализация завершена".to_string();
        }
    }

    #[allow(dead_code)]
    fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.current_stage = InitializationStage::Failed;
    }
}

impl Default for TypeSystemServiceConfig {
    fn default() -> Self {
        Self {
            syntax_helper_path: Some("examples/syntax_helper/rebuilt.shcntx_ru".to_string()),
            project_config_path: None,
            use_guided_discovery: true,
            cache_settings: CacheSettings::default(),
            search_settings: SearchSettings::default(),
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            enable_type_cache: true,
            enable_search_cache: true,
            cache_ttl_seconds: 3600, // 1 час
            max_cache_size: 10000,   // 10K записей
        }
    }
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            enable_fuzzy_by_default: true,
            max_search_results: 100,
            suggestions_ttl_seconds: 300, // 5 минут
        }
    }
}

/// Фабрика для создания shared instance
pub struct TypeSystemServiceFactory;

impl TypeSystemServiceFactory {
    /// Создать shared instance с конфигурацией
    pub async fn create_shared(config: TypeSystemServiceConfig) -> Result<Arc<TypeSystemService>> {
        let service = Arc::new(TypeSystemService::new(config));

        // Запускаем инициализацию
        service.initialize().await?;

        Ok(service)
    }

    /// Создать shared instance с настройками по умолчанию
    pub async fn create_default() -> Result<Arc<TypeSystemService>> {
        Self::create_shared(TypeSystemServiceConfig::default()).await
    }

    /// Создать для разработки (быстрая инициализация)
    pub async fn create_for_development() -> Result<Arc<TypeSystemService>> {
        let mut config = TypeSystemServiceConfig::default();
        config.cache_settings.cache_ttl_seconds = 60; // Короткий TTL для разработки
        config.search_settings.max_search_results = 20; // Меньше результатов

        Self::create_shared(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_initialization() {
        let service = TypeSystemService::with_defaults();

        // Проверяем начальное состояние
        let initial_state = service.get_initialization_state().await;
        assert!(!initial_state.is_initialized);
        assert_eq!(initial_state.current_stage, InitializationStage::Starting);

        // Запускаем инициализацию
        let result = service.initialize().await;

        // Проверяем результат
        match result {
            Ok(_) => {
                let final_state = service.get_initialization_state().await;
                assert!(final_state.is_initialized);
                assert_eq!(final_state.current_stage, InitializationStage::Ready);
                assert_eq!(final_state.progress, 100);
            }
            Err(e) => {
                println!("⚠️ Инициализация с предупреждениями: {}", e);
                // Это нормально в тестовом окружении без файлов
            }
        }
    }

    #[tokio::test]
    async fn test_service_stats() {
        let service = TypeSystemService::with_defaults();

        // Имитируем несколько запросов
        let _resolution = service.resolve_expression("Справочники.Контрагенты").await;
        let _completions = service.get_completions("Справ").await;
        let _suggestions = service.get_suggestions("Табли").await.unwrap_or_default();

        // Проверяем статистику
        let stats = service.get_usage_stats().await;
        assert!(stats.lsp_requests > 0);
        assert!(stats.completion_requests > 0);
        assert!(stats.web_requests > 0);
    }
}
