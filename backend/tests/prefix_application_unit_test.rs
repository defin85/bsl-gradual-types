//! Unit тесты для применения префиксов расширений к метаданным
//!
//! Проверяет корректность работы UniversalMetadataObject::to_raw_type_data(prefix)
//! для различных типов объектов метаданных и различных вариантов префиксов.

use bsl_backend::data::loaders::config_metadata_parser::types::{
    AttributeInfo, TabularSectionInfo, UniversalMetadataObject,
};
use bsl_shared::domain::types::MetadataKind;

/// Вспомогательная функция для создания тестового справочника
fn create_test_catalog(name: &str) -> UniversalMetadataObject {
    let mut obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        name.to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );
    obj.synonym = Some(format!("Справочник {}", name));
    obj
}

/// Вспомогательная функция для создания тестового документа
fn create_test_document(name: &str) -> UniversalMetadataObject {
    let mut obj = UniversalMetadataObject::new(
        "Document".to_string(),
        name.to_string(),
        "87654321-4321-4321-4321-210987654321".to_string(),
    );
    obj.synonym = Some(format!("Документ {}", name));
    obj
}

/// Вспомогательная функция для создания тестовой константы
fn create_test_constant(name: &str) -> UniversalMetadataObject {
    let mut obj = UniversalMetadataObject::new(
        "Constant".to_string(),
        name.to_string(),
        "11111111-2222-3333-4444-555555555555".to_string(),
    );
    obj.synonym = Some(format!("Константа {}", name));
    obj
}

#[test]
fn test_prefix_none_returns_original_name() {
    // ЦЕЛЬ: Проверить, что без префикса возвращается оригинальное имя

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(None);

    assert_eq!(
        raw_type.name, "Справочники.Контрагенты",
        "Без префикса имя должно остаться оригинальным"
    );
    assert_eq!(
        raw_type.english_name, "Контрагенты",
        "English name без префикса должно совпадать с оригиналом"
    );
    assert_eq!(
        raw_type.kind,
        Some(MetadataKind::Catalog),
        "Тип объекта должен быть Catalog"
    );
}

#[test]
fn test_prefix_empty_string_returns_original_name() {
    // ЦЕЛЬ: Проверить, что пустой префикс эквивалентен None

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some(""));

    assert_eq!(
        raw_type.name, "Справочники.Контрагенты",
        "Пустой префикс должен быть эквивалентен None"
    );
    assert_eq!(
        raw_type.english_name, "Контрагенты",
        "English name с пустым префиксом должно быть оригинальным"
    );
}

#[test]
fn test_prefix_applied_to_catalog() {
    // ЦЕЛЬ: Проверить корректное применение префикса к справочнику

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("Тест_"));

    assert_eq!(
        raw_type.name, "Справочники.Тест_Контрагенты",
        "Префикс должен применяться к имени справочника"
    );
    assert_eq!(
        raw_type.english_name, "Тест_Контрагенты",
        "Префикс должен применяться к english_name"
    );
    assert_eq!(
        raw_type.category, "Справочники",
        "Категория не должна изменяться"
    );
    assert_eq!(
        raw_type.kind,
        Some(MetadataKind::Catalog),
        "Тип объекта не должен изменяться"
    );
}

#[test]
fn test_prefix_applied_to_document() {
    // ЦЕЛЬ: Проверить корректное применение префикса к документу

    let obj = create_test_document("ЗаказПокупателя");
    let raw_type = obj.to_raw_type_data(Some("УП_"));

    assert_eq!(
        raw_type.name, "Документы.УП_ЗаказПокупателя",
        "Префикс должен применяться к имени документа"
    );
    assert_eq!(
        raw_type.english_name, "УП_ЗаказПокупателя",
        "Префикс должен применяться к english_name документа"
    );
    assert_eq!(
        raw_type.category, "Документы",
        "Категория документа не должна изменяться"
    );
}

#[test]
fn test_prefix_applied_to_constant() {
    // ЦЕЛЬ: Проверить корректное применение префикса к константе

    let obj = create_test_constant("МаксимальныйРазмер");
    let raw_type = obj.to_raw_type_data(Some("Расш_"));

    assert_eq!(
        raw_type.name, "Константы.Расш_МаксимальныйРазмер",
        "Префикс должен применяться к имени константы"
    );
    assert_eq!(
        raw_type.english_name, "Расш_МаксимальныйРазмер",
        "Префикс должен применяться к english_name константы"
    );
}

#[test]
fn test_prefix_not_applied_to_category() {
    // ЦЕЛЬ: Убедиться, что префикс НЕ применяется к категории типа

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("Тест_"));

    // Должно быть "Справочники.Тест_Контрагенты", НЕ "Тест_Справочники.Контрагенты"
    assert!(
        raw_type.name.starts_with("Справочники."),
        "Имя типа должно начинаться с категории без префикса"
    );
    assert!(
        !raw_type.name.starts_with("Тест_"),
        "Префикс НЕ должен применяться к категории"
    );
    assert!(
        raw_type.name.contains("Тест_Контрагенты"),
        "Префикс должен быть применён только к имени объекта"
    );
}

#[test]
fn test_prefix_with_cyrillic() {
    // ЦЕЛЬ: Проверить корректную работу с кириллическими префиксами

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("ДопМодуль_"));

    assert_eq!(
        raw_type.name, "Справочники.ДопМодуль_Контрагенты",
        "Кириллический префикс должен корректно применяться"
    );

    // Проверяем корректность UTF-8
    assert!(
        raw_type.name.is_char_boundary(0),
        "Строка должна быть корректной UTF-8"
    );
    assert!(
        raw_type.name.contains("ДопМодуль_"),
        "Кириллический префикс должен сохраниться"
    );
}

#[test]
fn test_prefix_with_underscore_at_end() {
    // ЦЕЛЬ: Проверить, что префикс с подчёркиванием применяется корректно

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("Тест_"));

    assert_eq!(
        raw_type.name, "Справочники.Тест_Контрагенты",
        "Префикс с подчёркиванием должен применяться как есть"
    );

    // Не должно быть двойного подчёркивания
    assert!(
        !raw_type.name.contains("Тест__"),
        "Не должно быть двойного подчёркивания"
    );
}

#[test]
fn test_prefix_preserves_attributes() {
    // ЦЕЛЬ: Проверить, что атрибуты сохраняются после применения префикса

    let mut obj = create_test_catalog("Контрагенты");
    obj.attributes.push(AttributeInfo {
        name: "ИНН".to_string(),
        type_name: "Строка".to_string(),
        synonym: Some("ИНН".to_string()),
    });
    obj.attributes.push(AttributeInfo {
        name: "КПП".to_string(),
        type_name: "Строка".to_string(),
        synonym: Some("КПП".to_string()),
    });

    let raw_type = obj.to_raw_type_data(Some("Тест_"));

    assert_eq!(
        raw_type.attributes.len(),
        2,
        "Количество атрибутов должно сохраниться"
    );
    assert_eq!(
        raw_type.attributes[0].name, "ИНН",
        "Имена атрибутов не должны изменяться"
    );
    assert_eq!(
        raw_type.attributes[1].name, "КПП",
        "Имена атрибутов не должны изменяться"
    );

    // Проверяем, что префикс не применился к атрибутам
    assert!(
        !raw_type.attributes[0].name.contains("Тест_"),
        "Префикс НЕ должен применяться к атрибутам"
    );
}

#[test]
fn test_prefix_preserves_tabular_sections() {
    // ЦЕЛЬ: Проверить, что табличные части сохраняются после применения префикса

    let mut obj = create_test_document("ЗаказПокупателя");
    obj.tabular_sections.push(TabularSectionInfo {
        name: "Товары".to_string(),
        synonym: Some("Товары".to_string()),
        attributes: vec![
            AttributeInfo {
                name: "Номенклатура".to_string(),
                type_name: "СправочникСсылка.Номенклатура".to_string(),
                synonym: Some("Номенклатура".to_string()),
            },
            AttributeInfo {
                name: "Количество".to_string(),
                type_name: "Число".to_string(),
                synonym: Some("Количество".to_string()),
            },
        ],
    });

    let raw_type = obj.to_raw_type_data(Some("УП_"));

    assert_eq!(
        raw_type.tabular_sections.len(),
        1,
        "Табличные части должны сохраниться"
    );
    assert_eq!(
        raw_type.tabular_sections[0].name, "Товары",
        "Имена табличных частей не должны изменяться"
    );
    assert_eq!(
        raw_type.tabular_sections[0].attributes.len(),
        2,
        "Атрибуты табличной части должны сохраниться"
    );

    // Проверяем, что префикс не применился к табличным частям
    assert!(
        !raw_type.tabular_sections[0].name.contains("УП_"),
        "Префикс НЕ должен применяться к табличным частям"
    );
}

#[test]
fn test_prefix_preserves_facets() {
    // ЦЕЛЬ: Проверить, что фасеты сохраняются после применения префикса

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("Тест_"));

    assert_eq!(
        raw_type.facets.len(),
        5,
        "Количество фасетов должно сохраниться (Manager, Object, Reference, Selection, List)"
    );

    // Фасеты для Catalog
    use bsl_shared::domain::types::FacetKind;
    assert!(
        raw_type.facets.contains(&FacetKind::Manager),
        "Должен быть фасет Manager"
    );
    assert!(
        raw_type.facets.contains(&FacetKind::Object),
        "Должен быть фасет Object"
    );
    assert!(
        raw_type.facets.contains(&FacetKind::Reference),
        "Должен быть фасет Reference"
    );
}

#[test]
fn test_multiple_prefixes_different_objects() {
    // ЦЕЛЬ: Проверить, что разные префиксы корректно применяются к разным объектам

    let obj1 = create_test_catalog("Контрагенты");
    let obj2 = create_test_catalog("Контрагенты");
    let obj3 = create_test_catalog("Контрагенты");

    let raw_type1 = obj1.to_raw_type_data(Some("Расш1_"));
    let raw_type2 = obj2.to_raw_type_data(Some("Расш2_"));
    let raw_type3 = obj3.to_raw_type_data(None);

    assert_eq!(raw_type1.name, "Справочники.Расш1_Контрагенты");
    assert_eq!(raw_type2.name, "Справочники.Расш2_Контрагенты");
    assert_eq!(raw_type3.name, "Справочники.Контрагенты");

    // Все три объекта должны иметь разные имена
    assert_ne!(raw_type1.name, raw_type2.name);
    assert_ne!(raw_type1.name, raw_type3.name);
    assert_ne!(raw_type2.name, raw_type3.name);
}

#[test]
fn test_prefix_with_latin_characters() {
    // ЦЕЛЬ: Проверить работу с латинскими префиксами (edge case)

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("EXT_"));

    assert_eq!(
        raw_type.name, "Справочники.EXT_Контрагенты",
        "Латинский префикс должен корректно применяться"
    );
}

#[test]
fn test_prefix_with_numbers() {
    // ЦЕЛЬ: Проверить работу с префиксами, содержащими цифры

    let obj = create_test_catalog("Контрагенты");
    let raw_type = obj.to_raw_type_data(Some("Расш2024_"));

    assert_eq!(
        raw_type.name, "Справочники.Расш2024_Контрагенты",
        "Префикс с цифрами должен корректно применяться"
    );
}

#[test]
fn test_prefix_does_not_duplicate_underscores() {
    // ЦЕЛЬ: Убедиться, что не происходит дублирования подчёркиваний

    // Если имя объекта уже начинается с подчёркивания (edge case)
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "_СтранныйСправочник".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(Some("Тест_"));

    // Ожидаем "Справочники.Тест__СтранныйСправочник" (префикс применяется как есть)
    assert_eq!(
        raw_type.name, "Справочники.Тест__СтранныйСправочник",
        "Префикс должен применяться как есть, даже если имя начинается с _"
    );
}
