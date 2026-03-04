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
