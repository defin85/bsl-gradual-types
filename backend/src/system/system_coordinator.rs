//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture
//! Координирует только System Layer компоненты

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::path::Path;
use tracing::{info, warn};

use bsl_shared::engine::AnalysisEngine;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use crate::application::type_system_service::TypeSystemService;
use crate::data::loaders::SyntaxHelperParser;
use crate::data::adapters::convert_syntax_helper_to_raw;

use super::basic_observability::BasicObservability;
use super::parser_coordinator::ParserCoordinator;
use super::simple_cache::AnalysisCache;

/// Упрощенный системный координатор
///
/// Заменяет CentralTypeSystem, координирует только System Layer компоненты
pub struct SystemCoordinator {
    // === SYSTEM LAYER COMPONENTS ONLY ===
    cache: Arc<AnalysisCache>,
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

        // 2. Simple parsing
        let parser = Arc::new(ParserCoordinator::with_fallback());

        // 3. Basic observability
        let observability = Arc::new(BasicObservability::default());

        Self {
            cache,
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
        _config_path: Option<&Path>,
    ) -> Result<(), StartupError> {
        self.observability.log_startup();

        info!("🎯 SystemCoordinator: инициализация System Layer...");

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
            repository.load_types(platform_raw_data)
                .map_err(StartupError::PlatformTypesError)?;

            let stats = repository.get_stats();
            info!("📊 Загружено {} типов из синтаксис-помощника", stats.total_types);
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

        info!("💾 SystemCoordinator: прогрев кеша...");
        self.cache.warm_cache()?;

        info!("✅ SystemCoordinator: система готова!");
        Ok(())
    }

    /// Загрузка базовых типов как fallback
    fn load_fallback_types(repository: &Arc<InMemoryTypeRepository>) -> Result<(), StartupError> {
        use bsl_shared::domain::types::{RawTypeData, RawDataSource};

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

        repository.load_types(basic_types)
            .map_err(StartupError::PlatformTypesError)?;

        info!("✅ Базовые типы загружены: 4 типа");
        Ok(())
    }

    /// Получить компоненты для создания TypeSystemService
    pub fn get_system_components(&self) -> (Arc<AnalysisCache>, Arc<ParserCoordinator>) {
        (self.cache.clone(), self.parser.clone())
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
