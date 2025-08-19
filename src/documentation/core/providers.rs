//! Трейты и базовые реализации провайдеров документации

use async_trait::async_trait;
use anyhow::Result;

use super::hierarchy::{
    DocumentationNode, TypeDocumentationFull, RootCategoryNode
};
use crate::documentation::search::AdvancedSearchQuery;

/// Базовый трейт для всех провайдеров документации
#[async_trait]
pub trait DocumentationProvider: Send + Sync {
    /// Уникальный идентификатор провайдера
    fn provider_id(&self) -> &str;
    
    /// Название провайдера для отображения
    fn display_name(&self) -> &str;
    
    /// Инициализация провайдера
    async fn initialize(&self, config: &ProviderConfig) -> Result<()>;
    
    /// Получить корневую категорию для иерархии
    async fn get_root_category(&self) -> Result<RootCategoryNode>;
    
    /// Получить детали типа по ID
    async fn get_type_details(&self, type_id: &str) -> Result<Option<TypeDocumentationFull>>;
    
    /// Поиск типов в провайдере
    async fn search_types(&self, query: &AdvancedSearchQuery) -> Result<Vec<DocumentationNode>>;
    
    /// Получить все доступные типы (для индексации)
    async fn get_all_types(&self) -> Result<Vec<TypeDocumentationFull>>;
    
    /// Получить статистику провайдера
    async fn get_statistics(&self) -> Result<super::statistics::ProviderStatistics>;
    
    /// Получить статус инициализации
    async fn get_initialization_status(&self) -> Result<super::statistics::InitializationStatus>;
    
    /// Проверить доступность источника данных
    async fn check_availability(&self) -> Result<bool>;
    
    /// Обновить данные (при изменении источника)
    async fn refresh(&self) -> Result<()>;
}

/// Базовая конфигурация провайдера
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Источник данных
    pub data_source: String,
    
    /// Настройки производительности
    pub performance_settings: PerformanceSettings,
    
    /// Настройки кеширования
    pub cache_settings: CacheSettings,
    
    /// Дополнительные параметры
    pub additional_params: std::collections::HashMap<String, String>,
}

/// Настройки производительности
#[derive(Debug, Clone)]
pub struct PerformanceSettings {
    /// Количество рабочих потоков
    pub worker_threads: usize,
    
    /// Размер батча для обработки
    pub batch_size: usize,
    
    /// Таймаут операций (мс)
    pub operation_timeout_ms: u64,
    
    /// Показывать прогресс
    pub show_progress: bool,
}

/// Настройки кеширования провайдера
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Включить кеширование
    pub enabled: bool,
    
    /// Максимальный размер кеша
    pub max_cache_size: usize,
    
    /// TTL кеша (секунды)
    pub cache_ttl_seconds: u64,
    
    /// Стратегия вытеснения
    pub eviction_strategy: EvictionStrategy,
}

/// Стратегия вытеснения из кеша
#[derive(Debug, Clone)]
pub enum EvictionStrategy {
    /// Least Recently Used
    LRU,
    
    /// First In First Out
    FIFO,
    
    /// Least Frequently Used
    LFU,
    
    /// Time To Live
    TTL,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            batch_size: 100,
            operation_timeout_ms: 30000, // 30 секунд
            show_progress: true,
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size: 10000,
            cache_ttl_seconds: 3600, // 1 час
            eviction_strategy: EvictionStrategy::LRU,
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            data_source: String::new(),
            performance_settings: PerformanceSettings::default(),
            cache_settings: CacheSettings::default(),
            additional_params: std::collections::HashMap::new(),
        }
    }
}

/// Результат валидации провайдера
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Валидный ли провайдер
    pub is_valid: bool,
    
    /// Сообщения об ошибках
    pub error_messages: Vec<String>,
    
    /// Предупреждения
    pub warnings: Vec<String>,
    
    /// Рекомендации по улучшению
    pub recommendations: Vec<String>,
}

/// Расширенный трейт для провайдеров с валидацией
#[async_trait]
pub trait ValidatableProvider: DocumentationProvider {
    /// Валидация корректности данных провайдера
    async fn validate(&self) -> Result<ValidationResult>;
    
    /// Самодиагностика провайдера
    async fn self_diagnostics(&self) -> Result<DiagnosticsReport>;
}

/// Отчет диагностики
#[derive(Debug, Clone)]
pub struct DiagnosticsReport {
    /// Статус здоровья провайдера
    pub health_status: HealthStatus,
    
    /// Подробная информация
    pub details: Vec<DiagnosticItem>,
    
    /// Рекомендации по исправлению
    pub action_items: Vec<String>,
}

/// Статус здоровья
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Элемент диагностики
#[derive(Debug, Clone)]
pub struct DiagnosticItem {
    /// Категория проблемы
    pub category: String,
    
    /// Сообщение
    pub message: String,
    
    /// Уровень важности
    pub severity: DiagnosticSeverity,
    
    /// Детали для отладки
    pub debug_info: Option<String>,
}

/// Уровень важности диагностики
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
    Critical,
}