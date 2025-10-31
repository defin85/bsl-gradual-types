//! Integration tests for tabular sections support in Web API
//!
//! Tests verify that:
//! 1. Tabular sections are correctly parsed from configuration XML
//! 2. TabularSectionDto is properly serialized to JSON
//! 3. API endpoints return tabular section data with proper camelCase names
//! 4. Edge cases are handled (empty attributes, no tabular sections, etc.)

#[cfg(test)]
mod tabular_sections_api_tests {
    use bsl_backend::system::system_coordinator::SystemCoordinator;
    use bsl_shared::api::dtos::{TabularSectionAttributeDto, TabularSectionDto};
    use std::path::Path;

    /// Helper function to create a test coordinator
    async fn create_test_coordinator() -> SystemCoordinator {
        let coordinator = SystemCoordinator::new();
        let config_path = Path::new("examples/conf/conf_test");

        coordinator
            .start_with_paths(None, Some(config_path), None)
            .await
            .expect("Failed to start coordinator");

        coordinator
    }

    #[tokio::test]
    async fn test_api_returns_tabular_sections_for_zakaznarjady() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let result = service
            .search_types_as_dto("Документы.ЗаказНаряды")
            .await
            .expect("Failed to search types");

        assert!(
            !result.types.is_empty(),
            "Should find документы.ЗаказНаряды"
        );

        let doc = &result.types[0];

        assert_eq!(
            doc.tabular_sections.len(),
            2,
            "Document should have 2 tabular sections"
        );

        let raboty = doc
            .tabular_sections
            .iter()
            .find(|ts| ts.name == "Работы")
            .expect("Should find 'Работы' tabular section");

        assert_eq!(raboty.attributes.len(), 1, "Работы should have 1 attribute");
        assert_eq!(
            raboty.attributes[0].name, "ВидРаботы",
            "First attribute should be 'ВидРаботы'"
        );
        assert_eq!(
            raboty.attributes[0].attr_type,
            Some("xs:string".to_string()),
            "Attribute type should be xs:string"
        );

        let storony = doc
            .tabular_sections
            .iter()
            .find(|ts| ts.name == "Стороны")
            .expect("Should find 'Стороны' tabular section");

        assert_eq!(
            storony.attributes.len(),
            1,
            "Стороны should have 1 attribute"
        );
        assert_eq!(
            storony.attributes[0].name, "Сторона",
            "First attribute should be 'Сторона'"
        );

        assert!(
            storony.attributes[0].attr_type.is_some(),
            "Сторона should have a type"
        );
        let attr_type = storony.attributes[0].attr_type.as_ref().unwrap();
        assert!(
            attr_type.contains("CatalogRef.Контрагенты"),
            "Type should contain CatalogRef.Контрагенты"
        );

        println!("✓ Test 1 PASSED: Document has 2 tabular sections with correct attributes");
    }

    #[tokio::test]
    async fn test_platform_types_have_no_tabular_sections() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let result = service
            .search_types_as_dto("Массив")
            .await
            .expect("Failed to search types");

        assert!(!result.types.is_empty(), "Should find Массив type");

        let array_type = &result.types[0];

        assert_eq!(
            array_type.tabular_sections.len(),
            0,
            "Platform types should have no tabular sections"
        );

        println!("✓ Test 2 PASSED: Platform types have empty tabular sections");
    }

    #[test]
    fn test_tabular_section_dto_serialization_camel_case() {
        let dto = TabularSectionDto {
            name: "Работы".to_string(),
            attributes: vec![TabularSectionAttributeDto {
                name: "ВидРаботы".to_string(),
                attr_type: Some("xs:string".to_string()),
            }],
        };

        let json_str = serde_json::to_string(&dto).expect("Failed to serialize");

        assert!(json_str.contains("\"name\":"), "Should contain name field");
        assert!(
            json_str.contains("\"attributes\":"),
            "Should contain attributes array"
        );
        assert!(
            json_str.contains("\"attrType\":"),
            "Should use camelCase 'attrType' not 'attr_type'"
        );

        assert!(
            !json_str.contains("\"attr_type\":"),
            "Should not contain snake_case attr_type"
        );

        println!("✓ Test 3 PASSED: CamelCase serialization works");
        println!("  JSON: {}", json_str);
    }

    #[test]
    fn test_tabular_section_attribute_null_type() {
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

        let json_str = serde_json::to_string(&dto).expect("Failed to serialize");
        let json_val: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to parse JSON");

        let attributes = json_val["attributes"]
            .as_array()
            .expect("attributes should be array");

        assert_eq!(attributes[0]["name"], "ВидРаботы");
        assert_eq!(attributes[0]["attrType"], "xs:string");

        assert_eq!(attributes[1]["name"], "Количество");
        assert!(
            attributes[1].get("attrType").is_none(),
            "attrType should be skipped when None"
        );

        println!("✓ Test 4 PASSED: Null attribute types are skipped");
    }

    #[test]
    fn test_empty_tabular_sections_skipped() {
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

        let json_str = serde_json::to_string(&type_dto).expect("Failed to serialize");

        assert!(
            !json_str.contains("tabularSections"),
            "Empty tabularSections should be skipped"
        );

        println!("✓ Test 5 PASSED: Empty tabular sections are skipped in serialization");
    }

    #[test]
    fn test_multiple_attributes_in_tabular_section() {
        let dto = TabularSectionDto {
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
                TabularSectionAttributeDto {
                    name: "Стоимость".to_string(),
                    attr_type: Some("xs:decimal".to_string()),
                },
            ],
        };

        let json_str = serde_json::to_string(&dto).expect("Failed to serialize");
        let json_val: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to parse JSON");

        let attributes = json_val["attributes"]
            .as_array()
            .expect("attributes should be array");

        assert_eq!(attributes.len(), 3, "Should have 3 attributes");
        assert_eq!(attributes[0]["name"], "ВидРаботы");
        assert_eq!(attributes[1]["name"], "Количество");
        assert_eq!(attributes[2]["name"], "Стоимость");

        println!("✓ Test 6 PASSED: Multiple attributes serialized correctly");
    }

    #[tokio::test]
    async fn test_composite_attribute_type_preserved() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let result = service
            .search_types_as_dto("Документы.ЗаказНаряды")
            .await
            .expect("Failed to search types");

        assert!(
            !result.types.is_empty(),
            "Should find документы.ЗаказНаряды"
        );

        let doc = &result.types[0];
        let storony = doc
            .tabular_sections
            .iter()
            .find(|ts| ts.name == "Стороны")
            .expect("Should find 'Стороны' tabular section");

        let attr_type = storony.attributes[0]
            .attr_type
            .as_ref()
            .expect("Should have type");

        assert!(
            attr_type.contains("CatalogRef.Контрагенты"),
            "Should contain first type variant"
        );
        assert!(
            attr_type.contains("CatalogRef.Организации"),
            "Should contain second type variant"
        );
        assert!(
            attr_type.contains("xs:string"),
            "Should contain string variant"
        );

        println!("✓ Test 7 PASSED: Composite attribute types preserved");
        println!("  Type: {}", attr_type);
    }

    #[tokio::test]
    async fn test_search_with_partial_name() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let result = service
            .search_types_as_dto("ЗаказНаряды")
            .await
            .expect("Failed to search types");

        if !result.types.is_empty() {
            let doc = &result.types[0];

            if !doc.tabular_sections.is_empty() {
                assert_eq!(
                    doc.tabular_sections.len(),
                    2,
                    "Should have 2 tabular sections"
                );
            }

            println!(
                "✓ Test 8 PASSED: Found document with {} tabular sections",
                doc.tabular_sections.len()
            );
        }
    }

    #[tokio::test]
    async fn test_platform_types_regression() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let platform_types = vec!["Массив", "Строка", "Число", "Структура", "Соответствие"];

        for type_name in platform_types {
            let result = service
                .search_types_as_dto(type_name)
                .await
                .expect("Failed to search types");

            if !result.types.is_empty() {
                let type_dto = &result.types[0];

                assert!(!type_dto.name.is_empty(), "Name should not be empty");
                assert_eq!(
                    type_dto.tabular_sections.len(),
                    0,
                    "Platform type {} should have no tabular sections",
                    type_name
                );
            }
        }

        println!("✓ Test 9 PASSED: No regression in platform types");
    }

    #[test]
    fn test_dto_structure_schema() {
        let dto = TabularSectionDto {
            name: "TestSection".to_string(),
            attributes: vec![],
        };

        let json_str = serde_json::to_string(&dto).expect("Failed to serialize");
        let json_val: serde_json::Value =
            serde_json::from_str(&json_str).expect("Failed to parse JSON");

        assert!(json_val["name"].is_string(), "name should be string");
        assert!(
            json_val["attributes"].is_array(),
            "attributes should be array"
        );

        println!("✓ Test 10 PASSED: DTO schema is valid");
    }

    #[test]
    fn test_tabular_section_round_trip() {
        let original = TabularSectionDto {
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

        let json_str = serde_json::to_string(&original).expect("Failed to serialize");

        let restored: TabularSectionDto =
            serde_json::from_str(&json_str).expect("Failed to deserialize");

        assert_eq!(restored.name, original.name);
        assert_eq!(restored.attributes.len(), original.attributes.len());

        for (i, (orig_attr, rest_attr)) in original
            .attributes
            .iter()
            .zip(restored.attributes.iter())
            .enumerate()
        {
            assert_eq!(
                rest_attr.name, orig_attr.name,
                "Attribute {} name mismatch",
                i
            );
            assert_eq!(
                rest_attr.attr_type, orig_attr.attr_type,
                "Attribute {} type mismatch",
                i
            );
        }

        println!("✓ Test 11 PASSED: Round-trip serialization successful");
    }

    #[tokio::test]
    async fn test_all_tabular_sections_returned() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let result = service
            .search_types_as_dto("ЗаказНаряды")
            .await
            .expect("Failed to search types");

        if !result.types.is_empty() {
            let doc = &result.types[0];
            let tab_sections: Vec<_> = doc.tabular_sections.iter().map(|ts| &ts.name).collect();

            assert!(
                tab_sections.contains(&&"Работы".to_string()),
                "Should have Работы"
            );
            assert!(
                tab_sections.contains(&&"Стороны".to_string()),
                "Should have Стороны"
            );

            println!(
                "✓ Test 12 PASSED: All tabular sections returned: {:?}",
                tab_sections
            );
        }
    }

    #[tokio::test]
    async fn test_attribute_names_preserved() {
        let coordinator = create_test_coordinator().await;

        let service = coordinator
            .type_service()
            .expect("Failed to get TypeSystemService");

        let result = service
            .search_types_as_dto("ЗаказНаряды")
            .await
            .expect("Failed to search types");

        if !result.types.is_empty() {
            let doc = &result.types[0];

            for section in &doc.tabular_sections {
                for attr in &section.attributes {
                    assert!(!attr.name.is_empty(), "Attribute name should not be empty");
                    assert!(
                        attr.name.chars().any(|c| c as u32 > 127),
                        "Attribute name should contain Cyrillic characters"
                    );
                }
            }

            println!("✓ Test 13 PASSED: Attribute names preserved correctly");
        }
    }
}
