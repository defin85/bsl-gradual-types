//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture
//! Координирует только System Layer компоненты

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::path::Path;
use tracing::info;

use bsl_shared::engine::AnalysisEngine;
use crate::application::type_system_service::TypeSystemService;

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
        config_path: Option<&Path>,
    ) -> Result<(), StartupError> {
        self.observability.log_startup();

        info!("🎯 SystemCoordinator: инициализация System Layer...");

        // Создаем AnalysisEngine который управляет Domain Layer
        let analysis_engine = AnalysisEngine::new_with_init(syntax_helper_path, config_path)
            .await
            .map_err(|e| StartupError::PlatformTypesError(e))?;

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
