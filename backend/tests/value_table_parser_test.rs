//! Тест парсинга параметров и возвращаемого типа для ТаблицаЗначений.Вставить
//!
//! Проверяет корректность извлечения:
//! - Параметр Индекс: Число (обязательный)
//! - Возвращаемое значение: СтрокаТаблицыЗначений

#![allow(clippy::bool_assert_comparison)]

use bsl_backend::data::loaders::syntax_helper::html_extractors::HtmlExtractor;
use scraper::Html;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_value_table_insert_parameters() {
    let mut html_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    html_path.pop(); // Выходим из backend
    html_path.push("examples");
    html_path.push("syntax_helper");
    html_path.push("rebuilt.shcntx_ru");
    html_path.push("objects");
    html_path.push("catalog234");
    html_path.push("catalog236");
    html_path.push("ValueTable");
    html_path.push("methods");
    html_path.push("Insert582.html");

    // Пропускаем тест если файл не существует
    if !html_path.exists() {
        eprintln!("⚠️  Пропуск теста: файл не найден {:?}", html_path);
        return;
    }

    let html_content = fs::read_to_string(&html_path).expect("Не удалось прочитать HTML файл");

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();

    // =========================================================================
    // Тест 1: Проверка параметров
    // =========================================================================
    let params = extractor.extract_parameters(&document);

    println!("📊 Извлечено параметров: {}", params.len());
    for (i, param) in params.iter().enumerate() {
        println!(
            "  [{}] {} : {:?} ({})",
            i,
            param.name,
            param.type_name,
            if param.is_optional {
                "необязательный"
            } else {
                "обязательный"
            }
        );
    }

    // ТаблицаЗначений.Вставить(Индекс)
    assert_eq!(params.len(), 1, "Должен быть 1 параметр");

    // Параметр: Индекс (обязательный, Число)
    assert_eq!(params[0].name, "Индекс");
    assert_eq!(params[0].type_name, Some("Число".to_string()));
    assert_eq!(params[0].is_optional, false);
    assert!(
        params[0]
            .description
            .as_ref()
            .map(|d| d.contains("Индекс вставляемой"))
            .unwrap_or(false),
        "Описание должно содержать текст об индексе вставляемой строки"
    );

    // =========================================================================
    // Тест 2: Проверка возвращаемого типа (через HtmlExtractor)
    // =========================================================================
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    println!("🔍 Возвращаемый тип: {:?}", return_type);
    println!("🔍 Описание возврата: {:?}", return_desc);

    assert_eq!(
        return_type,
        Some("СтрокаТаблицыЗначений".to_string()),
        "Возвращаемый тип должен быть СтрокаТаблицыЗначений"
    );

    assert!(
        return_desc
            .as_ref()
            .map(|d| d.contains("Вставленная строка"))
            .unwrap_or(false),
        "Описание должно содержать информацию о вставленной строке"
    );

    println!("✅ Парсинг параметров и возвращаемого типа работает корректно");
}

/// Извлекает возвращаемый тип из HTML вручную (временное решение)
fn extract_return_type_from_html(html: &str) -> Option<String> {
    use regex::Regex;

    // Ищем паттерн:
    // <p class="V8SH_chapter">Возвращаемое значение:</p>
    // Тип: <a href="...">СтрокаТаблицыЗначений</a>
    //
    // ВАЖНО: Используем greedy режим [^<]+ чтобы захватить весь текст до <
    let return_regex =
        Regex::new(r#"Возвращаемое значение:[\s\S]{0,200}?Тип:\s*(?:<a[^>]*>)?([^<]+)(?:</a>)?"#)
            .ok()?;

    if let Some(cap) = return_regex.captures(html) {
        let mut return_type = cap.get(1)?.as_str().trim().to_string();
        // Убираем точку в конце если есть
        if return_type.ends_with('.') {
            return_type.pop();
        }
        return Some(return_type.trim().to_string());
    }

    None
}

#[test]
fn test_extract_return_type_regex() {
    // Тест regex на реальном HTML фрагменте
    let html_fragment = r#"<p class="V8SH_chapter">Возвращаемое значение:</p>Тип: <a href="v8help://SyntaxHelperContext/objects/catalog234/catalog236/ValueTableRow.html">СтрокаТаблицыЗначений</a>. <br>Вставленная строка."#;

    let return_type = extract_return_type_from_html(html_fragment);

    assert_eq!(
        return_type,
        Some("СтрокаТаблицыЗначений".to_string()),
        "Должен извлечь СтрокаТаблицыЗначений из HTML фрагмента"
    );
}
