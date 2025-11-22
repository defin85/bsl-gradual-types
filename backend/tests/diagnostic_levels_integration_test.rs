//! Integration Tests для Phase 3 Milestone 3.6: Enhanced Diagnostic Messages
//!
//! End-to-end тестирование:
//! 1. Настройки DiagnosticsSettings (detailLevel, showHints)
//! 2. Валидация с различными уровнями детализации
//! 3. Интеграция с LSP server
//! 4. Динамическое изменение настроек

#[cfg(test)]
mod diagnostic_levels_integration_tests {
    use bsl_shared::domain::types::DiagnosticSeverity;
    use bsl_shared::domain::validators::TypeErrorKind;
    use bsl_shared::formatting::DetailLevel;
    use bsl_shared::ir::Span;

    // ===== Test 3.1: DetailLevel.from_str() conversion =====

    #[test]
    fn test_diagnostics_detail_level_conversion_compact() {
        let level = DetailLevel::from_str("compact");
        assert_eq!(level, DetailLevel::Compact);
    }

    #[test]
    fn test_diagnostics_detail_level_conversion_full() {
        let level = DetailLevel::from_str("full");
        assert_eq!(level, DetailLevel::Full);
    }

    #[test]
    fn test_diagnostics_detail_level_conversion_detailed() {
        let level = DetailLevel::from_str("detailed");
        assert_eq!(level, DetailLevel::Detailed);
    }

    #[test]
    fn test_diagnostics_detail_level_default_for_unknown() {
        let level = DetailLevel::from_str("unknown_setting");
        assert_eq!(level, DetailLevel::Full);
    }

    // ===== Test 3.2: Diagnostic message formatting with detail levels =====

    #[test]
    fn test_compact_diagnostics_only_type_info() {
        println!("\n=== Test 3.2.1: Compact diagnostics - type info only ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "НесуществующийМетод".to_string(),
            variable_name: Some("myArray".to_string()),
        };

        let span = Span::new(1, 0, 1, 20);
        let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Compact);

        // Compact должна быть короче с минимум информации
        assert!(diagnostic.message.contains("'НесуществующийМетод'"));
        assert!(diagnostic.message.contains("'Массив'"));
        assert!(!diagnostic.message.contains("myArray"));
        assert!(!diagnostic.message.contains("💡"));

        println!("✅ Message: {}", diagnostic.message);
    }

    #[test]
    fn test_full_diagnostics_with_variable_name() {
        println!("\n=== Test 3.2.2: Full diagnostics - with variable name ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "НесуществующийМетод".to_string(),
            variable_name: Some("myArray".to_string()),
        };

        let span = Span::new(1, 0, 1, 20);
        let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Full);

        // Full должна содержать имя переменной
        assert!(diagnostic.message.contains("myArray"));
        assert!(!diagnostic.message.contains("💡")); // Без подсказок

        println!("✅ Message: {}", diagnostic.message);
    }

    #[test]
    fn test_detailed_diagnostics_with_hints() {
        println!("\n=== Test 3.2.3: Detailed diagnostics - with hints ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "НесуществующийМетод".to_string(),
            variable_name: Some("myArray".to_string()),
        };

        let span = Span::new(1, 0, 1, 20);
        let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

        // Detailed должна содержать и переменную и подсказку
        assert!(diagnostic.message.contains("myArray"));
        assert!(diagnostic.message.contains("💡 Подсказка:"));

        println!("✅ Message:\n{}", diagnostic.message);
    }

    // ===== Test 3.3: Different error types with detail levels =====

    #[test]
    fn test_parameter_type_mismatch_all_levels() {
        println!("\n=== Test 3.3.1: ParameterTypeMismatch across all levels ===");

        let error = TypeErrorKind::IncorrectParameterType {
            method_name: "Вставить".to_string(),
            param_index: 0,
            expected: "Число".to_string(),
            actual: "Строка".to_string(),
            variable_name: Some("ТЗ".to_string()),
            param_variable_name: Some("индекс".to_string()),
        };

        let span = Span::new(1, 0, 1, 20);

        let compact = error.to_diagnostic_with_detail(span, DetailLevel::Compact);
        let full = error.to_diagnostic_with_detail(span, DetailLevel::Full);
        let detailed = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

        // Compact: только типы
        assert!(!compact.message.contains("ТЗ"));
        assert!(!compact.message.contains("индекс"));
        assert!(compact.message.contains("Число"));
        assert!(compact.message.contains("Строка"));
        println!("Compact: {}", compact.message);

        // Full: с переменными, без подсказок
        assert!(full.message.contains("ТЗ") || full.message.contains("индекс"));
        assert!(!full.message.contains("💡"));
        println!("Full: {}", full.message);

        // Detailed: с переменными и подсказками
        assert!(detailed.message.contains("💡 Подсказка:"));
        println!("Detailed:\n{}\n", detailed.message);
    }

    #[test]
    fn test_nonexistent_property_all_levels() {
        println!("\n=== Test 3.3.2: NonExistentProperty across all levels ===");

        let error = TypeErrorKind::NonExistentProperty {
            object_type: "ТаблицаЗначений".to_string(),
            property_name: "НесуществующееСвойство".to_string(),
            variable_name: Some("таблица".to_string()),
        };

        let span = Span::new(2, 5, 2, 15);

        let compact = error.to_diagnostic_with_detail(span, DetailLevel::Compact);
        let full = error.to_diagnostic_with_detail(span, DetailLevel::Full);
        let detailed = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

        // Compact: минимум
        assert!(!compact.message.contains("таблица"));

        // Full: с переменной
        assert!(full.message.contains("таблица"));

        // Detailed: с подсказкой
        assert!(detailed.message.contains("💡"));

        println!(
            "✅ Compact: {}\n✅ Full: {}\n✅ Detailed:\n{}",
            compact.message, full.message, detailed.message
        );
    }

    // ===== Test 3.4: Graceful degradation when variable_name is missing =====

    #[test]
    fn test_full_level_fallback_without_variable_name() {
        println!("\n=== Test 3.4.1: Full level fallback to Brief without variable ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "Метод".to_string(),
            variable_name: None,
        };

        let full = error.to_diagnostic_with_detail(Span::new(1, 0, 1, 10), DetailLevel::Full);
        let compact = error.to_diagnostic_with_detail(Span::new(1, 0, 1, 10), DetailLevel::Compact);

        // Full без variable_name должна fallback к Brief (как компакт)
        assert_eq!(full.message, compact.message);

        println!("✅ Full fallback message: {}", full.message);
    }

    #[test]
    fn test_detailed_level_still_works_without_variable_name() {
        println!("\n=== Test 3.4.2: Detailed level without variable_name ===");

        let error = TypeErrorKind::SimpleTypeAsCollection {
            type_name: "Строка".to_string(),
            operation: "Добавить".to_string(),
            variable_name: None,
        };

        let detailed = error.to_diagnostic_with_detail(Span::new(1, 0, 1, 10), DetailLevel::Detailed);

        // Detailed должна просто работать, fallback к Brief + подсказка
        assert!(detailed.message.contains("'Строка'"));
        assert!(detailed.message.contains("'Добавить'"));
        assert!(detailed.message.contains("💡")); // Подсказка должна быть

        println!("✅ Detailed message:\n{}", detailed.message);
    }

    // ===== Test 3.5: Span information preservation =====

    #[test]
    fn test_diagnostic_preserves_span_across_levels() {
        println!("\n=== Test 3.5: Span preservation across detail levels ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "Метод".to_string(),
            variable_name: None,
        };

        let span = Span::new(10, 25, 10, 35);

        let compact = error.to_diagnostic_with_detail(span, DetailLevel::Compact);
        let full = error.to_diagnostic_with_detail(span, DetailLevel::Full);
        let detailed = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

        // All should preserve span info
        assert_eq!(compact.line, 10);
        assert_eq!(compact.column, 25);
        assert_eq!(compact.end_line, 10);
        assert_eq!(compact.end_column, 35);

        assert_eq!(full.line, 10);
        assert_eq!(full.column, 25);
        assert_eq!(full.end_line, 10);
        assert_eq!(full.end_column, 35);

        assert_eq!(detailed.line, 10);
        assert_eq!(detailed.column, 25);
        assert_eq!(detailed.end_line, 10);
        assert_eq!(detailed.end_column, 35);

        println!("✅ Span preserved: line {}, col {}-{}", compact.line, compact.column, compact.end_column);
    }

    // ===== Test 3.6: Severity level consistency =====

    #[test]
    fn test_all_diagnostics_are_errors() {
        println!("\n=== Test 3.6: All diagnostics have Error severity ===");

        let errors = vec![
            TypeErrorKind::NonExistentMethod {
                object_type: "Массив".to_string(),
                method_name: "Метод".to_string(),
                variable_name: None,
            },
            TypeErrorKind::NonExistentProperty {
                object_type: "Объект".to_string(),
                property_name: "Свойство".to_string(),
                variable_name: None,
            },
            TypeErrorKind::IncorrectParameterType {
                method_name: "Функция".to_string(),
                param_index: 0,
                expected: "Число".to_string(),
                actual: "Строка".to_string(),
                variable_name: None,
                param_variable_name: None,
            },
            TypeErrorKind::SimpleTypeAsCollection {
                type_name: "Число".to_string(),
                operation: "Добавить".to_string(),
                variable_name: None,
            },
        ];

        let span = Span::new(1, 0, 1, 10);

        for error in errors {
            let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Full);
            assert_eq!(
                diagnostic.severity,
                DiagnosticSeverity::Error,
                "All TypeErrorKind should be Error severity"
            );
        }

        println!("✅ All diagnostics have Error severity");
    }

    // ===== Test 3.7: Message length expectations =====

    #[test]
    fn test_message_length_hierarchy() {
        println!("\n=== Test 3.7: Message length: Compact < Full < Detailed ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "ТаблицаЗначений".to_string(),
            method_name: "НесуществующийМетод".to_string(),
            variable_name: Some("переменная".to_string()),
        };

        let span = Span::new(1, 0, 1, 20);

        let compact = error.to_diagnostic_with_detail(span, DetailLevel::Compact);
        let full = error.to_diagnostic_with_detail(span, DetailLevel::Full);
        let detailed = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

        println!("Compact length: {}", compact.message.len());
        println!("Full length: {}", full.message.len());
        println!("Detailed length: {}", detailed.message.len());

        // Generally, detailed should be longest
        assert!(detailed.message.len() > full.message.len(), "Detailed should have hints");
        // Full should be >= Compact
        assert!(full.message.len() >= compact.message.len());

        println!("✅ Message length hierarchy OK");
    }

    // ===== Test 3.8: Russian language quality =====

    #[test]
    fn test_russian_text_quality() {
        println!("\n=== Test 3.8: Russian text quality ===");

        let errors = vec![
            TypeErrorKind::NonExistentMethod {
                object_type: "Массив".to_string(),
                method_name: "Метод".to_string(),
                variable_name: Some("переменная".to_string()),
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
                variable_name: Some("x".to_string()),
                param_variable_name: Some("y".to_string()),
            },
        ];

        let span = Span::new(1, 0, 1, 10);

        for error in errors {
            let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Detailed);

            // Check for Cyrillic characters
            let has_cyrillic = diagnostic.message
                .chars()
                .any(|c| c as u32 >= 0x0400 && c as u32 <= 0x04FF);

            assert!(has_cyrillic, "Message should contain Russian text");

            // Check for common Russian words
            let has_meaningful_content = diagnostic.message.contains("не")
                || diagnostic.message.contains("Не")
                || diagnostic.message.contains("типа")
                || diagnostic.message.contains("Подсказка");

            assert!(has_meaningful_content, "Message should be meaningful");
        }

        println!("✅ All messages have good Russian quality");
    }

    // ===== Test 3.9: Multiple errors scenario =====

    #[test]
    fn test_multiple_errors_at_different_locations() {
        println!("\n=== Test 3.9: Multiple errors at different locations ===");

        let errors = vec![
            (
                Span::new(1, 0, 1, 10),
                TypeErrorKind::NonExistentMethod {
                    object_type: "Массив".to_string(),
                    method_name: "Метод1".to_string(),
                    variable_name: Some("var1".to_string()),
                },
            ),
            (
                Span::new(3, 5, 3, 15),
                TypeErrorKind::NonExistentProperty {
                    object_type: "Объект".to_string(),
                    property_name: "Свойство".to_string(),
                    variable_name: Some("var2".to_string()),
                },
            ),
            (
                Span::new(5, 10, 5, 20),
                TypeErrorKind::IncorrectParameterType {
                    method_name: "Функция".to_string(),
                    param_index: 0,
                    expected: "Число".to_string(),
                    actual: "Строка".to_string(),
                    variable_name: Some("var3".to_string()),
                    param_variable_name: Some("param".to_string()),
                },
            ),
        ];

        for (span, error) in errors {
            let diagnostic = error.to_diagnostic_with_detail(span, DetailLevel::Full);

            assert_eq!(diagnostic.line, span.start_line);
            assert_eq!(diagnostic.column, span.start_column);
            assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
            assert!(!diagnostic.message.is_empty());
        }

        println!("✅ Multiple errors processed correctly");
    }

    // ===== Test 3.10: Backward compatibility =====

    #[test]
    fn test_old_api_still_works() {
        println!("\n=== Test 3.10: Old API (to_diagnostic) backward compatibility ===");

        let error = TypeErrorKind::NonExistentMethod {
            object_type: "Массив".to_string(),
            method_name: "Метод".to_string(),
            variable_name: Some("переменная".to_string()),
        };

        // Old API without detail level
        let diagnostic = error.to_diagnostic(Span::new(1, 0, 1, 10));

        // Should work and produce a valid diagnostic
        assert!(!diagnostic.message.is_empty());
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
        assert_eq!(diagnostic.line, 1);

        // Should default to Brief format (no variable)
        assert!(!diagnostic.message.contains("переменная"));

        println!("✅ Old API still works: {}", diagnostic.message);
    }
}
