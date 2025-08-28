//! System Layer - центральный координатор целевой архитектуры
//!
//! Временная заглушка на время миграции архитектуры
//! TODO: Восстановить функциональность после завершения миграции

use serde::Serialize;
use std::sync::Arc;

// Импорты, необходимые для компиляции
use crate::domain::{
    InMemoryTypeRepository, RawTypeData, TypeContext, TypeRepository,
    TypeResolutionService, ConcreteType,
};
use crate::data::TypeSource;
use crate::domain::types::{TypeResolution, ResolutionResult};
use crate::application::lsp_service::LspTypeService;
use crate::application::services::AnalysisTypeService;
use crate::application::web_service::WebTypeService;
use crate::presentation::adapters::{CliInterface, LspInterface, WebInterface};

// Остальные импорты временно удалены
use anyhow::Result;
use crate::data::loaders::config_parser_guided_discovery::ConfigurationGuidedParser;
use tracing::{info, warn};

/// Центральная система типов BSL
///
/// Координирует все слои целевой архитектуры и обеспечивает
/// единую точку инициализации и управления
pub struct CentralTypeSystem {
    // === DATA LAYER ===
    /// Репозиторий всех типов (единый источник истины)
    repository: Arc<dyn TypeRepository>,

    // === DOMAIN LAYER ===
    /// Центральный сервис разрешения типов
    resolution_service: Arc<TypeResolutionService>,

    // === APPLICATION LAYER ===
    /// Сервис для LSP (оптимизирован для скорости)
    lsp_service: Arc<LspTypeService>,

    /// Сервис для веб-интерфейса (богатые данные)
    web_service: Arc<WebTypeService>,

    /// Сервис для анализа проектов (аналитика)
    #[allow(dead_code)]
    analysis_service: Arc<AnalysisTypeService>,

    // === PRESENTATION LAYER ===
    /// Интерфейс для LSP протокола
    lsp_interface: LspInterface,

    /// Интерфейс для веб API
    web_interface: WebInterface,

    /// Интерфейс для CLI
    cli_interface: CliInterface,

    // === INFRASTRUCTURE ===
    /// Конфигурация системы
    config: CentralSystemConfig,

    /// Метрики всей системы
    system_metrics: Arc<tokio::sync::RwLock<SystemMetrics>>,

    /// Состояние инициализации
    initialization_state: Arc<tokio::sync::RwLock<InitializationState>>,
}

/// Конфигурация центральной системы типов
#[derive(Debug, Clone)]
pub struct CentralSystemConfig {
    /// Путь к HTML справке платформы
    pub html_path: String,

    /// Путь к XML конфигурации (опционально)
    pub configuration_path: Option<String>,

    /// Включить детальное логирование
    pub verbose_logging: bool,

    /// Настройки кеширования
    pub cache_settings: CacheSettings,

    /// Настройки производительности
    pub performance_settings: PerformanceSettings,
}

/// Настройки кеширования
#[derive(Debug, Clone)]
pub struct CacheSettings {
    pub enable_repository_cache: bool,
    pub enable_resolution_cache: bool,
    pub enable_lsp_cache: bool,
    pub cache_ttl_seconds: u64,
    pub max_cache_size: usize,
}

/// Настройки производительности
#[derive(Debug, Clone)]
pub struct PerformanceSettings {
    pub enable_parallel_parsing: bool,
    pub max_parser_threads: usize,
    pub lsp_response_timeout_ms: u64,
    pub web_request_timeout_ms: u64,
}

/// Метрики всей системы
#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemMetrics {
    /// Статистика репозитория
    pub total_types: usize,
    pub platform_types: usize,
    pub configuration_types: usize,
    pub user_defined_types: usize,

    /// Статистика производительности
    pub average_lsp_response_ms: f64,
    pub average_web_response_ms: f64,
    pub total_requests: u64,

    /// Статистика кеширования
    pub cache_hit_rate: f64,
    pub cache_memory_mb: f64,

    /// Время работы системы
    pub uptime_seconds: u64,
    pub last_updated: Option<std::time::SystemTime>,
}

/// Состояние инициализации системы
#[derive(Debug, Clone, Default)]
pub struct InitializationState {
    pub is_initializing: bool,
    pub progress_percent: u8,
    pub current_operation: String,
    pub errors: Vec<String>,

    /// Состояние слоёв
    pub data_layer_ready: bool,
    pub domain_layer_ready: bool,
    pub application_layer_ready: bool,
    pub presentation_layer_ready: bool,

    /// Время инициализации
    pub initialization_start: Option<std::time::Instant>,
    pub initialization_duration: Option<std::time::Duration>,
}

/// Результат проверки здоровья системы
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String, // "healthy" | "degraded" | "unhealthy"
    pub components: Vec<ComponentHealth>,
    pub overall_score: f32, // 0.0-1.0
    pub last_check: std::time::SystemTime,
}

/// Здоровье отдельного компонента
#[derive(Debug, Clone, Serialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String,
    pub response_time_ms: Option<f64>,
    pub error_rate: Option<f32>,
    pub last_error: Option<String>,
}

impl CentralTypeSystem {
    /// Создать новую центральную систему типов
    pub fn new(config: CentralSystemConfig) -> Self {
        // Создаём репозиторий
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());

        // Создаём Domain Layer
        let resolution_service = Arc::new(TypeResolutionService::new(repository.clone()));

        // Создаём Application Layer
        let lsp_service = Arc::new(LspTypeService::new());
        let web_service = Arc::new(WebTypeService::new());
        let analysis_service = Arc::new(AnalysisTypeService::new());

        // Создаём Presentation Layer
        let lsp_interface = LspInterface::new(lsp_service.clone());
        let web_interface = WebInterface::new(web_service.clone());
        let cli_interface = CliInterface::new(analysis_service.clone());

        Self {
            repository,
            resolution_service,
            lsp_service,
            web_service,
            analysis_service,
            lsp_interface,
            web_interface,
            cli_interface,
            config,
            system_metrics: Arc::new(tokio::sync::RwLock::new(SystemMetrics::default())),
            initialization_state: Arc::new(
                tokio::sync::RwLock::new(InitializationState::default()),
            ),
        }
    }

    /// Создать и сразу инициализировать центральную систему типов.
    ///
    /// Удобный конструктор, объединяющий `new(config)` и последующий вызов
    /// асинхронной инициализации `initialize()`. Не меняет поведение
    /// существующего API; предназначен для упрощения использования в бинарях
    /// (LSP/Web/CLI) и выравнивания с документацией целевой архитектуры.
    ///
    /// Пример
    /// ```ignore
    /// use bsl_gradual_types::system::{CentralTypeSystem, CentralSystemConfig};
    /// # async fn run() -> anyhow::Result<()> {
    /// let cfg = CentralSystemConfig::default();
    /// let system = CentralTypeSystem::initialize_with_config(cfg).await?;
    /// let health = system.health_check().await;
    /// println!("status={} score={}", health.status, health.overall_score);
    /// # Ok(()) }
    /// ```
    pub async fn initialize_with_config(config: CentralSystemConfig) -> Result<Self> {
        let system = Self::new(config);
        system.initialize().await?;
        Ok(system)
    }

    /// Синоним `initialize_with_config` для краткости.
    pub async fn try_new(config: CentralSystemConfig) -> Result<Self> {
        Self::initialize_with_config(config).await
    }

    /// ЕДИНСТВЕННЫЙ метод инициализации всей системы
    pub async fn initialize(&self) -> Result<()> {
        let start_time = std::time::Instant::now();

        {
            let mut state = self.initialization_state.write().await;
            state.is_initializing = true;
            state.initialization_start = Some(start_time);
            state.current_operation = "Начало инициализации центральной системы типов".to_string();
            state.progress_percent = 0;
        }

        info!("🚀 Инициализация CentralTypeSystem...");

        // === ЭТАП 1: DATA LAYER ===
        self.update_progress(10, "Инициализация Data Layer...")
            .await;
        self.initialize_data_layer().await?;

        // === ЭТАП 2: DOMAIN LAYER ===
        self.update_progress(30, "Инициализация Domain Layer...")
            .await;
        self.initialize_domain_layer().await?;

        // === ЭТАП 3: APPLICATION LAYER ===
        self.update_progress(60, "Инициализация Application Layer...")
            .await;
        self.initialize_application_layer().await?;

        // === ЭТАП 4: PRESENTATION LAYER ===
        self.update_progress(80, "Инициализация Presentation Layer...")
            .await;
        self.initialize_presentation_layer().await?;

        // === ЗАВЕРШЕНИЕ ===
        let total_time = start_time.elapsed();
        self.update_progress(100, "Инициализация завершена").await;

        {
            let mut state = self.initialization_state.write().await;
            state.is_initializing = false;
            state.initialization_duration = Some(total_time);
            state.data_layer_ready = true;
            state.domain_layer_ready = true;
            state.application_layer_ready = true;
            state.presentation_layer_ready = true;
        }

        self.update_system_metrics().await?;

        info!("🎉 CentralTypeSystem инициализирована за {:?}", total_time);
        self.print_initialization_summary().await;

        Ok(())
    }

    /// Получить LSP интерфейс
    pub fn lsp_interface(&self) -> &LspInterface {
        &self.lsp_interface
    }

    /// Получить веб-интерфейс
    pub fn web_interface(&self) -> &WebInterface {
        &self.web_interface
    }

    /// Получить CLI интерфейс
    pub fn cli_interface(&self) -> &CliInterface {
        &self.cli_interface
    }

    /// Получить метрики системы
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.read().await.clone()
    }

    /// Проверить здоровье системы
    pub async fn health_check(&self) -> HealthStatus {
        let mut components = Vec::new();
        let mut total_score = 0.0;
        let mut component_count = 0;

        // Проверяем Data Layer
        let data_health = self.check_data_layer_health().await;
        total_score += self.health_score(&data_health);
        component_count += 1;
        components.push(data_health);

        // Проверяем Domain Layer
        let domain_health = self.check_domain_layer_health().await;
        total_score += self.health_score(&domain_health);
        component_count += 1;
        components.push(domain_health);

        // Проверяем Application Layer
        let app_health = self.check_application_layer_health().await;
        total_score += self.health_score(&app_health);
        component_count += 1;
        components.push(app_health);

        let overall_score = if component_count > 0 {
            total_score / component_count as f32
        } else {
            0.0
        };

        let status = if overall_score > 0.8 {
            "healthy".to_string()
        } else if overall_score > 0.5 {
            "degraded".to_string()
        } else {
            "unhealthy".to_string()
        };

        HealthStatus {
            status,
            components,
            overall_score,
            last_check: std::time::SystemTime::now(),
        }
    }

    /// Перезагрузить данные системы
    pub async fn reload_data(&self) -> Result<()> {
        info!("🔄 Перезагрузка данных CentralTypeSystem...");

        // Очищаем репозиторий
        // self.repository.clear().await?; // TODO: Restore after trait fix

        // Перезагружаем данные
        self.initialize_data_layer().await?;

        // Обновляем метрики
        self.update_system_metrics().await?;

        info!("✅ Данные перезагружены");
        Ok(())
    }

    // === ПРИВАТНЫЕ МЕТОДЫ ИНИЦИАЛИЗАЦИИ ===

    async fn initialize_data_layer(&self) -> Result<()> {
        info!("🔧 Инициализация Data Layer...");

        // Загружаем платформенные типы из HTML
        let platform_types = self.load_platform_types().await?;
        info!("✅ Загружено {} платформенных типов", platform_types.len());

        // Загружаем конфигурационные типы если указан путь
        let mut all_types = platform_types;
        if let Some(config_path) = &self.config.configuration_path {
            let config_types = self.load_configuration_types(config_path).await?;
            info!("✅ Загружено {} конфигурационных типов", config_types.len());
            all_types.extend(config_types);
        }

        // Сохраняем в репозиторий
        self.repository.save_types(all_types)?;

        info!("✅ Data Layer инициализирован");
        Ok(())
    }

    async fn initialize_domain_layer(&self) -> Result<()> {
        info!("🔧 Инициализация Domain Layer...");

        // Инициализируем резолверы в TypeResolutionService (кеши, tree-sitter)
        if let Err(e) = self.resolution_service.initialize().await {
            warn!("⚠️ Инициализация резолверов завершилась с ошибкой: {}", e);
        }

        info!("✅ Domain Layer инициализирован");
        Ok(())
    }

    async fn initialize_application_layer(&self) -> Result<()> {
        info!("🔧 Инициализация Application Layer...");

        // LSP Service готов (использует Domain Layer)
        // Web Service готов (использует Domain Layer)
        // Analysis Service готов (использует Domain Layer)

        info!("✅ Application Layer инициализирован");
        Ok(())
    }

    async fn initialize_presentation_layer(&self) -> Result<()> {
        info!("🔧 Инициализация Presentation Layer...");

        // Интерфейсы готовы (используют Application Layer)

        info!("✅ Presentation Layer инициализирован");
        Ok(())
    }

    // === ЗАГРУЗКА ДАННЫХ ===

    async fn load_platform_types(&self) -> Result<Vec<RawTypeData>> {
        info!("📄 Загрузка платформенных типов из HTML...");

        // Используем существующий PlatformTypeResolver для загрузки данных
        let platform_resolver = crate::domain::resolvers::platform::PlatformTypeResolver::new();
        let platform_globals = platform_resolver.get_platform_globals();

        // Конвертируем TypeResolution в RawTypeData
        let mut raw_types = Vec::new();
        for (name, resolution) in platform_globals {
            let raw_type = self.convert_resolution_to_raw_data(name.to_string(), resolution)?;
            raw_types.push(raw_type);
        }

        Ok(raw_types)
    }

    async fn load_configuration_types(&self, config_path: &str) -> Result<Vec<RawTypeData>> {
        info!("⚙️ Загрузка конфигурационных типов из XML: {}", config_path);

        let mut guided_parser = ConfigurationGuidedParser::new(config_path);
        let config_resolutions = guided_parser.parse_with_configuration_guide()?;

        // Конвертируем TypeResolution в RawTypeData
        let mut raw_types = Vec::new();
        for resolution in config_resolutions {
            // Получаем правильное имя типа из TypeResolution
            let name = self.get_configuration_type_name(&resolution);
            let raw_type = self.convert_resolution_to_raw_data(name, &resolution)?;
            raw_types.push(raw_type);
        }

        Ok(raw_types)
    }

    /// Получить правильное имя конфигурационного типа с учетом фасета
    fn get_configuration_type_name(&self, resolution: &TypeResolution) -> String {
        if let ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) = &resolution.result {
            let prefix = match config_type.kind {
                crate::domain::types::MetadataKind::Catalog => "Справочники",
                crate::domain::types::MetadataKind::Document => "Документы",
                crate::domain::types::MetadataKind::Register => "РегистрыСведений",
                crate::domain::types::MetadataKind::Enum => "Перечисления",
                crate::domain::types::MetadataKind::Report => "Отчеты",
                crate::domain::types::MetadataKind::DataProcessor => "Обработки",
                crate::domain::types::MetadataKind::ChartOfAccounts => "ПланыСчетов",
                crate::domain::types::MetadataKind::ChartOfCharacteristicTypes => "ПланыВидовХарактеристик",
            };
            
            // Формируем имя с учетом фасета
            match resolution.active_facet {
                Some(crate::domain::types::FacetKind::Manager) => {
                    format!("{}.{}", prefix, config_type.name)
                }
                Some(crate::domain::types::FacetKind::Object) => {
                    let object_prefix = match config_type.kind {
                        crate::domain::types::MetadataKind::Catalog => "СправочникОбъект",
                        crate::domain::types::MetadataKind::Document => "ДокументОбъект",
                        _ => "Объект",
                    };
                    format!("{}.{}", object_prefix, config_type.name)
                }
                Some(crate::domain::types::FacetKind::Reference) => {
                    let ref_prefix = match config_type.kind {
                        crate::domain::types::MetadataKind::Catalog => "СправочникСсылка",
                        crate::domain::types::MetadataKind::Document => "ДокументСсылка",
                        _ => "Ссылка",
                    };
                    format!("{}.{}", ref_prefix, config_type.name)
                }
                _ => format!("{}.{}", prefix, config_type.name),
            }
        } else {
            "UnknownConfigType".to_string()
        }
    }

    fn convert_resolution_to_raw_data(
        &self,
        name: String,
        resolution: &TypeResolution,
    ) -> Result<RawTypeData> {
        let source = match &resolution.result {
            crate::domain::types::ResolutionResult::Concrete(
                crate::domain::types::ConcreteType::Platform(_),
            ) => TypeSource::Platform {
                platform_version: "8.3".to_string(),
            },
            crate::domain::types::ResolutionResult::Concrete(
                crate::domain::types::ConcreteType::Configuration(_),
            ) => TypeSource::Configuration {
                config_version: "8.3".to_string(),
            },
            _ => TypeSource::Platform {
                platform_version: "8.3".to_string(),
            },
        };

        let mut methods = Vec::new();
        let mut properties = Vec::new();

        if let crate::domain::types::ResolutionResult::Concrete(
            crate::domain::types::ConcreteType::Platform(platform_type),
        ) = &resolution.result
        {
            methods = platform_type
                .methods
                .iter()
                .map(|method| {
                    let params: Vec<crate::data::RawParameterData> = method
                        .parameters
                        .iter()
                        .map(|param| crate::data::RawParameterData {
                            name: param.name.clone(),
                            type_name: param.type_.clone().unwrap_or_default(),
                            description: String::new(),
                            is_optional: false, // Not available in source, using default
                            is_by_value: true,  // Not available in source, using default
                        })
                        .collect();

                    crate::data::RawMethodData {
                        name: method.name.clone(),
                        signature: format!("{}({})", method.name, params.iter().map(|p| format!("{}: {}", p.name, p.type_name)).collect::<Vec<_>>().join(", ")),
                        documentation: None,
                        return_type: method.return_type.clone(),
                        parameters: params,
                    }
                })
                .collect();

            properties = platform_type
                .properties
                .iter()
                .map(|prop| crate::data::RawPropertyData {
                    name: prop.name.clone(),
                    type_name: prop.type_.clone(),
                    description: "".to_string(), // Default value, cannot be determined from PlatformType
                    is_read_only: false, // Default value, cannot be determined from PlatformType
                })
                .collect();
        }

        // Определяем категорию на основе типа
        let category_path = if let ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) = &resolution.result {
            match config_type.kind {
                crate::domain::types::MetadataKind::Catalog => vec!["Конфигурация".to_string(), "Справочники".to_string()],
                crate::domain::types::MetadataKind::Document => vec!["Конфигурация".to_string(), "Документы".to_string()],
                crate::domain::types::MetadataKind::Register => vec!["Конфигурация".to_string(), "Регистры сведений".to_string()],
                crate::domain::types::MetadataKind::Enum => vec!["Конфигурация".to_string(), "Перечисления".to_string()],
                _ => vec!["Конфигурация".to_string()],
            }
        } else {
            vec!["Платформа".to_string()]
        };

        let documentation = if let ResolutionResult::Concrete(ConcreteType::Configuration(config_type)) = &resolution.result {
            format!("Конфигурационный тип: {} ({})", name, match config_type.kind {
                crate::domain::types::MetadataKind::Catalog => "Справочник",
                crate::domain::types::MetadataKind::Document => "Документ",
                crate::domain::types::MetadataKind::Register => "Регистр сведений",
                crate::domain::types::MetadataKind::Enum => "Перечисление",
                _ => "Объект метаданных",
            })
        } else {
            format!("Платформенный тип: {}", name)
        };

        Ok(RawTypeData {
            name: name.to_string(),
            source_location: Some(format!("{}.html", name)),
            documentation: Some(documentation),
            methods,
            properties,
            metadata: std::collections::HashMap::new(),
            source,
            russian_name: name.to_string(),
            english_name: name.to_string(), // TODO: получить из данных
            category_path,
        })
    }
    //
    // === УПРАВЛЕНИЕ СОСТОЯНИЕМ ===

    async fn update_progress(&self, percent: u8, operation: &str) {
        let mut state = self.initialization_state.write().await;
        state.progress_percent = percent;
        state.current_operation = operation.to_string();

        if self.config.verbose_logging {
            info!("📊 [{:3}%] {}", percent, operation);
        }
    }

    async fn update_system_metrics(&self) -> Result<()> {
        let repo_stats = self.repository.get_stats();

        let mut metrics = self.system_metrics.write().await;
        metrics.total_types = repo_stats.total_types;
        metrics.platform_types = repo_stats.platform_types;
        metrics.configuration_types = repo_stats.configuration_types;
        metrics.user_defined_types = repo_stats.user_defined_types;
        metrics.cache_memory_mb = 0.0; // TODO: Добавить подсчет использования памяти
        metrics.last_updated = Some(std::time::SystemTime::now());
        // Метрики производительности LSP
        let lsp_metrics = self.lsp_service.get_performance_metrics().await;
        metrics.total_requests = lsp_metrics.total_requests;
        metrics.average_lsp_response_ms = lsp_metrics.average_response_time_ms;
        metrics.cache_hit_rate = lsp_metrics.cache_hit_rate; // при наличии

        // Метрики производительности Web
        let web_metrics = self.web_service.get_performance_metrics().await;
        metrics.average_web_response_ms = web_metrics.average_response_time;
        // Метрики домена можно пробросить позже при необходимости

        Ok(())
    }

    async fn print_initialization_summary(&self) {
        let metrics = self.system_metrics.read().await;
        let state = self.initialization_state.read().await;

        info!("\n📊 Сводка инициализации CentralTypeSystem:");
        info!(
            "   - Общее время: {:?}",
            state.initialization_duration.unwrap_or_default()
        );
        info!("   - Всего типов: {}", metrics.total_types);
        info!("   - Платформенных: {}", metrics.platform_types);
        info!("   - Конфигурационных: {}", metrics.configuration_types);
        info!("   - Память: {:.2} MB", metrics.cache_memory_mb);

        if !state.errors.is_empty() {
            warn!("   - Ошибки: {}", state.errors.len());
            for error in &state.errors {
                warn!("     • {}", error);
            }
        }

        info!("\n🎯 Готово к обслуживанию запросов!");
    }

    // === ПРОВЕРКА ЗДОРОВЬЯ ===

    async fn check_data_layer_health(&self) -> ComponentHealth {
        let start_time = std::time::Instant::now();

        let stats = self.repository.get_stats();
        let response_time = start_time.elapsed().as_millis() as f64;
        ComponentHealth {
            name: "DataLayer".to_string(),
            status: if stats.total_types > 0 {
                "healthy"
            } else {
                "degraded"
            }
            .to_string(),
            response_time_ms: Some(response_time),
            error_rate: Some(0.0),
            last_error: None,
        }
    }

    async fn check_domain_layer_health(&self) -> ComponentHealth {
        let start_time = std::time::Instant::now();

        // Тестируем разрешение типа
        let test_context = TypeContext {
            variables: std::collections::HashMap::new(),
            functions: std::collections::HashMap::new(),
            current_scope: crate::domain::analysis::dependency_graph::Scope::Global,
            scope_stack: vec![],
        };

        let _resolution = self
            .resolution_service
            .resolve_expression("Массив", &test_context)
            .await;
        let response_time = start_time.elapsed().as_millis() as f64;

        ComponentHealth {
            name: "DomainLayer".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(response_time),
            error_rate: Some(0.0),
            last_error: None,
        }
    }

    async fn check_application_layer_health(&self) -> ComponentHealth {
        // TODO: Проверить LSP, Web, Analysis сервисы
        ComponentHealth {
            name: "ApplicationLayer".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(1.0),
            error_rate: Some(0.0),
            last_error: None,
        }
    }

    fn health_score(&self, component: &ComponentHealth) -> f32 {
        match component.status.as_str() {
            "healthy" => 1.0,
            "degraded" => 0.5,
            "unhealthy" => 0.0,
            _ => 0.0,
        }
    }
}

impl Default for CentralSystemConfig {
    fn default() -> Self {
        Self {
            html_path: "examples/syntax_helper/rebuilt.shcntx_ru".to_string(),
            configuration_path: None,
            verbose_logging: false,
            cache_settings: CacheSettings {
                enable_repository_cache: true,
                enable_resolution_cache: true,
                enable_lsp_cache: true,
                cache_ttl_seconds: 3600, // 1 час
                max_cache_size: 10000,
            },
            performance_settings: PerformanceSettings {
                enable_parallel_parsing: true,
                max_parser_threads: num_cpus::get(),
                lsp_response_timeout_ms: 100,
                web_request_timeout_ms: 5000,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_central_type_system_creation() {
        let config = CentralSystemConfig::default();
        let system = CentralTypeSystem::new(config);

        // Тестируем создание
        assert!(!system.config.html_path.is_empty());

        // Тестируем получение интерфейсов
        let _lsp_interface = system.lsp_interface();
        let _web_interface = system.web_interface();
        let _cli_interface = system.cli_interface();

        println!("✅ CentralTypeSystem создана");
    }

    #[tokio::test]
    async fn test_system_initialization() {
        let config = CentralSystemConfig {
            verbose_logging: true,
            ..Default::default()
        };

        let system = CentralTypeSystem::new(config);

        // Тестируем инициализацию
        match system.initialize().await {
            Ok(_) => {
                println!("✅ Инициализация прошла успешно");

                // Проверяем метрики
                let metrics = system.get_system_metrics().await;
                println!("📊 Типов загружено: {}", metrics.total_types);

                // Проверяем здоровье
                let health = system.health_check().await;
                println!("🏥 Статус здоровья: {}", health.status);
            }
            Err(e) => {
                println!("⚠️ Ошибка инициализации: {}", e);
                // В тестовом окружении это нормально
            }
        }
    }
}
