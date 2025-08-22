//! BSL Documentation & Visualization System - Enterprise Edition
//!
//! Полноценная система документации типов BSL, превосходящая стандартную справку 1С
//!
//! # Архитектура
//!
//! - `core` - центральная система и координация
//! - `platform` - документация платформенных типов
//! - `configuration` - документация конфигурационных типов  
//! - `search` - система поиска и индексации
//! - `render` - рендеринг в разные форматы

pub mod configuration;
pub mod core;
pub mod platform;
pub mod render;
pub mod search;

// Re-exports для удобства
pub use configuration::ConfigurationDocumentationProvider;
pub use core::{BslDocumentationSystem, DocumentationNode, TypeDocumentationFull};
pub use platform::PlatformDocumentationProvider;
pub use render::{HtmlDocumentationRenderer, RenderEngine};
pub use search::{
    AdvancedSearchQuery, DocumentationSearchEngine, SearchFilters, SearchOptions, SearchPagination,
    SearchResults, SearchSort, SortDirection, SortField,
};
