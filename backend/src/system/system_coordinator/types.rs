//! Типы данных для SystemCoordinator

/// Результат загрузки метаданных конфигурации
#[derive(Debug, Clone)]
pub struct LoadMetadataResult {
    /// Количество загруженных базовых конфигураций
    pub base_config_count: usize,
    /// Количество загруженных расширений
    pub extensions_count: usize,
    /// Общее количество типов
    pub total_types: usize,
}

/// Ошибки инициализации системы
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
