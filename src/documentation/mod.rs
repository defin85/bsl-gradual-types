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

pub mod core;
pub mod platform;
pub mod configuration;
pub mod search;
pub mod render;

// Re-exports для удобства
pub use core::{BslDocumentationSystem, DocumentationNode, TypeDocumentationFull};
pub use platform::PlatformDocumentationProvider;
pub use configuration::ConfigurationDocumentationProvider;
pub use search::{DocumentationSearchEngine, AdvancedSearchQuery, SearchResults};
pub use render::{RenderEngine, HtmlDocumentationRenderer};