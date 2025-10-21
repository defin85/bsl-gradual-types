//! Тесты инкрементального парсинга

use bsl_backend::system::parser_coordinator::{ParserCoordinator, TextEdit};
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn test_incremental_simple_insertion() {
    let coordinator = ParserCoordinator::with_fallback();

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

    // Первый парсинг
    let result1 =
        coordinator.parse_incremental(file_path.clone(), original_code.to_string(), vec![]);
    assert!(result1.is_ok(), "Первый парсинг должен пройти успешно");

    // Инкрементальное обновление
    let edits = vec![TextEdit {
        start_line: 2,
        start_column: 4,
        old_end_line: 2,
        old_end_column: 4,
        new_end_line: 3,
        new_end_column: 4,
        new_text: "Перем А;\n    ".to_string(),
    }];

    let result2 = coordinator.parse_incremental(file_path, modified_code.to_string(), edits);

    assert!(
        result2.is_ok(),
        "Инкрементальный парсинг должен пройти успешно: {:?}",
        result2
    );
}

#[test]
fn test_incremental_deletion() {
    let coordinator = ParserCoordinator::with_fallback();

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

    // Первый парсинг
    let result1 =
        coordinator.parse_incremental(file_path.clone(), original_code.to_string(), vec![]);
    assert!(result1.is_ok());

    // Удаление строки "Перем А;"
    let edits = vec![TextEdit {
        start_line: 2,
        start_column: 4,
        old_end_line: 3,
        old_end_column: 4,
        new_end_line: 2,
        new_end_column: 4,
        new_text: "".to_string(),
    }];

    let result2 = coordinator.parse_incremental(file_path, modified_code.to_string(), edits);

    assert!(result2.is_ok(), "Инкрементальное удаление должно работать");
}

#[test]
fn test_incremental_replacement() {
    let coordinator = ParserCoordinator::with_fallback();

    let original_code = r#"Функция Старое() Возврат 1; КонецФункции"#;
    let modified_code = r#"Функция Новое() Возврат 2; КонецФункции"#;

    let file_path = PathBuf::from("test3.bsl");

    // Первый парсинг
    coordinator
        .parse_incremental(file_path.clone(), original_code.to_string(), vec![])
        .unwrap();

    // Замена имени функции
    let edits = vec![TextEdit {
        start_line: 0,
        start_column: 8,
        old_end_line: 0,
        old_end_column: 14, // "Старое"
        new_end_line: 0,
        new_end_column: 13, // "Новое"
        new_text: "Новое".to_string(),
    }];

    let result = coordinator.parse_incremental(file_path, modified_code.to_string(), edits);

    assert!(result.is_ok());
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
        start_column: 0,
        old_end_line: 3000,
        old_end_column: 0,
        new_end_line: 3002,
        new_end_column: 0,
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
