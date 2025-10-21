//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture
//! Координирует только System Layer компоненты

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

use crate::application::type_system_service::TypeSystemService;
use crate::data::adapters::convert_syntax_helper_to_raw;
use crate::data::loaders::SyntaxHelperParser;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::RawTypeData;
use bsl_shared::engine::AnalysisEngine;

use super::basic_observability::BasicObservability;
use super::ir_cache::IrCache;
use super::parser_coordinator::ParserCoordinator;
use super::simple_cache::AnalysisCache;

/// Упрощенный системный координатор
///
/// Заменяет CentralTypeSystem, координирует только System Layer компоненты
pub struct SystemCoordinator {
    // === SYSTEM LAYER COMPONENTS ONLY ===
    cache: Arc<AnalysisCache>,
    ir_cache: Arc<IrCache>, // Milestone 2.13: IR кеширование для LSP hover
    parser: Arc<ParserCoordinator>,
    observability: Arc<BasicObservability>,

    // === ANALYSIS ENGINE CACHE ===
    analysis_engine_cache: Mutex<Option<Arc<AnalysisEngine>>>,

    // === TYPE SERVICE CACHE ===
    type_service_cache: Mutex<Option<Arc<TypeSystemService>>>,
}

impl Default for SystemCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCoordinator {
    /// Создать новый системный координатор
    pub fn new() -> Self {
        // ТОЛЬКО System Layer компоненты согласно архитектурной диаграмме

        // 1. Simple caching
        let cache = Arc::new(AnalysisCache::new(1000)); // Simple LRU

        // 2. IR caching (Milestone 2.13)
        let ir_cache = Arc::new(IrCache::new(100)); // 100 файлов (~10 MB RAM)

        // 3. Simple parsing
        let parser = Arc::new(ParserCoordinator::with_fallback());

        // 4. Basic observability
        let observability = Arc::new(BasicObservability::default());

        Self {
            cache,
            ir_cache,
            parser,
            observability,
            analysis_engine_cache: Mutex::new(None),
            type_service_cache: Mutex::new(None),
        }
    }

    /// Инициализация системы с реальным парсингом синтаксис-помощника
    pub async fn start(&self) -> Result<(), StartupError> {
        self.start_with_paths(None, None).await
    }

    /// Инициализация системы с настраиваемыми путями
    pub async fn start_with_paths(
        &self,
        syntax_helper_path: Option<&Path>,
        config_path: Option<&Path>,
    ) -> Result<(), StartupError> {
        self.observability.log_startup();

        info!("🎯 SystemCoordinator: инициализация System Layer...");

        // ✅ КРИТИЧЕСКИ ВАЖНО: Очищаем кеши при повторной инициализации
        // Это гарантирует, что TypeSystemService получит НОВЫЙ AnalysisEngine с НОВЫМ TypeRepository
        {
            let mut engine_cache = self.analysis_engine_cache.lock().unwrap();
            let mut service_cache = self.type_service_cache.lock().unwrap();

            if engine_cache.is_some() || service_cache.is_some() {
                info!("🔄 Очищаем кеши AnalysisEngine и TypeSystemService для повторной инициализации");
                *engine_cache = None;
                *service_cache = None;
            }
        }

        // === PHASE 3: Infrastructure инициализация в SystemCoordinator ===

        // 1. Создаем Infrastructure компоненты (Data Layer)
        info!("📦 SystemCoordinator: инициализация Data Layer loaders...");
        let mut syntax_parser = SyntaxHelperParser::new();

        // 2. Загружаем синтаксис-помощник если путь указан
        if let Some(syntax_path) = syntax_helper_path {
            info!("📂 Загружаем синтаксис-помощник: {}", syntax_path.display());

            match syntax_parser.parse_syntax_helper(syntax_path) {
                Ok(()) => {
                    info!("✅ Парсинг синтаксис-помощника завершен успешно");
                }
                Err(e) => {
                    warn!("⚠️ Ошибка парсинга синтаксис-помощника: {}", e);
                    info!("📦 Будем использовать базовые типы платформы 1С...");
                }
            }
        }

        // 3. Создаем Domain Layer компоненты
        info!("🧠 SystemCoordinator: инициализация Domain Layer...");
        let repository = Arc::new(InMemoryTypeRepository::new());

        // 4. Загружаем данные в репозиторий (через Adapters)
        let database = syntax_parser.export_database();
        if !database.nodes.is_empty() {
            let platform_raw_data = convert_syntax_helper_to_raw(&database);
            repository
                .load_types(platform_raw_data)
                .map_err(StartupError::PlatformTypesError)?;

            let stats = repository.get_stats();
            info!(
                "📊 Загружено {} типов из синтаксис-помощника",
                stats.total_types
            );
        } else {
            // Загружаем базовые типы как fallback
            Self::load_fallback_types(&repository)?;
        }

        // 5. Создаем Domain resolver
        let resolver = Arc::new(TypeResolver::new(repository.clone()));

        // 6. Создаем упрощенный AnalysisEngine (без Infrastructure зависимостей)
        let analysis_engine = AnalysisEngine::new(resolver, repository);

        // Кешируем AnalysisEngine
        {
            let mut cache = self.analysis_engine_cache.lock().unwrap();
            *cache = Some(Arc::new(analysis_engine));
        }

        // 7. Загружаем метаданные конфигурации если путь указан
        if let Some(config_path) = config_path {
            info!(
                "📂 Загружаем метаданные конфигурации: {}",
                config_path.display()
            );

            match self.load_configuration_metadata(config_path) {
                Ok(count) => {
                    info!("✅ Загружено {} объектов метаданных конфигурации", count);
                }
                Err(e) => {
                    warn!("⚠️ Ошибка загрузки метаданных конфигурации: {}", e);
                    info!("📦 Продолжаем работу с типами платформы...");
                }
            }
        }

        info!("💾 SystemCoordinator: прогрев кеша...");
        self.cache.warm_cache()?;

        info!("✅ SystemCoordinator: система готова!");
        Ok(())
    }

    /// Загрузка базовых типов как fallback
    fn load_fallback_types(repository: &Arc<InMemoryTypeRepository>) -> Result<(), StartupError> {
        use bsl_shared::domain::types::{RawDataSource, RawTypeData};

        info!("📦 Загружаем базовые типы платформы 1С...");

        let basic_types = vec![
            RawTypeData {
                name: "Строка".to_string(),
                english_name: "String".to_string(),
                description: "Строковый тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Число".to_string(),
                english_name: "Number".to_string(),
                description: "Числовой тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Булево".to_string(),
                english_name: "Boolean".to_string(),
                description: "Логический тип данных".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
            RawTypeData {
                name: "Дата".to_string(),
                english_name: "Date".to_string(),
                description: "Тип данных для работы с датой и временем".to_string(),
                category: "Примитивные типы".to_string(),
                source: RawDataSource::Platform,
                ..Default::default()
            },
        ];

        repository
            .load_types(basic_types)
            .map_err(StartupError::PlatformTypesError)?;

        info!("✅ Базовые типы загружены: 4 типа");
        Ok(())
    }

    /// Получить компоненты для создания TypeSystemService
    pub fn get_system_components(&self) -> (Arc<AnalysisCache>, Arc<ParserCoordinator>) {
        (self.cache.clone(), self.parser.clone())
    }

    /// Получить ParserCoordinator (Milestone 2.18: для синтаксических ошибок в LSP)
    pub fn parser_coordinator(&self) -> Option<Arc<ParserCoordinator>> {
        Some(self.parser.clone())
    }

    /// Получить IR Cache
    pub fn ir_cache(&self) -> Arc<IrCache> {
        self.ir_cache.clone()
    }

    /// Получить AnalysisEngine (делегирует Domain Layer логику)
    pub fn get_analysis_engine(&self) -> Option<Arc<AnalysisEngine>> {
        let cache = self.analysis_engine_cache.lock().unwrap();
        cache.clone()
    }

    /// Создать TypeSystemService (singleton)
    ///
    /// Согласно архитектуре: TypeSystemService использует AnalysisEngine для доступа к Domain Layer
    pub fn type_service(&self) -> Option<Arc<TypeSystemService>> {
        let mut cache = self.type_service_cache.lock().unwrap();
        if let Some(service) = cache.as_ref() {
            return Some(service.clone());
        }

        // Получаем AnalysisEngine
        let analysis_engine = {
            let engine_cache = self.analysis_engine_cache.lock().unwrap();
            engine_cache.clone()
        };

        if let Some(engine) = analysis_engine {
            // TypeSystemService теперь использует AnalysisEngine вместо прямого доступа к Domain Layer
            let service = Arc::new(TypeSystemService::new(
                engine,
                self.cache.clone(),
                self.parser.clone(),
                self.ir_cache.clone(), // Milestone 2.13: передаём IR Cache
            ));

            *cache = Some(service.clone());
            Some(service)
        } else {
            None
        }
    }

    /// Получить AnalysisEngine для CLI/прямого использования
    pub fn analysis_engine(&self) -> Option<Arc<AnalysisEngine>> {
        let cache = self.analysis_engine_cache.lock().unwrap();
        cache.clone()
    }

    /// Health check
    pub fn health_status(&self) -> crate::system::basic_observability::HealthStatus {
        self.observability.health_check()
    }

    /// Загрузить метаданные конфигурации через универсальный парсер
    ///
    /// Использует ConfigurationDiscovery для автоматического обнаружения всех объектов метаданных
    /// и загружает их в TypeRepository текущего AnalysisEngine.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use bsl_backend::system::SystemCoordinator;
    ///
    /// let coordinator = SystemCoordinator::new();
    /// let config_path = Path::new("examples/conf/conf_test");
    /// let loaded = coordinator.load_configuration_metadata(config_path).unwrap();
    /// println!("Загружено {} объектов метаданных", loaded);
    /// ```
    pub fn load_configuration_metadata(&self, config_path: &Path) -> Result<usize> {
        use crate::data::loaders::ConfigurationDiscovery;

        info!("🔍 Загрузка метаданных конфигурации из {:?}", config_path);

        // Создаём discovery и обнаруживаем все объекты
        let discovery = ConfigurationDiscovery::new(config_path.to_path_buf());
        let metadata_objects = discovery
            .discover_all_metadata()
            .map_err(|e| anyhow::anyhow!("Не удалось обнаружить метаданные: {}", e))?;

        info!(
            "📦 Обнаружено {} объектов метаданных",
            metadata_objects.len()
        );

        // Получаем текущий AnalysisEngine или создаём новый
        let engine = self.analysis_engine().ok_or_else(|| {
            anyhow::anyhow!("AnalysisEngine не инициализирован. Вызовите start() сначала.")
        })?;

        // Получаем TypeRepository из AnalysisEngine
        let repository = engine.get_repository();

        // Конвертируем все объекты в RawTypeData
        let raw_types: Vec<RawTypeData> = metadata_objects
            .into_iter()
            .map(|obj| obj.to_raw_type_data())
            .collect();

        let count = raw_types.len();

        // Загружаем все типы в репозиторий за один вызов
        repository.load_types(raw_types)?;

        info!("✅ Загружено {} типов из конфигурации", count);
        Ok(count)
    }
}

// === ERROR TYPES ===

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("Failed to load platform types: {0}")]
    PlatformTypesError(#[from] anyhow::Error),
    #[error("Cache initialization failed: {0}")]
    CacheError(String),
}

/// Информация о символе для LSP
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub symbol_type: String,
    pub line: u32,
    pub column: u32,
}
