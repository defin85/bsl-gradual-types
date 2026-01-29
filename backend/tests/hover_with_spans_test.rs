//! Milestone 2.11: Task A3 - Интеграционный тест hover с реальными Span
//!
//! Проверяет что:
//! 1. find_node_at_position() находит узлы по реальным координатам из tree-sitter
//! 2. find_variable_at_position() работает корректно
//! 3. Hover показывает разную информацию для разных переменных (не одинаковую)

mod support;

#[tokio::test]
async fn test_hover_on_variable_declaration() {
    // Тестируем hover на переменной в объявлении
    let code = r#"
Функция ТестМассива()
    МойМассив = Новый Массив();
    МойМассив.Добавить("элемент");
    Возврат МойМассив;
КонецФункции
"#;

    let deps_bundle = support::deps_bundle_v2_fallback();

    // Hover на "МойМассив" в строке 2 (declaration)
    // Координаты (line=2, column=5) должны попасть в span переменной "МойМассив"
    let hover1 = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5);

    assert!(
        hover1.is_some(),
        "Hover должен вернуть информацию для МойМассив в строке 2"
    );
    let hover_text = hover1.unwrap();

    // Проверяем что hover содержит информацию о типе Массив
    assert!(hover_text.contains("Массив") || hover_text.contains("МойМассив"),
        "Hover должен содержать информацию о типе Массив или имени переменной МойМассив\nActual: {}", hover_text);
}

#[tokio::test]
async fn test_hover_on_variable_usage() {
    // Тестируем hover на переменной при использовании
    let code = r#"
Функция ТестМассива()
    МойМассив = Новый Массив();
    МойМассив.Добавить("элемент");
    Возврат МойМассив;
КонецФункции
"#;

    let deps_bundle = support::deps_bundle_v2_fallback();

    // Hover на "МойМассив" в строке 3 (method call)
    let hover2 = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 3, 5);

    assert!(
        hover2.is_some(),
        "Hover должен вернуть информацию для МойМассив в строке 3"
    );
    let hover_text = hover2.unwrap();

    // Проверяем что hover содержит информацию о методе Добавить или типе Массив
    assert!(
        hover_text.contains("Добавить")
            || hover_text.contains("Массив")
            || hover_text.contains("МойМассив"),
        "Hover должен содержать информацию о методе Добавить или типе Массив\nActual: {}",
        hover_text
    );
}

#[tokio::test]
async fn test_hover_shows_different_info_for_different_variables() {
    // Тестируем что hover показывает разную информацию для разных переменных
    let code = r#"
Процедура ТестРазныхТипов()
    МойМассив = Новый Массив();
    МояСтрока = "текст";
    МоеЧисло = 42;
КонецПроцедуры
"#;

    let deps_bundle = support::deps_bundle_v2_fallback();

    // Hover на "МойМассив" в строке 2
    let hover1 =
        support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 5).unwrap_or_default();

    // Hover на "МояСтрока" в строке 3
    let hover2 =
        support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 3, 5).unwrap_or_default();

    // Hover на "МоеЧисло" в строке 4
    let hover3 =
        support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 4, 5).unwrap_or_default();

    // КЛЮЧЕВАЯ ПРОВЕРКА: hover НЕ должен быть одинаковым для всех переменных
    // (проблема из Milestone 2.11 - без реальных Span все переменные показывали одинаковую информацию)
    assert_ne!(
        hover1, hover2,
        "Hover для МойМассив и МояСтрока должен отличаться"
    );
    assert_ne!(
        hover2, hover3,
        "Hover для МояСтрока и МоеЧисло должен отличаться"
    );
    assert_ne!(
        hover1, hover3,
        "Hover для МойМассив и МоеЧисло должен отличаться"
    );

    // Проверяем что каждый hover содержит правильный тип
    assert!(
        hover1.contains("Массив") || hover1.contains("МойМассив"),
        "Hover для МойМассив должен содержать 'Массив'\nActual: {}",
        hover1
    );
    assert!(
        hover2.contains("Строка") || hover2.contains("МояСтрока") || hover2.contains("текст"),
        "Hover для МояСтрока должен содержать 'Строка'\nActual: {}",
        hover2
    );
    assert!(
        hover3.contains("Число") || hover3.contains("МоеЧисло") || hover3.contains("42"),
        "Hover для МоеЧисло должен содержать 'Число'\nActual: {}",
        hover3
    );
}

#[tokio::test]
async fn test_hover_on_function_parameter() {
    // Тестируем hover на параметре функции
    let code = r#"
Функция СложитьЧисла(Число1, Число2)
    Результат = Число1 + Число2;
    Возврат Результат;
КонецФункции
"#;

    let deps_bundle = support::deps_bundle_v2_fallback();

    // Hover на "Число1" в строке 2 (usage in assignment)
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 2, 16);

    assert!(
        hover.is_some(),
        "Hover должен вернуть информацию для параметра Число1"
    );
    let hover_text = hover.unwrap();

    // Проверяем что hover содержит информацию о параметре
    assert!(
        hover_text.contains("Число1") || hover_text.contains("Результат"),
        "Hover должен содержать информацию о переменной\nActual: {}",
        hover_text
    );
}

#[tokio::test]
async fn test_hover_on_method_name() {
    // Тестируем hover на имени метода
    let code = r#"
Процедура ТестМетода()
    Массив = Новый Массив();
    Массив.Добавить("элемент");
КонецПроцедуры
"#;

    let deps_bundle = support::deps_bundle_v2_fallback();

    // Hover на "Добавить" в строке 3
    // Координаты (line=3, column=12) должны попасть в span метода "Добавить"
    let hover = support::hover_for_code(deps_bundle.as_ref(), "inline.bsl", code, 3, 12);

    assert!(
        hover.is_some(),
        "Hover должен вернуть информацию для метода Добавить"
    );
    let hover_text = hover.unwrap();

    // Проверяем что hover содержит информацию о методе или об объекте
    assert!(
        hover_text.contains("Добавить")
            || hover_text.contains("Массив")
            || hover_text.contains("элемент"),
        "Hover должен содержать информацию о методе или типе\nActual: {}",
        hover_text
    );
}

#[tokio::test]
async fn test_span_contains_correct_position() {
    // Тестируем что Span.contains() корректно определяет вхождение byte offset
    use bsl_shared::ir::Span;

    // Диапазон [start, end) в UTF-8 byte offsets.
    let span = Span::new(10, 20);

    // Позиции внутри span должны вернуть true
    assert!(span.contains(10), "start должен входить в span");
    assert!(span.contains(15), "середина должна входить в span");

    // `end` - exclusive
    assert!(!span.contains(20), "end (exclusive) не должен входить в span");

    // Позиции снаружи span должны вернуть false
    assert!(!span.contains(9), "позиция перед началом не должна входить в span");
    assert!(!span.contains(21), "позиция после конца не должна входить в span");
}
