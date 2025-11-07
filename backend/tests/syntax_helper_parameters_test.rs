//! Интеграционные тесты для извлечения параметров методов из реальных HTML файлов синтакс-помощника
//!
//! Проверяет корректность парсинга реальной HTML структуры с div.V8SH_rubric

use bsl_backend::data::loaders::syntax_helper::html_extractors::HtmlExtractor;
use scraper::Html;
use std::fs;
use std::path::PathBuf;

/// Получить путь к примерам синтакс-помощника
fn get_syntax_helper_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Выходим из backend
    path.push("examples");
    path.push("syntax_helper");
    path.push("rebuilt.shcntx_ru");
    path.push("objects");
    path.push("catalog234");
    path.push("Array");
    path.push("methods");
    path
}

#[test]
fn test_extract_parameters_from_array_insert() {
    let html_path = get_syntax_helper_path().join("Insert771.html");

    // Пропускаем тест если файл не существует
    if !html_path.exists() {
        eprintln!("⚠️  Пропуск теста: файл не найден {:?}", html_path);
        return;
    }

    let html_content = fs::read_to_string(&html_path)
        .expect("Не удалось прочитать HTML файл");

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    // Массив.Вставить(Индекс, Значение)
    assert_eq!(params.len(), 2, "Должно быть 2 параметра");

    // Первый параметр: Индекс (обязательный, Число)
    assert_eq!(params[0].name, "Индекс");
    assert_eq!(params[0].type_name, Some("Число".to_string()));
    assert_eq!(params[0].is_optional, false);
    assert!(params[0].description.is_some(), "Должно быть описание");

    // Второй параметр: Значение (необязательный, Произвольный)
    assert_eq!(params[1].name, "Значение");
    assert_eq!(params[1].type_name, Some("Произвольный".to_string()));
    assert_eq!(params[1].is_optional, true);
}

#[test]
fn test_extract_parameters_from_array_find() {
    let html_path = get_syntax_helper_path().join("Find2748.html");

    if !html_path.exists() {
        eprintln!("⚠️  Пропуск теста: файл не найден {:?}", html_path);
        return;
    }

    let html_content = fs::read_to_string(&html_path)
        .expect("Не удалось прочитать HTML файл");

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    // Массив.Найти(Значение)
    assert_eq!(params.len(), 1, "Должен быть 1 параметр");

    assert_eq!(params[0].name, "Значение");
    assert_eq!(params[0].type_name, Some("Произвольный".to_string()));
    assert_eq!(params[0].is_optional, false);
}

#[test]
fn test_extract_parameters_from_array_get() {
    let html_path = get_syntax_helper_path().join("Get1389.html");

    if !html_path.exists() {
        eprintln!("⚠️  Пропуск теста: файл не найден {:?}", html_path);
        return;
    }

    let html_content = fs::read_to_string(&html_path)
        .expect("Не удалось прочитать HTML файл");

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    // Массив.Получить(Индекс)
    assert_eq!(params.len(), 1, "Должен быть 1 параметр");

    assert_eq!(params[0].name, "Индекс");
    assert_eq!(params[0].type_name, Some("Число".to_string()));
    assert_eq!(params[0].is_optional, false);
}

#[test]
fn test_extract_parameters_from_array_add() {
    let html_path = get_syntax_helper_path().join("Add772.html");

    if !html_path.exists() {
        eprintln!("⚠️  Пропуск теста: файл не найден {:?}", html_path);
        return;
    }

    let html_content = fs::read_to_string(&html_path)
        .expect("Не удалось прочитать HTML файл");

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    // Массив.Добавить(Значение)
    assert_eq!(params.len(), 1, "Должен быть 1 параметр");

    assert_eq!(params[0].name, "Значение");
    assert_eq!(params[0].type_name, Some("Произвольный".to_string()));
    assert_eq!(params[0].is_optional, true, "Параметр должен быть необязательным");
}

#[test]
fn test_extract_parameters_from_array_count() {
    let html_path = get_syntax_helper_path().join("Count769.html");

    if !html_path.exists() {
        eprintln!("⚠️  Пропуск теста: файл не найден {:?}", html_path);
        return;
    }

    let html_content = fs::read_to_string(&html_path)
        .expect("Не удалось прочитать HTML файл");

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    // Массив.Количество() - метод без параметров
    assert_eq!(params.len(), 0, "Не должно быть параметров");
}
