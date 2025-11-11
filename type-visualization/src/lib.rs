//! BSL Type Visualization Library - Milestone 2.5
//!
//! Единый компонент для визуализации типов BSL:
//! - Trait TypeRenderer для разных форматов (HTML, JSON, Markdown)
//! - Переиспользуемый код для Web Frontend и VSCode Extension
//! - Унифицированные стили и темизация
//!
//! Цель: DRY - один компонент для всех UI

pub mod html_renderer;
pub mod json_renderer;
pub mod markdown_renderer;
pub mod styles;
pub mod theme;

use anyhow::Result;
use bsl_shared::api::dtos::TypeDto;

/// Trait для рендеринга типов в разные форматы
pub trait TypeRenderer {
    /// Рендерить информацию о типе
    fn render_type_info(&self, type_dto: &TypeDto) -> Result<String>;

    /// Рендерить список методов
    fn render_methods(&self, type_name: &str, methods: &[String]) -> Result<String>;

    /// Рендерить список свойств
    fn render_properties(&self, type_name: &str, properties: &[String]) -> Result<String>;

    /// Рендерить метрики
    fn render_metrics(&self, total_types: usize, total_methods: usize) -> Result<String>;
}

/// Опции темизации
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// Светлая тема
    Light,
    /// Тёмная тема
    Dark,
    /// Высококонтрастная тема
    HighContrast,
    /// Автоопределение (по умолчанию VS Code)
    #[default]
    Auto,
}

/// Настройки рендеринга
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Режим темы
    pub theme: ThemeMode,
    /// Включить подсветку синтаксиса
    pub syntax_highlight: bool,
    /// Включить навигацию (ссылки)
    pub enable_links: bool,
    /// Компактный режим (минимум отступов)
    pub compact: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Auto,
            syntax_highlight: true,
            enable_links: true,
            compact: false,
        }
    }
}

// Re-exports
pub use html_renderer::HtmlRenderer;
pub use json_renderer::JsonRenderer;
pub use markdown_renderer::MarkdownRenderer;
pub use styles::Styles;
pub use theme::Theme;
