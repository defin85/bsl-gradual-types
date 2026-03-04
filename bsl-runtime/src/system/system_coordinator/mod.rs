//! System Coordinator - упрощенная замена CentralTypeSystem
//!
//! Единая точка координации всех компонентов системы типов согласно Simple Architecture.
//! Координирует только System Layer компоненты.
//!
//! # Модули
//!
//! - `coordinator` - основная структура SystemCoordinator и core методы
//! - `lifecycle` - инициализация системы и загрузка типов платформы
//! - `config_loader` - загрузка метаданных конфигураций
//! - `types` - вспомогательные типы (ошибки, результаты)

mod config_loader;
mod coordinator;
mod lifecycle;
mod types;

// Реэкспорты публичного API
pub use coordinator::SystemCoordinator;
pub use types::{
    CacheClearReport, CacheScope, CacheStatsReport, CacheToggleResult, ConfigIndexCache,
    DiskCacheStatsReport, DomainBundle, LoadMetadataResult, ObjectKey, StartupError, SymbolInfo,
};

#[cfg(test)]
mod tests;
