//! Статистика и метрики системы документации

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Статистика провайдера
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatistics {
    /// Всего типов
    pub total_types: usize,
    
    /// Всего методов
    pub total_methods: usize,
    
    /// Всего свойств
    pub total_properties: usize,
    
    /// Время последней загрузки (мс)
    pub last_load_time_ms: u64,
    
    /// Использование памяти (MB)
    pub memory_usage_mb: f64,
}

/// Статус инициализации
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