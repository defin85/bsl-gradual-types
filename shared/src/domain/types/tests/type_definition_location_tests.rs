//! Tests for Milestone 3.14: Go To Definition Location

use crate::domain::type_definition_location::TypeDefinitionLocation;
use crate::domain::types::*;

#[test]
fn test_primitive_definition_location() {
    let string = TypeResolution::known(ConcreteType::Primitive(PrimitiveType::String));
    let loc = string.get_definition_location();

    assert!(loc.is_some());
    assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Primitive));
}

#[test]
fn test_platform_definition_location() {
    let array = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Массив".to_string(),
    }));
    let loc = array.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Platform {
        type_name,
        docs_uri,
    }) = loc
    {
        assert_eq!(type_name, "Массив");
        assert!(docs_uri.is_some());
    } else {
        panic!("Expected Platform location");
    }
}

#[test]
fn test_configuration_definition_location() {
    let catalog = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
        kind: MetadataKind::Catalog,
        name: "Контрагенты".to_string(),
        facet: None,
        attributes: vec![],
        tabular_sections: vec![],
    }));
    let loc = catalog.get_definition_location();

    assert!(loc.is_some());
    assert!(matches!(
        loc.unwrap(),
        TypeDefinitionLocation::Configuration { .. }
    ));
}

#[test]
fn test_generic_definition_location() {
    let generic = TypeResolution {
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
    let loc = generic.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
        assert_eq!(type_name, "Массив");
    } else {
        panic!("Expected Platform location for Generic base type");
    }
}

#[test]
fn test_dynamic_no_definition_location() {
    let dynamic = TypeResolution::unknown();
    let loc = dynamic.get_definition_location();

    assert!(loc.is_none());
}

#[test]
fn test_union_definition_location_first_type() {
    let union = TypeResolution {
        result: ResolutionResult::Union(vec![
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: 0.7,
            },
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::Number),
                weight: 0.3,
            },
        ]),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let loc = union.get_definition_location();

    // Should return location of first (String) type
    assert!(loc.is_some());
    assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Primitive));
}

#[test]
fn test_nullable_definition_location_inner() {
    let nullable = TypeResolution {
        result: ResolutionResult::Nullable(Box::new(ConcreteType::Platform(PlatformType {
            name: "ТаблицаЗначений".to_string(),
        }))),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let loc = nullable.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
        assert_eq!(type_name, "ТаблицаЗначений");
    } else {
        panic!("Expected Platform location for Nullable inner type");
    }
}

#[test]
fn test_special_type_definition_location() {
    let null = TypeResolution::known(ConcreteType::Special(SpecialType::Null));
    let loc = null.get_definition_location();

    assert!(loc.is_some());
    // Special types return Primitive (no navigation)
    assert!(matches!(loc.unwrap(), TypeDefinitionLocation::Primitive));
}

#[test]
fn test_global_function_definition_location() {
    let func = TypeResolution::known(ConcreteType::GlobalFunction(GlobalFunctionInfo {
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
    let loc = func.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
        assert_eq!(type_name, "СтрДлина");
    } else {
        panic!("Expected Platform location for GlobalFunction");
    }
}

#[test]
fn test_tabular_row_definition_location() {
    let tr = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.ЗаказНаряды".to_string(),
        tabular_section_name: "Работы".to_string(),
        attributes: vec![],
    }));
    let loc = tr.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Configuration { metadata_path, .. }) = loc {
        assert!(metadata_path.to_string_lossy().contains("ЗаказНаряды"));
    } else {
        panic!("Expected Configuration location for TabularRow");
    }
}

#[test]
fn test_intersection_definition_location_first_type() {
    let intersection = TypeResolution {
        result: ResolutionResult::Intersection(vec![
            ConcreteType::Platform(PlatformType {
                name: "Массив".to_string(),
            }),
            ConcreteType::Platform(PlatformType {
                name: "ФиксированныйМассив".to_string(),
            }),
        ]),
        certainty: Certainty::Known,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let loc = intersection.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Platform { type_name, .. }) = loc {
        // Should return first type (Массив)
        assert_eq!(type_name, "Массив");
    } else {
        panic!("Expected Platform location for Intersection first type");
    }
}

#[test]
fn test_empty_union_no_definition_location() {
    let empty_union = TypeResolution {
        result: ResolutionResult::Union(vec![]),
        certainty: Certainty::Unknown,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let loc = empty_union.get_definition_location();

    assert!(loc.is_none());
}

#[test]
fn test_empty_intersection_no_definition_location() {
    let empty_intersection = TypeResolution {
        result: ResolutionResult::Intersection(vec![]),
        certainty: Certainty::Unknown,
        source: ResolutionSource::Static,
        metadata: ResolutionMetadata::default(),
        active_facet: None,
        available_facets: vec![],
    };
    let loc = empty_intersection.get_definition_location();

    assert!(loc.is_none());
}

#[test]
fn test_configuration_location_has_metadata_key() {
    let doc = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
        kind: MetadataKind::Document,
        name: "ЗаказПокупателя".to_string(),
        facet: Some(FacetKind::Object),
        attributes: vec![],
        tabular_sections: vec![],
    }));
    let loc = doc.get_definition_location();

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Configuration { metadata_path, .. }) = loc {
        let path_str = metadata_path.to_string_lossy();
        // Check that path contains correct prefix and name
        assert!(path_str.contains("Документы"));
        assert!(path_str.contains("ЗаказПокупателя"));
    } else {
        panic!("Expected Configuration location");
    }
}

// ============================================================================
// Milestone 3.14: Go To Definition with Module Paths
// ============================================================================

#[test]
fn test_configuration_with_module_paths() {
    use crate::domain::type_definition_location::ModulePaths;
    use std::path::PathBuf;

    let catalog = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
        kind: MetadataKind::Catalog,
        name: "Контрагенты".to_string(),
        facet: Some(FacetKind::Object),
        attributes: vec![],
        tabular_sections: vec![],
    }));

    let module_paths = ModulePaths {
        object_module: Some(PathBuf::from("Catalogs/Контрагенты/Ext/ObjectModule.bsl")),
        manager_module: Some(PathBuf::from("Catalogs/Контрагенты/Ext/ManagerModule.bsl")),
        recordset_module: None,
    };

    let loc = catalog.get_definition_location_with_modules(Some(&module_paths));

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Configuration {
        metadata_path,
        module_paths: mp,
    }) = loc
    {
        // metadata_path should contain type
        assert!(metadata_path.to_string_lossy().contains("Справочники"));
        assert!(metadata_path.to_string_lossy().contains("Контрагенты"));

        // module_paths should be copied
        assert!(mp.object_module.is_some());
        assert!(mp.manager_module.is_some());
        assert!(mp.recordset_module.is_none());
        assert!(mp
            .object_module
            .as_ref()
            .unwrap()
            .to_string_lossy()
            .contains("ObjectModule.bsl"));
    } else {
        panic!("Expected Configuration location with module paths");
    }
}

#[test]
fn test_configuration_without_module_paths_fallback() {
    let catalog = TypeResolution::known(ConcreteType::Configuration(ConfigurationType {
        kind: MetadataKind::Catalog,
        name: "Товары".to_string(),
        facet: None,
        attributes: vec![],
        tabular_sections: vec![],
    }));

    // Without module_paths - should work like old method
    let loc = catalog.get_definition_location_with_modules(None);

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Configuration { module_paths, .. }) = loc {
        // module_paths should be empty (default)
        assert!(!module_paths.has_any_module());
    } else {
        panic!("Expected Configuration location");
    }
}

#[test]
fn test_platform_type_ignores_module_paths() {
    use crate::domain::type_definition_location::ModulePaths;
    use std::path::PathBuf;

    let array = TypeResolution::known(ConcreteType::Platform(PlatformType {
        name: "Массив".to_string(),
    }));

    let module_paths = ModulePaths {
        object_module: Some(PathBuf::from("some/path.bsl")),
        manager_module: None,
        recordset_module: None,
    };

    // Platform type should ignore module_paths
    let loc = array.get_definition_location_with_modules(Some(&module_paths));

    assert!(loc.is_some());
    // Should return Platform location, not Configuration
    assert!(matches!(
        loc.unwrap(),
        TypeDefinitionLocation::Platform { .. }
    ));
}

#[test]
fn test_tabular_row_with_module_paths() {
    use crate::domain::type_definition_location::ModulePaths;
    use std::path::PathBuf;

    let tr = TypeResolution::known(ConcreteType::TabularRow(TabularRowType {
        parent_type: "Документы.ЗаказПокупателя".to_string(),
        tabular_section_name: "Товары".to_string(),
        attributes: vec![],
    }));

    let module_paths = ModulePaths {
        object_module: Some(PathBuf::from(
            "Documents/ЗаказПокупателя/Ext/ObjectModule.bsl",
        )),
        manager_module: None,
        recordset_module: None,
    };

    let loc = tr.get_definition_location_with_modules(Some(&module_paths));

    assert!(loc.is_some());
    if let Some(TypeDefinitionLocation::Configuration {
        metadata_path,
        module_paths: mp,
    }) = loc
    {
        // metadata_path should contain parent type and tabular section name
        assert!(metadata_path.to_string_lossy().contains("ЗаказПокупателя"));
        assert!(metadata_path.to_string_lossy().contains("Товары"));

        // module_paths should be copied
        assert!(mp.object_module.is_some());
    } else {
        panic!("Expected Configuration location for TabularRow");
    }
}

#[test]
fn test_dynamic_with_module_paths() {
    use crate::domain::type_definition_location::ModulePaths;
    use std::path::PathBuf;

    let dynamic = TypeResolution::unknown();

    let module_paths = ModulePaths {
        object_module: Some(PathBuf::from("some/path.bsl")),
        manager_module: None,
        recordset_module: None,
    };

    // Dynamic type should have no location even with module_paths
    let loc = dynamic.get_definition_location_with_modules(Some(&module_paths));
    assert!(loc.is_none());
}
