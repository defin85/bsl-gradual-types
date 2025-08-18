//! Ядро системы документации BSL

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::core::types::{TypeResolution, FacetKind};
use super::platform::PlatformDocumentationProvider;
use super::configuration::ConfigurationDocumentationProvider;
use super::search::{DocumentationSearchEngine, AdvancedSearchQuery, SearchResults};
use super::render::RenderEngine;

pub mod hierarchy;
pub mod providers;
pub mod cache;
pub mod statistics;

pub use hierarchy::*;
pub use providers::*;
pub use cache::*;
pub use statistics::*;

/// Центральная система документации BSL
/// 
/// Координирует работу всех провайдеров и предоставляет единый API
/// для доступа к документации типов
pub struct BslDocumentationSystem {
    /// Провайдер платформенных типов
    platform_provider: Arc<PlatformDocumentationProvider>,
    
    /// Провайдер конфигурационных типов
    configuration_provider: Arc<ConfigurationDocumentationProvider>,
    
    /// Система поиска и индексации
    search_engine: Arc<DocumentationSearchEngine>,
    
    /// Кеш для производительности
    cache_manager: Arc<DocumentationCache>,
    
    /// Система рендеринга
    render_engine: Arc<RenderEngine>,
    
    /// Собранная иерархия типов
    hierarchy_cache: Arc<RwLock<Option<TypeHierarchy>>>,
    
    /// Статус инициализации
    initialization_status: Arc<RwLock<InitializationStatus>>,
}

/// Статус инициализации системы
#[derive(Debug, Clone, Serialize)]
pub struct InitializationStatus {
    /// Инициализируется ли система
    pub is_initializing: bool,
    
    /// Процент завершения (0-100)
    pub progress_percent: u8,
    
    /// Текущая операция
    pub current_operation: String,
    
    /// Детали прогресса
    pub details: InitializationDetails,
    
    /// Ошибки инициализации
    pub errors: Vec<String>,
}

/// Детали прогресса инициализации
#[derive(Debug, Clone, Serialize)]
pub struct InitializationDetails {
    /// Платформенные типы
    pub platform_types: ProviderStatus,
    
    /// Конфигурационные типы  
    pub configuration_types: ProviderStatus,
    
    /// Индексы поиска
    pub search_indexes: ProviderStatus,
    
    /// Кеш
    pub cache: ProviderStatus,
}

/// Статус провайдера
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    /// Статус загрузки
    pub status: LoadingStatus,
    
    /// Загружено элементов
    pub loaded_items: usize,
    
    /// Всего элементов
    pub total_items: usize,
    
    /// Время загрузки (мс)
    pub loading_time_ms: u64,
    
    /// Сообщения об ошибках
    pub error_messages: Vec<String>,
}

/// Статус загрузки
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum LoadingStatus {
    /// Не начата
    NotStarted,
    
    /// Загружается
    Loading,
    
    /// Завершена успешно
    Completed,
    
    /// Завершена с ошибками
    CompletedWithErrors,
    
    /// Критическая ошибка
    Failed,
}

impl BslDocumentationSystem {
    /// Создать новую систему документации
    pub fn new() -> Self {
        Self {
            platform_provider: Arc::new(PlatformDocumentationProvider::new()),
            configuration_provider: Arc::new(ConfigurationDocumentationProvider::new()),
            search_engine: Arc::new(DocumentationSearchEngine::new()),
            cache_manager: Arc::new(DocumentationCache::new()),
            render_engine: Arc::new(RenderEngine::new()),
            hierarchy_cache: Arc::new(RwLock::new(None)),
            initialization_status: Arc::new(RwLock::new(InitializationStatus::default())),
        }
    }
    
    /// Инициализировать систему асинхронно
    pub async fn initialize(&self, config: DocumentationConfig) -> Result<()> {
        // Устанавливаем статус инициализации
        {
            let mut status = self.initialization_status.write().await;
            status.is_initializing = true;
            status.current_operation = "Инициализация системы документации".to_string();
            status.progress_percent = 0;
        }
        
        // Инициализируем платформенные типы
        let platform_provider_config = providers::ProviderConfig {
            data_source: config.platform_config.syntax_helper_path.clone(),
            ..Default::default()
        };
        self.platform_provider.initialize(&platform_provider_config).await?;
        
        // Инициализируем конфигурационные типы
        if let Some(config_path) = &config.configuration_path {
            let config_provider_config = providers::ProviderConfig {
                data_source: config_path.clone(),
                ..Default::default()
            };
            self.configuration_provider.initialize(&config_provider_config).await?;
        }
        
        // Строим поисковые индексы
        self.build_search_indexes().await?;
        
        // Собираем полную иерархию
        self.build_type_hierarchy().await?;
        
        // Завершаем инициализацию
        {
            let mut status = self.initialization_status.write().await;
            status.is_initializing = false;
            status.progress_percent = 100;
            status.current_operation = "Система документации готова".to_string();
        }
        
        Ok(())
    }
    
    /// Получить статус инициализации
    pub async fn get_initialization_status(&self) -> InitializationStatus {
        self.initialization_status.read().await.clone()
    }
    
    /// Получить полную иерархию типов
    pub async fn get_type_hierarchy(&self) -> Result<TypeHierarchy> {
        let hierarchy = self.hierarchy_cache.read().await;
        
        match hierarchy.as_ref() {
            Some(h) => Ok(h.clone()),
            None => {
                // Если кеш пуст, строим заново
                drop(hierarchy);
                self.build_type_hierarchy().await?;
                let hierarchy = self.hierarchy_cache.read().await;
                Ok(hierarchy.as_ref().unwrap().clone())
            }
        }
    }
    
    /// Поиск в документации
    pub async fn search(&self, query: AdvancedSearchQuery) -> Result<SearchResults> {
        self.search_engine.search(query).await
    }
    
    /// Получить детали типа
    pub async fn get_type_details(&self, type_id: &str) -> Result<Option<TypeDocumentationFull>> {
        // Сначала проверяем кеш
        if let Some(cached) = self.cache_manager.get_type_details(type_id).await {
            return Ok(Some(cached));
        }
        
        // Ищем в провайдерах
        if let Some(details) = self.platform_provider.get_type_details(type_id).await? {
            self.cache_manager.store_type_details(type_id, &details).await;
            return Ok(Some(details));
        }
        
        if let Some(details) = self.configuration_provider.get_type_details(type_id).await? {
            self.cache_manager.store_type_details(type_id, &details).await;
            return Ok(Some(details));
        }
        
        Ok(None)
    }
    
    /// Получить статистику системы
    pub async fn get_statistics(&self) -> Result<DocumentationStatistics> {
        let platform_stats = self.platform_provider.get_statistics().await?;
        let config_stats = self.configuration_provider.get_statistics().await?;
        let search_stats = self.search_engine.get_statistics().await?;
        
        Ok(DocumentationStatistics {
            platform: platform_stats,
            configuration: config_stats,
            search: search_stats,
            cache_hits: self.cache_manager.get_hit_rate().await,
            total_memory_mb: self.estimate_memory_usage().await,
        })
    }
    
    // Приватные методы инициализации
    
    async fn build_search_indexes(&self) -> Result<()> {
        {
            let mut status = self.initialization_status.write().await;
            status.current_operation = "Построение поисковых индексов".to_string();
            status.progress_percent = 70;
        }
        
        self.search_engine.build_indexes(
            &self.platform_provider,
            &self.configuration_provider
        ).await
    }
    
    async fn build_type_hierarchy(&self) -> Result<()> {
        {
            let mut status = self.initialization_status.write().await;
            status.current_operation = "Сборка иерархии типов".to_string();
            status.progress_percent = 90;
        }
        
        let mut root_categories = Vec::new();
        
        // Добавляем платформенные типы
        if let Ok(platform_category) = self.platform_provider.get_root_category().await {
            root_categories.push(platform_category);
        }
        
        // Добавляем конфигурационные типы
        if let Ok(config_category) = self.configuration_provider.get_root_category().await {
            root_categories.push(config_category);
        }
        
        // Создаем иерархию
        let hierarchy = hierarchy::TypeHierarchy {
            root_categories,
            statistics: hierarchy::HierarchyStatistics {
                total_nodes: 0, // TODO: подсчитать
                node_counts: std::collections::HashMap::new(),
                max_depth: 0,
                build_time_ms: 0,
            },
            navigation_index: hierarchy::NavigationIndex {
                by_id: std::collections::HashMap::new(),
                by_russian_name: std::collections::HashMap::new(),
                by_english_name: std::collections::HashMap::new(),
                by_facet: std::collections::HashMap::new(),
                reverse_relations: std::collections::HashMap::new(),
            },
            metadata: hierarchy::HierarchyMetadata {
                schema_version: "1.0.0".to_string(),
                created_at: chrono::Utc::now(),
                data_sources: Vec::new(),
                build_config: hierarchy::BuildConfig::default(),
            },
        };
        
        *self.hierarchy_cache.write().await = Some(hierarchy);
        Ok(())
    }
    
    async fn estimate_memory_usage(&self) -> f64 {
        // TODO: Реализовать подсчет использования памяти
        0.0
    }
}

/// Конфигурация системы документации
#[derive(Debug, Clone)]
pub struct DocumentationConfig {
    /// Конфигурация платформенных типов
    pub platform_config: PlatformConfig,
    
    /// Путь к конфигурации (опционально)
    pub configuration_path: Option<String>,
    
    /// Настройки кеширования
    pub cache_config: CacheConfig,
    
    /// Настройки поиска
    pub search_config: SearchConfig,
}

/// Конфигурация платформенных типов
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// Путь к справке синтакс-помощника
    pub syntax_helper_path: String,
    
    /// Версия платформы
    pub platform_version: String,
    
    /// Показывать прогресс парсинга
    pub show_progress: bool,
    
    /// Количество потоков для парсинга
    pub worker_threads: usize,
}

/// Статистика системы документации
#[derive(Debug, Clone, Serialize)]
pub struct DocumentationStatistics {
    /// Статистика платформенных типов
    pub platform: ProviderStatistics,
    
    /// Статистика конфигурационных типов
    pub configuration: ProviderStatistics,
    
    /// Статистика поиска
    pub search: crate::documentation::search::SearchStatistics,
    
    /// Процент попаданий в кеш
    pub cache_hits: f64,
    
    /// Общее использование памяти (MB)
    pub total_memory_mb: f64,
}

impl Default for InitializationStatus {
    fn default() -> Self {
        Self {
            is_initializing: false,
            progress_percent: 0,
            current_operation: "Не инициализирована".to_string(),
            details: InitializationDetails::default(),
            errors: Vec::new(),
        }
    }
}

impl Default for InitializationDetails {
    fn default() -> Self {
        Self {
            platform_types: ProviderStatus::default(),
            configuration_types: ProviderStatus::default(),
            search_indexes: ProviderStatus::default(),
            cache: ProviderStatus::default(),
        }
    }
}

impl Default for ProviderStatus {
    fn default() -> Self {
        Self {
            status: LoadingStatus::NotStarted,
            loaded_items: 0,
            total_items: 0,
            loading_time_ms: 0,
            error_messages: Vec::new(),
        }
    }
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        Self {
            platform_config: PlatformConfig {
                syntax_helper_path: "examples/syntax_helper/rebuilt.shcntx_ru".to_string(),
                platform_version: "8.3.23".to_string(),
                show_progress: true,
                worker_threads: 4,
            },
            configuration_path: None,
            cache_config: CacheConfig::default(),
            search_config: SearchConfig::default(),
        }
    }
}

/// Конфигурация кеширования
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Максимальный размер кеша (элементов)
    pub max_cache_size: usize,
    
    /// TTL для кеша (секунды)
    pub cache_ttl_seconds: u64,
    
    /// Включить персистентный кеш
    pub persistent_cache: bool,
    
    /// Путь для персистентного кеша
    pub cache_directory: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 10000,
            cache_ttl_seconds: 3600, // 1 час
            persistent_cache: true,
            cache_directory: ".cache/bsl-docs".to_string(),
        }
    }
}

/// Конфигурация поиска
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Максимум результатов на страницу
    pub max_results_per_page: usize,
    
    /// Включить семантический поиск
    pub enable_semantic_search: bool,
    
    /// Включить нечеткий поиск
    pub enable_fuzzy_search: bool,
    
    /// Минимальный score для результатов
    pub min_score_threshold: f64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results_per_page: 50,
            enable_semantic_search: false, // Опционально
            enable_fuzzy_search: true,
            min_score_threshold: 0.1,
        }
    }
}