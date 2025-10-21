//! Integration tests for tabular sections support - via TypeSystemService
//!
//! Tests verify that TabularSectionDto structures are correctly used
//! in the API responses

#[cfg(test)]
mod tabular_sections_dto_tests {
    use bsl_shared::api::dtos::{TabularSectionAttributeDto, TabularSectionDto};

    // ========================================================================
    // Test 1: TabularSectionDto - camelCase serialization
    // ========================================================================

    #[test]
    fn test_tabular_section_dto_camel_case() {
        let dto = TabularSectionDto {
            name: "Работы".to_string(),
            attributes: vec![TabularSectionAttributeDto {
                name: "ВидРаботы".to_string(),
                attr_type: Some("xs:string".to_string()),
            }],
        };

        let json = serde_json::to_string(&dto).expect("Failed to serialize");

        println!("Serialized: {}", json);

        assert!(json.contains("\"name\":\"Работы\""));
        assert!(json.contains("\"attributes\":["));
        assert!(json.contains("\"attrType\":\"xs:string\""));
        assert!(!json.contains("\"attr_type\":"));

        println!("✓ Test 1 PASSED: CamelCase serialization works");
    }

    // ========================================================================
    // Test 2: Multiple tabular sections
    // ========================================================================

    #[test]
    fn test_multiple_tabular_sections() {
        let sections = vec![
            TabularSectionDto {
                name: "Работы".to_string(),
                attributes: vec![TabularSectionAttributeDto {
                    name: "ВидРаботы".to_string(),
                    attr_type: Some("xs:string".to_string()),
                }],
            },
            TabularSectionDto {
                name: "Стороны".to_string(),
                attributes: vec![TabularSectionAttributeDto {
                    name: "Сторона".to_string(),
                    attr_type: Some("cfg:CatalogRef.Контрагенты".to_string()),
                }],
            },
        ];

        let json = serde_json::to_string(&sections).expect("Failed to serialize");

        println!("Serialized sections: {}", json);

        // Verify both sections are present
        assert!(json.contains("\"name\":\"Работы\""));
        assert!(json.contains("\"name\":\"Стороны\""));

        // Deserialize and verify
        let deserialized: Vec<TabularSectionDto> =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].name, "Работы");
        assert_eq!(deserialized[1].name, "Стороны");

        println!("✓ Test 2 PASSED: Multiple tabular sections serialization");
    }

    // ========================================================================
    // Test 3: Null attribute type is skipped
    // ========================================================================

    #[test]
    fn test_null_attribute_type_skipped() {
        let dto = TabularSectionDto {
            name: "Работы".to_string(),
            attributes: vec![
                TabularSectionAttributeDto {
                    name: "ВидРаботы".to_string(),
                    attr_type: Some("xs:string".to_string()),
                },
                TabularSectionAttributeDto {
                    name: "Количество".to_string(),
                    attr_type: None,
                },
            ],
        };

        let json = serde_json::to_string(&dto).expect("Failed to serialize");
        let json_val: serde_json::Value =
            serde_json::from_str(&json).expect("Failed to parse");

        println!("JSON: {}", json);

        let attrs = &json_val["attributes"];
        assert_eq!(attrs[0]["name"], "ВидРаботы");
        assert_eq!(attrs[0]["attrType"], "xs:string");

        assert_eq!(attrs[1]["name"], "Количество");
        assert!(attrs[1].get("attrType").is_none());

        println!("✓ Test 3 PASSED: Null attribute types are skipped");
    }

    // ========================================================================
    // Test 4: Round-trip serialization
    // ========================================================================

    #[test]
    fn test_round_trip_serialization() {
        let original = TabularSectionDto {
            name: "Работы".to_string(),
            attributes: vec![
                TabularSectionAttributeDto {
                    name: "ВидРаботы".to_string(),
                    attr_type: Some("xs:string".to_string()),
                },
                TabularSectionAttributeDto {
                    name: "Количество".to_string(),
                    attr_type: Some("xs:decimal".to_string()),
                },
            ],
        };

        // Serialize
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Deserialize
        let restored: TabularSectionDto =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify
        assert_eq!(restored.name, original.name);
        assert_eq!(restored.attributes.len(), original.attributes.len());

        for (i, (orig, rest)) in original
            .attributes
            .iter()
            .zip(restored.attributes.iter())
            .enumerate()
        {
            assert_eq!(rest.name, orig.name, "Attribute {} name mismatch", i);
            assert_eq!(rest.attr_type, orig.attr_type, "Attribute {} type mismatch", i);
        }

        println!("✓ Test 4 PASSED: Round-trip serialization successful");
    }

    // ========================================================================
    // Test 5: Empty attributes list
    // ========================================================================

    #[test]
    fn test_empty_attributes() {
        let dto = TabularSectionDto {
            name: "Работы".to_string(),
            attributes: vec![],
        };

        let json = serde_json::to_string(&dto).expect("Failed to serialize");

        println!("JSON with empty attributes: {}", json);

        assert!(json.contains("\"name\":\"Работы\""));
        assert!(json.contains("\"attributes\":[]"));

        let restored: TabularSectionDto =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.name, "Работы");
        assert_eq!(restored.attributes.len(), 0);

        println!("✓ Test 5 PASSED: Empty attributes list handled");
    }

    // ========================================================================
    // Test 6: Composite type attribute
    // ========================================================================

    #[test]
    fn test_composite_type_attribute() {
        let composite_type = "cfg:CatalogRef.Контрагенты, cfg:CatalogRef.Организации, xs:string";

        let dto = TabularSectionDto {
            name: "Стороны".to_string(),
            attributes: vec![TabularSectionAttributeDto {
                name: "Сторона".to_string(),
                attr_type: Some(composite_type.to_string()),
            }],
        };

        let json = serde_json::to_string(&dto).expect("Failed to serialize");

        println!("JSON with composite type: {}", json);

        // All type variants should be present
        assert!(json.contains("CatalogRef.Контрагенты"));
        assert!(json.contains("CatalogRef.Организации"));
        assert!(json.contains("xs:string"));

        let restored: TabularSectionDto =
            serde_json::from_str(&json).expect("Failed to deserialize");

        let attr_type = &restored.attributes[0].attr_type;
        assert!(attr_type.is_some());
        assert!(attr_type.as_ref().unwrap().contains("CatalogRef.Контрагенты"));

        println!("✓ Test 6 PASSED: Composite types preserved");
    }

    // ========================================================================
    // Test 7: Empty tabular sections in TypeDto are skipped
    // ========================================================================

    #[test]
    fn test_empty_tabular_sections_skipped_in_type_dto() {
        let type_dto = bsl_shared::api::dtos::TypeDto {
            id: "Documents.ЗаказНаряды".to_string(),
            name: "ЗаказНаряды".to_string(),
            category: "Documents".to_string(),
            certainty: 100,
            certainty_text: "Known".to_string(),
            facets: vec![],
            methods_count: None,
            methods: vec![],
            attributes_count: None,
            properties: vec![],
            enum_values: None,
            tabular_sections: vec![],
            source: "Configuration".to_string(),
            flow_sensitive: false,
            description: "".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        };

        let json = serde_json::to_string(&type_dto).expect("Failed to serialize");

        println!("TypeDto JSON (empty tabular_sections): {}", json);

        // Empty tabular_sections should be skipped
        assert!(!json.contains("tabularSections"));

        println!("✓ Test 7 PASSED: Empty tabular sections skipped");
    }

    // ========================================================================
    // Test 8: TypeDto with tabular sections
    // ========================================================================

    #[test]
    fn test_type_dto_with_tabular_sections() {
        let type_dto = bsl_shared::api::dtos::TypeDto {
            id: "Documents.ЗаказНаряды".to_string(),
            name: "ЗаказНаряды".to_string(),
            category: "Documents".to_string(),
            certainty: 100,
            certainty_text: "Known".to_string(),
            facets: vec![],
            methods_count: None,
            methods: vec![],
            attributes_count: None,
            properties: vec![],
            enum_values: None,
            tabular_sections: vec![
                TabularSectionDto {
                    name: "Работы".to_string(),
                    attributes: vec![TabularSectionAttributeDto {
                        name: "ВидРаботы".to_string(),
                        attr_type: Some("xs:string".to_string()),
                    }],
                },
                TabularSectionDto {
                    name: "Стороны".to_string(),
                    attributes: vec![TabularSectionAttributeDto {
                        name: "Сторона".to_string(),
                        attr_type: Some("cfg:CatalogRef.Контрагенты".to_string()),
                    }],
                },
            ],
            source: "Configuration".to_string(),
            flow_sensitive: false,
            description: "".to_string(),
            union_types: None,
            flow_analysis: None,
            connections: None,
            warning: None,
            recommendation: None,
        };

        let json = serde_json::to_string(&type_dto).expect("Failed to serialize");

        println!("TypeDto JSON with tabular_sections: {}", json);

        // Tabular sections should be present
        assert!(json.contains("\"tabularSections\":["));
        assert!(json.contains("\"name\":\"Работы\""));
        assert!(json.contains("\"name\":\"Стороны\""));

        // Deserialize and verify
        let restored: bsl_shared::api::dtos::TypeDto =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.tabular_sections.len(), 2);
        assert_eq!(restored.tabular_sections[0].name, "Работы");
        assert_eq!(restored.tabular_sections[1].name, "Стороны");

        println!("✓ Test 8 PASSED: TypeDto with tabular sections serialized correctly");
    }

    // ========================================================================
    // Test 9: Cyrillic attribute names preserved
    // ========================================================================

    #[test]
    fn test_cyrillic_names_preserved() {
        let dto = TabularSectionDto {
            name: "ТабличнаяЧасть".to_string(),
            attributes: vec![
                TabularSectionAttributeDto {
                    name: "Наименование".to_string(),
                    attr_type: Some("xs:string".to_string()),
                },
                TabularSectionAttributeDto {
                    name: "Количество".to_string(),
                    attr_type: Some("xs:decimal".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&dto).expect("Failed to serialize");

        println!("JSON with Cyrillic: {}", json);

        // Verify Cyrillic is properly encoded in JSON
        assert!(json.contains("ТабличнаяЧасть") || json.contains("\\u"));

        // Deserialize and verify
        let restored: TabularSectionDto =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(restored.name, "ТабличнаяЧасть");
        assert_eq!(restored.attributes[0].name, "Наименование");
        assert_eq!(restored.attributes[1].name, "Количество");

        println!("✓ Test 9 PASSED: Cyrillic names preserved");
    }

    // ========================================================================
    // Test 10: Attribute type variants
    // ========================================================================

    #[test]
    fn test_attribute_type_variants() {
        let test_cases = vec![
            ("xs:string", "Simple type"),
            ("xs:decimal", "Decimal type"),
            ("xs:boolean", "Boolean type"),
            ("cfg:CatalogRef.Контрагенты", "Catalog reference"),
            ("cfg:DocumentRef.ПоступлениеТоваров", "Document reference"),
            ("cfg:CatalogRef.Контрагенты, xs:string", "Multiple types"),
        ];

        for (type_str, desc) in test_cases {
            let dto = TabularSectionDto {
                name: "TestSection".to_string(),
                attributes: vec![TabularSectionAttributeDto {
                    name: "TestAttr".to_string(),
                    attr_type: Some(type_str.to_string()),
                }],
            };

            let json = serde_json::to_string(&dto).expect(&format!("Failed for {}", desc));
            let restored: TabularSectionDto = serde_json::from_str(&json)
                .expect(&format!("Failed to deserialize for {}", desc));

            assert_eq!(
                restored.attributes[0].attr_type.as_ref().unwrap(),
                type_str,
                "Type mismatch for {}",
                desc
            );

            println!("✓ Variant '{}' preserved", desc);
        }

        println!("✓ Test 10 PASSED: All attribute type variants work");
    }
}
