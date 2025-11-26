//! MILESTONE 3.11 Phase 4: Tests for Hover Facet & Context Information
//!
//! Tests covering:
//! 1. Hover shows active facet with description
//! 2. Hover shows context requirements
//! 3. Methods are grouped/labeled by facet (if multiple facets)
//! 4. Context badges (🖥️ Server, 💻 Client, 🌐 Universal) in method list

#[cfg(test)]
mod hover_facets_tests {
    use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter, OutputFormat};
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::runtime_context::ContextRequirements;
    use bsl_shared::domain::types::{
        Certainty, ConcreteType, FacetKind, PlatformType, RawDataSource, RawMethodData,
        RawParamData, RawTypeData, ResolutionMetadata, ResolutionResult, ResolutionSource,
        TypeResolution,
    };
    use bsl_shared::formatting::DetailLevel;
    use std::sync::Arc;

    // ============================================================================
    // Helper: Создать repository с типом Справочник для тестирования
    // ============================================================================

    fn create_catalog_test_repository() -> Arc<InMemoryTypeRepository> {
        let repo = Arc::new(InMemoryTypeRepository::new());

        let methods = vec![
            // Manager facet methods
            RawMethodData {
                name: "СоздатьЭлемент".to_string(),
                english_name: "CreateItem".to_string(),
                return_type: "СправочникОбъект.Контрагенты".to_string(),
                params: vec![],
                description: Some("Создать новый элемент справочника".to_string()),
                is_deprecated: false,
                is_constructor: false,
                context_requirements: Some(ContextRequirements::ServerOnly),
                return_facet: Some(FacetKind::Object),
            },
            RawMethodData {
                name: "НайтиПоКоду".to_string(),
                english_name: "FindByCode".to_string(),
                return_type: "СправочникСсылка.Контрагенты".to_string(),
                params: vec![RawParamData {
                    name: "Код".to_string(),
                    param_type: "Строка".to_string(),
                    is_optional: false,
                    default_value: None,
                }],
                description: Some("Найти элемент по коду".to_string()),
                is_deprecated: false,
                is_constructor: false,
                context_requirements: Some(ContextRequirements::ServerOnly),
                return_facet: Some(FacetKind::Reference),
            },
            RawMethodData {
                name: "ПустаяСсылка".to_string(),
                english_name: "EmptyRef".to_string(),
                return_type: "СправочникСсылка.Контрагенты".to_string(),
                params: vec![],
                description: Some("Получить пустую ссылку".to_string()),
                is_deprecated: false,
                is_constructor: false,
                context_requirements: Some(ContextRequirements::Universal),
                return_facet: Some(FacetKind::Reference),
            },
        ];

        let test_type = RawTypeData {
            name: "СправочникМенеджер.Контрагенты".to_string(),
            english_name: "CatalogManager.Counterparties".to_string(),
            description: "Справочник Контрагенты (Manager facet)".to_string(),
            category: "Catalog".to_string(),
            source: RawDataSource::Platform,
            methods,
            properties: vec![],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_resolution_with_facet(facet: FacetKind) -> TypeResolution {
        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "СправочникМенеджер.Контрагенты".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(facet),
            available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
        }
    }

    // ============================================================================
    // TEST 4.1: Hover показывает активный фасет (только для DetailLevel::Detailed)
    // ============================================================================

    #[test]
    fn test_hover_shows_manager_facet() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Должен показывать фасет
        assert!(
            hover.contains("Фасет"),
            "Hover должен содержать секцию 'Фасет'"
        );
        assert!(
            hover.contains("Менеджер"),
            "Hover должен показывать 'Менеджер' для Manager facet"
        );
        assert!(
            hover.contains("создание, поиск"),
            "Hover должен показывать описание фасета"
        );
    }

    #[test]
    fn test_hover_shows_object_facet() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Object);

        let hover = formatter.format_variable("ОбъектКонтрагента", &resolution);

        assert!(hover.contains("Объект"), "Должен показывать 'Объект'");
        assert!(
            hover.contains("изменяемый объект"),
            "Должен показывать описание Object facet"
        );
    }

    #[test]
    fn test_hover_shows_reference_facet() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Reference);

        let hover = formatter.format_variable("СсылкаНаКонтрагента", &resolution);

        assert!(hover.contains("Ссылка"), "Должен показывать 'Ссылка'");
        assert!(
            hover.contains("ссылка на элемент"),
            "Должен показывать описание Reference facet"
        );
    }

    #[test]
    fn test_hover_shows_available_facets() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Должен показывать доступные фасеты
        assert!(
            hover.contains("Доступные фасеты"),
            "Hover должен показывать доступные фасеты"
        );
        assert!(
            hover.contains("Менеджер") && hover.contains("Объект"),
            "Должен перечислить доступные фасеты"
        );
    }

    // ============================================================================
    // TEST 4.2: Hover НЕ показывает фасеты для Compact/Full уровня
    // ============================================================================

    #[test]
    fn test_hover_compact_does_not_show_facets() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Compact level НЕ должен показывать фасеты
        assert!(
            !hover.contains("Фасет"),
            "Compact level НЕ должен показывать фасеты"
        );
    }

    #[test]
    fn test_hover_full_does_not_show_facets() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Full level НЕ должен показывать фасеты
        assert!(
            !hover.contains("Фасет"),
            "Full level НЕ должен показывать фасеты"
        );
    }

    // ============================================================================
    // TEST 4.3: Context badges в методах (только для Detailed level)
    // ============================================================================

    #[test]
    fn test_hover_shows_context_badges_for_methods() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Должны быть context badges
        assert!(
            hover.contains("🖥️ Server") || hover.contains("Server"),
            "ServerOnly методы должны иметь Server badge"
        );
        assert!(
            hover.contains("🌐 Universal") || hover.contains("Universal"),
            "Universal методы должны иметь Universal badge"
        );
    }

    #[test]
    fn test_hover_no_context_badges_for_full_level() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            max_methods: 10,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Full level НЕ должен показывать context badges
        assert!(
            !hover.contains("🖥️") && !hover.contains("🌐"),
            "Full level НЕ должен показывать context badges"
        );
    }

    // ============================================================================
    // TEST 4.4: Методы с разными context requirements
    // ============================================================================

    #[test]
    fn test_hover_shows_server_only_methods() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // ServerOnly метод
        assert!(
            hover.contains("СоздатьЭлемент"),
            "Должен показывать метод СоздатьЭлемент"
        );
        assert!(
            hover.contains("Server"),
            "СоздатьЭлемент должен иметь Server badge"
        );
    }

    #[test]
    fn test_hover_shows_universal_methods() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let resolution = create_test_resolution_with_facet(FacetKind::Manager);

        let hover = formatter.format_variable("СправочникКонтрагенты", &resolution);

        // Universal метод
        assert!(
            hover.contains("ПустаяСсылка"),
            "Должен показывать метод ПустаяСсылка"
        );
        assert!(
            hover.contains("Universal") || hover.contains("Везде"),
            "ПустаяСсылка должен иметь Universal badge"
        );
    }

    // ============================================================================
    // TEST 4.5: Edge cases
    // ============================================================================

    #[test]
    fn test_hover_without_active_facet() {
        let repo = create_catalog_test_repository();
        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        // Резолюция БЕЗ активного фасета
        let mut resolution = create_test_resolution_with_facet(FacetKind::Manager);
        resolution.active_facet = None;

        let hover = formatter.format_variable("Переменная", &resolution);

        // Не должно быть информации о фасете
        assert!(
            !hover.contains("Фасет:"),
            "Без active_facet не должно быть секции Фасет"
        );
    }

    #[test]
    fn test_hover_methods_without_context_requirements() {
        let repo = Arc::new(InMemoryTypeRepository::new());

        // Создаём тип с методами БЕЗ context_requirements
        let methods = vec![RawMethodData {
            name: "МетодБезКонтекста".to_string(),
            english_name: "MethodWithoutContext".to_string(),
            return_type: "Строка".to_string(),
            params: vec![],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None, // ← БЕЗ требований
            return_facet: None,
        }];

        let test_type = RawTypeData {
            name: "ТестовыйТип".to_string(),
            english_name: "TestType".to_string(),
            description: "Тип для тестирования".to_string(),
            category: "Test".to_string(),
            source: RawDataSource::Platform,
            methods,
            properties: vec![],
            facets: vec![],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            module_paths: None,
        };

        repo.load_types(vec![test_type]).unwrap();

        let metadata_lookup = TypeMetadataLookup::new(repo.clone());

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТестовыйТип".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let hover = formatter.format_variable("Переменная", &resolution);

        // Метод должен быть показан, но БЕЗ context badge
        assert!(
            hover.contains("МетодБезКонтекста"),
            "Метод должен быть показан"
        );
        // НЕ должно быть badges
        assert!(
            !hover.contains("🖥️") && !hover.contains("🌐") && !hover.contains("💻"),
            "Без context_requirements не должно быть badges"
        );
    }
}
