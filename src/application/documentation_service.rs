//! Сервис документации типов (плоская архитектура)
//!
//! Оборачивает провайдеры документации платформы и конфигурации,
//! а также движок поиска документации для удобного использования
//! из Web/CLI/LSP сервисов.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::documentation::{
    AdvancedSearchQuery, ConfigurationDocumentationProvider, DocumentationSearchEngine,
    PlatformDocumentationProvider, SearchResults,
};

/// Высокоуровневый сервис документации
pub struct DocumentationService {
    /// Провайдер документации платформенных типов
    platform: Arc<RwLock<Option<PlatformDocumentationProvider>>>,
    /// Провайдер документации конфигурационных типов
    configuration: Arc<RwLock<Option<ConfigurationDocumentationProvider>>>,
    /// Поисковый движок по документации
    search: Arc<RwLock<Option<DocumentationSearchEngine>>>,
}

impl Default for DocumentationService {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentationService {
    /// Создать пустой сервис документации
    pub fn new() -> Self {
        Self {
            platform: Arc::new(RwLock::new(None)),
            configuration: Arc::new(RwLock::new(None)),
            search: Arc::new(RwLock::new(None)),
        }
    }

    /// Инициализировать провайдер платформенной документации
    pub async fn init_platform_provider(&self) -> Result<()> {
        let provider = PlatformDocumentationProvider::new();
        let mut guard = self.platform.write().await;
        *guard = Some(provider);
        Ok(())
    }

    /// Инициализировать провайдер документации конфигурации
    pub async fn init_configuration_provider(&self) -> Result<()> {
        let provider = ConfigurationDocumentationProvider::new();
        let mut guard = self.configuration.write().await;
        *guard = Some(provider);
        Ok(())
    }

    /// Инициализировать поисковый движок документации
    pub async fn init_search(&self) -> Result<()> {
        // Простейшая инициализация; детали наполнения индексами будут добавлены позже
        let engine = DocumentationSearchEngine::new();
        let mut guard = self.search.write().await;
        *guard = Some(engine);
        Ok(())
    }

    /// Выполнить расширенный поиск по документации
    pub async fn search(&self, query: AdvancedSearchQuery) -> Result<SearchResults> {
        let guard = self.search.read().await;
        let engine = guard
            .as_ref()
            .expect("DocumentationSearchEngine не инициализирован");
        // Для дальнейшего развития: подтягивать данные из провайдеров в индексы
        let results = engine.search(query).await?;
        Ok(results)
    }
}

