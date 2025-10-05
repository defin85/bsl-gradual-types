//! Markdown Renderer для документации и README

use anyhow::Result;
use bsl_shared::api::dtos::TypeDto;

use crate::TypeRenderer;

/// Markdown рендерер
pub struct MarkdownRenderer;

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRenderer for MarkdownRenderer {
    fn render_type_info(&self, type_dto: &TypeDto) -> Result<String> {
        let mut md = String::new();

        // Header
        md.push_str(&format!("# Тип: {}\n\n", type_dto.name));

        // Metadata
        md.push_str(&format!("**Категория:** {}\n\n", type_dto.category));
        md.push_str(&format!(
            "**Уверенность:** {} ({}%)\n\n",
            type_dto.certainty_text, type_dto.certainty
        ));
        md.push_str(&format!("**Источник:** {}\n\n", type_dto.source));

        // Facets
        if !type_dto.facets.is_empty() {
            md.push_str("**Фасеты:** ");
            md.push_str(&type_dto.facets.join(", "));
            md.push_str("\n\n");
        }

        // Description
        if !type_dto.description.is_empty() {
            md.push_str("## Описание\n\n");
            md.push_str(&type_dto.description);
            md.push_str("\n\n");
        }

        // Methods
        if !type_dto.methods.is_empty() {
            md.push_str(&format!(
                "## Методы ({})\n\n",
                type_dto.methods.len()
            ));
            for method in &type_dto.methods {
                md.push_str(&format!("- `{}`\n", method));
            }
            md.push_str("\n");
        }

        // Properties
        if !type_dto.properties.is_empty() {
            md.push_str(&format!(
                "## Свойства ({})\n\n",
                type_dto.properties.len()
            ));
            for prop in &type_dto.properties {
                md.push_str(&format!("- `{}`\n", prop));
            }
            md.push_str("\n");
        }

        // Enum values
        if let Some(ref values) = type_dto.enum_values {
            md.push_str(&format!(
                "## Значения перечисления ({})\n\n",
                values.len()
            ));
            for value in values {
                md.push_str(&format!("- `{}`\n", value));
            }
            md.push_str("\n");
        }

        Ok(md)
    }

    fn render_methods(&self, type_name: &str, methods: &[String]) -> Result<String> {
        let mut md = String::new();

        md.push_str(&format!("# Методы типа: {}\n\n", type_name));
        md.push_str(&format!("Всего методов: **{}**\n\n", methods.len()));

        for method in methods {
            md.push_str(&format!("- `{}`\n", method));
        }

        Ok(md)
    }

    fn render_properties(&self, type_name: &str, properties: &[String]) -> Result<String> {
        let mut md = String::new();

        md.push_str(&format!("# Свойства типа: {}\n\n", type_name));
        md.push_str(&format!("Всего свойств: **{}**\n\n", properties.len()));

        for prop in properties {
            md.push_str(&format!("- `{}`\n", prop));
        }

        Ok(md)
    }

    fn render_metrics(&self, total_types: usize, total_methods: usize) -> Result<String> {
        let mut md = String::new();

        md.push_str("# Метрики системы типов\n\n");
        md.push_str(&format!("- **Всего типов:** {}\n", total_types));
        md.push_str(&format!("- **Всего методов:** {}\n", total_methods));

        Ok(md)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_renderer_methods() {
        let renderer = MarkdownRenderer::new();
        let methods = vec!["Создать".to_string(), "Удалить".to_string()];

        let md = renderer.render_methods("Справочники", &methods).unwrap();

        assert!(md.contains("# Методы типа: Справочники"));
        assert!(md.contains("Всего методов: **2**"));
        assert!(md.contains("- `Создать`"));
        assert!(md.contains("- `Удалить`"));
    }
}
