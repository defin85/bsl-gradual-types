//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

use crate::application::TypeSystemService; // MOVED to Application Layer
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository, TypeResolver};

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

    // === APPLICATION LAYER ===
    type_service: Arc<TypeSystemService>,

    // === DOMAIN LAYER (для будущего расширения) ===
    #[allow(dead_code)]
    type_resolver: Arc<TypeResolver>,
    #[allow(dead_code)]
    repository: Arc<dyn TypeRepository>,
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

        // 5. Unified application service (moved to Application Layer)
        let type_service = Arc::new(TypeSystemService::new(
            type_resolver.clone(),
            cache.clone(),
            parser.clone(),
        ));

        Self {
            cache,
            parser,
            observability,
            type_service,
            type_resolver,
            repository,
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

        info!("🎭 SystemCoordinator: инициализация сервиса типов...");
        self.type_service.initialize()?;

        info!("✅ SystemCoordinator: система готова!");
        Ok(())
    }

    /// Получить сервис типов для веб-интерфейса
    pub fn get_type_service(&self) -> Arc<TypeSystemService> {
        self.type_service.clone()
    }

    /// Получить unified API для всех интерфейсов
    pub fn type_service(&self) -> Arc<TypeSystemService> {
        self.type_service.clone()
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
