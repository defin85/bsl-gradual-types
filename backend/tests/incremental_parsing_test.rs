//! Тесты инкрементального парсинга

use bsl_backend::system::parser_coordinator::{ParserCoordinator, TextEdit};
use std::path::PathBuf;
use std::time::Instant;

fn utf16_position_at_byte(source: &str, byte_index: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut last_line_start = 0usize;

    for (idx, b) in source.as_bytes().iter().enumerate() {
        if idx >= byte_index {
            break;
        }
        if *b == b'\n' {
            line += 1;
            last_line_start = idx + 1;
        }
    }

    let col_utf16 = source[last_line_start..byte_index]
        .encode_utf16()
        .count() as u32;
    (line, col_utf16)
}

fn assert_incremental_matches_full(
    file_path: PathBuf,
    original_code: &str,
    modified_code: &str,
    edits: Vec<TextEdit>,
) {
    let coordinator = ParserCoordinator::with_fallback();
    coordinator
        .parse_incremental(file_path.clone(), original_code.to_string(), vec![])
        .expect("initial parse ok");

    let incremental = coordinator
        .parse_incremental(file_path, modified_code.to_string(), edits)
        .expect("incremental parse ok");

    let full = ParserCoordinator::with_fallback()
        .parse(modified_code)
        .expect("full parse ok");

    let incr_json = serde_json::to_value(&incremental).expect("serialize incremental");
    let full_json = serde_json::to_value(&full).expect("serialize full");
    assert_eq!(incr_json, full_json, "incremental parse must match full parse");
}

#[test]
fn test_incremental_simple_insertion() {
    // Исходный код
    let original_code = r#"
Функция Тест()
    Возврат 42;
КонецФункции
"#;

    // Вставка новой строки
    let modified_code = r#"
Функция Тест()
    Перем А;
    Возврат 42;
КонецФункции
"#;

    let file_path = PathBuf::from("test.bsl");

    // Инкрементальное обновление
    let edits = vec![TextEdit {
        start_line: 2,
        start_utf16_column: 4,
        old_end_line: 2,
        old_end_utf16_column: 4,
        new_text: "Перем А;\n    ".to_string(),
    }];

    assert_incremental_matches_full(file_path, original_code, modified_code, edits);
}

#[test]
fn test_incremental_deletion() {
    let original_code = r#"
Функция Тест()
    Перем А;
    Перем Б;
    Возврат А + Б;
КонецФункции
"#;

    let modified_code = r#"
Функция Тест()
    Перем Б;
    Возврат А + Б;
КонецФункции
"#;

    let file_path = PathBuf::from("test2.bsl");

    // Удаление строки "Перем А;"
    let edits = vec![TextEdit {
        start_line: 2,
        start_utf16_column: 4,
        old_end_line: 3,
        old_end_utf16_column: 4,
        new_text: "".to_string(),
    }];

    assert_incremental_matches_full(file_path, original_code, modified_code, edits);
}

#[test]
fn test_incremental_replacement() {
    let original_code = r#"Функция Старое() Возврат 1; КонецФункции"#;
    let modified_code = r#"Функция Новое() Возврат 2; КонецФункции"#;

    let file_path = PathBuf::from("test3.bsl");

    // Замена имени функции
    let edits = vec![TextEdit {
        start_line: 0,
        start_utf16_column: 8,
        old_end_line: 0,
        old_end_utf16_column: 14, // "Старое"
        new_text: "Новое".to_string(),
    }];

    assert_incremental_matches_full(file_path, original_code, modified_code, edits);
}

#[test]
fn test_incremental_performance_comparison() {
    let coordinator = ParserCoordinator::with_fallback();

    // Большой файл (1000 строк)
    let mut large_code = String::new();
    for i in 0..1000 {
        large_code.push_str(&format!(
            "Функция Функция{}()\n    Возврат {};\nКонецФункции\n\n",
            i, i
        ));
    }

    let file_path = PathBuf::from("large.bsl");

    // Полный парсинг
    let start_full = Instant::now();
    coordinator
        .parse_incremental(file_path.clone(), large_code.clone(), vec![])
        .unwrap();
    let duration_full = start_full.elapsed();

    println!("Полный парсинг: {:?}", duration_full);

    // Инкрементальное изменение (одна строка)
    let mut modified_code = large_code.clone();
    modified_code.push_str("Функция НоваяФункция()\n    Возврат 999;\nКонецФункции\n");

    let edits = vec![TextEdit {
        start_line: 3000,
        start_utf16_column: 0,
        old_end_line: 3000,
        old_end_utf16_column: 0,
        new_text: "Функция НоваяФункция()\n    Возврат 999;\nКонецФункции\n".to_string(),
    }];

    let start_incr = Instant::now();
    coordinator
        .parse_incremental(file_path, modified_code, edits)
        .unwrap();
    let duration_incr = start_incr.elapsed();

    println!("Инкрементальный парсинг: {:?}", duration_incr);

    // Инкрементальный парсинг должен быть быстрее
    // (на практике может быть не всегда, зависит от изменений)
    println!(
        "Ускорение: {:.2}x",
        duration_full.as_secs_f64() / duration_incr.as_secs_f64()
    );
}

#[test]
fn test_cache_reuse() {
    let coordinator = ParserCoordinator::with_fallback();

    let code = r#"Функция Тест() Возврат 42; КонецФункции"#;
    let file_path = PathBuf::from("cached.bsl");

    // Первый парсинг
    let start1 = Instant::now();
    coordinator
        .parse_incremental(file_path.clone(), code.to_string(), vec![])
        .unwrap();
    let duration1 = start1.elapsed();

    // Второй парсинг того же кода (должен использовать кеш)
    let start2 = Instant::now();
    coordinator
        .parse_incremental(file_path, code.to_string(), vec![])
        .unwrap();
    let duration2 = start2.elapsed();

    println!("Первый парсинг: {:?}", duration1);
    println!("Кешированный парсинг: {:?}", duration2);

    // Второй парсинг должен быть значительно быстрее
    assert!(
        duration2 < duration1,
        "Кешированный парсинг должен быть быстрее"
    );
}

#[test]
fn test_incremental_unicode_emoji_insertion_after_emoji_utf16() {
    let original_code = "Процедура Тест()\n    Сообщить(\"a😀b\");\nКонецПроцедуры\n";
    let modified_code = "Процедура Тест()\n    Сообщить(\"a😀Xb\");\nКонецПроцедуры\n";
    let file_path = PathBuf::from("unicode_emoji_insert.bsl");

    let marker = "\"a😀b\"";
    let start = original_code.find(marker).expect("marker exists");
    let after_emoji_byte = start + "\"".len() + "a😀".len();
    let (line, col_utf16) = utf16_position_at_byte(original_code, after_emoji_byte);

    let edits = vec![TextEdit {
        start_line: line,
        start_utf16_column: col_utf16,
        old_end_line: line,
        old_end_utf16_column: col_utf16,
        new_text: "X".to_string(),
    }];

    assert_incremental_matches_full(file_path, original_code, modified_code, edits);
}

#[test]
fn test_incremental_unicode_multiline_insert_with_cyrillic_tail() {
    let original_code = "Процедура Тест()\n    Возврат;\nКонецПроцедуры\n";
    let modified_code =
        "Процедура Тест()\n    Перем Я;\n    Возврат;\nКонецПроцедуры\n";
    let file_path = PathBuf::from("unicode_multiline_insert.bsl");

    let insert_at = original_code
        .find("    Возврат;")
        .expect("anchor exists");
    let (line, col_utf16) = utf16_position_at_byte(original_code, insert_at);

    let edits = vec![TextEdit {
        start_line: line,
        start_utf16_column: col_utf16,
        old_end_line: line,
        old_end_utf16_column: col_utf16,
        new_text: "    Перем Я;\n".to_string(),
    }];

    assert_incremental_matches_full(file_path, original_code, modified_code, edits);
}

#[test]
fn test_incremental_two_edits_with_line_shift() {
    let original_code = "Процедура Тест()\n    Сообщить(\"a😀b\");\n    Сообщить(\"ok\");\nКонецПроцедуры\n";
    let modified_code = "Процедура Тест()\n    Перем Значение;\n    Сообщить(\"a😀Xb\");\n    Сообщить(\"ok\");\nКонецПроцедуры\n";
    let file_path = PathBuf::from("unicode_two_edits.bsl");

    // Edit 1: insert a new line at the beginning of the body (line 1, column 0).
    let edits_line_insert = TextEdit {
        start_line: 1,
        start_utf16_column: 0,
        old_end_line: 1,
        old_end_utf16_column: 0,
        new_text: "    Перем Значение;\n".to_string(),
    };

    // Edit 2: insert after emoji in the (shifted) Сообщить line.
    let marker = "\"a😀b\"";
    let start = original_code.find(marker).expect("marker exists");
    let after_emoji_byte = start + "\"".len() + "a😀".len();
    let (orig_line, col_utf16) = utf16_position_at_byte(original_code, after_emoji_byte);
    let shifted_line = orig_line + 1;

    let edits_emoji_insert = TextEdit {
        start_line: shifted_line,
        start_utf16_column: col_utf16,
        old_end_line: shifted_line,
        old_end_utf16_column: col_utf16,
        new_text: "X".to_string(),
    };

    assert_incremental_matches_full(
        file_path,
        original_code,
        modified_code,
        vec![edits_line_insert, edits_emoji_insert],
    );
}
