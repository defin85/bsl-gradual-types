//! Tests for Milestone 3.18: TypeResolution Constructors and Utilities

use crate::domain::types::*;

// === primitive() tests ===

#[test]
fn test_primitive_string_russian() {
    let t = TypeResolution::primitive("Строка");
    assert_eq!(t.certainty, Certainty::Known);
    assert_eq!(t.type_name(), "Строка");
    assert!(matches!(
        t.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String))
    ));
}

#[test]
fn test_primitive_string_english() {
    let t = TypeResolution::primitive("String");
    assert_eq!(t.certainty, Certainty::Known);
    assert_eq!(t.type_name(), "Строка");
}

#[test]
fn test_primitive_number_russian() {
    let t = TypeResolution::primitive("Число");
    assert_eq!(t.type_name(), "Число");
    assert!(matches!(
        t.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number))
    ));
}

#[test]
fn test_primitive_number_english() {
    let t = TypeResolution::primitive("Number");
    assert_eq!(t.type_name(), "Число");
}

#[test]
fn test_primitive_boolean_russian() {
    let t = TypeResolution::primitive("Булево");
    assert_eq!(t.type_name(), "Булево");
    assert!(matches!(
        t.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean))
    ));
}

#[test]
fn test_primitive_boolean_english() {
    let t = TypeResolution::primitive("Boolean");
    assert_eq!(t.type_name(), "Булево");
}

#[test]
fn test_primitive_date_russian() {
    let t = TypeResolution::primitive("Дата");
    assert_eq!(t.type_name(), "Дата");
    assert!(matches!(
        t.result,
        ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Date))
    ));
}

#[test]
fn test_primitive_date_english() {
    let t = TypeResolution::primitive("Date");
    assert_eq!(t.type_name(), "Дата");
}

#[test]
fn test_primitive_case_insensitive() {
    // Should recognize regardless of case
    let t1 = TypeResolution::primitive("СТРОКА");
    let t2 = TypeResolution::primitive("строка");
    let t3 = TypeResolution::primitive("Строка");

    assert_eq!(t1.type_name(), "Строка");
    assert_eq!(t2.type_name(), "Строка");
    assert_eq!(t3.type_name(), "Строка");
}

#[test]
fn test_primitive_unknown_creates_platform() {
    // Unrecognized name creates Platform type
    let t = TypeResolution::primitive("Массив");
    assert_eq!(t.certainty, Certainty::Known);
    assert_eq!(t.type_name(), "Массив");
    assert!(matches!(
        t.result,
        ResolutionResult::Concrete(ConcreteType::Platform(_))
    ));
}

#[test]
fn test_primitive_source_is_static() {
    let t = TypeResolution::primitive("Строка");
    assert_eq!(t.source, ResolutionSource::Static);
}

// === inferred() tests ===

#[test]
fn test_inferred_certainty() {
    let t = TypeResolution::inferred("Число");
    assert_eq!(t.certainty, Certainty::Inferred);
    assert_eq!(t.type_name(), "Число");
}

#[test]
fn test_inferred_source_is_inferred() {
    let t = TypeResolution::inferred("Строка");
    assert_eq!(t.source, ResolutionSource::Inferred);
}

#[test]
fn test_inferred_weak_certainty() {
    let t = TypeResolution::inferred_weak("Булево");
    assert_eq!(t.certainty, Certainty::InferredWeak);
    assert_eq!(t.type_name(), "Булево");
}

#[test]
fn test_inferred_weak_source_is_inferred() {
    let t = TypeResolution::inferred_weak("Дата");
    assert_eq!(t.source, ResolutionSource::Inferred);
}

#[test]
fn test_inferred_unknown_type() {
    // Unrecognized type also creates Platform
    let t = TypeResolution::inferred("ТаблицаЗначений");
    assert_eq!(t.certainty, Certainty::Inferred);
    assert_eq!(t.type_name(), "ТаблицаЗначений");
}

// === metadata_type() tests ===

#[test]
fn test_metadata_type_catalog_with_manager() {
    let t = TypeResolution::metadata_type(
        MetadataKind::Catalog,
        "Контрагенты",
        Some(FacetKind::Manager),
    );
    assert_eq!(t.certainty, Certainty::Known);
    assert_eq!(t.active_facet, Some(FacetKind::Manager));
    assert_eq!(t.type_name(), "СправочникМенеджер.Контрагенты");
}

#[test]
fn test_metadata_type_document_with_object() {
    let t = TypeResolution::metadata_type(
        MetadataKind::Document,
        "ЗаказПокупателя",
        Some(FacetKind::Object),
    );
    assert_eq!(t.active_facet, Some(FacetKind::Object));
    assert_eq!(t.type_name(), "ДокументОбъект.ЗаказПокупателя");
}

#[test]
fn test_metadata_type_without_facet() {
    let t = TypeResolution::metadata_type(MetadataKind::Catalog, "Номенклатура", None);
    assert_eq!(t.active_facet, None);
    assert!(t.available_facets.is_empty());
    assert_eq!(t.type_name(), "Справочники.Номенклатура");
}

#[test]
fn test_metadata_type_with_reference() {
    let t = TypeResolution::metadata_type(
        MetadataKind::Document,
        "СчетНаОплату",
        Some(FacetKind::Reference),
    );
    assert_eq!(t.active_facet, Some(FacetKind::Reference));
}

#[test]
fn test_metadata_type_available_facets() {
    let t = TypeResolution::metadata_type(
        MetadataKind::Catalog,
        "Валюты",
        Some(FacetKind::Selection),
    );
    assert!(t.available_facets.contains(&FacetKind::Selection));
}

#[test]
fn test_metadata_type_enum() {
    let t = TypeResolution::metadata_type(MetadataKind::Enum, "ВидыОперации", None);
    assert_eq!(t.type_name(), "Перечисления.ВидыОперации");
}

#[test]
fn test_metadata_type_register() {
    let t = TypeResolution::metadata_type(MetadataKind::InformationRegister, "КурсыВалют", None);
    assert_eq!(t.type_name(), "РегистрыСведений.КурсыВалют");
}

// === is_unknown() tests ===

#[test]
fn test_is_unknown_true() {
    let t = TypeResolution::unknown();
    assert!(t.is_unknown());
}

#[test]
fn test_is_unknown_false_for_primitive() {
    let t = TypeResolution::primitive("Строка");
    assert!(!t.is_unknown());
}

#[test]
fn test_is_unknown_false_for_known() {
    let t = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number));
    assert!(!t.is_unknown());
}

#[test]
fn test_is_unknown_false_for_inferred() {
    let t = TypeResolution::inferred("Число");
    assert!(!t.is_unknown());
}

#[test]
fn test_is_unknown_false_for_inferred_weak() {
    let t = TypeResolution::inferred_weak("Число");
    assert!(!t.is_unknown());
}

#[test]
fn test_is_unknown_false_for_metadata() {
    let t = TypeResolution::metadata_type(MetadataKind::Catalog, "Товары", None);
    assert!(!t.is_unknown());
}

// === type_name() tests ===

#[test]
fn test_type_name_primitive() {
    assert_eq!(TypeResolution::primitive("Строка").type_name(), "Строка");
    assert_eq!(TypeResolution::primitive("Число").type_name(), "Число");
    assert_eq!(TypeResolution::primitive("Булево").type_name(), "Булево");
    assert_eq!(TypeResolution::primitive("Дата").type_name(), "Дата");
}

#[test]
fn test_type_name_platform() {
    let t = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "ТаблицаЗначений".to_string(),
    }));
    assert_eq!(t.type_name(), "ТаблицаЗначений");
}

#[test]
fn test_type_name_configuration() {
    let t = TypeResolution::metadata_type(MetadataKind::Catalog, "Контрагенты", None);
    assert_eq!(t.type_name(), "Справочники.Контрагенты");
}

#[test]
fn test_type_name_special_undefined() {
    let t = TypeResolution::known(ConcreteType::Special(SpecialType::Undefined));
    assert_eq!(t.type_name(), "Неопределено");
}

#[test]
fn test_type_name_special_null() {
    let t = TypeResolution::known(ConcreteType::Special(SpecialType::Null));
    assert_eq!(t.type_name(), "Null");
}

#[test]
fn test_type_name_special_type() {
    let t = TypeResolution::known(ConcreteType::Special(SpecialType::Type));
    assert_eq!(t.type_name(), "Тип");
}

#[test]
fn test_type_name_global_function() {
    let t = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
        name: "СтрДлина".to_string(),
        english_name: None,
        description: None,
        parameters: vec![],
        return_type: None,
        return_description: None,
        polymorphic: false,
        pure: false,
        contexts: vec![],
        category: None,
    }));
    assert_eq!(t.type_name(), "СтрДлина");
}

#[test]
fn test_type_name_tabular_row() {
    let t = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.Заказ".to_string(),
        tabular_section_name: "Товары".to_string(),
        attributes: vec![],
    }));
    assert_eq!(t.type_name(), "СтрокаТовары");
}

#[test]
fn test_type_name_generic() {
    let t = TypeResolution {
        result: ResolutionResult::Generic(GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
        }),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Массив<Строка>");
}

#[test]
fn test_type_name_generic_multiple_params() {
    let t = TypeResolution {
        result: ResolutionResult::Generic(GenericType {
            base_type: "Соответствие".to_string(),
            type_params: vec![
                ConcreteType::Primitive(PrimitiveType::String),
                ConcreteType::Primitive(PrimitiveType::Number),
            ],
        }),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Соответствие<Строка, Число>");
}

#[test]
fn test_type_name_generic_empty_params() {
    let t = TypeResolution {
        result: ResolutionResult::Generic(GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![],
        }),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Массив");
}

#[test]
fn test_type_name_union() {
    let t = TypeResolution {
        result: ResolutionResult::Union(vec![
            WeightedType::new(ConcreteType::Primitive(PrimitiveType::String)),
            WeightedType::new(ConcreteType::Primitive(PrimitiveType::Number)),
        ]),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Строка | Число");
}

#[test]
fn test_type_name_union_empty() {
    let t = TypeResolution {
        result: ResolutionResult::Union(vec![]),
        certainty: Certainty::Unknown,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Dynamic");
}

#[test]
fn test_type_name_intersection() {
    let t = TypeResolution {
        result: ResolutionResult::Intersection(vec![
            ConcreteType::Platform(PlatformType {
                name: "Массив".to_string(),
            }),
            ConcreteType::Platform(PlatformType {
                name: "Коллекция".to_string(),
            }),
        ]),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Массив & Коллекция");
}

#[test]
fn test_type_name_intersection_empty() {
    let t = TypeResolution {
        result: ResolutionResult::Intersection(vec![]),
        certainty: Certainty::Unknown,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Dynamic");
}

#[test]
fn test_type_name_nullable() {
    let t = TypeResolution {
        result: ResolutionResult::Nullable(Box::new(ConcreteType::Primitive(
            PrimitiveType::String,
        ))),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    assert_eq!(t.type_name(), "Строка?");
}

#[test]
fn test_type_name_dynamic() {
    let t = TypeResolution::unknown();
    assert_eq!(t.type_name(), "Dynamic");
}
