//! Унифицированные стили для визуализации
//!
//! Общие CSS стили переиспользуемые в Web и VSCode

/// Класс для работы со стилями
pub struct Styles;

impl Styles {
    /// Получить все стили как единую строку
    pub fn get_all() -> &'static str {
        UNIFIED_STYLES
    }

    /// Получить только базовые стили (без темы)
    pub fn get_base() -> &'static str {
        BASE_COMPONENT_STYLES
    }

    /// Получить стили для VS Code integration
    pub fn get_vscode() -> &'static str {
        VSCODE_SPECIFIC_STYLES
    }
}

const UNIFIED_STYLES: &str = r#"
/* Base component styles */
.type-visualization {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 14px;
    line-height: 1.6;
}

.type-card {
    background: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 16px;
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
}

.certainty-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
}
"#;

const BASE_COMPONENT_STYLES: &str = r#"
.type-info-card {
    padding: 16px;
    border-radius: 8px;
}

.methods-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
}

.properties-list {
    list-style: none;
    padding: 0;
}
"#;

const VSCODE_SPECIFIC_STYLES: &str = r#"
/* VS Code specific overrides */
body.vscode-light {
    --bg-primary: #ffffff;
    --text-primary: #212121;
}

body.vscode-dark {
    --bg-primary: #1e1e1e;
    --text-primary: #d4d4d4;
}

body.vscode-high-contrast {
    --bg-primary: #000000;
    --text-primary: #ffffff;
}
"#;
