//! Тест обнаружения синтаксических ошибок
//!
//! Проверяет, что tree-sitter парсер корректно обнаруживает синтаксические ошибки,
//! такие как незакрытые конструкции.

use bsl_backend::system::LineIndex;
use bsl_backend::system::ParserCoordinator;

fn utf16_position(index: &LineIndex, source: &str, byte_offset: u32) -> (u32, u32) {
    let point = index.byte_offset_to_point(source, byte_offset as usize);
    let utf16_column = index.byte_column_to_utf16(source, point.row, point.column);
    (point.row as u32, utf16_column)
}

#[test]
fn test_unclosed_if_statement_detected() {
    let parser = ParserCoordinator::with_fallback();

    // Код с незакрытым Если
    let source = r#"
Функция ТестНезакрытогоЕсли()
    Перем Результат;

    Если Истина Тогда
        Результат = 42;
    // Отсутствует КонецЕсли!

    Возврат Результат;
КонецФункции
"#;

    // Парсинг
    let parse_result = parser.parse(source);
    let index = LineIndex::new(source);

    println!("\n=== РЕЗУЛЬТАТ ПАРСИНГА ===");
    match &parse_result {
        Ok(result) => {
            println!("✅ Парсинг завершён");
            println!("Найдено ошибок: {}", result.syntax_errors.len());

            if result.has_errors() {
                println!("\n📋 Список синтаксических ошибок:");
                for (idx, error) in result.syntax_errors.iter().enumerate() {
                    let (start_line, start_column) =
                        utf16_position(&index, source, error.span.start);
                    let (end_line, end_column) = utf16_position(&index, source, error.span.end);
                    println!("\n{}. Ошибка #{}", idx + 1, idx + 1);
                    println!("   Тип: {:?}", error.error_type);
                    println!("   Сообщение: {}", error.message);
                    println!(
                        "   Позиция: строка {}, колонка {} - строка {}, колонка {}",
                        start_line, start_column, end_line, end_column
                    );
                }
            } else {
                println!("⚠️ ПРОБЛЕМА: Синтаксические ошибки не обнаружены!");
            }

            // Проверяем, что ошибки были обнаружены
            assert!(
                result.has_errors(),
                "Tree-sitter должен обнаружить синтаксическую ошибку (незакрытый Если)!"
            );
        }
        Err(e) => {
            println!("❌ Парсинг провалился: {}", e);
            panic!("Парсер должен возвращать ParseResult даже при ошибках (partial recovery)");
        }
    }
    println!("=========================\n");
}

#[test]
fn test_missing_endif_keyword() {
    let parser = ParserCoordinator::with_fallback();

    // Код с отсутствующим КонецЕсли
    let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Привет");

    Возврат Истина;
КонецФункции
"#;

    let parse_result = parser.parse(source).expect("Парсинг должен завершиться");

    println!("\n=== ТЕСТ: Отсутствует КонецЕсли ===");
    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            println!("- {}", error.message);
        }
    }

    assert!(
        parse_result.has_errors(),
        "Должны быть обнаружены синтаксические ошибки"
    );
    println!("==================================\n");
}

#[test]
fn test_valid_code_no_errors() {
    let parser = ParserCoordinator::with_fallback();

    // Корректный код без ошибок
    let source = r#"
Функция ТестКорректногоКода()
    Перем Результат;

    Если Истина Тогда
        Результат = 42;
    КонецЕсли;

    Возврат Результат;
КонецФункции
"#;

    let parse_result = parser.parse(source).expect("Парсинг должен завершиться");

    println!("\n=== ТЕСТ: Корректный код ===");
    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    assert!(
        !parse_result.has_errors(),
        "Корректный код НЕ должен иметь синтаксических ошибок"
    );
    println!("============================\n");
}

#[test]
fn test_multiple_syntax_errors() {
    let parser = ParserCoordinator::with_fallback();

    // Код с несколькими ошибками
    let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Ошибка 1");
    // Нет КонецЕсли

    Для Счётчик = 1 По 10 Цикл
        Сообщить(Счётчик);
    // Нет КонецЦикла

    Возврат;
КонецФункции
"#;

    let parse_result = parser.parse(source).expect("Парсинг должен завершиться");
    let index = LineIndex::new(source);

    println!("\n=== ТЕСТ: Множественные ошибки ===");
    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    for (idx, error) in parse_result.syntax_errors.iter().enumerate() {
        let (line, column) = utf16_position(&index, source, error.span.start);
        println!("{}. {} [{}:{}]", idx + 1, error.message, line, column);
    }

    assert!(
        parse_result.has_errors(),
        "Должны быть обнаружены множественные синтаксические ошибки"
    );
    println!("==================================\n");
}
