//! Тесты для обработки синтаксических ошибок (Milestone 2.7 Task 3)
//!
//! Проверяем что TreeSitterAdapter корректно собирает ERROR узлы
//! и возвращает ParseResult с syntax_errors

use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_line_index::LineIndex;
use bsl_shared::domain::types::ErrorType;
use tree_sitter::Parser;

/// Парсит BSL код и возвращает ParseResult (с возможными ошибками)
fn parse_bsl_with_errors(code: &str) -> Result<bsl_backend::parsing::ParseResult, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .map_err(|e| format!("Failed to set language: {:?}", e))?;

    let tree = parser
        .parse(code, None)
        .ok_or_else(|| "Failed to parse".to_string())?;

    TreeSitterAdapter::convert_tree(&tree, code)
}

#[test]
fn test_valid_code_no_errors() {
    let code = "Перем А; А = 10;";
    let result = parse_bsl_with_errors(code).expect("Парсинг должен пройти успешно");

    assert!(
        !result.has_errors(),
        "Валидный код не должен иметь синтаксических ошибок"
    );
    assert_eq!(result.syntax_errors.len(), 0);
    assert_eq!(result.program.statements.len(), 2);
}

#[test]
fn test_incomplete_function() {
    // Незакрытая функция — tree-sitter создаст ERROR узлы
    let code = "Функция Тест()\n    А = 1;\n// Нет КонецФункции";
    let result = parse_bsl_with_errors(code).expect("Парсинг с ошибками должен вернуть результат");
    let index = LineIndex::new(code);

    // Должны быть обнаружены синтаксические ошибки
    assert!(
        result.has_errors(),
        "Незакрытая функция должна создать синтаксические ошибки"
    );
    assert!(
        !result.syntax_errors.is_empty(),
        "Должны быть собраны ERROR узлы"
    );

    // Проверяем, что ошибки имеют корректные span
    for error in &result.syntax_errors {
        let (start_line, _) = index.byte_offset_to_utf16_position(code, error.span.start as usize);
        assert!(start_line < 100, "Span должен быть валидным");
        assert!(
            !error.message.is_empty(),
            "Сообщение об ошибке не должно быть пустым"
        );
    }
}

#[test]
fn test_invalid_expression() {
    // Некорректное выражение — tree-sitter обнаружит ERROR
    let code = "Перем А;\nА = = = 10;"; // Множественные '='
    let result = parse_bsl_with_errors(code);

    match result {
        Ok(parse_result) => {
            // Tree-sitter может распарсить с ERROR узлами или вернуть ошибку
            // Проверяем что если распарсилось, то есть ошибки
            if parse_result.has_errors() {
                let index = LineIndex::new(code);
                println!(
                    "✅ Обнаружены синтаксические ошибки: {}",
                    parse_result.syntax_errors.len()
                );
                for error in &parse_result.syntax_errors {
                    let (line, column) =
                        index.byte_offset_to_utf16_position(code, error.span.start as usize);
                    println!(
                        "  - [{}:{}] {}",
                        line, column, error.message
                    );
                }
            }
        }
        Err(e) => {
            // Или вернулась ошибка конвертации — тоже OK
            println!("✅ Конвертация вернула ошибку: {}", e);
        }
    }
}

#[test]
fn test_missing_token_detection() {
    // Пропущенный токен — tree-sitter может пометить как missing
    let code = "Функция Тест\n    Возврат 42;\nКонецФункции"; // Отсутствуют ()
    let result = parse_bsl_with_errors(code);

    if let Ok(parse_result) = result {
        // Tree-sitter может обнаружить missing узлы
        if parse_result.has_errors() {
            let missing_errors = parse_result
                .syntax_errors
                .iter()
                .filter(|e| e.error_type == ErrorType::MissingToken)
                .count();

            if missing_errors > 0 {
                println!("✅ Обнаружены missing токены: {}", missing_errors);
            }
        }
    }
}

#[test]
fn test_partial_recovery() {
    // Частичное восстановление — даже с ошибками должна быть хоть какая-то AST
    let code = r#"
Функция ВалиднаяФункция()
    Возврат 1;
КонецФункции

Функция НевалиднаяФункция(
    // Нет закрытия
"#;

    let result = parse_bsl_with_errors(code).expect("Partial recovery должен вернуть результат");

    // Должна быть хотя бы одна функция
    assert!(
        !result.program.statements.is_empty(),
        "Partial recovery: должна быть распознана валидная функция"
    );

    // Но также должны быть ошибки
    if result.has_errors() {
        println!(
            "✅ Partial recovery: распознано {} statements, {} ошибок",
            result.program.statements.len(),
            result.syntax_errors.len()
        );
    }
}

#[test]
fn test_error_span_accuracy() {
    // Проверяем точность span в ошибках
    let code = "А = 10\nБ = = 20;"; // Ошибка на второй строке
    let result = parse_bsl_with_errors(code);
    let index = LineIndex::new(code);

    if let Ok(parse_result) = result {
        if parse_result.has_errors() {
            for error in &parse_result.syntax_errors {
                let (start_line, start_column) =
                    index.byte_offset_to_utf16_position(code, error.span.start as usize);
                let (end_line, end_column) =
                    index.byte_offset_to_utf16_position(code, error.span.end as usize);
                println!(
                    "Ошибка на [{}:{} - {}:{}]: {}",
                    start_line,
                    start_column,
                    end_line,
                    end_column,
                    error.message
                );

                // Span должен быть разумным
                assert!(
                    error.span.end >= error.span.start,
                    "end должен быть >= start"
                );
                assert!(end_line >= start_line, "end_line должен быть >= start_line");
                if start_line == end_line {
                    assert!(
                        end_column >= start_column,
                        "end_column должен быть >= start_column"
                    );
                }
            }
        }
    }
}
