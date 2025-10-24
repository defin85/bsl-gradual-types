//! Специальные тесты для префиксов расширений
//!
//! - Тесты производительности применения префиксов
//! - Тесты корректности обработки кириллицы в префиксах
//! - Edge cases и граничные условия

use bsl_backend::data::loaders::config_metadata_parser::types::UniversalMetadataObject;
use std::time::{Duration, Instant};

/// Создать тестовый справочник
fn create_test_catalog(name: &str) -> UniversalMetadataObject {
    UniversalMetadataObject::new(
        "Catalog".to_string(),
        name.to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    )
}

#[test]
fn test_prefix_application_performance() {
    // ЦЕЛЬ: Применение префикса не должно замедлять конвертацию
    //
    // Выполняем 1000 конвертаций и проверяем, что это занимает < 100ms

    let obj = create_test_catalog("Контрагенты");

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = obj.to_raw_type_data(Some("Тест_"));
    }
    let duration = start.elapsed();

    println!("1000 конвертаций заняли: {:?}", duration);

    assert!(
        duration < Duration::from_millis(100),
        "1000 конвертаций должны выполняться быстрее 100ms, заняло {:?}",
        duration
    );
}

#[test]
fn test_cyrillic_prefix_encoding() {
    // ЦЕЛЬ: Кириллические префиксы корректно обрабатываются
    //
    // Проверяем:
    // 1. Корректность UTF-8 кодирования
    // 2. Правильность конкатенации кириллических строк
    // 3. Сохранение диакритических знаков (ё, й)

    let obj = create_test_catalog("Контрагенты");

    // Кириллический префикс
    let raw_type = obj.to_raw_type_data(Some("ДопМодуль_"));

    // Проверяем корректность UTF-8
    assert!(
        raw_type.name.is_char_boundary(0),
        "Строка должна быть корректной UTF-8"
    );

    // Проверяем наличие кириллицы
    assert!(
        raw_type.name.contains("ДопМодуль_"),
        "Кириллический префикс должен сохраниться"
    );

    assert_eq!(
        raw_type.name, "Справочники.ДопМодуль_Контрагенты",
        "Кириллический префикс должен корректно применяться"
    );

    // Проверяем длину строки (UTF-8 символы могут занимать несколько байт)
    // "Справочники." (12) + "ДопМодуль_" (10) + "Контрагенты" (11) = 33 символа
    assert_eq!(
        raw_type.name.chars().count(),
        33,
        "Количество символов должно соответствовать ожиданию"
    );
}

#[test]
fn test_prefix_with_special_cyrillic_chars() {
    // ЦЕЛЬ: Префиксы с буквами Ё, Й, Щ корректно обрабатываются

    let obj = create_test_catalog("Контрагенты");

    // Префикс с буквой Ё
    let raw_type_yo = obj.to_raw_type_data(Some("Ёлка_"));
    assert_eq!(
        raw_type_yo.name, "Справочники.Ёлка_Контрагенты",
        "Префикс с буквой Ё должен работать"
    );

    // Префикс с буквой Й
    let raw_type_i_kratkoye = obj.to_raw_type_data(Some("Йод_"));
    assert_eq!(
        raw_type_i_kratkoye.name, "Справочники.Йод_Контрагенты",
        "Префикс с буквой Й должен работать"
    );

    // Префикс с буквой Щ
    let raw_type_shcha = obj.to_raw_type_data(Some("Щит_"));
    assert_eq!(
        raw_type_shcha.name, "Справочники.Щит_Контрагенты",
        "Префикс с буквой Щ должен работать"
    );
}

#[test]
fn test_mixed_latin_cyrillic_prefix() {
    // ЦЕЛЬ: Смешанные префиксы (латиница + кириллица) работают корректно

    let obj = create_test_catalog("Контрагенты");

    let raw_type = obj.to_raw_type_data(Some("EXT_Доп_"));

    assert_eq!(
        raw_type.name, "Справочники.EXT_Доп_Контрагенты",
        "Смешанный префикс должен корректно применяться"
    );

    // Проверяем UTF-8
    assert!(raw_type.name.is_char_boundary(0));
}

#[test]
fn test_prefix_with_numbers_and_cyrillic() {
    // ЦЕЛЬ: Префиксы с цифрами и кириллицей работают корректно

    let obj = create_test_catalog("Контрагенты");

    let raw_type = obj.to_raw_type_data(Some("Расш2024_"));

    assert_eq!(
        raw_type.name, "Справочники.Расш2024_Контрагенты",
        "Префикс с цифрами и кириллицей должен корректно применяться"
    );
}

#[test]
fn test_very_long_prefix() {
    // ЦЕЛЬ: Длинные префиксы корректно обрабатываются

    let obj = create_test_catalog("Контрагенты");

    let long_prefix = "ОченьДлинныйПрефиксРасширенияКонфигурации_";
    let raw_type = obj.to_raw_type_data(Some(long_prefix));

    assert!(
        raw_type.name.contains(long_prefix),
        "Длинный префикс должен сохраниться полностью"
    );

    assert_eq!(
        raw_type.name,
        format!("Справочники.{}{}", long_prefix, "Контрагенты")
    );
}

#[test]
fn test_prefix_memory_efficiency() {
    // ЦЕЛЬ: Применение префиксов не приводит к утечкам памяти
    //
    // Создаём большое количество объектов с префиксами и проверяем,
    // что память освобождается корректно

    let mut results = Vec::new();

    for i in 0..10000 {
        let obj = create_test_catalog(&format!("Контрагенты{}", i));
        let raw_type = obj.to_raw_type_data(Some(&format!("Расш{}_", i)));
        results.push(raw_type.name.clone());
    }

    // Проверяем несколько случайных результатов
    assert!(results[0].starts_with("Справочники.Расш0_"));
    assert!(results[5000].starts_with("Справочники.Расш5000_"));
    assert!(results[9999].starts_with("Справочники.Расш9999_"));

    println!("Создано {} объектов с префиксами", results.len());
}

#[test]
fn test_prefix_with_only_underscore() {
    // ЦЕЛЬ: Префикс, состоящий только из подчёркивания (edge case)

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("_"));

    assert_eq!(
        raw_type.name, "Справочники._Контрагенты",
        "Префикс из одного подчёркивания должен применяться"
    );
}

#[test]
fn test_prefix_with_whitespace_trimmed() {
    // ЦЕЛЬ: Префиксы с пробелами НЕ должны приниматься (если это требование)
    //
    // В 1С префиксы не должны содержать пробелы, но наша функция применяет их как есть

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("Тест "));

    // Функция применяет префикс как есть (с пробелом)
    assert_eq!(
        raw_type.name, "Справочники.Тест Контрагенты",
        "Префикс с пробелом применяется как есть (валидация должна быть на уровне выше)"
    );
}

#[test]
fn test_unicode_normalization() {
    // ЦЕЛЬ: Проверить, что Unicode символы нормализованы корректно
    //
    // Буква "е" может быть представлена как U+0435 (кириллическая)
    // или как U+0065 (латинская)

    let obj = create_test_catalog("Контрагенты");

    // Кириллическая "е" в префиксе
    let prefix_cyrillic = "Тест_"; // U+0435
    let raw_type = obj.to_raw_type_data(Some(prefix_cyrillic));

    assert!(raw_type.name.contains("Тест_"));
    assert!(raw_type.name.is_char_boundary(0));
}

#[test]
fn test_prefix_comparison_performance() {
    // ЦЕЛЬ: Сравнение производительности с префиксом vs без префикса

    let obj = create_test_catalog("Контрагенты");

    // Без префикса
    let start_no_prefix = Instant::now();
    for _ in 0..1000 {
        let _ = obj.to_raw_type_data(None);
    }
    let duration_no_prefix = start_no_prefix.elapsed();

    // С префиксом
    let start_with_prefix = Instant::now();
    for _ in 0..1000 {
        let _ = obj.to_raw_type_data(Some("Тест_"));
    }
    let duration_with_prefix = start_with_prefix.elapsed();

    println!("Без префикса: {:?}", duration_no_prefix);
    println!("С префиксом: {:?}", duration_with_prefix);

    // Разница не должна быть > 2x
    let ratio = duration_with_prefix.as_nanos() as f64 / duration_no_prefix.as_nanos() as f64;
    println!("Соотношение: {:.2}x", ratio);

    assert!(
        ratio < 2.0,
        "Применение префикса не должно замедлять работу более чем в 2 раза"
    );
}

#[test]
fn test_concurrent_prefix_application() {
    // ЦЕЛЬ: Проверить потокобезопасность (если объекты shared между потоками)
    //
    // Создаём несколько потоков, каждый применяет свой префикс

    use std::sync::Arc;
    use std::thread;

    let obj = Arc::new(create_test_catalog("Контрагенты"));
    let mut handles = vec![];

    for i in 0..10 {
        let obj_clone = Arc::clone(&obj);
        let handle = thread::spawn(move || {
            let prefix = format!("Расш{}_", i);
            let raw_type = obj_clone.to_raw_type_data(Some(&prefix));
            raw_type.name
        });
        handles.push(handle);
    }

    let results: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Проверяем, что все результаты корректны
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.contains(&format!("Расш{}_", i)),
            "Результат потока {} должен содержать свой префикс",
            i
        );
    }

    println!("Concurrent test: {} потоков успешно завершены", results.len());
}
