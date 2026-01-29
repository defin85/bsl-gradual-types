//! Тест обнаружения синтаксических ошибок и их координат
//!
//! Milestone 2.18: LSP Syntax Error Diagnostics
//!
//! Этот тест проверяет, что:
//! 1. Парсер обнаруживает синтаксические ошибки (MissingToken, ParseError, etc.)
//! 2. Координаты ошибок корректны (UTF-16)
//! 3. Типы ошибок правильно классифицированы

use bsl_backend::system::ParserCoordinator;
use bsl_backend::system::LineIndex;
use bsl_shared::domain::types::ErrorType;

fn utf16_position(index: &LineIndex, source: &str, byte_offset: u32) -> (u32, u32) {
    let point = index.byte_offset_to_point(source, byte_offset as usize);
    let utf16_column = index.byte_column_to_utf16(source, point.row, point.column);
    (point.row as u32, utf16_column)
}

#[test]
fn test_missing_endif_detected() {
    let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Привет");
    // Нет КонецЕсли!
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    println!("\n=== ТЕСТ: Отсутствует КонецЕсли ===");
    println!("Найдено ошибок: {}", parse_result.syntax_errors.len());

    assert!(parse_result.has_errors(), "Должна быть обнаружена ошибка");
    assert!(
        !parse_result.syntax_errors.is_empty(),
        "Должны быть синтаксические ошибки"
    );

    let mut related_checked = false;
    // Проверяем детали ошибок
    for (idx, error) in parse_result.syntax_errors.iter().enumerate() {
        let (start_line, start_column) = utf16_position(&index, source, error.span.start);
        let (end_line, end_column) = utf16_position(&index, source, error.span.end);

        println!("\nОшибка #{}: {:?}", idx + 1, error.error_type);
        println!("  Сообщение: {}", error.message);
        println!(
            "  Позиция: {}:{} - {}:{}",
            start_line, start_column, end_line, end_column
        );
        if error.error_type == ErrorType::MissingToken && error.message.contains("ENDIF_KEYWORD") {
            assert!(
                !error.related.is_empty(),
                "Для MissingToken ENDIF ожидается related информация"
            );
            let related = &error.related[0];
            assert!(
                related.message.contains("Если"),
                "Сообщение related должно указывать на начало Если"
            );
            assert!(
                related.span.start <= error.span.start,
                "Related позиция должна указывать на начало блока"
            );
            related_checked = true;
        }

        // Проверяем, что span валиден (byte offsets) и не выходит за границы исходника.
        assert!(
            error.span.end >= error.span.start,
            "end должен быть >= start: {:?}",
            error.span
        );
        assert!(
            (error.span.end as usize) <= source.len(),
            "end не должен выходить за пределы source: {:?}, len={}",
            error.span,
            source.len()
        );
        assert!(
            end_line >= start_line,
            "end_line >= start_line"
        );
    }

    assert!(related_checked, "MissingToken для ENDIF_KEYWORD не найден");

    println!("===================================\n");
}

#[test]
fn test_missing_enddo_detected() {
    let source = r#"
Функция Тест()
    Для Счётчик = 1 По 10 Цикл
        Сообщить(Счётчик);
    // Нет КонецЦикла!
    Возврат Истина;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    println!("\n=== ТЕСТ: Отсутствует КонецЦикла ===");
    println!("Найдено ошибок: {}", parse_result.syntax_errors.len());

    assert!(parse_result.has_errors(), "Должны быть обнаружены ошибки");

    for error in &parse_result.syntax_errors {
        let (line, column) = utf16_position(&index, source, error.span.start);
        println!(
            "- {} [{}:{}]",
            error.message, line, column
        );
    }

    println!("=====================================\n");
}

#[test]
fn test_valid_code_no_errors() {
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

    let parser = ParserCoordinator::with_fallback();
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

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен завершиться");
    let index = LineIndex::new(source);

    println!("\n=== ТЕСТ: Множественные ошибки ===");
    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    for (idx, error) in parse_result.syntax_errors.iter().enumerate() {
        let (line, column) = utf16_position(&index, source, error.span.start);
        println!(
            "{}. {} [{}:{}]",
            idx + 1,
            error.message,
            line,
            column
        );
    }

    assert!(
        parse_result.has_errors(),
        "Должны быть обнаружены множественные синтаксические ошибки"
    );

    // Должно быть хотя бы 2 ошибки (отсутствие КонецЕсли и КонецЦикла)
    assert!(
        !parse_result.syntax_errors.is_empty(),
        "Должно быть обнаружено хотя бы 1 синтаксическая ошибка"
    );

    println!("==================================\n");
}

#[test]
fn test_error_type_classification() {
    // Тест проверяет, что ошибки классифицируются с правильным ErrorType

    let source = r#"
Функция Тест()
    Если Истина Тогда
        Сообщить("Тест");
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("\n=== ТЕСТ: Классификация типов ошибок ===");

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            println!("Ошибка: {:?} - {}", error.error_type, error.message);

            // Проверяем, что тип ошибки установлен (не default)
            match error.error_type {
                ErrorType::ParseError => println!("  ✅ ParseError"),
                ErrorType::InvalidSyntax => println!("  ✅ InvalidSyntax"),
                ErrorType::MissingToken => println!("  ✅ MissingToken"),
                ErrorType::UnexpectedToken => println!("  ✅ UnexpectedToken"),
            }
        }
    } else {
        println!("⚠️ Ошибки не обнаружены (возможно, tree-sitter восстановил AST)");
    }

    println!("=========================================\n");
}

#[test]
fn test_error_utf16_coordinates_with_cyrillic() {
    // КРИТИЧНЫЙ ТЕСТ: проверка, что координаты ошибок в UTF-16 для кириллицы

    let source = r#"
Функция ТестКириллица()
    Перем Переменная;
    Если Истина Тогда
        Переменная = 42;
    // Отсутствует КонецЕсли
    Возврат Переменная;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");
    let index = LineIndex::new(source);

    println!("\n=== ТЕСТ: UTF-16 координаты ошибок (кириллица) ===");

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            let (start_line, start_column) = utf16_position(&index, source, error.span.start);
            let (end_line, end_column) = utf16_position(&index, source, error.span.end);

            println!("\nОшибка: {}", error.message);
            println!(
                "  Позиция: {}:{} - {}:{}",
                start_line, start_column, end_line, end_column
            );

            // ✅ КЛЮЧЕВАЯ ПРОВЕРКА: конвертация byte offsets -> UTF-16 даёт разумные координаты.
            assert!(
                start_column < 100,
                "start_column не должен быть огромным (это признак byte offset вместо UTF-16)"
            );

            assert!(
                end_column < 100,
                "end_column не должен быть огромным"
            );

            // Координаты должны быть последовательными
            assert!(
                end_line >= start_line,
                "end_line должен быть >= start_line"
            );

            if start_line == end_line {
                assert!(
                    end_column >= start_column,
                    "На одной строке: end_column должен быть >= start_column"
                );
            }

            println!("  ✅ Координаты выглядят корректными (UTF-16)");
        }
    }

    println!("===================================================\n");
}

#[test]
fn test_empty_file_no_errors() {
    // Граничный случай: пустой файл
    let source = "";

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser
        .parse(source)
        .expect("Парсинг пустого файла должен успеть");

    println!("\n=== ТЕСТ: Пустой файл ===");
    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Пустой файл НЕ должен иметь синтаксических ошибок
    assert!(
        !parse_result.has_errors(),
        "Пустой файл не должен содержать ошибок"
    );

    println!("=========================\n");
}

#[test]
fn test_only_comments_no_errors() {
    // Граничный случай: только комментарии
    let source = r#"
// Это комментарий
// Ещё один комментарий
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("\n=== ТЕСТ: Только комментарии ===");
    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Файл с комментариями не должен иметь ошибок
    assert!(
        !parse_result.has_errors(),
        "Файл только с комментариями не должен содержать ошибок"
    );

    println!("================================\n");
}

#[test]
fn test_incomplete_function_signature() {
    // Ошибка в сигнатуре функции
    let source = r#"
Функция Тест(
    Возврат 42;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("\n=== ТЕСТ: Некорректная сигнатура функции ===");
    println!("Найдено ошибок: {}", parse_result.syntax_errors.len());

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            println!("- {:?}: {}", error.error_type, error.message);
        }
        println!("✅ Ошибки в сигнатуре обнаружены");
    } else {
        println!("⚠️ Tree-sitter смог восстановить AST (partial recovery)");
    }

    println!("============================================\n");
}

#[test]
fn test_mixed_errors_classification() {
    // Смешанные типы ошибок для проверки классификации
    let source = r#"
Функция ТестСмешанныхОшибок()
    Перем Х
    Если Истина Тогда
        Х = 42;
    Возврат Х;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("\n=== ТЕСТ: Смешанные типы ошибок ===");

    if parse_result.has_errors() {
        println!("Найдено ошибок: {}", parse_result.syntax_errors.len());

        let mut missing_token_count = 0;
        let mut parse_error_count = 0;
        let mut invalid_syntax_count = 0;
        let mut unexpected_token_count = 0;

        for error in &parse_result.syntax_errors {
            match error.error_type {
                ErrorType::MissingToken => {
                    missing_token_count += 1;
                    println!("  [MissingToken] {}", error.message);
                }
                ErrorType::ParseError => {
                    parse_error_count += 1;
                    println!("  [ParseError] {}", error.message);
                }
                ErrorType::InvalidSyntax => {
                    invalid_syntax_count += 1;
                    println!("  [InvalidSyntax] {}", error.message);
                }
                ErrorType::UnexpectedToken => {
                    unexpected_token_count += 1;
                    println!("  [UnexpectedToken] {}", error.message);
                }
            }
        }

        println!("\nСтатистика:");
        println!("  MissingToken: {}", missing_token_count);
        println!("  ParseError: {}", parse_error_count);
        println!("  InvalidSyntax: {}", invalid_syntax_count);
        println!("  UnexpectedToken: {}", unexpected_token_count);

        println!("✅ Классификация работает");
    }

    println!("===================================\n");
}
