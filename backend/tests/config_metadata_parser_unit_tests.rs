//! Unit-тесты для универсального парсера метаданных конфигурации
//!
//! Тестирует:
//! - MetadataKind::from_xml_tag() - все 20+ типов
//! - UniversalMetadataObject::determine_facets() - корректность фасетов
//! - Edge cases - пустые значения, некорректные данные
//! - Graceful degradation - обработка ошибок

use bsl_shared::domain::types::{FacetKind, MetadataKind};

// ============================================================================
// 1. Unit-тесты для MetadataKind::from_xml_tag()
// ============================================================================

#[test]
fn test_from_xml_tag_catalog() {
    let kind = MetadataKind::from_xml_tag("Catalog");
    assert_eq!(kind, Some(MetadataKind::Catalog));
}

#[test]
fn test_from_xml_tag_document() {
    let kind = MetadataKind::from_xml_tag("Document");
    assert_eq!(kind, Some(MetadataKind::Document));
}

#[test]
fn test_from_xml_tag_enum() {
    let kind = MetadataKind::from_xml_tag("Enum");
    assert_eq!(kind, Some(MetadataKind::Enum));
}

#[test]
fn test_from_xml_tag_information_register() {
    let kind = MetadataKind::from_xml_tag("InformationRegister");
    assert_eq!(kind, Some(MetadataKind::InformationRegister));
}

#[test]
fn test_from_xml_tag_accumulation_register() {
    let kind = MetadataKind::from_xml_tag("AccumulationRegister");
    assert_eq!(kind, Some(MetadataKind::AccumulationRegister));
}

#[test]
fn test_from_xml_tag_accounting_register() {
    let kind = MetadataKind::from_xml_tag("AccountingRegister");
    assert_eq!(kind, Some(MetadataKind::AccountingRegister));
}

#[test]
fn test_from_xml_tag_calculation_register() {
    let kind = MetadataKind::from_xml_tag("CalculationRegister");
    assert_eq!(kind, Some(MetadataKind::CalculationRegister));
}

#[test]
fn test_from_xml_tag_chart_of_accounts() {
    let kind = MetadataKind::from_xml_tag("ChartOfAccounts");
    assert_eq!(kind, Some(MetadataKind::ChartOfAccounts));
}

#[test]
fn test_from_xml_tag_chart_of_characteristic_types() {
    let kind = MetadataKind::from_xml_tag("ChartOfCharacteristicTypes");
    assert_eq!(kind, Some(MetadataKind::ChartOfCharacteristicTypes));
}

#[test]
fn test_from_xml_tag_chart_of_calculation_types() {
    let kind = MetadataKind::from_xml_tag("ChartOfCalculationTypes");
    assert_eq!(kind, Some(MetadataKind::ChartOfCalculationTypes));
}

#[test]
fn test_from_xml_tag_report() {
    let kind = MetadataKind::from_xml_tag("Report");
    assert_eq!(kind, Some(MetadataKind::Report));
}

#[test]
fn test_from_xml_tag_data_processor() {
    let kind = MetadataKind::from_xml_tag("DataProcessor");
    assert_eq!(kind, Some(MetadataKind::DataProcessor));
}

#[test]
fn test_from_xml_tag_business_process() {
    let kind = MetadataKind::from_xml_tag("BusinessProcess");
    assert_eq!(kind, Some(MetadataKind::BusinessProcess));
}

#[test]
fn test_from_xml_tag_task() {
    let kind = MetadataKind::from_xml_tag("Task");
    assert_eq!(kind, Some(MetadataKind::Task));
}

#[test]
fn test_from_xml_tag_exchange_plan() {
    let kind = MetadataKind::from_xml_tag("ExchangePlan");
    assert_eq!(kind, Some(MetadataKind::ExchangePlan));
}

#[test]
fn test_from_xml_tag_constant() {
    let kind = MetadataKind::from_xml_tag("Constant");
    assert_eq!(kind, Some(MetadataKind::Constant));
}

#[test]
fn test_from_xml_tag_role() {
    let kind = MetadataKind::from_xml_tag("Role");
    assert_eq!(kind, Some(MetadataKind::Role));
}

#[test]
fn test_from_xml_tag_common_module() {
    let kind = MetadataKind::from_xml_tag("CommonModule");
    assert_eq!(kind, Some(MetadataKind::CommonModule));
}

#[test]
fn test_from_xml_tag_subsystem() {
    let kind = MetadataKind::from_xml_tag("Subsystem");
    assert_eq!(kind, Some(MetadataKind::Subsystem));
}

#[test]
fn test_from_xml_tag_language() {
    let kind = MetadataKind::from_xml_tag("Language");
    assert_eq!(kind, Some(MetadataKind::Language));
}

#[test]
fn test_from_xml_tag_unknown_type() {
    let kind = MetadataKind::from_xml_tag("UnknownObjectType");
    assert_eq!(kind, None, "Неизвестный тип должен возвращать None");
}

#[test]
fn test_from_xml_tag_empty_string() {
    let kind = MetadataKind::from_xml_tag("");
    assert_eq!(kind, None, "Пустая строка должна возвращать None");
}

#[test]
fn test_from_xml_tag_case_sensitive() {
    // Проверяем, что функция чувствительна к регистру
    let kind = MetadataKind::from_xml_tag("catalog"); // lowercase
    assert_eq!(kind, None, "Неверный регистр должен возвращать None");
}

// ============================================================================
// 2. Unit-тесты для UniversalMetadataObject::determine_facets()
// ============================================================================

use bsl_backend::data::loaders::config_metadata_parser::types::UniversalMetadataObject;

#[test]
fn test_catalog_facets() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 5, "Справочник должен иметь 5 фасетов");
    assert!(obj.facets.contains(&FacetKind::Manager));
    assert!(obj.facets.contains(&FacetKind::Object));
    assert!(obj.facets.contains(&FacetKind::Reference));
    assert!(obj.facets.contains(&FacetKind::Selection));
    assert!(obj.facets.contains(&FacetKind::List));
}

#[test]
fn test_document_facets() {
    let obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 5, "Документ должен иметь 5 фасетов");
    assert!(obj.facets.contains(&FacetKind::Manager));
    assert!(obj.facets.contains(&FacetKind::Object));
    assert!(obj.facets.contains(&FacetKind::Reference));
    assert!(obj.facets.contains(&FacetKind::Selection));
    assert!(obj.facets.contains(&FacetKind::List));
}

#[test]
fn test_enum_facets() {
    let obj = UniversalMetadataObject::new(
        "Enum".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 2, "Перечисление должно иметь 2 фасета");
    assert!(obj.facets.contains(&FacetKind::Manager));
    assert!(obj.facets.contains(&FacetKind::Reference));
}

#[test]
fn test_information_register_facets() {
    let obj = UniversalMetadataObject::new(
        "InformationRegister".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(
        obj.facets.len(),
        3,
        "Регистр сведений должен иметь 3 фасета"
    );
    assert!(obj.facets.contains(&FacetKind::Manager));
    assert!(obj.facets.contains(&FacetKind::Object));
    assert!(obj.facets.contains(&FacetKind::Selection));
}

#[test]
fn test_report_facets() {
    let obj = UniversalMetadataObject::new(
        "Report".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 2, "Отчет должен иметь 2 фасета");
    assert!(obj.facets.contains(&FacetKind::Manager));
    assert!(obj.facets.contains(&FacetKind::Object));
}

#[test]
fn test_constant_facets() {
    let obj = UniversalMetadataObject::new(
        "Constant".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 1, "Константа должна иметь 1 фасет");
    assert!(obj.facets.contains(&FacetKind::Manager));
}

#[test]
fn test_common_module_facets() {
    let obj = UniversalMetadataObject::new(
        "CommonModule".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 1, "Общий модуль должен иметь 1 фасет");
    assert!(obj.facets.contains(&FacetKind::Singleton));
}

#[test]
fn test_role_facets() {
    let obj = UniversalMetadataObject::new(
        "Role".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(obj.facets.len(), 1, "Роль должна иметь 1 фасет");
    assert!(obj.facets.contains(&FacetKind::Metadata));
}

#[test]
fn test_unknown_type_facets() {
    let obj = UniversalMetadataObject::new(
        "UnknownType".to_string(),
        "Test".to_string(),
        "00000000-0000-0000-0000-000000000000".to_string(),
    );

    assert_eq!(
        obj.facets.len(),
        0,
        "Неизвестный тип должен иметь пустой список фасетов"
    );
    assert_eq!(
        obj.object_type, None,
        "Неизвестный тип должен иметь object_type = None"
    );
}

// ============================================================================
// 3. Edge Cases тесты
// ============================================================================

#[test]
fn test_cyrillic_object_name() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "ТестоваяКириллица".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    assert_eq!(obj.name, "ТестоваяКириллица");
    assert_eq!(obj.object_type, Some(MetadataKind::Catalog));
}

#[test]
fn test_empty_synonym() {
    let mut obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    obj.synonym = Some("".to_string());

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(
        raw_type.description, "",
        "Пустой синоним должен корректно конвертироваться"
    );
}

#[test]
fn test_no_synonym() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    assert_eq!(obj.synonym, None, "Синоним должен быть None по умолчанию");

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(
        raw_type.description, "",
        "Отсутствующий синоним должен конвертироваться в пустую строку"
    );
}

#[test]
fn test_uuid_format() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "e3cbde3c-52ed-494d-bd38-ed777a837517".to_string(),
    );

    assert_eq!(obj.uuid, "e3cbde3c-52ed-494d-bd38-ed777a837517");
    assert_eq!(obj.uuid.len(), 36, "UUID должен быть длиной 36 символов");
}

#[test]
fn test_empty_attributes_and_tabular_sections() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    assert_eq!(
        obj.attributes.len(),
        0,
        "Атрибуты должны быть пустыми по умолчанию"
    );
    assert_eq!(
        obj.tabular_sections.len(),
        0,
        "Табличные части должны быть пустыми по умолчанию"
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(raw_type.attributes.len(), 0);
    assert_eq!(raw_type.tabular_sections.len(), 0);
}

// ============================================================================
// 4. Конвертация в RawTypeData
// ============================================================================

#[test]
fn test_full_type_name_formation() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Контрагенты".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(raw_type.name, "Справочники.Контрагенты");
}

#[test]
fn test_document_full_type_name() {
    let obj = UniversalMetadataObject::new(
        "Document".to_string(),
        "ЗаказНаряды".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(raw_type.name, "Документы.ЗаказНаряды");
}

#[test]
fn test_information_register_full_type_name() {
    let obj = UniversalMetadataObject::new(
        "InformationRegister".to_string(),
        "ТестовыйРегистр".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(raw_type.name, "РегистрыСведений.ТестовыйРегистр");
}

#[test]
fn test_constant_full_type_name() {
    let obj = UniversalMetadataObject::new(
        "Constant".to_string(),
        "ОсновнаяОрганизация".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(raw_type.name, "Константы.ОсновнаяОрганизация");
}

#[test]
fn test_unknown_type_full_name() {
    let obj = UniversalMetadataObject::new(
        "UnknownType".to_string(),
        "СтранныйОбъект".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(
        raw_type.name, "СтранныйОбъект",
        "Неизвестный тип должен использовать просто имя"
    );
    assert_eq!(raw_type.kind, None);
}

#[test]
fn test_raw_data_source_is_configuration() {
    use bsl_shared::domain::types::RawDataSource;

    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(raw_type.source, RawDataSource::Configuration);
}

#[test]
fn test_facets_preserved_in_raw_type() {
    let obj = UniversalMetadataObject::new(
        "Catalog".to_string(),
        "Test".to_string(),
        "12345678-1234-1234-1234-123456789012".to_string(),
    );

    let raw_type = obj.to_raw_type_data(None);
    assert_eq!(
        raw_type.facets.len(),
        obj.facets.len(),
        "Фасеты должны сохраняться"
    );
    assert_eq!(raw_type.facets, obj.facets);
}
