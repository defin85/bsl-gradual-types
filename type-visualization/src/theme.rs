//! Темизация для визуализации типов
//!
//! Поддержка VS Code тем: light, dark, high-contrast
//! Использование CSS переменных для адаптивности

use crate::ThemeMode;

/// Тема визуализации
pub struct Theme {
    mode: ThemeMode,
}

impl Theme {
    /// Создать тему из режима
    pub fn from_mode(mode: &ThemeMode) -> Self {
        Self { mode: mode.clone() }
    }

    /// Получить CSS для темы
    pub fn get_css(&self) -> String {
        let base_styles = self.get_base_styles();
        let theme_vars = self.get_theme_variables();

        format!(
            r#"{}
{}"#,
            theme_vars, base_styles
        )
    }

    /// Получить CSS переменные для темы
    fn get_theme_variables(&self) -> &str {
        match self.mode {
            ThemeMode::Light => LIGHT_THEME_VARS,
            ThemeMode::Dark => DARK_THEME_VARS,
            ThemeMode::HighContrast => HIGH_CONTRAST_THEME_VARS,
            ThemeMode::Auto => {
                // Для VSCode используем CSS переменные редактора
                VSCODE_AUTO_THEME_VARS
            }
        }
    }

    /// Базовые стили (независимые от темы)
    fn get_base_styles(&self) -> &str {
        BASE_STYLES
    }

    /// Получить цвет для категории типа
    pub fn get_category_color(&self, category: &str) -> &str {
        match category {
            "Примитивный" | "Primitive" => "#4CAF50", // Зелёный
            "Платформа" | "Platform" => "#2196F3",    // Синий
            "Справочник" | "Reference" => "#FF9800",  // Оранжевый
            "Документ" | "Document" => "#9C27B0",     // Фиолетовый
            "Перечисление" | "Enum" => "#00BCD4",     // Циан
            _ => "#757575",                           // Серый по умолчанию
        }
    }
}

// ============================================================================
// Theme Variables
// ============================================================================

const VSCODE_AUTO_THEME_VARS: &str = r#"
:root {
    /* Автоопределение из VS Code */
    --bg-primary: var(--vscode-editor-background, #1e1e1e);
    --bg-secondary: var(--vscode-sideBar-background, #252526);
    --bg-hover: var(--vscode-list-hoverBackground, #2a2d2e);

    --text-primary: var(--vscode-editor-foreground, #d4d4d4);
    --text-secondary: var(--vscode-descriptionForeground, #cccccc);
    --text-muted: var(--vscode-disabledForeground, #656565);

    --border-color: var(--vscode-panel-border, #3c3c3c);
    --accent-color: var(--vscode-focusBorder, #007acc);

    --code-bg: var(--vscode-textCodeBlock-background, #0d0d0d);
    --code-text: var(--vscode-textPreformat-foreground, #d7ba7d);

    --success-color: #4ec9b0;
    --warning-color: #ce9178;
    --error-color: #f48771;
}
"#;

const LIGHT_THEME_VARS: &str = r#"
:root {
    --bg-primary: #ffffff;
    --bg-secondary: #f5f5f5;
    --bg-hover: #e0e0e0;

    --text-primary: #212121;
    --text-secondary: #424242;
    --text-muted: #757575;

    --border-color: #e0e0e0;
    --accent-color: #1976d2;

    --code-bg: #f5f5f5;
    --code-text: #c7254e;

    --success-color: #388e3c;
    --warning-color: #f57c00;
    --error-color: #d32f2f;
}
"#;

const DARK_THEME_VARS: &str = r#"
:root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #252526;
    --bg-hover: #2a2d2e;

    --text-primary: #d4d4d4;
    --text-secondary: #cccccc;
    --text-muted: #969696;

    --border-color: #3c3c3c;
    --accent-color: #007acc;

    --code-bg: #0d0d0d;
    --code-text: #d7ba7d;

    --success-color: #4ec9b0;
    --warning-color: #ce9178;
    --error-color: #f48771;
}
"#;

const HIGH_CONTRAST_THEME_VARS: &str = r#"
:root {
    --bg-primary: #000000;
    --bg-secondary: #0c0c0c;
    --bg-hover: #2b2b2b;

    --text-primary: #ffffff;
    --text-secondary: #ffffff;
    --text-muted: #c0c0c0;

    --border-color: #6fc3df;
    --accent-color: #0e639c;

    --code-bg: #000000;
    --code-text: #d4d4d4;

    --success-color: #7cb342;
    --warning-color: #ffa500;
    --error-color: #f44747;
}
"#;

// ============================================================================
// Base Styles
// ============================================================================

const BASE_STYLES: &str = r#"
* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: var(--text-primary);
    background-color: var(--bg-primary);
    padding: 20px;
}

h1 {
    font-size: 24px;
    font-weight: 600;
    margin-bottom: 20px;
    color: var(--text-primary);
}

/* Type Card */
.type-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 16px;
    transition: box-shadow 0.2s;
}

.type-card:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.type-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
}

.type-name {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
}

.type-category {
    padding: 4px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
    color: white;
}

.type-meta {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
    font-size: 13px;
}

.certainty {
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 500;
}

.certainty-high {
    background: var(--success-color);
    color: white;
}

.certainty-medium {
    background: var(--warning-color);
    color: white;
}

.certainty-low {
    background: var(--error-color);
    color: white;
}

.source {
    color: var(--text-secondary);
}

/* Facets */
.facets {
    margin-top: 8px;
    margin-bottom: 12px;
}

.facet {
    display: inline-block;
    padding: 2px 8px;
    margin-right: 6px;
    background: var(--bg-hover);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-secondary);
}

/* Methods & Properties */
.methods-section,
.properties-section {
    margin-top: 12px;
}

.methods-list,
.properties-list {
    list-style: none;
    padding-left: 0;
    margin-top: 8px;
}

.methods-list li,
.properties-list li {
    padding: 4px 0;
}

.methods-list li.more {
    color: var(--text-muted);
    font-style: italic;
}

code {
    background: var(--code-bg);
    color: var(--code-text);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: "Consolas", "Monaco", "Courier New", monospace;
    font-size: 13px;
}

/* Description */
.description {
    margin-top: 16px;
    padding: 12px;
    background: var(--bg-hover);
    border-left: 3px solid var(--accent-color);
    border-radius: 4px;
}

.description p {
    color: var(--text-secondary);
    line-height: 1.5;
}

/* Enum Values */
.enum-values {
    margin-top: 16px;
}

.enum-list {
    list-style: disc;
    padding-left: 20px;
    margin-top: 8px;
}

.enum-list li {
    padding: 2px 0;
}

/* Metrics */
.metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
    margin-top: 20px;
}

.metric-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 20px;
    text-align: center;
}

.metric-value {
    font-size: 36px;
    font-weight: 700;
    color: var(--accent-color);
    margin-bottom: 8px;
}

.metric-label {
    font-size: 14px;
    color: var(--text-secondary);
}

/* Containers */
.type-info-container,
.methods-container,
.properties-container,
.metrics-container {
    max-width: 900px;
    margin: 0 auto;
}
"#;
