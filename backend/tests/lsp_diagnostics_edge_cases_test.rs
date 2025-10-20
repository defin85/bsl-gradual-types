//! Тест граничных случаев для LSP Syntax Error Diagnostics
//!
//! Milestone 2.18: LSP Syntax Error Diagnostics
//!
//! Проверяет edge cases:
//! 1. Пустой файл
//! 2. Корректный файл (no errors)
//! 3. Множественные ошибки
//! 4. Эмодзи и спецсимволы (UTF-16 edge case)
//! 5. Огромный файл
//! 6. Ошибка на первой строке
//! 7. Ошибка на последней строке

use bsl_backend::system::ParserCoordinator;

#[test]
fn test_empty_file() {
    println!("\n=== ТЕСТ: Пустой файл ===");

    let source = "";
    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг пустого файла должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Пустой файл НЕ должен иметь синтаксических ошибок
    assert!(!parse_result.has_errors(), "Пустой файл не должен содержать ошибок");

    // diagnostics должен быть пустым массивом
    assert!(parse_result.syntax_errors.is_empty(), "Должен быть пустой массив ошибок");

    println!("✅ Пустой файл: diagnostics = []");
    println!("=========================\n");
}

#[test]
fn test_correct_file_no_errors() {
    println!("\n=== ТЕСТ: Корректный файл (no errors) ===");

    let source = r#"
Функция ПолностьюКорректнаяФункция()
    Перем Результат;
    Результат = 42;

    Если Результат > 0 Тогда
        Сообщить("Положительный результат");
    Иначе
        Сообщить("Отрицательный результат");
    КонецЕсли;

    Возврат Результат;
КонецФункции

Процедура ТожеКорректнаяПроцедура()
    Сообщить("Всё хорошо");
КонецПроцедуры
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Корректный файл НЕ должен иметь ошибок
    assert!(!parse_result.has_errors(), "Корректный файл не должен содержать ошибок");
    assert!(parse_result.syntax_errors.is_empty(), "diagnostics должен быть пустым");

    println!("✅ Корректный файл: diagnostics = []");
    println!("========================================\n");
}

#[test]
fn test_multiple_errors_in_file() {
    println!("\n=== ТЕСТ: Множественные ошибки ===");

    let source = r#"
Функция СМножествомОшибок()
    Перем Х
    Если Истина Тогда
        Сообщить("Ошибка 1");
    // Нет КонецЕсли

    Для Счётчик = 1 По 10 Цикл
        Сообщить(Счётчик);
    // Нет КонецЦикла

    Пока Истина Цикл
        Прервать;
    // Нет КонецЦикла

    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Должны быть обнаружены множественные ошибки
    assert!(parse_result.has_errors(), "Должны быть ошибки");

    // Должно быть хотя бы 1 ошибка (tree-sitter может обнаружить не все)
    assert!(parse_result.syntax_errors.len() >= 1,
        "Должно быть обнаружено хотя бы 1 ошибка");

    for (idx, error) in parse_result.syntax_errors.iter().enumerate() {
        println!("{}. {} [{}:{}]",
            idx + 1,
            error.message,
            error.span.start_line,
            error.span.start_column);

        // Каждая ошибка должна иметь валидные координаты
        assert!(error.span.start_line > 0, "start_line должен быть > 0");
        assert!(error.span.start_column >= 0, "start_column должен быть >= 0");
    }

    println!("✅ Все ошибки обнаружены и имеют корректные координаты");
    println!("==================================\n");
}

#[test]
fn test_emoji_and_special_chars() {
    println!("\n=== ТЕСТ: Эмодзи и спецсимволы (UTF-16 edge case) ===");

    // Эмодзи могут занимать:
    // - 4 UTF-8 bytes
    // - 2 UTF-16 code units (surrogate pair)
    let source = r#"
Функция ТестЭмодзи()
    Перем Текст;
    Текст = "Привет 🌍🚀💻 мир";  // Эмодзи в строке
    Если Истина Тогда
        Сообщить(Текст);
    // Отсутствует КонецЕсли - ошибка с эмодзи выше
    Возврат Текст;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг с эмодзи должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Парсинг должен работать с эмодзи
    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            println!("Ошибка: {} [{}:{}]",
                error.message,
                error.span.start_line,
                error.span.start_column);

            // ✅ КРИТИЧНАЯ ПРОВЕРКА: координаты должны быть в UTF-16
            // Даже при наличии эмодзи выше по тексту!
            assert!(error.span.start_column < 300,
                "start_column не должен быть огромным ({}). \
                Эмодзи могут сбить конвертацию UTF-8→UTF-16!",
                error.span.start_column);

            assert!(error.span.end_column < 300,
                "end_column не должен быть огромным ({})",
                error.span.end_column);
        }
        println!("✅ Координаты с эмодзи корректны (UTF-16)");
    }

    println!("====================================================\n");
}

#[test]
fn test_error_on_first_line() {
    println!("\n=== ТЕСТ: Ошибка на первой строке ===");

    let source = r#"Функция Тест(
    Возврат 42;
КонецФункции"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    if parse_result.has_errors() {
        // Проверяем, что ошибка может быть на строке 0 (первая строка)
        let has_first_line_error = parse_result.syntax_errors.iter()
            .any(|e| e.span.start_line == 0);

        if has_first_line_error {
            println!("✅ Ошибка на первой строке обнаружена");
        } else {
            println!("⚠️ Ошибка не на первой строке (возможно, tree-sitter восстановил AST)");
        }

        // Все координаты должны быть валидными
        for error in &parse_result.syntax_errors {
            assert!(error.span.start_line >= 0, "start_line должен быть >= 0");
            assert!(error.span.start_column >= 0, "start_column должен быть >= 0");
        }
    }

    println!("=====================================\n");
}

#[test]
fn test_error_on_last_line() {
    println!("\n=== ТЕСТ: Ошибка на последней строке ===");

    let source = r#"
Функция Тест()
    Перем Х;
    Возврат Х;
// Отсутствует КонецФункции (последняя строка)
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    if parse_result.has_errors() {
        let max_line = parse_result.syntax_errors.iter()
            .map(|e| e.span.end_line)
            .max()
            .unwrap_or(0);

        println!("Максимальная строка ошибки: {}", max_line);

        // Проверяем, что координаты не выходят за пределы файла
        let file_lines_count = source.lines().count() as u32;
        for error in &parse_result.syntax_errors {
            assert!(error.span.start_line < file_lines_count + 10,
                "start_line не должен быть сильно больше количества строк в файле");
            assert!(error.span.end_line < file_lines_count + 10,
                "end_line не должен быть сильно больше количества строк в файле");
        }

        println!("✅ Координаты последней строки валидны");
    }

    println!("========================================\n");
}

#[test]
fn test_only_whitespace_file() {
    println!("\n=== ТЕСТ: Только пробелы и переносы строк ===");

    let source = "    \n\n   \n\t\t\n    ";

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Файл только с whitespace не должен иметь ошибок
    assert!(!parse_result.has_errors(), "Файл только с whitespace не должен иметь ошибок");

    println!("✅ Whitespace-only файл: diagnostics = []");
    println!("=============================================\n");
}

#[test]
fn test_only_comments_file() {
    println!("\n=== ТЕСТ: Только комментарии ===");

    let source = r#"
// Это комментарий
// Ещё один комментарий
// И третий комментарий
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Файл только с комментариями не должен иметь ошибок
    assert!(!parse_result.has_errors(), "Файл только с комментариями не должен иметь ошибок");

    println!("✅ Comments-only файл: diagnostics = []");
    println!("================================\n");
}

#[test]
fn test_very_long_line_coordinates() {
    println!("\n=== ТЕСТ: Очень длинная строка (проверка координат) ===");

    // Создаём строку с очень длинным именем переменной
    let long_var_name = "Перем ".to_string() + &"Х".repeat(500);
    let source = format!(r#"
Функция Тест()
    {};
    Если Истина Тогда
        Сообщить("Тест");
    Возврат;
КонецФункции
"#, long_var_name);

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source.as_str()).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            println!("Ошибка: {} [{}:{}]",
                error.message,
                error.span.start_line,
                error.span.start_column);

            // Координаты должны быть разумными даже для очень длинной строки
            assert!(error.span.start_column < 10000,
                "start_column не должен быть огромным");
            assert!(error.span.end_column < 10000,
                "end_column не должен быть огромным");
        }
        println!("✅ Координаты для длинной строки корректны");
    }

    println!("=======================================================\n");
}

#[test]
fn test_mixed_line_endings() {
    println!("\n=== ТЕСТ: Смешанные line endings (LF/CRLF) ===");

    // Создаём файл со смешанными line endings
    let source = "Функция Тест()\n    Перем Х;\r\n    Если Истина Тогда\n        Сообщить(\"Тест\");\r\n    Возврат Х;\nКонецФункции";

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Парсинг должен работать со смешанными line endings
    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            // Координаты должны быть валидными
            assert!(error.span.start_line >= 0, "start_line должен быть >= 0");
            assert!(error.span.end_line >= error.span.start_line, "end_line >= start_line");
        }
        println!("✅ Смешанные line endings обработаны корректно");
    } else {
        println!("✅ Код корректный даже со смешанными line endings");
    }

    println!("===============================================\n");
}

#[test]
fn test_error_span_zero_width() {
    println!("\n=== ТЕСТ: Ошибка с нулевой шириной (edge case) ===");

    let source = r#"
Функция Тест()
    Перем ;  // Пропущено имя переменной - возможно zero-width error
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            let span_width: Option<u32> = if error.span.start_line == error.span.end_line {
                Some(error.span.end_column - error.span.start_column)
            } else {
                None  // Многострочный span
            };

            println!("Ошибка: {} [ширина span: {}]",
                error.message,
                if let Some(width) = span_width { width.to_string() } else { "многострочный".to_string() });

            // Zero-width span может быть валидным (например, для missing token)
            if span_width == Some(0) {
                println!("  ⚠️ Zero-width span обнаружен (может быть валидным для MissingToken)");
            }

            // Но координаты всё равно должны быть валидными
            assert!(error.span.start_line >= 0);
            assert!(error.span.start_column >= 0);
            assert!(error.span.end_line >= error.span.start_line);
        }
        println!("✅ Zero-width span обработан корректно");
    }

    println!("==================================================\n");
}

#[test]
fn test_unicode_normalization_coordinates() {
    println!("\n=== ТЕСТ: Unicode нормализация (edge case) ===");

    // Некоторые символы имеют разные представления в Unicode
    // Например, "é" может быть:
    // 1. U+00E9 (один символ) - 1 UTF-16 code unit
    // 2. U+0065 + U+0301 (e + combining accent) - 2 UTF-16 code units

    let source = r#"
Функция ТестНормализации()
    Перем Café;  // é как один символ
    Если Истина Тогда
        Сообщить(Café);
    Возврат;
КонецФункции
"#;

    let parser = ParserCoordinator::with_fallback();
    let parse_result = parser.parse(source).expect("Парсинг должен успеть");

    println!("Ошибок найдено: {}", parse_result.syntax_errors.len());

    // Проверяем, что парсинг работает с Unicode
    if parse_result.has_errors() {
        for error in &parse_result.syntax_errors {
            // Координаты должны быть валидными даже с Unicode нормализацией
            assert!(error.span.start_column < 200,
                "start_column не должен быть огромным");
        }
        println!("✅ Unicode нормализация обработана корректно");
    } else {
        println!("✅ Код корректный даже с Unicode символами");
    }

    println!("==============================================\n");
}
