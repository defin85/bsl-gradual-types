//! Тесты для hover formatter
//!
//! Тесты для всех компонентов модуля hover_formatter.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::helpers::hover_formatter::builder::HoverBuilder;
    use crate::helpers::hover_formatter::config::{HoverFormatConfig, HoverOutputFormat};
    use crate::helpers::hover_formatter::formatter::HoverFormatter;
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::signature_index::{
        ContextRequirements, MethodSignature, SignatureSource,
    };
    use bsl_shared::domain::types::{
        Certainty, ConcreteType, ParameterInfo, PlatformType, RawMethodData, RawPropertyData,
        ResolutionMetadata, ResolutionResult, ResolutionSource, TypeResolution,
    };
    use bsl_shared::formatting::DetailLevel;
    use std::sync::Arc;

    #[test]
    fn test_hover_format_config_default() {
        let config = HoverFormatConfig::default();
        assert_eq!(config.max_methods, 10);
        assert_eq!(config.max_properties, 5);
        assert_eq!(config.output_format, HoverOutputFormat::Markdown);
    }

    #[test]
    fn test_hover_builder_basic() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_header("Переменная", "МассивДанных")
            .build();

        assert!(result.contains("Переменная"));
        assert!(result.contains("МассивДанных"));
    }

    #[test]
    fn test_format_function_signature_with_docs() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let formatter = HoverFormatter::new(HoverFormatConfig::default(), metadata_lookup);

        let signature = MethodSignature::new(
            "ТестФункция".to_string(),
            None,
            vec![ParameterInfo {
                name: "Параметр".to_string(),
                type_name: Some("Строка".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            }],
            Some("Число".to_string()),
            Some("Описание функции".to_string()),
            Some("Описание возврата".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        let result = formatter.format_function_signature("Функция", &signature);

        assert!(result.contains("Функция"));
        assert!(result.contains("ТестФункция([Параметр: Строка])"));
        assert!(result.contains("-> Число"));
        assert!(result.contains("Описание функции"));
        assert!(result.contains("Описание возврата"));
    }

    #[test]
    fn test_output_format_markdown_vs_plaintext() {
        // Test Markdown format
        let config_md = HoverFormatConfig {
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };
        let result_md = HoverBuilder::new(&config_md)
            .add_header("Тест", "Значение")
            .build();
        assert!(result_md.contains("**Тест:**"));

        // Test PlainText format
        let config_txt = HoverFormatConfig {
            output_format: HoverOutputFormat::PlainText,
            ..Default::default()
        };
        let result_txt = HoverBuilder::new(&config_txt)
            .add_header("Тест", "Значение")
            .build();
        assert!(result_txt.contains("Тест:"));
        assert!(!result_txt.contains("**"));
    }

    #[test]
    fn test_certainty_formatting_known() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_certainty(&Certainty::Known)
            .build();

        assert!(result.contains("Known (100%)"));
    }

    #[test]
    fn test_certainty_formatting_inferred() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_certainty(&Certainty::Inferred)
            .build();

        assert!(result.contains("Inferred (80%)"));
    }

    #[test]
    fn test_certainty_formatting_inferred_weak() {
        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_certainty(&Certainty::InferredWeak)
            .build();

        assert!(result.contains("InferredWeak (50%)"));
    }

    #[test]
    fn test_generic_type_formatting() {
        use bsl_shared::domain::types::{ConcreteType, GenericType, PrimitiveType};

        let generic = GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let config = HoverFormatConfig::default();
        let result = HoverBuilder::new(&config)
            .add_type_info(&resolution)
            .build();

        assert!(result.contains("Массив<Строка>"));
    }

    // Helper functions for testing
    fn create_test_repository_with_methods(method_count: usize) -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{FacetKind, RawDataSource, RawTypeData};

        let repo = Arc::new(InMemoryTypeRepository::new());

        let methods: Vec<RawMethodData> = (0..method_count)
            .map(|i| RawMethodData {
                name: format!("Метод{}", i),
                english_name: format!("Method{}", i),
                return_type: "Строка".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            })
            .collect();

        let test_type = RawTypeData {
            name: "ТестовыйТип".to_string(),
            english_name: "TestType".to_string(),
            description: "Тип для тестирования".to_string(),
            category: "Test".to_string(),
            source: RawDataSource::Platform,
            methods,
            properties: vec![],
            facets: vec![FacetKind::Object],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_repository_with_properties(
        property_count: usize,
    ) -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{FacetKind, RawDataSource, RawTypeData};

        let repo = Arc::new(InMemoryTypeRepository::new());

        let properties: Vec<RawPropertyData> = (0..property_count)
            .map(|i| RawPropertyData {
                name: format!("Свойство{}", i),
                prop_type: "Строка".to_string(),
                is_readonly: false,
            })
            .collect();

        let test_type = RawTypeData {
            name: "ТестовыйТип".to_string(),
            english_name: "TestType".to_string(),
            description: "Тип для тестирования".to_string(),
            category: "Test".to_string(),
            source: RawDataSource::Platform,
            methods: vec![],
            properties,
            facets: vec![FacetKind::Object],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_resolution() -> TypeResolution {
        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТестовыйТип".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    #[test]
    fn test_methods_with_limit() {
        let repo = create_test_repository_with_methods(20);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            max_methods: 10,
            detail_level: DetailLevel::Full,
            ..Default::default()
        };

        let resolution = create_test_resolution();

        let result = HoverBuilder::new(&config)
            .add_methods(&resolution, &metadata_lookup)
            .build();

        assert!(result.contains("показано 10 из 20"));
        assert!(result.contains("Метод0"));
        assert!(result.contains("Метод9"));
        assert!(!result.contains("Метод10"));
        assert!(result.contains("... и ещё 10 методов"));
    }

    #[test]
    fn test_properties_with_limit() {
        let repo = create_test_repository_with_properties(10);
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            max_properties: 5,
            detail_level: DetailLevel::Full,
            ..Default::default()
        };

        let resolution = create_test_resolution();

        let result = HoverBuilder::new(&config)
            .add_properties(&resolution, &metadata_lookup)
            .build();

        assert!(result.contains("показано 5 из 10"));
        assert!(result.contains("Свойство0"));
        assert!(result.contains("Свойство4"));
        assert!(!result.contains("Свойство5"));
        assert!(result.contains("... и ещё 5 свойств"));
    }

    // === MILESTONE 3.16: Тесты для format_unknown_metadata_object ===

    #[test]
    fn test_format_unknown_metadata_object_markdown() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        let result = formatter.format_unknown_metadata_object(
            MetadataKind::Catalog,
            "Контрагенты",
            &[
                "Контрагент".to_string(),
                "КонтрагентыПоставщики".to_string(),
            ],
        );

        assert!(result.contains("## Справочник \"Контрагенты\" не найден"));
        assert!(result.contains("Объект не существует в загруженной конфигурации"));
        assert!(result.contains("### Возможно, вы имели в виду:"));
        assert!(result.contains("- `Контрагент`"));
        assert!(result.contains("- `КонтрагентыПоставщики`"));
        assert!(result.contains("BSL: Parse Configuration"));
    }

    #[test]
    fn test_format_unknown_metadata_object_plaintext() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            output_format: HoverOutputFormat::PlainText,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        let result = formatter.format_unknown_metadata_object(
            MetadataKind::Document,
            "ЗаказПокупателя",
            &[],
        );

        assert!(result.contains("Документ \"ЗаказПокупателя\" не найден"));
        assert!(!result.contains("##"));
        assert!(!result.contains("Возможно, вы имели в виду"));
    }

    #[test]
    fn test_format_unknown_metadata_object_without_suggestions() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let result = formatter.format_unknown_metadata_object(
            MetadataKind::Enum,
            "НесуществующееПеречисление",
            &[],
        );

        assert!(result.contains("## Перечисление \"НесуществующееПеречисление\" не найден"));
        assert!(!result.contains("### Возможно, вы имели в виду:"));
        assert!(result.contains("BSL: Parse Configuration"));
    }

    #[test]
    fn test_format_unknown_metadata_object_different_kinds() {
        use bsl_shared::domain::types::MetadataKind;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let test_cases = vec![
            (MetadataKind::Catalog, "Справочник"),
            (MetadataKind::Document, "Документ"),
            (MetadataKind::InformationRegister, "Регистр сведений"),
            (MetadataKind::AccumulationRegister, "Регистр накопления"),
            (MetadataKind::Report, "Отчет"),
            (MetadataKind::DataProcessor, "Обработка"),
        ];

        for (kind, expected_name) in test_cases {
            let result = formatter.format_unknown_metadata_object(kind, "Тест", &[]);
            assert!(
                result.contains(&format!("{} \"Тест\" не найден", expected_name)),
                "Failed for kind {:?}: expected '{}', got: {}",
                kind,
                expected_name,
                result
            );
        }
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_none_for_known_type() {
        use bsl_shared::domain::types::ConfigurationType;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: bsl_shared::domain::types::MetadataKind::Catalog,
                name: "Контрагенты".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(formatter
            .check_unknown_metadata_object(&resolution)
            .is_none());
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_none_for_platform_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "Массив".to_string(),
            })),
            certainty: Certainty::InferredWeak,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(formatter
            .check_unknown_metadata_object(&resolution)
            .is_none());
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_none_when_config_not_loaded() {
        use bsl_shared::domain::types::ConfigurationType;

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: bsl_shared::domain::types::MetadataKind::Catalog,
                name: "НесуществующийСправочник".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::InferredWeak,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(formatter
            .check_unknown_metadata_object(&resolution)
            .is_none());
    }

    #[test]
    fn test_check_unknown_metadata_object_returns_some_when_object_not_found() {
        use bsl_shared::domain::types::{ConfigurationType, FacetKind, RawDataSource, RawTypeData};

        let repo = Arc::new(InMemoryTypeRepository::new());

        let existing_catalog = RawTypeData {
            name: "Справочники.Контрагенты".to_string(),
            english_name: "Catalogs.Contractors".to_string(),
            description: "Справочник контрагентов".to_string(),
            category: "Справочники".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager],
            kind: Some(bsl_shared::domain::types::MetadataKind::Catalog),
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
        };
        repo.load_types(vec![existing_catalog]).unwrap();

        let metadata_lookup = TypeMetadataLookup::new(repo);
        let config = HoverFormatConfig::default();
        let formatter = HoverFormatter::new(config, metadata_lookup);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind: bsl_shared::domain::types::MetadataKind::Catalog,
                name: "НесуществующийСправочник".to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::InferredWeak,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let result = formatter.check_unknown_metadata_object(&resolution);
        assert!(result.is_some());

        let (kind, name) = result.unwrap();
        assert_eq!(kind, bsl_shared::domain::types::MetadataKind::Catalog);
        assert_eq!(name, "НесуществующийСправочник");
    }

    // === Тесты для add_tabular_sections ===

    fn create_test_repository_with_tabular_sections() -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::{
            FacetKind, MetadataKind, RawAttributeData, RawDataSource, RawTabularSectionData,
            RawTypeData,
        };

        let repo = Arc::new(InMemoryTypeRepository::new());

        let document = RawTypeData {
            name: "Документы.ЗаказНаряды".to_string(),
            english_name: "Documents.WorkOrders".to_string(),
            description: "Документ заказ-наряды".to_string(),
            category: "Документы".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![],
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            attributes: vec![],
            tabular_sections: vec![
                RawTabularSectionData {
                    name: "Работы".to_string(),
                    attributes: vec![
                        RawAttributeData {
                            name: "Номенклатура".to_string(),
                            attr_type: "СправочникСсылка.Номенклатура".to_string(),
                        },
                        RawAttributeData {
                            name: "Количество".to_string(),
                            attr_type: "Число".to_string(),
                        },
                    ],
                },
                RawTabularSectionData {
                    name: "Материалы".to_string(),
                    attributes: vec![RawAttributeData {
                        name: "Материал".to_string(),
                        attr_type: "СправочникСсылка.Номенклатура".to_string(),
                    }],
                },
            ],
            enum_values: vec![],
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
        };

        repo.load_types(vec![document]).unwrap();
        repo
    }

    fn create_config_resolution(
        type_name: &str,
        kind: bsl_shared::domain::types::MetadataKind,
        facet: Option<bsl_shared::domain::types::FacetKind>,
    ) -> TypeResolution {
        use bsl_shared::domain::types::ConfigurationType;

        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Configuration(ConfigurationType {
                kind,
                name: type_name.to_string(),
                facet: None,
                attributes: vec![],
                tabular_sections: vec![],
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: facet,
            available_facets: vec![],
        }
    }

    #[test]
    fn test_tabular_sections_in_hover() {
        use bsl_shared::domain::types::{FacetKind, MetadataKind};

        let repo = create_test_repository_with_tabular_sections();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let resolution = create_config_resolution(
            "ЗаказНаряды",
            MetadataKind::Document,
            Some(FacetKind::Object),
        );

        let result = HoverBuilder::new(&config)
            .add_tabular_sections(&resolution, &metadata_lookup)
            .build();

        assert!(result.contains("Табличные части"));
        assert!(result.contains("Работы"));
        assert!(result.contains("Материалы"));
        assert!(result.contains("2 колонок"));
        assert!(result.contains("1 колонок"));
    }

    #[test]
    fn test_tabular_sections_empty_for_manager_facet() {
        use bsl_shared::domain::types::{FacetKind, MetadataKind};

        let repo = create_test_repository_with_tabular_sections();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let resolution = create_config_resolution(
            "ЗаказНаряды",
            MetadataKind::Document,
            Some(FacetKind::Manager),
        );

        let result = HoverBuilder::new(&config)
            .add_tabular_sections(&resolution, &metadata_lookup)
            .build();

        assert!(!result.contains("Табличные части"));
    }

    #[test]
    fn test_tabular_sections_only_for_detailed_level() {
        use bsl_shared::domain::types::{FacetKind, MetadataKind};

        let repo = create_test_repository_with_tabular_sections();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            ..Default::default()
        };

        let resolution = create_config_resolution(
            "ЗаказНаряды",
            MetadataKind::Document,
            Some(FacetKind::Object),
        );

        let result = HoverBuilder::new(&config)
            .add_tabular_sections(&resolution, &metadata_lookup)
            .build();

        assert!(!result.contains("Табличные части"));
    }
}
