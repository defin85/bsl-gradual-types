//! Унифицированное форматирование LSP hover responses
//!
//! Этот модуль предоставляет чистый API для форматирования информации о типах
//! в различных форматах (Markdown, PlainText) с конфигурируемыми лимитами.
//!
//! # Примеры
//!
//! ```rust,no_run
//! use bsl_backend::helpers::hover_formatter::{HoverFormatter, HoverFormatConfig};
//! use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
//! use bsl_shared::domain::repository::InMemoryTypeRepository;
//! use bsl_shared::domain::types::TypeResolution;
//! use std::sync::Arc;
//!
//! // Создание TypeMetadataLookup
//! let repo = Arc::new(InMemoryTypeRepository::new());
//! let metadata_lookup = TypeMetadataLookup::new(repo);
//!
//! // Создание HoverFormatter с конфигурацией
//! let config = HoverFormatConfig {
//!     max_methods: 10,
//!     max_properties: 5,
//!     ..Default::default()
//! };
//! let formatter = HoverFormatter::new(config, metadata_lookup);
//!
//! // Форматирование hover для переменной
//! // let hover = formatter.format_variable("МассивДанных", &resolution);
//! // println!("{}", hover);
//! ```
//!
//! # Архитектурные преимущества
//!
//! - Устранение ~150 строк дублированного кода
//! - Конфигурируемые лимиты для компактных tooltips
//! - Separation of Concerns - TypeSystemService делегирует форматирование
//! - Переиспользование в LSP/Web/CLI
//!
//! # Структура модуля
//!
//! - `config` - типы конфигурации (OutputFormat, Theme, Locale, HoverFormatConfig)
//! - `formatter` - основной HoverFormatter
//! - `builder` - HoverBuilder для построения hover content
//! - `sections` - форматирование секций (методы, свойства, табличные части)
//! - `type_display` - отображение типов

mod builder;
mod config;
mod formatter;
mod sections;
#[cfg(test)]
mod tests;
mod type_display;

// Публичный API модуля
pub use config::{HoverFormatConfig, Locale, OutputFormat, Theme};
pub use formatter::HoverFormatter;

// Для внутреннего использования в тестах
#[doc(hidden)]
pub use builder::HoverBuilder;
