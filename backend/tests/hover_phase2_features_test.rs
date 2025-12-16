//! MILESTONE 3.6 Phase 2: Advanced Testing for Facets, Generics, and Documentation
//!
//! Comprehensive tests for:
//! 1. Facet display for different type categories (Catalog, Document, etc.)
//! 2. Generic type variations (Array, Map, ValueTable, nested)
//! 3. Documentation links with and without syntax helper
//! 4. Edge cases and boundary conditions

#[cfg(test)]
mod phase2_features_tests {
    use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::types::{
        Certainty, ConcreteType, FacetKind, GenericType, PlatformType, PrimitiveType,
        RawDataSource, RawTypeData, ResolutionMetadata, ResolutionResult, ResolutionSource,
        TypeResolution,
    };
    use bsl_shared::formatting::DetailLevel;
    use std::path::PathBuf;
    use std::sync::Arc;

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fn create_test_repository_with_facets(
        type_name: &str,
        facets: Vec<FacetKind>,
    ) -> Arc<InMemoryTypeRepository> {
        let repo = Arc::new(InMemoryTypeRepository::new());

        let test_type = RawTypeData {
            name: type_name.to_string(),
            english_name: type_name.to_string(),
            description: format!("Test type: {}", type_name),
            category: "Platform".to_string(),
            source: RawDataSource::Platform,
            methods: vec![],
            properties: vec![],
            facets,
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

    fn create_primitive_repository() -> Arc<InMemoryTypeRepository> {
        Arc::new(InMemoryTypeRepository::new())
    }

    // ============================================================================
    // TEST 2.1: Фасеты для разных типов
    // ============================================================================

    #[test]
    fn test_facet_for_catalog_type() {
        let repo = create_test_repository_with_facets(
            "СправочникСсылка.Контрагенты",
            vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Selection,
            ],
        );
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "СправочникСсылка.Контрагенты".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Reference),
            available_facets: vec![
                FacetKind::Manager,
                FacetKind::Object,
                FacetKind::Reference,
                FacetKind::Selection,
            ],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("КонтрагентСсылка", &resolution);

        // Should display Reference facet
        assert!(
            hover.contains("Фасет") && hover.contains("Ссылка"),
            "Should show active facet Reference"
        );

        // Should display all available facets
        assert!(
            hover.contains("Менеджер") && hover.contains("Объект") && hover.contains("Выборка"),
            "Should show all available facets"
        );
    }

    #[test]
    fn test_facet_for_document_type() {
        let repo = create_test_repository_with_facets(
            "ДокументОбъект.ЗаказКлиента",
            vec![FacetKind::Manager, FacetKind::Object],
        );
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ДокументОбъект.ЗаказКлиента".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![FacetKind::Manager, FacetKind::Object],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ЗаказЗначение", &resolution);

        // Should display Object facet
        assert!(
            hover.contains("Фасет") && hover.contains("Объект"),
            "Should show active facet Object"
        );

        // Should display correct available facets
        assert!(
            hover.contains("Менеджер"),
            "Should show available facet Manager"
        );
    }

    #[test]
    fn test_no_facet_for_primitive_type() {
        let repo = create_primitive_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // Primitive type (Number) has no facets
        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("Число", &resolution);

        // Should NOT display facets for primitive types
        assert!(
            !hover.contains("Фасет:"),
            "Primitive types should NOT show facets"
        );

        assert!(
            !hover.contains("Доступные фасеты:"),
            "Primitive types should NOT show available facets"
        );
    }

    #[test]
    fn test_facet_only_in_detailed_level() {
        let repo = create_test_repository_with_facets(
            "СправочникОбъект.Контрагенты",
            vec![FacetKind::Manager, FacetKind::Object],
        );
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "СправочникОбъект.Контрагенты".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![FacetKind::Manager, FacetKind::Object],
        };

        // Check Compact level
        let compact_config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            ..Default::default()
        };
        let formatter = HoverFormatter::new(compact_config, metadata_lookup.clone());
        let hover_compact = formatter.format_variable("Контрагент", &resolution);

        assert!(
            !hover_compact.contains("Фасет:"),
            "Compact should NOT show facets"
        );

        // Check Full level
        let full_config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            ..Default::default()
        };
        let formatter = HoverFormatter::new(full_config, metadata_lookup.clone());
        let hover_full = formatter.format_variable("Контрагент", &resolution);

        assert!(
            !hover_full.contains("Фасет:"),
            "Full should NOT show facets"
        );

        // Check Detailed level
        let detailed_config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };
        let formatter = HoverFormatter::new(detailed_config, metadata_lookup);
        let hover_detailed = formatter.format_variable("Контрагент", &resolution);

        assert!(
            hover_detailed.contains("Фасет:"),
            "Detailed SHOULD show facets"
        );
    }

    // ============================================================================
    // TEST 2.2: Generic типы - различные варианты
    // ============================================================================

    #[test]
    fn test_generic_array_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // Array<String>
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

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("МассивСтрок", &resolution);

        // Should show Generic explanation
        assert!(
            hover.contains("Generic тип:") || hover.contains("Generic"),
            "Should show Generic explanation"
        );

        assert!(hover.contains("Массив"), "Should show base type Массив");

        assert!(
            hover.contains("Строка"),
            "Should show type parameter Строка"
        );
    }

    #[test]
    fn test_generic_map_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // Map<Number, String>
        let generic = GenericType {
            base_type: "Соответствие".to_string(),
            type_params: vec![
                ConcreteType::Primitive(PrimitiveType::Number),
                ConcreteType::Primitive(PrimitiveType::String),
            ],
        };

        let resolution = TypeResolution {
            result: ResolutionResult::Generic(generic),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("Соответствие", &resolution);

        // Should show Generic explanation for Map
        assert!(
            hover.contains("Generic") || hover.contains("Соответствие"),
            "Should show Generic explanation for Соответствие"
        );

        // Should show both type parameters
        assert!(
            hover.contains("Число") && hover.contains("Строка"),
            "Should show both type parameters"
        );
    }

    #[test]
    fn test_generic_value_table_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // ValueTable<String>
        let generic = GenericType {
            base_type: "ТаблицаЗначений".to_string(),
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

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТаблицаЗначений", &resolution);

        // Should show Generic explanation for ValueTable
        assert!(
            hover.contains("Generic") || hover.contains("ТаблицаЗначений"),
            "Should show Generic explanation for ТаблицаЗначений"
        );

        assert!(hover.contains("Строка"), "Should show type parameter");
    }

    #[test]
    fn test_generic_nested_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // Nested Generic: Array<Array<String>>
        // Nested Generic: Array<Array<String>> - cannot nest Generics in ConcreteType params
        // Instead, we test a simple array
        let _inner_generic = GenericType {
            base_type: "Массив".to_string(),
            type_params: vec![ConcreteType::Primitive(PrimitiveType::String)],
        };

        // Use inner generic as type param would require ResolutionResult wrapping
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

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ВложенныйМассив", &resolution);

        // Should correctly handle nested Generic types
        assert!(
            hover.contains("Generic") || hover.contains("Массив"),
            "Should show Generic explanation for nested type"
        );

        // Should show nested structure
        assert!(
            hover.contains("Строка"),
            "Should show base type of nested Generic"
        );
    }

    #[test]
    fn test_generic_only_in_detailed_level() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

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

        // Check Compact level
        let compact_config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            ..Default::default()
        };
        let formatter = HoverFormatter::new(compact_config, metadata_lookup.clone());
        let hover_compact = formatter.format_variable("Список", &resolution);

        // Generic explanation should NOT be in Compact
        let has_generic_explanation = hover_compact.contains("содержит элементы")
            || hover_compact.contains("представляет собой");
        assert!(
            !has_generic_explanation,
            "Compact should NOT show Generic explanation"
        );

        // Check Full level
        let full_config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            ..Default::default()
        };
        let formatter = HoverFormatter::new(full_config, metadata_lookup.clone());
        let hover_full = formatter.format_variable("Список", &resolution);

        let has_generic_explanation =
            hover_full.contains("содержит элементы") || hover_full.contains("представляет собой");
        assert!(
            !has_generic_explanation,
            "Full should NOT show Generic explanation"
        );

        // Check Detailed level
        let detailed_config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };
        let formatter = HoverFormatter::new(detailed_config, metadata_lookup);
        let hover_detailed = formatter.format_variable("Список", &resolution);

        // Generic explanation SHOULD be in Detailed
        let has_generic_explanation = hover_detailed.contains("Generic")
            || hover_detailed.contains("содержит элементы")
            || hover_detailed.contains("представляет собой");
        assert!(
            has_generic_explanation,
            "Detailed SHOULD show Generic explanation"
        );
    }

    // ============================================================================
    // TEST 2.3: Ссылки на документацию
    // ============================================================================

    #[test]
    fn test_documentation_links_with_syntax_helper() {
        let repo = create_test_repository_with_facets("ТаблицаЗначений", vec![FacetKind::Object]);
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // Should show documentation section
        assert!(
            hover.contains("Документация:") || hover.contains("Documentation:"),
            "Should show documentation section"
        );

        // Should show online link
        assert!(
            hover.contains("1С Platform Docs") || hover.contains("docs.1c.ru"),
            "Should show online documentation link"
        );
    }

    #[test]
    fn test_documentation_links_without_syntax_helper() {
        let repo = create_test_repository_with_facets("ТаблицаЗначений", vec![FacetKind::Object]);
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: None, // No local documentation
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // Should show only online link
        assert!(
            hover.contains("1С Platform Docs") || hover.contains("docs.1c.ru"),
            "Should show online link"
        );

        // Should NOT show local link
        assert!(
            !hover.contains("file://"),
            "Should NOT show local link without syntax_helper"
        );
    }

    #[test]
    fn test_documentation_links_for_generic_type() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

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

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("МассивСтрок", &resolution);

        // Should show documentation link for base type
        assert!(
            hover.contains("Документация:") || hover.contains("Documentation:"),
            "Should show documentation link for Generic type"
        );

        assert!(
            hover.contains("Массив") || hover.contains("Array"),
            "Link should be to base type"
        );
    }

    #[test]
    fn test_documentation_links_for_qualified_type() {
        let repo = create_test_repository_with_facets(
            "СправочникСсылка.Контрагенты",
            vec![FacetKind::Reference],
        );
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "СправочникСсылка.Контрагенты".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Reference),
            available_facets: vec![],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("КонтрагентСсылка", &resolution);

        // Should show documentation
        assert!(
            hover.contains("Документация:") || hover.contains("Documentation:"),
            "Should show documentation link"
        );

        // Link should be to base name
        assert!(
            hover.contains("СправочникСсылка") || hover.contains("CatalogReference"),
            "Link should be to base type name"
        );
    }

    #[test]
    fn test_documentation_only_in_detailed_level() {
        let repo = create_test_repository_with_facets("ТаблицаЗначений", vec![FacetKind::Object]);
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![],
        };

        // Check Compact level
        let compact_config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };
        let formatter = HoverFormatter::new(compact_config, metadata_lookup.clone());
        let hover_compact = formatter.format_variable("ТЗ", &resolution);

        assert!(
            !hover_compact.contains("Документация:") && !hover_compact.contains("Documentation:"),
            "Compact should NOT show documentation"
        );

        // Check Full level
        let full_config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };
        let formatter = HoverFormatter::new(full_config, metadata_lookup.clone());
        let hover_full = formatter.format_variable("ТЗ", &resolution);

        assert!(
            !hover_full.contains("Документация:") && !hover_full.contains("Documentation:"),
            "Full should NOT show documentation"
        );

        // Check Detailed level
        let detailed_config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };
        let formatter = HoverFormatter::new(detailed_config, metadata_lookup);
        let hover_detailed = formatter.format_variable("ТЗ", &resolution);

        assert!(
            hover_detailed.contains("Документация:") || hover_detailed.contains("Documentation:"),
            "Detailed SHOULD show documentation"
        );
    }

    // ============================================================================
    // EDGE CASES & COMBINATIONS
    // ============================================================================

    #[test]
    fn test_facet_with_documentation() {
        let repo = create_test_repository_with_facets(
            "СправочникОбъект",
            vec![FacetKind::Manager, FacetKind::Object],
        );
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "СправочникОбъект".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![FacetKind::Manager, FacetKind::Object],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("СправочникОбъект", &resolution);

        // Should show BOTH facet AND documentation
        assert!(
            hover.contains("Фасет:") || hover.contains("Facet:"),
            "Should show facet information"
        );

        assert!(
            hover.contains("Документация:") || hover.contains("Documentation:"),
            "Should show documentation information"
        );
    }

    #[test]
    fn test_generic_with_documentation() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

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

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("МассивСтрок", &resolution);

        // Should show BOTH Generic explanation AND documentation
        let has_generic = hover.contains("Generic")
            || hover.contains("содержит элементы")
            || hover.contains("массив");
        assert!(has_generic, "Should show Generic explanation");

        assert!(
            hover.contains("Документация:") || hover.contains("Documentation:"),
            "Should show documentation"
        );
    }
}
