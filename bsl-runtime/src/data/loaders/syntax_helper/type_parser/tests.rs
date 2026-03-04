use super::*;

/// Тест парсинга простого типа
#[test]
fn test_assemble_types_simple() {
    let fragments = vec![TypeFragment::TypeName("Строка".to_string())];
    assert_eq!(TypeParser::assemble_types(&fragments), "Строка");
}

/// Тест парсинга union типа
#[test]
fn test_assemble_types_union() {
    let fragments = vec![
        TypeFragment::TypeName("Число".to_string()),
        TypeFragment::UnionSeparator,
        TypeFragment::TypeName("Строка".to_string()),
    ];
    assert_eq!(TypeParser::assemble_types(&fragments), "Число | Строка");
}

/// Тест парсинга faceted типа
#[test]
fn test_assemble_types_faceted() {
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя справочника".to_string()),
        TypeFragment::GenericClose,
    ];
    assert_eq!(
        TypeParser::assemble_types(&fragments),
        "СправочникСсылка.<T>"
    );
}

/// Тест парсинга faceted + union
#[test]
fn test_assemble_types_faceted_union() {
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя справочника".to_string()),
        TypeFragment::GenericClose,
        TypeFragment::UnionSeparator,
        TypeFragment::TypeName("Неопределено".to_string()),
    ];
    assert_eq!(
        TypeParser::assemble_types(&fragments),
        "СправочникСсылка.<T> | Неопределено"
    );
}

/// Тест для malformed HTML: unclosed generic
///
/// Проверяет, что парсер корректно обрабатывает ситуацию,
/// когда GenericOpen не имеет парного GenericClose
#[test]
fn test_assemble_types_unclosed_generic() {
    // Malformed: открыли generic, но не закрыли
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя".to_string()),
        // GenericClose отсутствует
    ];

    let result = TypeParser::assemble_types(&fragments);

    // Тип должен быть возвращен БЕЗ паники
    assert!(!result.is_empty(), "Результат не должен быть пустым");

    // Проверяем, что добавлен <T> для robustness
    // Trailing точка удаляется при финализации типа, затем добавляется <T>
    assert_eq!(
        result, "СправочникСсылка<T>",
        "Unclosed generic должен быть закрыт автоматически"
    );
}

/// Тест для unclosed generic с union separator после
#[test]
fn test_assemble_types_unclosed_generic_with_union() {
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя".to_string()),
        // GenericClose отсутствует, но есть union separator
        TypeFragment::UnionSeparator,
        TypeFragment::TypeName("Строка".to_string()),
    ];

    let result = TypeParser::assemble_types(&fragments);

    // in_generic сбрасывается при UnionSeparator
    // Поэтому СправочникСсылка. без <T>, затем Строка
    assert!(!result.is_empty(), "Результат не должен быть пустым");
    assert!(
        result.contains("Строка"),
        "Union тип должен содержать 'Строка'"
    );
}

/// Тест парсинга фрагментов из HTML
#[test]
fn test_parse_type_fragments() {
    let type_line = r#" <a href="...">СправочникСсылка.</a><span>&lt;</span><a href="...">Имя справочника</a><span>&gt;</span>"#;
    let fragments = TypeParser::parse_type_fragments(type_line);

    // Должны быть: TypeName("СправочникСсылка."), GenericOpen, TypeName("Имя справочника"), GenericClose
    assert!(fragments
        .iter()
        .any(|f| matches!(f, TypeFragment::TypeName(s) if s == "СправочникСсылка.")));
    assert!(fragments
        .iter()
        .any(|f| matches!(f, TypeFragment::GenericOpen)));
    assert!(fragments
        .iter()
        .any(|f| matches!(f, TypeFragment::TypeName(s) if s == "Имя справочника")));
    assert!(fragments
        .iter()
        .any(|f| matches!(f, TypeFragment::GenericClose)));
}

/// Тест извлечения строки типа
#[test]
fn test_extract_type_line() {
    let section = r#"
        Тип: <a href="type">Строка</a>. <br>
        Описание результата.
    "#;

    let (type_line, description) = TypeParser::extract_type_line(section);

    assert!(type_line.contains("<a"));
    assert!(type_line.contains("Строка"));
    assert_eq!(description, Some("Описание результата.".to_string()));
}

/// Тест поиска секции возвращаемого значения
#[test]
fn test_find_return_value_section() {
    let html = r#"
        <p class="V8SH_chapter">Параметры:</p>
        ...
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">Строка</a>. <br>
        Описание.
        <p class="V8SH_chapter">Пример:</p>
    "#;

    let section = TypeParser::find_return_value_section(html);

    assert!(section.is_some());
    let section = section.unwrap();
    assert!(section.contains("Тип:"));
    assert!(section.contains("Строка"));
    assert!(!section.contains("Пример:"));
}

/// Тест полного парсинга возвращаемого типа
#[test]
fn test_parse_return_type_full() {
    let html = r#"
        <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">СправочникСсылка.</a><span>&lt;</span><a href="type">Имя</a><span>&gt;</span>, <a href="type">Неопределено</a>. <br>
        Описание результата.
        </html>
    "#;

    let (return_type, description) = TypeParser::parse_return_type(html);

    assert_eq!(
        return_type,
        Some("СправочникСсылка.<T> | Неопределено".to_string())
    );
    assert_eq!(description, Some("Описание результата.".to_string()));
}

/// Тест: нет секции возвращаемого значения
#[test]
fn test_parse_return_type_no_section() {
    let html = r#"
        <html>
        <p class="V8SH_chapter">Параметры:</p>
        Только параметры, нет возвращаемого значения.
        </html>
    "#;

    let (return_type, description) = TypeParser::parse_return_type(html);

    assert_eq!(return_type, None);
    assert_eq!(description, None);
}
