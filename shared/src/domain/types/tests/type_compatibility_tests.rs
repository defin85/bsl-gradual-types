//! Tests for Milestone 3.13: Object-Based Type Comparison

use crate::domain::types::*;

#[test]
fn test_primitive_compatibility() {
    let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
    let number = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::Number));

    assert!(string.is_compatible_with(&string).is_compatible());
    assert!(!string.is_compatible_with(&number).is_compatible());
}

#[test]
fn test_facet_compatibility_object_to_reference() {
    let obj = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Object),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };
    let ref_ = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Reference),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Reference),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    // Object -> Reference: OK
    assert!(obj.is_compatible_with(&ref_).is_compatible());
    // Reference -> Object: NOT OK
    assert!(!ref_.is_compatible_with(&obj).is_compatible());
}

#[test]
fn test_dynamic_compatible_with_everything() {
    let dynamic = TypeResolution::unknown();
    let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));

    assert!(dynamic.is_compatible_with(&string).is_compatible());
    assert!(string.is_compatible_with(&dynamic).is_compatible());
}

#[test]
fn test_union_compatibility() {
    let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
    let union = TypeResolution {
        result: ResolutionResult::Union(vec![
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: 0.5,
            },
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::Number),
                weight: 0.5,
            },
        ]),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    assert!(string.is_compatible_with(&union).is_compatible());
}

#[test]
fn test_platform_type_case_insensitive() {
    let t1 = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Массив".to_string(),
    }));
    let t2 = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "массив".to_string(),
    }));

    assert!(t1.is_compatible_with(&t2).is_compatible());
}

#[test]
fn test_manager_incompatible_with_object() {
    let manager = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Manager),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Manager),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };
    let object = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Object),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    // Manager -> Object: NOT OK
    assert!(!manager.is_compatible_with(&object).is_compatible());
}

#[test]
fn test_type_ref_hash() {
    let ref1 = TypeRef::new("Массив");
    let ref2 = TypeRef::new("массив");
    let ref3 = TypeRef::new("МАССИВ");

    // Same hash for case-insensitive names
    assert_eq!(ref1.type_hash, ref2.type_hash);
    assert_eq!(ref2.type_hash, ref3.type_hash);
}

#[test]
fn test_type_compatibility_reason() {
    let compatible = TypeCompatibility::Compatible;
    let incompatible = TypeCompatibility::Incompatible {
        reason: "Test reason".to_string(),
    };

    assert_eq!(compatible.reason(), "");
    assert_eq!(incompatible.reason(), "Test reason");
}

#[test]
fn test_generic_type_compatibility() {
    let array_string = TypeResolution {
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
    let array_string2 = TypeResolution {
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
    let array_number = TypeResolution {
        result: ResolutionResult::Generic(GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![ConcreteType::Primitive(PrimitiveType::Number)],
        }),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    // Same generic types are compatible
    assert!(array_string
        .is_compatible_with(&array_string2)
        .is_compatible());
    // Different generic params are incompatible
    assert!(!array_string
        .is_compatible_with(&array_number)
        .is_compatible());
}

#[test]
fn test_different_configuration_types_incompatible() {
    let catalog = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Object),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };
    let document = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Document,
            name: "ЗаказПокупателя".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Object),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    // Different configuration types are incompatible
    assert!(!catalog.is_compatible_with(&document).is_compatible());
}

// ============================================================================
// Milestone 3.13 Additional Tests: Intersection, TabularRow, GlobalFunction
// ============================================================================

#[test]
fn test_intersection_compatibility_all_must_match() {
    // Intersection requires compatibility with ALL types
    let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
    let intersection = TypeResolution {
        result: ResolutionResult::Intersection(vec![
            ConcreteType::Primitive(PrimitiveType::String),
            ConcreteType::Primitive(PrimitiveType::Number),
        ]),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    // String is not compatible with Intersection(String, Number) because it's not compatible with Number
    assert!(!string.is_compatible_with(&intersection).is_compatible());
}

#[test]
fn test_tabular_row_same_parent_compatible() {
    let tr1 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.ЗаказНаряды".to_string(),
        tabular_section_name: "Работы".to_string(),
        attributes: vec![],
    }));
    let tr2 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.ЗаказНаряды".to_string(),
        tabular_section_name: "Работы".to_string(),
        attributes: vec![],
    }));

    assert!(tr1.is_compatible_with(&tr2).is_compatible());
}

#[test]
fn test_tabular_row_different_section_incompatible() {
    let tr1 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.ЗаказНаряды".to_string(),
        tabular_section_name: "Работы".to_string(),
        attributes: vec![],
    }));
    let tr2 = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.ЗаказНаряды".to_string(),
        tabular_section_name: "Материалы".to_string(),
        attributes: vec![],
    }));

    assert!(!tr1.is_compatible_with(&tr2).is_compatible());
}

#[test]
fn test_global_function_same_name_compatible() {
    let f1 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
        name: "СтрДлина".to_string(),
        english_name: Some("StrLen".to_string()),
        description: None,
        parameters: vec![],
        return_type: Some("Число".to_string()),
        return_description: None,
        polymorphic: false,
        pure: true,
        contexts: vec![],
        category: None,
    }));
    let f2 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
        name: "СтрДлина".to_string(),
        english_name: Some("StrLen".to_string()),
        description: None,
        parameters: vec![],
        return_type: Some("Число".to_string()),
        return_description: None,
        polymorphic: false,
        pure: true,
        contexts: vec![],
        category: None,
    }));

    assert!(f1.is_compatible_with(&f2).is_compatible());
}

#[test]
fn test_global_function_different_name_incompatible() {
    let f1 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
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
    let f2 = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
        name: "СтрНайти".to_string(),
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

    assert!(!f1.is_compatible_with(&f2).is_compatible());
}

#[test]
fn test_nested_generic_compatibility() {
    // Массив<Массив<Строка>> should be compatible with itself
    let outer = GenericType {
        base_type: "Массив".to_string(),
        type_params: vec![ConcreteType::Platform(PlatformType {
            name: "Массив".to_string(),
        })],
    };

    let t1 = TypeResolution {
        result: ResolutionResult::Generic(outer.clone()),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let t2 = TypeResolution {
        result: ResolutionResult::Generic(outer),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };

    assert!(t1.is_compatible_with(&t2).is_compatible());
}

#[test]
fn test_nullable_with_concrete() {
    // Nullable<String> should be compatible with String
    let nullable = TypeResolution {
        result: ResolutionResult::Nullable(Box::new(ConcreteType::Primitive(
            PrimitiveType::String,
        ))),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));

    assert!(nullable.is_compatible_with(&string).is_compatible());
}

#[test]
fn test_configuration_same_kind_different_name_incompatible() {
    let cfg1 = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
        kind: MetadataKind::Catalog,
        name: "Контрагенты".to_string(),
        facet: None,
        attributes: vec![],
        tabular_sections: vec![],
    }));
    let cfg2 = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
        kind: MetadataKind::Catalog,
        name: "Номенклатура".to_string(),
        facet: None,
        attributes: vec![],
        tabular_sections: vec![],
    }));

    // Different catalogs are incompatible
    assert!(!cfg1.is_compatible_with(&cfg2).is_compatible());
}

#[test]
fn test_selection_facet_compatible_with_selection() {
    let sel1 = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Selection),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Selection),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };
    let sel2 = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Selection),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Selection),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    assert!(sel1.is_compatible_with(&sel2).is_compatible());
}

#[test]
fn test_list_facet_incompatible_with_object() {
    let list = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::List),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::List),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };
    let obj = TypeResolution {
        result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
            kind: MetadataKind::Catalog,
            name: "Контрагенты".to_string(),
            facet: Some(FacetKind::Object),
            attributes: vec![],
            tabular_sections: vec![],
        })),
        active_facet: Some(FacetKind::Object),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        available_facets: vec![],
    };

    // List is incompatible with Object
    assert!(!list.is_compatible_with(&obj).is_compatible());
}

#[test]
fn test_compatibility_is_compatible() {
    let compatible = TypeCompatibility::Compatible;
    let incompatible = TypeCompatibility::Incompatible {
        reason: "Type mismatch".to_string(),
    };

    assert!(compatible.is_compatible());
    assert!(!incompatible.is_compatible());
}
