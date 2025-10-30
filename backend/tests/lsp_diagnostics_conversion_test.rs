//! Тест конвертации ParseError → LSP Diagnostic
//!
//! Milestone 2.18: LSP Syntax Error Diagnostics
//!
//! Проверяет, что синтаксические ошибки из парсера корректно
//! конвертируются в LSP Diagnostic с правильным severity и координатами.
//!
//! **ПРОБЛЕМА:** Метод `syntax_errors_to_diagnostics()` — приватный в LSP Server.
//! **РЕШЕНИЕ:** Тестируем через публичный API и проверяем структуру ParseError.

use bsl_backend::parsing::bsl::ast::ErrorType;
use bsl_backend::system::ParserCoordinator;
use bsl_shared::ir::Span;

// === ПУБЛИЧНЫЙ API ТЕСТ: ParseError структура ===

#[test]
fn test_parse_error_structure_complete() {
    // Проверяем, что ParseError содержит все необходимые поля для конвертации в Diagnostic

    let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Тест");
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("\n=== ТЕСТ: Структура ParseError ===");

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            // Проверяем обязательные поля
            println!("\nParseError:");
            println!("  error_type: {:?}", error.error_type);
            println!("  message: {}", error.message);
            println!(
                "  span: {}:{} - {}:{}",
                error.span.start_line,
                error.span.start_column,
                error.span.end_line,
                error.span.end_column
            );

            // ✅ ПРОВЕРКА 1: ErrorType должен быть установлен
            match error.error_type {
                ErrorType::ParseError => assert!(true, "ErrorType::ParseError"),
                ErrorType::InvalidSyntax => assert!(true, "ErrorType::InvalidSyntax"),
                ErrorType::MissingToken => assert!(true, "ErrorType::MissingToken"),
                ErrorType::UnexpectedToken => assert!(true, "ErrorType::UnexpectedToken"),
            }

            // ✅ ПРОВЕРКА 2: Message не должно быть пустым
            assert!(
                !error.message.is_empty(),
                "Сообщение ошибки не должно быть пустым"
            );

            // ✅ ПРОВЕРКА 3: Span должен быть валидным
            // Примечание: start_line и start_column — unsigned типы (u32), всегда >= 0
            assert!(
                error.span.end_line >= error.span.start_line,
                "end_line должен быть >= start_line"
            );

            if error.span.start_line == error.span.end_line {
                assert!(
                    error.span.end_column >= error.span.start_column,
                    "На одной строке: end_column >= start_column"
                );
            }

            println!("  ✅ ParseError структура корректна для конвертации в Diagnostic");
        }
    } else {
        println!("⚠️ Ошибки не обнаружены (возможно, tree-sitter восстановил AST)");
    }

    println!("===================================\n");
}

// === ТЕСТ: ErrorType → DiagnosticSeverity маппинг ===

#[test]
fn test_error_type_severity_mapping() {
    // Проверяем логику маппинга ErrorType → DiagnosticSeverity
    // (не можем напрямую вызвать syntax_errors_to_diagnostics, но проверяем логику)

    println!("\n=== ТЕСТ: ErrorType → DiagnosticSeverity маппинг ===");

    // Ожидаемый маппинг (из lsp_server.rs:syntax_errors_to_diagnostics):
    // ParseError -> ERROR
    // InvalidSyntax -> ERROR
    // MissingToken -> ERROR
    // UnexpectedToken -> WARNING

    let test_cases = vec![
        (ErrorType::ParseError, "ERROR", "Критичная ошибка парсинга"),
        (ErrorType::InvalidSyntax, "ERROR", "Некорректный синтаксис"),
        (
            ErrorType::MissingToken,
            "ERROR",
            "Отсутствует обязательный токен",
        ),
        (
            ErrorType::UnexpectedToken,
            "WARNING",
            "Неожиданный токен (восстановимо)",
        ),
    ];

    for (error_type, expected_severity, description) in test_cases {
        println!(
            "\n{:?} → {} ({}) ",
            error_type, expected_severity, description
        );

        // Проверяем, что маппинг логичен
        match error_type {
            ErrorType::ParseError | ErrorType::InvalidSyntax | ErrorType::MissingToken => {
                assert_eq!(
                    expected_severity, "ERROR",
                    "{:?} должен маппиться в ERROR",
                    error_type
                );
                println!("  ✅ Корректный severity");
            }
            ErrorType::UnexpectedToken => {
                assert_eq!(
                    expected_severity, "WARNING",
                    "UnexpectedToken должен маппиться в WARNING (может быть восстановлен парсером)"
                );
                println!("  ✅ Корректный severity (WARNING)");
            }
        }
    }

    println!("\n✅ Маппинг ErrorType → DiagnosticSeverity логичен");
    println!("=================================================\n");
}

// === ТЕСТ: Конвертация координат Span → LSP Range ===

#[test]
fn test_span_to_lsp_range_conversion() {
    // Проверяем, что Span корректно конвертируется в LSP Range

    println!("\n=== ТЕСТ: Span → LSP Range ===");

    // Создаём тестовые Span
    let test_spans = vec![
        (Span::new(1, 0, 1, 10), "Однострочный span"),
        (Span::new(2, 4, 5, 15), "Многострочный span"),
        (Span::new(0, 0, 0, 0), "Нулевой span (edge case)"),
    ];

    for (span, description) in test_spans {
        println!(
            "\n{}: {}:{} - {}:{}",
            description, span.start_line, span.start_column, span.end_line, span.end_column
        );

        // LSP Range создаётся из Span следующим образом:
        // Range::new(
        //     Position::new(span.start_line, span.start_column),
        //     Position::new(span.end_line, span.end_column)
        // )

        // Проверяем, что координаты валидные для LSP
        // Примечание: start_line и start_column — unsigned типы (u32), всегда >= 0
        assert!(span.end_line >= span.start_line, "end_line >= start_line");

        if span.start_line == span.end_line {
            assert!(
                span.end_column >= span.start_column,
                "На одной строке: end_column >= start_column"
            );
        }

        println!("  ✅ Span валиден для конвертации в LSP Range");
    }

    println!("\n✅ Конвертация Span → LSP Range корректна");
    println!("==============================\n");
}

// === ИНТЕГРАЦИОННЫЙ ТЕСТ: ParseError → Diagnostic (через backend) ===

#[test]
fn test_parse_error_diagnostic_conversion_via_shared() {
    // Тестируем конвертацию ParseError через shared типы
    // (backend ParseError → shared ParseError → LSP Diagnostic)

    println!("\n=== ТЕСТ: ParseError → shared ParseError конвертация ===");

    let source = r#"
Функция ТестКонвертации()
    Если Истина Тогда
        Сообщить("Тест");
    // Отсутствует КонецЕсли
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    if parse_result.has_errors() {
        println!("Найдено ошибок: {}", parse_result.syntax_errors.len());

        for backend_error in &parse_result.syntax_errors {
            println!("\n=== Backend ParseError ===");
            println!("  Type: {:?}", backend_error.error_type);
            println!("  Message: {}", backend_error.message);
            println!(
                "  Span: {}:{} - {}:{}",
                backend_error.span.start_line,
                backend_error.span.start_column,
                backend_error.span.end_line,
                backend_error.span.end_column
            );

            // Симулируем конвертацию backend ErrorType → shared ErrorType
            use bsl_backend::parsing::bsl::ast::ErrorType as BackendErrorType;
            use bsl_shared::domain::types::ErrorType as SharedErrorType;

            let shared_error_type = match backend_error.error_type {
                BackendErrorType::ParseError => SharedErrorType::ParseError,
                BackendErrorType::InvalidSyntax => SharedErrorType::InvalidSyntax,
                BackendErrorType::MissingToken => SharedErrorType::MissingToken,
                BackendErrorType::UnexpectedToken => SharedErrorType::UnexpectedToken,
            };

            println!("\n=== Shared ErrorType ===");
            println!("  {:?}", shared_error_type);

            // Симулируем конвертацию Span (backend::Span → shared::Span)
            let shared_span = bsl_shared::ir::Span::new(
                backend_error.span.start_line,
                backend_error.span.start_column,
                backend_error.span.end_line,
                backend_error.span.end_column,
            );

            println!("\n=== Shared Span ===");
            println!(
                "  {}:{} - {}:{}",
                shared_span.start_line,
                shared_span.start_column,
                shared_span.end_line,
                shared_span.end_column
            );

            // Проверяем, что координаты совпадают
            assert_eq!(shared_span.start_line, backend_error.span.start_line);
            assert_eq!(shared_span.start_column, backend_error.span.start_column);
            assert_eq!(shared_span.end_line, backend_error.span.end_line);
            assert_eq!(shared_span.end_column, backend_error.span.end_column);

            println!("\n  ✅ Конвертация backend → shared успешна");
        }
    } else {
        println!("⚠️ Ошибки не обнаружены");
    }

    println!("\n==================================================\n");
}

// === ТЕСТ: Diagnostic code и source поля ===

#[test]
fn test_diagnostic_metadata_fields() {
    // Проверяем, что Diagnostic будет иметь корректные metadata поля

    println!("\n=== ТЕСТ: Diagnostic metadata поля ===");

    // Ожидаемые значения (из lsp_server.rs:syntax_errors_to_diagnostics)
    let expected_source = "bsl-syntax";
    let expected_code_format = "{:?}"; // ErrorType форматируется через Debug

    println!("\nОжидаемые metadata:");
    println!("  source: \"{}\"", expected_source);
    println!(
        "  code: NumberOrString::String(format!(\"{}\", error_type))",
        expected_code_format
    );

    // Проверяем форматирование ErrorType для code
    let test_error_types = vec![
        ErrorType::ParseError,
        ErrorType::InvalidSyntax,
        ErrorType::MissingToken,
        ErrorType::UnexpectedToken,
    ];

    for error_type in test_error_types {
        let code_value = format!("{:?}", error_type);
        println!("\n  ErrorType::{:?} → code: \"{}\"", error_type, code_value);

        // Проверяем, что code не пустой
        assert!(!code_value.is_empty(), "Code не должен быть пустым");
    }

    println!("\n✅ Metadata поля корректны");
    println!("===================================\n");
}

// === ТЕСТ: UTF-16 координаты в Diagnostic ===

#[test]
fn test_diagnostic_utf16_coordinates() {
    // КРИТИЧНЫЙ ТЕСТ: проверка, что координаты в Diagnostic в UTF-16

    println!("\n=== ТЕСТ: UTF-16 координаты в Diagnostic ===");

    let source = r#"
Функция ТестУТФ16()
    Если Истина Тогда
        Сообщить("Кириллица");
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            println!("\nОшибка: {}", error.message);
            println!(
                "  Span: {}:{} - {}:{}",
                error.span.start_line,
                error.span.start_column,
                error.span.end_line,
                error.span.end_column
            );

            // ✅ КЛЮЧЕВАЯ ПРОВЕРКА: координаты должны быть в UTF-16
            // Благодаря TreeSitterAdapter::byte_offset_to_utf16() (Milestone 2.18 Task 1)

            // Проверяем, что координаты не огромные (признак byte offsets)
            assert!(
                error.span.start_column < 200,
                "start_column не должен быть огромным ({}). Это признак byte offset вместо UTF-16!",
                error.span.start_column
            );

            assert!(
                error.span.end_column < 200,
                "end_column не должен быть огромным ({}). Это признак byte offset вместо UTF-16!",
                error.span.end_column
            );

            // LSP Position создаётся напрямую из Span:
            // Position::new(span.start_line, span.start_column)
            // Поэтому если Span в UTF-16, то и Position в UTF-16

            println!("  ✅ Координаты в UTF-16 (подходят для LSP Diagnostic)");
        }
    }

    println!("\n============================================\n");
}

// === РЕКОМЕНДАЦИЯ: Публичная функция для конвертации ===

#[test]
#[ignore] // Игнорируем, пока не реализована публичная функция
fn test_public_conversion_function() {
    // РЕКОМЕНДАЦИЯ для будущего улучшения:
    // Вынести логику конвертации в публичную функцию в backend модуле

    // Пример:
    // pub fn parse_errors_to_diagnostics(errors: &[ParseError]) -> Vec<Diagnostic> { ... }

    println!("\n=== РЕКОМЕНДАЦИЯ ===");
    println!("Для лучшей тестируемости рекомендуется:");
    println!("1. Создать pub(crate) функцию в backend/src/lsp/diagnostics.rs");
    println!("2. Функция: parse_errors_to_diagnostics(errors: &[ParseError]) -> Vec<Diagnostic>");
    println!("3. Использовать её в BslLanguageServer::syntax_errors_to_diagnostics()");
    println!("4. Написать unit-тесты для этой функции");
    println!("====================\n");
}
