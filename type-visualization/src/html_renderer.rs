//! HTML Renderer для визуализации типов
//!
//! Заменяет 7 разных HTML-генераторов в VSCode Extension
//! Переиспользует стили из Web Frontend

use anyhow::Result;
use bsl_shared::api::dtos::TypeDto;

use crate::{RenderOptions, Theme, TypeRenderer};

/// HTML рендерер для визуализации типов
pub struct HtmlRenderer {
    #[allow(dead_code)]
    options: RenderOptions,
    theme: Theme,
}

impl HtmlRenderer {
    /// Создать новый HTML рендерер
    pub fn new(options: RenderOptions) -> Self {
        let theme = Theme::from_mode(&options.theme);
        Self { options, theme }
    }

    /// Создать рендерер с темой по умолчанию
    pub fn with_default_theme() -> Self {
        Self::new(RenderOptions::default())
    }

    /// Сгенерировать полный HTML документ с header и стилями
    pub fn render_document(&self, title: &str, body: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
{}
    </style>
</head>
<body>
{}
</body>
</html>"#,
            title,
            self.theme.get_css(),
            body
        )
    }

    /// Рендерить карточку типа (компактный вид)
    fn render_type_card(&self, type_dto: &TypeDto) -> String {
        let certainty_class = self.get_certainty_class(type_dto.certainty);
        let category_color = self.theme.get_category_color(&type_dto.category);

        format!(
            r#"<div class="type-card">
    <div class="type-header">
        <span class="type-name">{}</span>
        <span class="type-category" style="background: {}">{}</span>
    </div>
    <div class="type-meta">
        <span class="certainty {}">{}</span>
        <span class="source">{}</span>
    </div>
    {}
    {}
    {}
</div>"#,
            type_dto.name,
            category_color,
            type_dto.category,
            certainty_class,
            type_dto.certainty_text,
            type_dto.source,
            self.render_facets(&type_dto.facets),
            self.render_methods_section(&type_dto.methods),
            self.render_properties_section(&type_dto.properties),
        )
    }

    /// Рендерить фасеты типа
    fn render_facets(&self, facets: &[String]) -> String {
        if facets.is_empty() {
            return String::new();
        }

        let facets_html: Vec<String> = facets
            .iter()
            .map(|f| format!(r#"<span class="facet">{}</span>"#, f))
            .collect();

        format!(
            r#"<div class="facets">
        <strong>Фасеты:</strong> {}
    </div>"#,
            facets_html.join(" ")
        )
    }

    /// Рендерить секцию методов
    fn render_methods_section(&self, methods: &[bsl_shared::api::dtos::MethodDto]) -> String {
        if methods.is_empty() {
            return String::new();
        }

        let count = methods.len();
        let methods_list = if count > 5 {
            // Показать первые 5 + счётчик
            let visible: Vec<String> = methods
                .iter()
                .take(5)
                .map(|m| {
                    let signature = self.format_method_signature(m);
                    format!(r#"<li><code>{}</code></li>"#, signature)
                })
                .collect();

            format!(
                r#"<ul class="methods-list">
            {}
            <li class="more">... и ещё {} методов</li>
        </ul>"#,
                visible.join("\n            "),
                count - 5
            )
        } else {
            let all: Vec<String> = methods
                .iter()
                .map(|m| {
                    let signature = self.format_method_signature(m);
                    format!(r#"<li><code>{}</code></li>"#, signature)
                })
                .collect();

            format!(
                r#"<ul class="methods-list">
            {}
        </ul>"#,
                all.join("\n            ")
            )
        };

        format!(
            r#"<div class="methods-section">
        <strong>Методы ({}):</strong>
        {}
    </div>"#,
            count, methods_list
        )
    }

    /// Форматировать сигнатуру метода
    fn format_method_signature(&self, method: &bsl_shared::api::dtos::MethodDto) -> String {
        let params_str = method
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect::<Vec<_>>()
            .join(", ");

        let return_str = method
            .return_type
            .as_ref()
            .map(|t| format!(" → {}", t))
            .unwrap_or_default();

        format!("{}({}){}", method.name, params_str, return_str)
    }

    /// Рендерить секцию свойств
    fn render_properties_section(&self, properties: &[String]) -> String {
        if properties.is_empty() {
            return String::new();
        }

        let count = properties.len();
        let props_list: Vec<String> = properties
            .iter()
            .map(|p| format!(r#"<li><code>{}</code></li>"#, p))
            .collect();

        format!(
            r#"<div class="properties-section">
        <strong>Свойства ({}):</strong>
        <ul class="properties-list">
            {}
        </ul>
    </div>"#,
            count,
            props_list.join("\n            ")
        )
    }

    /// Получить CSS класс для certainty
    fn get_certainty_class(&self, certainty: u8) -> &str {
        match certainty {
            90..=100 => "certainty-high",
            50..=89 => "certainty-medium",
            _ => "certainty-low",
        }
    }

    /// Рендерить enum значения (для перечислений платформы)
    fn render_enum_values(&self, enum_values: &[String]) -> String {
        if enum_values.is_empty() {
            return String::new();
        }

        let values_html: Vec<String> = enum_values
            .iter()
            .map(|v| format!(r#"<li><code>{}</code></li>"#, v))
            .collect();

        format!(
            r#"<div class="enum-values">
        <strong>Значения перечисления:</strong>
        <ul class="enum-list">
            {}
        </ul>
    </div>"#,
            values_html.join("\n            ")
        )
    }
}

impl TypeRenderer for HtmlRenderer {
    fn render_type_info(&self, type_dto: &TypeDto) -> Result<String> {
        let card_html = self.render_type_card(type_dto);

        // Добавить описание если есть
        let description_html = if !type_dto.description.is_empty() {
            format!(
                r#"<div class="description">
    <p>{}</p>
</div>"#,
                type_dto.description
            )
        } else {
            String::new()
        };

        // Добавить enum values если есть
        let enum_html = if let Some(ref values) = type_dto.enum_values {
            self.render_enum_values(values)
        } else {
            String::new()
        };

        let body = format!(
            r#"<div class="type-info-container">
    <h1>Тип: {}</h1>
    {}
    {}
    {}
</div>"#,
            type_dto.name, card_html, description_html, enum_html
        );

        Ok(self.render_document(&format!("Тип: {}", type_dto.name), &body))
    }

    fn render_methods(&self, type_name: &str, methods: &[String]) -> Result<String> {
        let methods_html: Vec<String> = methods
            .iter()
            .map(|m| format!(r#"<li><code>{}</code></li>"#, m))
            .collect();

        let body = format!(
            r#"<div class="methods-container">
    <h1>Методы типа: {}</h1>
    <p>Всего методов: <strong>{}</strong></p>
    <ul class="methods-list">
        {}
    </ul>
</div>"#,
            type_name,
            methods.len(),
            methods_html.join("\n        ")
        );

        Ok(self.render_document(&format!("Методы: {}", type_name), &body))
    }

    fn render_properties(&self, type_name: &str, properties: &[String]) -> Result<String> {
        let props_html: Vec<String> = properties
            .iter()
            .map(|p| format!(r#"<li><code>{}</code></li>"#, p))
            .collect();

        let body = format!(
            r#"<div class="properties-container">
    <h1>Свойства типа: {}</h1>
    <p>Всего свойств: <strong>{}</strong></p>
    <ul class="properties-list">
        {}
    </ul>
</div>"#,
            type_name,
            properties.len(),
            props_html.join("\n        ")
        );

        Ok(self.render_document(&format!("Свойства: {}", type_name), &body))
    }

    fn render_metrics(&self, total_types: usize, total_methods: usize) -> Result<String> {
        let body = format!(
            r#"<div class="metrics-container">
    <h1>Метрики системы типов</h1>
    <div class="metrics-grid">
        <div class="metric-card">
            <div class="metric-value">{}</div>
            <div class="metric-label">Всего типов</div>
        </div>
        <div class="metric-card">
            <div class="metric-value">{}</div>
            <div class="metric-label">Всего методов</div>
        </div>
    </div>
</div>"#,
            total_types, total_methods
        );

        Ok(self.render_document("Метрики", &body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_renderer_creation() {
        let renderer = HtmlRenderer::with_default_theme();
        assert!(renderer.options.syntax_highlight);
    }

    #[test]
    fn test_render_facets() {
        let renderer = HtmlRenderer::with_default_theme();
        let facets = vec!["Manager".to_string(), "Object".to_string()];
        let html = renderer.render_facets(&facets);

        assert!(html.contains("Manager"));
        assert!(html.contains("Object"));
        assert!(html.contains("facet"));
    }

    #[test]
    fn test_render_methods_section() {
        use bsl_shared::api::dtos::MethodDto;

        let renderer = HtmlRenderer::with_default_theme();

        // Создаём правильные MethodDto объекты
        let methods = vec![
            MethodDto {
                name: "Создать".to_string(),
                english_name: Some("Create".to_string()),
                return_type: Some("Произвольный".to_string()),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: true,
            },
            MethodDto {
                name: "Удалить".to_string(),
                english_name: Some("Delete".to_string()),
                return_type: None,
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
            },
        ];

        let html = renderer.render_methods_section(&methods);

        assert!(html.contains("Создать"));
        assert!(html.contains("Удалить"));
        assert!(html.contains("Методы (2)"));
    }
}
