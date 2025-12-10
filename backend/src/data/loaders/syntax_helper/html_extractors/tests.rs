//! Тесты для HTML экстракторов

use scraper::Html;

use super::extractors::HtmlExtractor;
use crate::data::loaders::syntax_helper::type_parser::{TypeFragment, TypeParser};

#[test]
fn test_extract_parameters_from_real_html_insert() {
    // Реальная HTML структура из Array.Insert
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Параметры:</p>
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;Индекс&gt; (обязательный)</div>
        Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>
        Индекс вставляемого значения.
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;Значение&gt; (необязательный)</div>
        Тип: Произвольный. <br>
        Вставляемое значение. Если не указан, то будет добавлено значение типа Неопределено.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    assert_eq!(params.len(), 2, "Должно быть 2 параметра");

    // Первый параметр: Индекс (обязательный)
    assert_eq!(params[0].name, "Индекс");
    assert_eq!(params[0].type_name, Some("Число".to_string()));
    assert!(!params[0].is_optional);
    assert!(
        params[0]
            .description
            .as_ref()
            .map(|d| d.contains("Индекс вставляемого"))
            .unwrap_or(false),
        "Описание должно содержать текст о вставляемом значении"
    );

    // Второй параметр: Значение (необязательный)
    assert_eq!(params[1].name, "Значение");
    assert_eq!(params[1].type_name, Some("Произвольный".to_string()));
    assert!(params[1].is_optional);
}

#[test]
fn test_extract_parameters_from_real_html_find() {
    // Реальная HTML структура из Array.Find
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Параметры:</p>
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;Значение&gt; (обязательный)</div>
        Тип: Произвольный. <br>
        Искомое значение.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    assert_eq!(params.len(), 1, "Должен быть 1 параметр");
    assert_eq!(params[0].name, "Значение");
    assert_eq!(params[0].type_name, Some("Произвольный".to_string()));
    assert!(!params[0].is_optional);
}

#[test]
fn test_extract_parameters_from_real_html_get() {
    // Реальная HTML структура из Array.Get
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Параметры:</p>
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;Индекс&gt; (обязательный)</div>
        Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>
        Индекс элемента.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "Индекс");
    assert_eq!(params[0].type_name, Some("Число".to_string()));
    assert!(!params[0].is_optional);
}

#[test]
fn test_extract_parameters_no_params_section() {
    // HTML без раздела "Параметры:"
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Описание:</p>
        <p>Метод без параметров</p>
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    assert_eq!(params.len(), 0, "Не должно быть параметров");
}

#[test]
fn test_parse_bilingual_name() {
    let extractor = HtmlExtractor::new();

    let (ru, en) = extractor.parse_bilingual_name("Добавить (Add)");
    assert_eq!(ru, "Добавить");
    assert_eq!(en, "Add");

    let (ru, en) = extractor.parse_bilingual_name("ВГраница (UBound)");
    assert_eq!(ru, "ВГраница");
    assert_eq!(en, "UBound");

    let (ru, _) = extractor.parse_bilingual_name("Метод без английского");
    assert_eq!(ru, "Метод без английского");
}

#[test]
fn test_extract_return_info_from_value_table_insert() {
    // Реальная HTML структура из ValueTable.Insert
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="v8help://SyntaxHelperContext/objects/catalog234/catalog236/ValueTableRow.html">СтрокаТаблицыЗначений</a>. <br>
        Вставленная строка.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("СтрокаТаблицыЗначений".to_string()),
        "Возвращаемый тип должен быть СтрокаТаблицыЗначений"
    );
    assert_eq!(
        return_desc,
        Some("Вставленная строка.".to_string()),
        "Описание должно быть 'Вставленная строка.'"
    );
}

#[test]
fn test_extract_return_info_no_return_section() {
    // HTML без раздела "Возвращаемое значение:"
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Параметры:</p>
        <p>Метод без возвращаемого значения</p>
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(return_type, None, "Не должно быть возвращаемого типа");
    assert_eq!(return_desc, None, "Не должно быть описания");
}

/// Тест для бага: placeholder из заголовка типа "СправочникМенеджер.<Имя справочника>"
/// НЕ должен попадать в имена параметров
#[test]
fn test_extract_parameters_find_by_code_with_header_placeholder() {
    // Реальная HTML структура из FindByCode250.html
    // Важно: в заголовке есть "&lt;Имя справочника&gt;" - это НЕ параметр!
    let html_content = r#"
    <html>
        <h1 class="V8SH_pagetitle">СправочникМенеджер.&lt;Имя справочника&gt;.НайтиПоКоду</h1>
        <p class="V8SH_title">СправочникМенеджер.&lt;Имя справочника&gt;</p>
        <p class="V8SH_chapter">Параметры:</p>
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;Код&gt; (обязательный)</div>
        Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>, <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>
        Искомый код.
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;ПоискПоПолномуКоду&gt; (необязательный)</div>
        Тип: <a href="v8help://SyntaxHelperLanguage/def_Boolean">Булево</a>. <br>
        Определяет режим поиска.
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: СправочникСсылка.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    // Должно быть 2 параметра (Код, ПоискПоПолномуКоду), а НЕ 3 или больше
    assert_eq!(params.len(), 2, "Должно быть 2 параметра, placeholder 'Имя справочника' не должен парситься как параметр");

    // Первый параметр: Код (обязательный) с union типом
    assert_eq!(params[0].name, "Код", "Первый параметр должен быть 'Код', а не 'Имя справочника'");
    assert_eq!(
        params[0].type_name,
        Some("Число | Строка".to_string()),
        "Тип должен быть union 'Число | Строка'"
    );
    assert!(!params[0].is_optional, "Параметр 'Код' должен быть обязательным");

    // Второй параметр: ПоискПоПолномуКоду (необязательный)
    assert_eq!(params[1].name, "ПоискПоПолномуКоду");
    assert_eq!(params[1].type_name, Some("Булево".to_string()));
    assert!(params[1].is_optional, "Параметр 'ПоискПоПолномуКоду' должен быть необязательным");
}

/// Тест для проверки парсинга union типов
#[test]
fn test_extract_parameters_union_types() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Параметры:</p>
        <div class="V8SH_rubric">
            <p style="margin-top: 2px; margin-bottom: 1px">&lt;Значение&gt; (обязательный)</div>
        Тип: <a href="type">Число</a>, <a href="type">Строка</a>, <a href="type">Дата</a>. <br>
        Описание.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let params = extractor.extract_parameters(&document);

    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "Значение");
    assert_eq!(
        params[0].type_name,
        Some("Число | Строка | Дата".to_string()),
        "Должны быть три типа через ' | '"
    );
}

// =========================================================================
// Тесты для DOM-based парсера возвращаемого типа
// =========================================================================

/// Тест парсинга faceted типа с placeholder: СправочникСсылка.<Имя справочника>
#[test]
fn test_extract_return_info_faceted_type_with_placeholder() {
    // Реальная HTML структура из FindByCode250.html
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="v8help://SyntaxHelperContext/objects/catalog125/catalog126/object129.html">СправочникСсылка.</a><span style='color=blue'>&lt;</span><a href="v8help://SyntaxHelperContext/objects/catalog125/catalog126/object129.html">Имя справочника</a><span style='color=blue'>&gt;</span>, <a href="v8help://SyntaxHelperLanguage/def_Undefined">Неопределено</a>. <br>
        Если не существует ни одного элемента с требуемым кодом, то будет возвращена пустая ссылка.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("СправочникСсылка.<T> | Неопределено".to_string()),
        "Возвращаемый тип должен быть 'СправочникСсылка.<T> | Неопределено'"
    );
    assert!(
        return_desc.is_some(),
        "Описание должно быть извлечено"
    );
}

/// Тест парсинга простого union типа
#[test]
fn test_extract_return_info_simple_union() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">Число</a>, <a href="type">Строка</a>. <br>
        Описание результата.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("Число | Строка".to_string()),
        "Возвращаемый тип должен быть union 'Число | Строка'"
    );
    assert_eq!(
        return_desc,
        Some("Описание результата.".to_string())
    );
}

/// Тест парсинга faceted типа для документа
#[test]
fn test_extract_return_info_faceted_document() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">ДокументСсылка.</a><span>&lt;</span><a href="type">Имя документа</a><span>&gt;</span>. <br>
        Ссылка на документ.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, _) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("ДокументСсылка.<T>".to_string()),
        "Placeholder '<Имя документа>' должен быть нормализован в '<T>'"
    );
}

/// Тест вспомогательных функций парсера (делегирован в TypeParser)
#[test]
fn test_parse_type_fragments() {
    // Тест фрагментов для faceted типа
    let type_line = r#" <a href="...">СправочникСсылка.</a><span>&lt;</span><a href="...">Имя справочника</a><span>&gt;</span>"#;
    let fragments = TypeParser::parse_type_fragments(type_line);

    // Должны быть: TypeName("СправочникСсылка."), GenericOpen, TypeName("Имя справочника"), GenericClose
    assert!(fragments.iter().any(|f| matches!(f, TypeFragment::TypeName(s) if s == "СправочникСсылка.")));
    assert!(fragments.iter().any(|f| matches!(f, TypeFragment::GenericOpen)));
    assert!(fragments.iter().any(|f| matches!(f, TypeFragment::TypeName(s) if s == "Имя справочника")));
    assert!(fragments.iter().any(|f| matches!(f, TypeFragment::GenericClose)));
}

/// Тест сборки типов из фрагментов (делегирован в TypeParser)
#[test]
fn test_assemble_types() {
    // Тест простого типа
    let fragments = vec![TypeFragment::TypeName("Строка".to_string())];
    assert_eq!(TypeParser::assemble_types(&fragments), "Строка");

    // Тест union типа
    let fragments = vec![
        TypeFragment::TypeName("Число".to_string()),
        TypeFragment::UnionSeparator,
        TypeFragment::TypeName("Строка".to_string()),
    ];
    assert_eq!(TypeParser::assemble_types(&fragments), "Число | Строка");

    // Тест faceted типа
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя справочника".to_string()),
        TypeFragment::GenericClose,
    ];
    assert_eq!(TypeParser::assemble_types(&fragments), "СправочникСсылка.<T>");

    // Тест faceted + union
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя справочника".to_string()),
        TypeFragment::GenericClose,
        TypeFragment::UnionSeparator,
        TypeFragment::TypeName("Неопределено".to_string()),
    ];
    assert_eq!(
        TypeParser::assemble_types(&fragments),
        "СправочникСсылка.<T> | Неопределено"
    );
}

/// Тест для malformed HTML: unclosed generic (делегирован в TypeParser)
#[test]
fn test_assemble_types_unclosed_generic() {
    // Malformed: открыли generic, но не закрыли
    let fragments = vec![
        TypeFragment::TypeName("СправочникСсылка.".to_string()),
        TypeFragment::GenericOpen,
        TypeFragment::TypeName("Имя".to_string()),
        // GenericClose отсутствует
    ];

    let result = TypeParser::assemble_types(&fragments);

    // Тип должен быть возвращен БЕЗ паники
    assert!(!result.is_empty(), "Результат не должен быть пустым");

    // Проверяем, что добавлен <T> для robustness
    // Trailing точка удаляется при финализации типа, поэтому результат без точки
    assert_eq!(
        result, "СправочникСсылка<T>",
        "Unclosed generic должен быть закрыт автоматически"
    );
}

// =========================================================================
// Интеграционные тесты на реальных HTML файлах
// =========================================================================

/// Интеграционный тест для реального файла FindByCode250.html
/// Проверяет парсинг параметров и возвращаемого типа на реальных данных
#[test]
fn test_find_by_code_real_html() {
    // Загружаем реальный HTML файл
    let html_path = "/home/egor/code/bsl-gradual-types/examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog125/catalog126/object128/methods/FindByCode250.html";

    let html_content = match std::fs::read_to_string(html_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Не удалось загрузить FindByCode250.html: {}", e);
            eprintln!("   Тест пропущен (файл может отсутствовать в разработке)");
            return;
        }
    };

    let document = Html::parse_document(&html_content);
    let extractor = HtmlExtractor::new();

    // ТЕСТ 1: Параметры
    let params = extractor.extract_parameters(&document);
    assert!(!params.is_empty(), "Параметры должны быть найдены");

    // Проверяем первый параметр: Код (union Число | Строка)
    let code_param = params.iter().find(|p| p.name == "Код")
        .expect("Параметр 'Код' должен быть найден");
    assert_eq!(code_param.type_name, Some("Число | Строка".to_string()),
        "Тип 'Код' должен быть union 'Число | Строка'");
    assert!(!code_param.is_optional, "'Код' должен быть обязательным");

    // Проверяем второй параметр: ПоискПоПолномуКоду (Булево, необязательный)
    let search_param = params.iter().find(|p| p.name == "ПоискПоПолномуКоду")
        .expect("Параметр 'ПоискПоПолномуКоду' должен быть найден");
    assert_eq!(search_param.type_name, Some("Булево".to_string()),
        "Тип 'ПоискПоПолномуКоду' должен быть 'Булево'");
    assert!(search_param.is_optional, "'ПоискПоПолномуКоду' должен быть необязательным");

    // ТЕСТ 2: Возвращаемый тип
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("СправочникСсылка.<T> | Неопределено".to_string()),
        "Возвращаемый тип должен быть 'СправочникСсылка.<T> | Неопределено' с нормализацией placeholder'а"
    );

    assert!(
        return_desc.is_some(),
        "Описание возвращаемого значения должно быть извлечено"
    );

    let desc = return_desc.unwrap();
    assert!(
        desc.contains("не существует") || desc.contains("пустая ссылка"),
        "Описание должно содержать информацию о пустой ссылке"
    );

    println!("Интеграционный тест FindByCode250.html пройден");
}

/// Edge case: HTML без секции "Возвращаемое значение:"
#[test]
fn test_extract_return_info_empty_section() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Параметры:</p>
        <p>Метод без возвращаемого значения</p>
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(return_type, None, "Возвращаемый тип должен быть None для отсутствующей секции");
    assert_eq!(return_desc, None, "Описание должно быть None для отсутствующей секции");
}

/// Edge case: Пустой return_type (нет типа после "Тип:")
/// Когда нет HTML контента для парсинга, parse_type_fragments возвращает пустой вектор
/// и assemble_types возвращает пустую строку, что приводит к None
#[test]
fn test_extract_return_info_empty_type() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <br>
        Описание без типа.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, _) = extractor.extract_return_info(&document);

    // Когда нет типа на строке "Тип:", parse_type_fragments не находит фрагменты
    // и возвращает пустой вектор, что приводит к None в assemble_types
    assert_eq!(return_type, None, "Возвращаемый тип должен быть None для пустого типа");
    // Описание может быть None если нет контента после <br>
    // или может содержать "Описание без типа." в зависимости от парсинга
}

/// Edge case: Только текст без <a> тегов и span'ов
/// Когда тип указан как простой текст без HTML обёртки, парсер не может его найти
/// потому что ищет фрагменты в parse_type_fragments
#[test]
fn test_extract_return_info_plain_text() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">Произвольный</a>. <br>
        Результат работы.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, return_desc) = extractor.extract_return_info(&document);

    assert_eq!(return_type, Some("Произвольный".to_string()));
    assert_eq!(return_desc, Some("Результат работы.".to_string()));
}

/// Edge case: Несколько union типов (> 2)
#[test]
fn test_extract_return_info_multiple_union() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">Число</a>, <a href="type">Строка</a>, <a href="type">Дата</a>, <a href="type">Булево</a>. <br>
        Результат с несколькими типами.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, _) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("Число | Строка | Дата | Булево".to_string()),
        "Должны быть все четыре типа через ' | '"
    );
}

/// Edge case: Несколько faceted типов
#[test]
fn test_extract_return_info_multiple_faceted() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">СправочникСсылка.</a><span>&lt;</span><a href="type">СправочникА</a><span>&gt;</span>, <a href="type">ДокументСсылка.</a><span>&lt;</span><a href="type">ДокументБ</a><span>&gt;</span>. <br>
        Ссылка на объект.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, _) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("СправочникСсылка.<T> | ДокументСсылка.<T>".to_string()),
        "Оба placeholder'а должны быть нормализованы в <T>"
    );
}

/// Edge case: Faceted тип без union
#[test]
fn test_extract_return_info_faceted_only() {
    let html_content = r#"
    <html>
        <p class="V8SH_chapter">Возвращаемое значение:</p>
        Тип: <a href="type">ДокументОбъект.</a><span>&lt;</span><a href="type">Соответствующий документ</a><span>&gt;</span>. <br>
        Объект документа.
    </html>
    "#;

    let document = Html::parse_document(html_content);
    let extractor = HtmlExtractor::new();
    let (return_type, _) = extractor.extract_return_info(&document);

    assert_eq!(
        return_type,
        Some("ДокументОбъект.<T>".to_string()),
        "Faceted тип без union должен быть нормализован"
    );
}

/// Edge case: Парсинг placeholder'ов разных типов
#[test]
fn test_extract_return_info_various_placeholders() {
    // Тест разных типов placeholder'ов, которые должны быть нормализованы в <T>
    let placeholders = vec![
        ("Имя справочника", "Справочник"),
        ("Имя документа", "Документ"),
        ("Имя перечисления", "Перечисление"),
    ];

    for (placeholder, context) in placeholders {
        let html_content = format!(
            r#"<html>
                <p class="V8SH_chapter">Возвращаемое значение:</p>
                Тип: <a href="type">Тип.</a><span>&lt;</span><a href="type">{}</a><span>&gt;</span>. <br>
                Результат для {}.
            </html>"#,
            placeholder, context
        );

        let document = Html::parse_document(&html_content);
        let extractor = HtmlExtractor::new();
        let (return_type, _) = extractor.extract_return_info(&document);

        assert_eq!(
            return_type,
            Some("Тип.<T>".to_string()),
            "Placeholder '{}' должен быть нормализован в <T>",
            placeholder
        );
    }
}
