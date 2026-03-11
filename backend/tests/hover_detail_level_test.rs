//! MILESTONE 3.6 Phase 1: Comprehensive Testing for Settings & Detail Levels
//!
//! Tests covering:
//! 1. DetailLevel enum parsing and behavior
//! 2. HoverFormatter with different detail levels
//! 3. Multiline method formatting
//! 4. showCertainty flag behavior
//! 5. Configuration defaults and boundaries

#[cfg(test)]
mod detail_level_tests {
    use bsl_backend::helpers::hover_formatter::{
        HoverFormatConfig, HoverFormatter, HoverOutputFormat,
    };
    use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::types::{
        Certainty, ConcreteType, FacetKind, PlatformType, RawDataSource, RawMethodData,
        RawPropertyData, RawTypeData, ResolutionMetadata, ResolutionResult, ResolutionSource,
        TypeResolution,
    };
    use bsl_shared::formatting::DetailLevel;
    use std::sync::Arc;

    // ============================================================================
    // TEST 1.1: DetailLevel::parse() - Парсинг уровней детализации
    // ============================================================================

    #[test]
    fn test_detail_level_from_str_compact() {
        assert_eq!(DetailLevel::parse("compact"), DetailLevel::Compact);
    }

    #[test]
    fn test_detail_level_from_str_full() {
        assert_eq!(DetailLevel::parse("full"), DetailLevel::Full);
    }

    #[test]
    fn test_detail_level_from_str_detailed() {
        assert_eq!(DetailLevel::parse("detailed"), DetailLevel::Detailed);
    }

    #[test]
    fn test_detail_level_case_insensitive_compact() {
        assert_eq!(DetailLevel::parse("COMPACT"), DetailLevel::Compact);
        assert_eq!(DetailLevel::parse("Compact"), DetailLevel::Compact);
        assert_eq!(DetailLevel::parse("CoMpAcT"), DetailLevel::Compact);
    }

    #[test]
    fn test_detail_level_case_insensitive_full() {
        assert_eq!(DetailLevel::parse("FULL"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse("Full"), DetailLevel::Full);
    }

    #[test]
    fn test_detail_level_case_insensitive_detailed() {
        assert_eq!(DetailLevel::parse("DETAILED"), DetailLevel::Detailed);
        assert_eq!(DetailLevel::parse("Detailed"), DetailLevel::Detailed);
    }

    #[test]
    fn test_detail_level_default_fallback_unknown() {
        // Неизвестное значение должно fallback к Full (по умолчанию)
        assert_eq!(DetailLevel::parse("unknown"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse("invalid"), DetailLevel::Full);
        assert_eq!(DetailLevel::parse(""), DetailLevel::Full);
    }

    // ============================================================================
    // TEST 1.2: HoverFormatter с разными DetailLevel
    // ============================================================================

    fn create_test_repository_with_methods(method_count: usize) -> Arc<InMemoryTypeRepository> {
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
            name: "ТаблицаЗначений".to_string(),
            english_name: "ValueTable".to_string(),
            description: "Таблица значений для тестирования".to_string(),
            category: "Platform".to_string(),
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
            metadata_path: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_repository_with_methods_and_properties(
        method_count: usize,
        property_count: usize,
    ) -> Arc<InMemoryTypeRepository> {
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

        let properties: Vec<RawPropertyData> = (0..property_count)
            .map(|i| RawPropertyData {
                name: format!("Свойство{}", i),
                prop_type: "Строка".to_string(),
                is_readonly: false,
            })
            .collect();

        let test_type = RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            english_name: "ValueTable".to_string(),
            description: "Таблица значений для тестирования".to_string(),
            category: "Platform".to_string(),
            source: RawDataSource::Platform,
            methods,
            properties,
            facets: vec![FacetKind::Object],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
            metadata_path: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    fn create_test_resolution() -> TypeResolution {
        TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    #[test]
    fn test_hover_compact_level_only_type() {
        let repo = create_test_repository_with_methods(10);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            show_certainty: false,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Compact должен содержать тип
        assert!(
            hover.contains("ТаблицаЗначений"),
            "Compact должен показывать тип"
        );

        // ❌ Compact НЕ должен содержать методы
        assert!(
            !hover.contains("Методы"),
            "Compact НЕ должен показывать методы"
        );

        // ❌ Compact НЕ должен содержать свойства
        assert!(
            !hover.contains("Свойства"),
            "Compact НЕ должен показывать свойства"
        );
    }

    #[test]
    fn test_hover_full_level_with_methods() {
        let repo = create_test_repository_with_methods(10);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            max_methods: 10,
            show_certainty: false,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Full должен содержать тип
        assert!(
            hover.contains("ТаблицаЗначений"),
            "Full должен показывать тип"
        );

        // ✅ Full ДОЛЖЕН содержать методы
        assert!(hover.contains("Методы"), "Full ДОЛЖЕН показывать методы");

        // ❌ Full НЕ должен содержать свойства
        assert!(
            !hover.contains("Свойства"),
            "Full НЕ должен показывать свойства"
        );
    }

    #[test]
    fn test_hover_detailed_level_with_methods_and_properties() {
        let repo = create_test_repository_with_methods_and_properties(10, 5);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            max_methods: 10,
            max_properties: 5,
            show_certainty: false,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Detailed должен содержать тип
        assert!(
            hover.contains("ТаблицаЗначений"),
            "Detailed должен показывать тип"
        );

        // ✅ Detailed ДОЛЖЕН содержать методы
        assert!(
            hover.contains("Методы"),
            "Detailed ДОЛЖЕН показывать методы"
        );

        // ✅ Detailed ДОЛЖЕН содержать свойства
        assert!(
            hover.contains("Свойства"),
            "Detailed ДОЛЖЕН показывать свойства"
        );
    }

    // ============================================================================
    // TEST 1.3: Multiline форматирование методов (4+ параметров)
    // ============================================================================

    fn create_repository_with_multiline_methods() -> Arc<InMemoryTypeRepository> {
        use bsl_shared::domain::types::RawParamData;

        let repo = Arc::new(InMemoryTypeRepository::new());

        // Метод с 3 параметрами (inline)
        let method_3_params = RawMethodData {
            name: "ВставитьКолонку3Парама".to_string(),
            english_name: "InsertColumn3Params".to_string(),
            return_type: "Число".to_string(),
            params: vec![
                RawParamData {
                    name: "Колонка".to_string(),
                    param_type: "Строка".to_string(),
                    is_optional: false,
                    default_value: None,
                },
                RawParamData {
                    name: "Позиция".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: false,
                    default_value: None,
                },
                RawParamData {
                    name: "Ширина".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: true,
                    default_value: Some("0".to_string()),
                },
            ],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        };

        // Метод с 4 параметрами (multiline)
        let method_4_params = RawMethodData {
            name: "ВставитьКолонку4Парама".to_string(),
            english_name: "InsertColumn4Params".to_string(),
            return_type: "Число".to_string(),
            params: vec![
                RawParamData {
                    name: "Колонка".to_string(),
                    param_type: "Строка".to_string(),
                    is_optional: false,
                    default_value: None,
                },
                RawParamData {
                    name: "Позиция".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: false,
                    default_value: None,
                },
                RawParamData {
                    name: "Ширина".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: true,
                    default_value: Some("0".to_string()),
                },
                RawParamData {
                    name: "Тип".to_string(),
                    param_type: "Строка".to_string(),
                    is_optional: true,
                    default_value: Some("\"\"".to_string()),
                },
            ],
            description: None,
            is_deprecated: false,
            is_constructor: false,
            context_requirements: None,
            return_facet: None,
        };

        let test_type = RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            english_name: "ValueTable".to_string(),
            description: "Таблица значений".to_string(),
            category: "Platform".to_string(),
            source: RawDataSource::Platform,
            methods: vec![method_3_params, method_4_params],
            properties: vec![],
            facets: vec![FacetKind::Object],
            kind: None,
            attributes: vec![],
            tabular_sections: vec![],
            enum_values: vec![],
            generic_info: None,
            collection_item_type: None,
            module_paths: None,
            metadata_path: None,
        };

        repo.load_types(vec![test_type]).unwrap();
        repo
    }

    #[test]
    fn test_method_inline_format_3_params() {
        let repo = create_repository_with_multiline_methods();
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Метод с 3 параметрами должен быть в inline формате
        assert!(
            hover.contains("ВставитьКолонку3Парама"),
            "Должен содержать имя метода"
        );

        // ✅ Проверяем что это inline формат (параметры на одной строке)
        // Ищем строку вида: "ВставитьКолонку3Парама(Колонка: Строка, Позиция: Число, Ширина: Число = 0) → Число"
        // НЕ должны быть разделены новыми строками
        let lines: Vec<&str> = hover.lines().collect();
        let method_line = lines
            .iter()
            .find(|line| line.contains("ВставитьКолонку3Парама"))
            .expect("Должна быть строка с методом");

        assert!(
            method_line.contains("(") && method_line.contains(")"),
            "Inline формат должен иметь параметры на одной строке"
        );
    }

    #[test]
    fn test_method_multiline_format_4_params() {
        let repo = create_repository_with_multiline_methods();
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Метод с 4 параметрами должен быть в multiline формате
        assert!(
            hover.contains("ВставитьКолонку4Парама"),
            "Должен содержать имя метода"
        );

        // ✅ Multiline формат должен содержать открывающую скобку с переносом
        // Должна быть строка вида: "ВставитьКолонку4Парама("
        // Затем строки с параметрами
        // Затем закрывающая скобка
        assert!(
            hover.contains("\n    "),
            "Multiline формат должен содержать отступы для параметров"
        );
    }

    // ============================================================================
    // TEST 1.4: showCertainty flag
    // ============================================================================

    #[test]
    fn test_certainty_shown_when_enabled() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            show_certainty: true, // ✅ Включено
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Должна быть строка с уверенностью (certainty)
        assert!(
            hover.contains("🟢") || hover.contains("Уверенность"),
            "Должна показывать уверенность когда show_certainty=true"
        );
    }

    #[test]
    fn test_certainty_hidden_when_disabled() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            show_certainty: false, // ❌ Отключено
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ❌ НЕ должна быть строка с уверенностью
        assert!(
            !hover.contains("Уверенность") && !hover.contains("Known") && !hover.contains("🟢"),
            "НЕ должна показывать уверенность когда show_certainty=false"
        );
    }

    #[test]
    fn test_certainty_different_levels() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Compact,
            show_certainty: true,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);

        // Тест Known
        let resolution_known = TypeResolution {
            certainty: Certainty::Known,
            ..create_test_resolution()
        };
        let hover_known = formatter.format_variable("ТЗ", &resolution_known);
        assert!(
            hover_known.contains("Known") || hover_known.contains("🟢"),
            "Should show Known certainty"
        );

        // Тест Inferred
        let resolution_inferred = TypeResolution {
            certainty: Certainty::Inferred,
            ..create_test_resolution()
        };
        let hover_inferred = formatter.format_variable("ТЗ", &resolution_inferred);
        assert!(
            hover_inferred.contains("Inferred") || hover_inferred.contains("🟡"),
            "Should show Inferred certainty"
        );

        // Тест InferredWeak
        let resolution_inferred_weak = TypeResolution {
            certainty: Certainty::InferredWeak,
            ..create_test_resolution()
        };
        let hover_inferred_weak = formatter.format_variable("ТЗ", &resolution_inferred_weak);
        assert!(
            hover_inferred_weak.contains("InferredWeak") || hover_inferred_weak.contains("🟠"),
            "Should show InferredWeak certainty"
        );

        // Тест Unknown
        let resolution_unknown = TypeResolution {
            certainty: Certainty::Unknown,
            ..create_test_resolution()
        };
        let hover_unknown = formatter.format_variable("ТЗ", &resolution_unknown);
        assert!(
            hover_unknown.contains("Unknown") || hover_unknown.contains("⚪"),
            "Should show Unknown certainty"
        );
    }

    // ============================================================================
    // TEST 2.1-2.2: Integration Tests for Configuration
    // ============================================================================

    #[test]
    fn test_hover_format_config_defaults() {
        let config = HoverFormatConfig::default();
        assert_eq!(config.max_methods, 10);
        assert_eq!(config.max_properties, 5);
        assert_eq!(config.detail_level, DetailLevel::Detailed);
        assert!(config.show_certainty);
        assert_eq!(config.output_format, HoverOutputFormat::Markdown);
    }

    #[test]
    fn test_hover_format_config_custom_values() {
        let config = HoverFormatConfig {
            max_methods: 5,
            max_properties: 3,
            detail_level: DetailLevel::Detailed,
            show_certainty: false,
            output_format: HoverOutputFormat::PlainText,
            ..Default::default()
        };

        assert_eq!(config.max_methods, 5);
        assert_eq!(config.max_properties, 3);
        assert_eq!(config.detail_level, DetailLevel::Detailed);
        assert!(!config.show_certainty);
        assert_eq!(config.output_format, HoverOutputFormat::PlainText);
    }

    // ============================================================================
    // TEST 3: Edge Cases
    // ============================================================================

    #[test]
    fn test_edge_case_max_methods_zero() {
        let repo = create_test_repository_with_methods(10);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            max_methods: 0, // Граничный случай: 0 методов
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Должно корректно обработать ноль методов
        assert!(hover.contains("ТаблицаЗначений"));
        // НЕ должно падать - это основная проверка
    }

    #[test]
    fn test_edge_case_max_methods_exceeds_available() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            max_methods: 100, // Больше чем есть методов
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Должно показать все доступные методы без ошибки
        assert!(hover.contains("Метод0"));
        assert!(hover.contains("Метод4"));
        assert!(
            !hover.contains("... и ещё"),
            "Не должно быть сообщения о дополнительных методах"
        );
    }

    #[test]
    fn test_edge_case_empty_methods_list() {
        let repo = create_test_repository_with_methods(0);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Должно корректно обработать отсутствие методов
        assert!(hover.contains("ТаблицаЗначений"));
        assert!(
            hover.contains("Методы недоступны"),
            "Должно быть предупреждение про отсутствие методов без syntax_helper"
        );
    }

    // ============================================================================
    // MILESTONE 3.6 Phase 2: Тесты для фасетов, Generic типов и документации
    // ============================================================================

    #[test]
    fn test_facet_info_shown_in_detailed_level() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // TypeResolution с активным фасетом Object и доступными фасетами
        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object), // ← Активный фасет
            available_facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference], // ← Доступные фасеты
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed, // Только в Detailed
            show_certainty: false,
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Должен показывать активный фасет
        assert!(hover.contains("Фасет:"), "Detailed должен показывать фасет");
        assert!(
            hover.contains("Объект"),
            "Должен показывать активный фасет Объект"
        );
        assert!(
            hover.contains("изменяемый объект"),
            "Должен показывать описание фасета"
        );

        // ✅ Должен показывать доступные фасеты
        assert!(
            hover.contains("Доступные фасеты:"),
            "Должен показывать доступные фасеты"
        );
        assert!(
            hover.contains("Менеджер") && hover.contains("Ссылка"),
            "Должен показывать все доступные фасеты на русском"
        );
    }

    #[test]
    fn test_facet_info_not_shown_in_full_level() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);

        let resolution = TypeResolution {
            result: ResolutionResult::Concrete(ConcreteType::Platform(PlatformType {
                name: "ТаблицаЗначений".to_string(),
            })),
            certainty: Certainty::Known,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: Some(FacetKind::Object),
            available_facets: vec![FacetKind::Manager, FacetKind::Object],
        };

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full, // НЕ Detailed
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ❌ НЕ должен показывать фасеты в Full режиме
        assert!(
            !hover.contains("Фасет:"),
            "Full НЕ должен показывать фасеты"
        );
    }

    #[test]
    fn test_generic_info_shown_in_detailed_level() {
        use bsl_shared::domain::types::{GenericType, PrimitiveType};

        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);

        // Generic тип: Массив<Строка>
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
            detail_level: DetailLevel::Detailed, // Только в Detailed
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("МассивСтрок", &resolution);

        // ✅ Должен показывать Generic тип пояснение
        assert!(
            hover.contains("Generic тип:"),
            "Detailed должен показывать Generic пояснение"
        );
        assert!(
            hover.contains("Базовый тип: Массив"),
            "Должен показывать базовый тип"
        );
        assert!(
            hover.contains("Параметры типа: Строка"),
            "Должен показывать параметры типа"
        );
        assert!(
            hover.contains("массив содержит элементы определённого типа"),
            "Должен показывать объяснение Generic типа"
        );
    }

    #[test]
    fn test_generic_info_not_shown_in_compact_level() {
        use bsl_shared::domain::types::{GenericType, PrimitiveType};

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
            detail_level: DetailLevel::Compact, // НЕ Detailed
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("МассивСтрок", &resolution);

        // ❌ НЕ должен показывать Generic пояснение в Compact режиме
        assert!(
            !hover.contains("Generic тип:"),
            "Compact НЕ должен показывать Generic пояснение"
        );
    }

    #[test]
    fn test_documentation_links_shown_in_detailed_level() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Detailed, // Только в Detailed
            syntax_helper_path: None,            // Без локальной документации
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ✅ Должен показывать ссылку на онлайн документацию
        assert!(
            hover.contains("Документация:"),
            "Detailed должен показывать секцию документации"
        );
        assert!(
            hover.contains("1С Platform Docs"),
            "Должен показывать ссылку на онлайн документацию"
        );
        assert!(
            hover.contains("https://docs.1c.ru/search?q="),
            "Должен содержать URL онлайн документации"
        );
    }

    #[test]
    fn test_documentation_links_not_shown_in_full_level() {
        let repo = create_test_repository_with_methods(5);
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let resolution = create_test_resolution();

        let config = HoverFormatConfig {
            detail_level: DetailLevel::Full, // НЕ Detailed
            ..Default::default()
        };

        let formatter = HoverFormatter::new(config, metadata_lookup);
        let hover = formatter.format_variable("ТЗ", &resolution);

        // ❌ НЕ должен показывать документацию в Full режиме
        assert!(
            !hover.contains("Документация:"),
            "Full НЕ должен показывать документацию"
        );
    }

    #[test]
    fn test_syntax_helper_path_in_config() {
        use std::path::PathBuf;

        let config = HoverFormatConfig {
            syntax_helper_path: Some(PathBuf::from("examples/syntax_helper")),
            ..Default::default()
        };

        assert!(config.syntax_helper_path.is_some());
        assert_eq!(
            config.syntax_helper_path.unwrap().to_str().unwrap(),
            "examples/syntax_helper"
        );
    }
}
