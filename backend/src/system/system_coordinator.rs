//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture

use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::info;

use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use crate::application::type_system_service::TypeSystemService;

use super::basic_observability::BasicObservability;
use super::parser_coordinator::ParserCoordinator;
use super::simple_cache::AnalysisCache;

/// Упрощенный системный координатор
///
/// Заменяет CentralTypeSystem, координирует 6-8 компонентов вместо 25-30
pub struct SystemCoordinator {
    // === SYSTEM COMPONENTS ===
    cache: Arc<AnalysisCache>,
    parser: Arc<ParserCoordinator>,
    observability: Arc<BasicObservability>,

    // === APPLICATION LAYER (removed to break circular dependency) ===
    // type_service will be created externally and injected when needed

    // === DOMAIN LAYER (для будущего расширения) ===
    #[allow(dead_code)]
    type_resolver: Arc<TypeResolver>,
    #[allow(dead_code)]
    repository: Arc<dyn TypeRepository>,

    // === CACHED APPLICATION SERVICE ===
    type_service_cache: Mutex<Option<Arc<TypeSystemService>>>,
}

impl SystemCoordinator {
    /// Создать новый системный координатор
    pub fn new() -> Self {
        // 1. Simple caching
        let cache = Arc::new(AnalysisCache::new(1000)); // Simple LRU

        // 2. Simple parsing
        let parser = Arc::new(ParserCoordinator::with_fallback());

        // 3. Basic observability
        let observability = Arc::new(BasicObservability::default());

        // 4. Domain layer (unchanged)
        let repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
        let type_resolver = Arc::new(TypeResolver::new(repository.clone()));

        // 5. Application service creation moved to external coordinator

        Self {
            cache,
            parser,
            observability,
            type_resolver,
            repository,
            type_service_cache: Mutex::new(None),
        }
    }

    /// Инициализация системы
    pub async fn start(&self) -> Result<(), StartupError> {
        self.observability.log_startup();

        // Простая инициализация без сложных состояний
        info!("🎯 SystemCoordinator: загрузка данных типов...");
        self.load_platform_types().await?;

        info!("💾 SystemCoordinator: прогрев кеша...");
        self.cache.warm_cache()?;

        info!("✅ SystemCoordinator: система готова!");
        Ok(())
    }

    /// Получить компоненты для создания TypeSystemService
    pub fn get_components(&self) -> (Arc<TypeResolver>, Arc<AnalysisCache>, Arc<ParserCoordinator>) {
        (self.type_resolver.clone(), self.cache.clone(), self.parser.clone())
    }

    /// Создать TypeSystemService (singleton)
    pub fn type_service(&self) -> Arc<TypeSystemService> {
        let mut cache = self.type_service_cache.lock().unwrap();
        if let Some(service) = cache.as_ref() {
            return service.clone();
        }

        let service = Arc::new(TypeSystemService::new(
            self.type_resolver.clone(),
            self.cache.clone(),
            self.parser.clone(),
        ));

        *cache = Some(service.clone());
        service
    }

    /// Health check
    pub fn health_status(&self) -> crate::system::basic_observability::HealthStatus {
        self.observability.health_check()
    }

    // === PRIVATE METHODS ===

    async fn load_platform_types(&self) -> Result<()> {
        // Упрощенная загрузка без сложных координаторов
        self.parser.load_platform_types(&self.repository).await
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
