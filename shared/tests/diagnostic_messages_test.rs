//! Тесты для Phase 3 Milestone 3.6: Enhanced Diagnostic Messages
//!
//! Тестирует:
//! 1. Три уровня детализации (Compact, Full, Detailed)
//! 2. Переменные имена (variable_name, param_variable_name)
//! 3. Graceful degradation
//! 4. Backward compatibility с to_diagnostic()
//! 5. Умные подсказки (hints)
//! 6. Edge cases (отсутствие имён переменных)

use bsl_shared::domain::types::DiagnosticSeverity;
use bsl_shared::domain::validators::TypeErrorKind;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::Span;

// ===== TEST 2.1: Brief/Compact Format Tests =====

#[test]
fn test_nonexistent_method_compact_format_no_variable_name() {
    println!("\n=== TEST 2.1.1: NonExistentMethod - Compact without variable_name ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "НесуществующийМетод".to_string(),
        variable_name: None,
    };

    let span = Span::new(10, 20);
    let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Compact);

    println!("Message: {}", diagnostic.message);

    // Compact НЕ должен показывать имя переменной
    assert!(diagnostic.message.contains("'НесуществующийМетод'"));
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(!diagnostic.message.contains("переменной")); // НЕ должно быть

    println!("✅ PASS: Compact format не содержит имя переменной");
    println!();
}

#[test]
fn test_nonexistent_method_compact_format_with_variable_name() {
    println!(
        "\n=== TEST 2.1.2: NonExistentMethod - Compact with variable_name (should ignore) ==="
    );

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "НесуществующийМетод".to_string(),
        variable_name: Some("списокИмен".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Compact);

    println!("Message: {}", diagnostic.message);

    // Compact ДОЛЖЕН игнорировать variable_name
    assert!(diagnostic.message.contains("'НесуществующийМетод'"));
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(!diagnostic.message.contains("списокИмен")); // НЕ должна быть видна переменная
    assert!(!diagnostic.message.contains("переменной"));

    println!("✅ PASS: Compact format игнорирует variable_name");
    println!();
}

#[test]
fn test_parameter_type_mismatch_compact_format() {
    println!("\n=== TEST 2.1.3: IncorrectParameterType - Compact format ===");

    let error = TypeErrorKind::IncorrectParameterType {
        method_name: "Вставить".to_string(),
        param_index: 0,
        expected: "Число".to_string(),
        actual: "Строка".to_string(),
        variable_name: Some("ТЗ".to_string()),
        param_variable_name: Some("индекс".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Compact);

    println!("Message: {}", diagnostic.message);

    // Compact НЕ должен показывать имена переменных, только типы
    assert!(diagnostic.message.contains("Число"));
    assert!(diagnostic.message.contains("Строка"));
    assert!(!diagnostic.message.contains("ТЗ"));
    assert!(!diagnostic.message.contains("индекс"));

    println!("✅ PASS: Compact format показывает только типы");
    println!();
}

// ===== TEST 2.2: Standard/Full Format Tests =====

#[test]
fn test_nonexistent_method_standard_format_with_variable_name() {
    println!("\n=== TEST 2.2.1: NonExistentMethod - Standard with variable_name ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "НесуществующийМетод".to_string(),
        variable_name: Some("списокИмен".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);

    println!("Message: {}", diagnostic.message);

    // Standard ДОЛЖЕН показывать имя переменной
    assert!(diagnostic.message.contains("списокИмен"));
    assert!(
        diagnostic.message.contains("переменной") || diagnostic.message.contains("'списокИмен'")
    );
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(!diagnostic.message.contains("💡")); // НЕ должна быть подсказка

    println!("✅ PASS: Standard format содержит имя переменной");
    println!();
}

#[test]
fn test_nonexistent_method_standard_format_without_variable_name() {
    println!("\n=== TEST 2.2.2: NonExistentMethod - Standard without variable_name (fallback) ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "Метод".to_string(),
        variable_name: None,
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);

    println!("Message: {}", diagnostic.message);

    // Standard без variable_name должен fallback к Brief формату
    assert!(diagnostic.message.contains("'Метод'"));
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(!diagnostic.message.contains("переменной"));

    println!("✅ PASS: Standard без variable_name fallback к Brief");
    println!();
}

#[test]
fn test_parameter_type_mismatch_standard_format() {
    println!("\n=== TEST 2.2.3: IncorrectParameterType - Standard format ===");

    let error = TypeErrorKind::IncorrectParameterType {
        method_name: "Вставить".to_string(),
        param_index: 0,
        expected: "Число".to_string(),
        actual: "Строка".to_string(),
        variable_name: Some("ТЗ".to_string()),
        param_variable_name: Some("индекс".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);

    println!("Message: {}", diagnostic.message);

    // Standard ДОЛЖЕН показывать ОБА имени переменных
    assert!(diagnostic.message.contains("'ТЗ'") || diagnostic.message.contains("ТЗ"));
    assert!(diagnostic.message.contains("'индекс'") || diagnostic.message.contains("индекс"));
    assert!(!diagnostic.message.contains("💡")); // НЕТ подсказок

    println!("✅ PASS: Standard format содержит оба имени переменных");
    println!();
}

// ===== TEST 2.3: Detailed Format Tests =====

#[test]
fn test_nonexistent_method_detailed_format_with_hint() {
    println!("\n=== TEST 2.3.1: NonExistentMethod - Detailed with hint ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "НесуществующийМетод".to_string(),
        variable_name: Some("списокИмен".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Detailed);

    println!("Message:\n{}", diagnostic.message);

    // Detailed ДОЛЖЕН содержать Standard контент + подсказку
    assert!(diagnostic.message.contains("списокИмен"));
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(diagnostic.message.contains("💡 Подсказка:")); // ← Подсказка ДОЛЖНА быть

    println!("✅ PASS: Detailed format содержит подсказку");
    println!();
}

#[test]
fn test_parameter_type_mismatch_detailed_format_with_hint() {
    println!("\n=== TEST 2.3.2: IncorrectParameterType - Detailed with hint ===");

    let error = TypeErrorKind::IncorrectParameterType {
        method_name: "Вставить".to_string(),
        param_index: 0,
        expected: "Число".to_string(),
        actual: "Строка".to_string(),
        variable_name: Some("ТЗ".to_string()),
        param_variable_name: Some("индекс".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Detailed);

    println!("Message:\n{}", diagnostic.message);

    // Detailed ДОЛЖЕН содержать оба имени + подсказку
    assert!(diagnostic.message.contains("'ТЗ'") || diagnostic.message.contains("ТЗ"));
    assert!(diagnostic.message.contains("'индекс'") || diagnostic.message.contains("индекс"));
    assert!(diagnostic.message.contains("💡 Подсказка:"));

    println!("✅ PASS: Detailed format содержит оба имени и подсказку");
    println!();
}

#[test]
fn test_nonexistent_property_detailed_format_with_hint() {
    println!("\n=== TEST 2.3.3: NonExistentProperty - Detailed with hint ===");

    let error = TypeErrorKind::NonExistentProperty {
        object_type: "ТаблицаЗначений".to_string(),
        property_name: "НесуществующееСвойство".to_string(),
        variable_name: Some("таблица".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Detailed);

    println!("Message:\n{}", diagnostic.message);

    assert!(diagnostic.message.contains("'таблица'"));
    assert!(diagnostic.message.contains("'ТаблицаЗначений'"));
    assert!(diagnostic.message.contains("💡 Подсказка:"));

    println!("✅ PASS: NonExistentProperty detailed format работает");
    println!();
}

#[test]
fn test_simple_type_as_collection_detailed_format_with_hint() {
    println!("\n=== TEST 2.3.4: SimpleTypeAsCollection - Detailed with hint ===");

    let error = TypeErrorKind::SimpleTypeAsCollection {
        type_name: "Число".to_string(),
        operation: "Добавить".to_string(),
        variable_name: Some("числовое_значение".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Detailed);

    println!("Message:\n{}", diagnostic.message);

    assert!(diagnostic.message.contains("'числовое_значение'"));
    assert!(diagnostic.message.contains("'Число'"));
    assert!(diagnostic.message.contains("'Добавить'"));
    assert!(diagnostic.message.contains("💡 Подсказка:"));
    assert!(diagnostic.message.contains("Массив") || diagnostic.message.contains("коллекц"));

    println!("✅ PASS: SimpleTypeAsCollection detailed format работает");
    println!();
}

// ===== TEST 2.4: Edge Cases - Missing variable_name =====

#[test]
fn test_property_without_variable_name_fallback_to_brief() {
    println!("\n=== TEST 2.4.1: NonExistentProperty without variable_name (fallback) ===");

    let error = TypeErrorKind::NonExistentProperty {
        object_type: "Массив".to_string(),
        property_name: "НесуществующееСвойство".to_string(),
        variable_name: None,
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);

    println!("Message: {}", diagnostic.message);

    // Без variable_name → fallback к Brief формату
    assert!(diagnostic.message.contains("'НесуществующееСвойство'"));
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(!diagnostic.message.contains("переменной"));

    println!("✅ PASS: Fallback к Brief работает");
    println!();
}

#[test]
fn test_collection_operation_without_variable_name_fallback() {
    println!("\n=== TEST 2.4.2: SimpleTypeAsCollection without variable_name (fallback) ===");

    let error = TypeErrorKind::SimpleTypeAsCollection {
        type_name: "Строка".to_string(),
        operation: "Очистить".to_string(),
        variable_name: None,
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);

    println!("Message: {}", diagnostic.message);

    assert!(diagnostic.message.contains("'Строка'"));
    assert!(!diagnostic.message.contains("переменной"));

    println!("✅ PASS: SimpleTypeAsCollection fallback работает");
    println!();
}

// ===== TEST 2.5: Backward Compatibility =====

#[test]
fn test_old_to_diagnostic_method_still_works() {
    println!("\n=== TEST 2.5.1: Old to_diagnostic() method backward compatibility ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "Метод".to_string(),
        variable_name: Some("x".to_string()),
    };

    // Старый метод to_diagnostic(span) должен работать
    let diagnostic = error.to_diagnostic(Span::new(10, 20));

    println!("Message: {}", diagnostic.message);

    // Должен использовать Brief формат по умолчанию (backward compatibility)
    assert!(!diagnostic.message.is_empty());
    assert!(diagnostic.message.contains("'Метод'"));
    assert!(diagnostic.message.contains("'Массив'"));
    assert!(!diagnostic.message.contains("'x'")); // Brief не показывает переменную

    println!("✅ PASS: Old to_diagnostic() работает и использует Brief");
    println!();
}

#[test]
fn test_diagnostic_has_correct_span_information() {
    println!("\n=== TEST 2.5.2: Diagnostic span information preserved ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "Метод".to_string(),
        variable_name: None,
    };

    let span = Span::new(50, 60);
    let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Full);

    assert_eq!(diagnostic.span, span);
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);

    println!("✅ PASS: Span информация сохранена");
    println!();
}

#[test]
fn test_parameter_mismatch_param_index_reflected() {
    println!("\n=== TEST 2.5.3: Parameter index in message ===");

    let error = TypeErrorKind::IncorrectParameterType {
        method_name: "Вставить".to_string(),
        param_index: 2,
        expected: "Число".to_string(),
        actual: "Строка".to_string(),
        variable_name: None,
        param_variable_name: None,
    };

    let diagnostic = error.to_diagnostic(Span::new(10, 20));

    println!("Message: {}", diagnostic.message);

    // Номер параметра должен быть в сообщении (index + 1)
    assert!(diagnostic.message.contains("#3")); // param_index 2 → #3

    println!("✅ PASS: Номер параметра корректен");
    println!();
}

// ===== TEST 2.6: Message Quality Checks =====

#[test]
fn test_messages_are_russian_and_readable() {
    println!("\n=== TEST 2.6: Messages are in Russian and readable ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "Метод".to_string(),
        variable_name: Some("переменная".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Detailed);

    println!("Message:\n{}\n", diagnostic.message);

    // Message должно содержать русские буквы
    assert!(diagnostic
        .message
        .chars()
        .any(|c| c as u32 >= 0x0400 && c as u32 <= 0x04FF));
    // Message должно содержать русские слова
    assert!(
        diagnostic.message.contains("не")
            || diagnostic.message.contains("Не")
            || diagnostic.message.contains("Проверьте")
    );

    println!("✅ PASS: Сообщение на русском и читаемо");
    println!();
}

// ===== TEST 2.7: Detailed Hints Format Checks =====

#[test]
fn test_detailed_hints_start_with_emoji() {
    println!("\n=== TEST 2.7.1: Detailed hints start with emoji ===");

    let error = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "Метод".to_string(),
        variable_name: Some("x".to_string()),
    };

    let diagnostic = error.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Detailed);

    println!("Message:\n{}\n", diagnostic.message);

    assert!(diagnostic.message.contains("💡 Подсказка:"));

    println!("✅ PASS: Подсказка начинается с эмодзи");
    println!();
}

#[test]
fn test_method_validator_with_variable_contextual() {
    println!("\n=== TEST 2.7.2: Method validation with variable context ===");

    // Test that the implementation correctly uses variable_name for context
    let error_with_var = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "НесуществующийМетод".to_string(),
        variable_name: Some("мойМассив".to_string()),
    };

    let error_without_var = TypeErrorKind::NonExistentMethod {
        object_type: "Массив".to_string(),
        method_name: "НесуществующийМетод".to_string(),
        variable_name: None,
    };

    let diag_with =
        error_with_var.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);
    let diag_without =
        error_without_var.to_diagnostic_with_detail(Span::new(10, 20), DetailLevel::Full);

    println!("With variable: {}", diag_with.message);
    println!("Without variable: {}", diag_without.message);

    // With variable should contain it, without should not try to reference a variable
    assert!(diag_with.message.contains("мойМассив"));
    assert!(!diag_without.message.contains("мойМассив"));

    println!("✅ PASS: Variable context handled correctly");
    println!();
}

// ===== TEST 2.8: All ErrorKind Variants Support =====

#[test]
fn test_all_error_kinds_have_brief_format() {
    println!("\n=== TEST 2.8: All ErrorKind variants support Compact format ===");

    let errors = vec![
        TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "Метод".to_string(),
            variable_name: Some("x".to_string()),
        },
        TypeErrorKind::NonExistentProperty {
            object_type: "Объект".to_string(),
            property_name: "Свойство".to_string(),
            variable_name: Some("obj".to_string()),
        },
        TypeErrorKind::IncorrectParameterType {
            method_name: "Функция".to_string(),
            param_index: 0,
            expected: "Число".to_string(),
            actual: "Строка".to_string(),
            variable_name: Some("var1".to_string()),
            param_variable_name: Some("var2".to_string()),
        },
        TypeErrorKind::SimpleTypeAsCollection {
            type_name: "Строка".to_string(),
            operation: "Добавить".to_string(),
            variable_name: Some("str".to_string()),
        },
    ];

    let span = Span::new(10, 20);

    for error in &errors {
        let compact = error.to_diagnostic_with_detail(span, DetailLevel::Compact);
        let full = error.to_diagnostic_with_detail(span, DetailLevel::Full);
        let detailed = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

        assert!(!compact.message.is_empty());
        assert!(!full.message.is_empty());
        assert!(!detailed.message.is_empty());

        println!("✓ {:?} has all 3 detail levels", error);
    }

    println!("✅ PASS: Все 4 ErrorKind варианта поддерживают все 3 DetailLevel");
    println!();
}
