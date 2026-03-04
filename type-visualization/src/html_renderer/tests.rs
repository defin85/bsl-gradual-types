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
