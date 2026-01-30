//! Конфигурация и типы для hover formatter
//!
//! Содержит типы для настройки форматирования hover responses.

use bsl_shared::formatting::DetailLevel;

/// Формат вывода hover информации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverOutputFormat {
    /// Markdown для LSP hover
    Markdown,
    /// Plain text для CLI
    PlainText,
}

/// Backward compatible re-export of Theme
///
/// Используйте `bsl_shared::formatting::Theme` напрямую для нового кода.
#[deprecated(since = "0.5.0", note = "Use bsl_shared::formatting::Theme instead")]
pub use bsl_shared::formatting::Theme;

/// Локаль для текстовых сообщений
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    Ru,
}

/// Конфигурация форматирования hover
#[derive(Debug, Clone)]
pub struct HoverFormatConfig {
    /// Максимальное количество методов для отображения
    pub max_methods: usize,
    /// Максимальное количество свойств для отображения
    pub max_properties: usize,
    /// MILESTONE 3.6 Phase 1: Уровень детализации (compact/full/detailed)
    pub detail_level: DetailLevel,
    /// MILESTONE 3.6 Phase 1: Показывать ли уверенность в типе
    pub show_certainty: bool,
    /// MILESTONE 3.6 Phase 2 - Task 2.3: Путь к Syntax Helper для документации
    pub syntax_helper_path: Option<std::path::PathBuf>,
    /// Формат вывода
    pub output_format: HoverOutputFormat,
    /// Тема оформления
    pub theme: Theme,
    /// Локаль
    pub locale: Locale,
}

impl Default for HoverFormatConfig {
    fn default() -> Self {
        Self {
            max_methods: 10,
            max_properties: 5,
            detail_level: DetailLevel::Detailed,
            show_certainty: true,
            syntax_helper_path: None,
            output_format: HoverOutputFormat::Markdown,
            theme: Theme::Dark,
            locale: Locale::Ru,
        }
    }
}
