//! Конфигурация и типы для hover formatter
//!
//! Содержит типы для настройки форматирования hover responses.

use bsl_shared::formatting::DetailLevel;

/// Порог уверенности, ниже которого тип считается "низкой уверенности"
/// и проверяется существование объекта метаданных.
/// Используется в check_unknown_metadata_object() для определения,
/// нужно ли показывать hover с ошибкой вместо стандартного hover.
pub const LOW_CONFIDENCE_THRESHOLD: f32 = 0.6;

/// Формат вывода hover информации
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Markdown для LSP hover
    Markdown,
    /// Plain text для CLI
    PlainText,
}

/// Тема оформления (для будущего использования с темами VSCode)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

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
    pub output_format: OutputFormat,
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
            output_format: OutputFormat::Markdown,
            theme: Theme::Dark,
            locale: Locale::Ru,
        }
    }
}
