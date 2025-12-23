//! Тесты для SignatureIndex
//!
//! Все тесты из оригинального модуля сохранены без изменений.

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::domain::signature_index::{
        ConstructorSignature, ContextRequirements, MethodSignature, SignatureIndex, SignatureSource,
    };
    use crate::domain::type_id::TypeId;
    use crate::domain::types::{FacetKind, MetadataKind, ParameterInfo, TypeResolution};

    fn add_test_constructors(index: &mut SignatureIndex) {
        index.add_constructor(
            TypeId::new("Массив"),
            ConstructorSignature {
                type_name: "Массив".to_string(),
                params: vec![ParameterInfo {
                    name: "Размер".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: true,
                    default_value: None,
                    description: None,
                }],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );
        index.add_constructor(
            TypeId::new("Соответствие"),
            ConstructorSignature {
                type_name: "Соответствие".to_string(),
                params: vec![],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 2,
            },
        );
        index.add_constructor(
            TypeId::new("ТаблицаЗначений"),
            ConstructorSignature {
                type_name: "ТаблицаЗначений".to_string(),
                params: vec![],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: false,
                generic_params_count: 0,
            },
        );
        index.add_constructor(
            TypeId::new("СписокЗначений"),
            ConstructorSignature {
                type_name: "СписокЗначений".to_string(),
                params: vec![],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );
        index.add_constructor(
            TypeId::new("ФиксированныйМассив"),
            ConstructorSignature {
                type_name: "ФиксированныйМассив".to_string(),
                params: vec![ParameterInfo {
                    name: "Массив".to_string(),
                    type_name: Some("Массив".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                }],
                facet: None,
                source: SignatureSource::Platform,
                is_collection: true,
                generic_params_count: 1,
            },
        );
    }

    #[test]
    fn test_signature_index_basic() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("Массив"), sig);

        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Добавить");
    }

    #[test]
    fn test_signature_index_case_insensitive() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("Массив"), sig);

        // Разный регистр должен работать
        let found = index.find_method("Массив", "добавить");
        assert!(found.is_some());

        let found2 = index.find_method("Массив", "ДОБАВИТЬ");
        assert!(found2.is_some());
    }

    #[test]
    fn test_signature_index_not_found() {
        let index = SignatureIndex::new();

        let found = index.find_method("Массив", "НесуществующийМетод");
        assert!(found.is_none());
    }

    #[test]
    fn test_add_and_find_constructor() {
        let mut index = SignatureIndex::new();

        let constructor = ConstructorSignature {
            type_name: "Массив".to_string(),
            params: vec![],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        };

        index.add_constructor(TypeId::new("Массив"), constructor);

        let found = index.find_constructor("Массив");
        assert!(found.is_some());
        assert_eq!(found.unwrap().type_name, "Массив");
    }

    #[test]
    fn test_find_constructor_case_insensitive() {
        let mut index = SignatureIndex::new();
        add_test_constructors(&mut index);

        // Поиск в разных регистрах
        assert!(index.find_constructor("Массив").is_some());
        assert!(index.find_constructor("массив").is_some());
        assert!(index.find_constructor("МАССИВ").is_some());
    }

    #[test]
    fn test_is_collection_type() {
        let mut index = SignatureIndex::new();
        add_test_constructors(&mut index);

        assert!(index.is_collection_type("Массив"));
        assert!(index.is_collection_type("Соответствие"));
        assert!(!index.is_collection_type("ТаблицаЗначений"));
    }

    #[test]
    fn test_get_generic_params_count() {
        let mut index = SignatureIndex::new();
        add_test_constructors(&mut index);

        assert_eq!(index.get_generic_params_count("Массив"), Some(1));
        assert_eq!(index.get_generic_params_count("Соответствие"), Some(2));
        assert_eq!(index.get_generic_params_count("ТаблицаЗначений"), Some(0));
    }

    #[test]
    fn test_constructors_added() {
        let mut index = SignatureIndex::new();
        add_test_constructors(&mut index);

        // Проверяем что все тестовые конструкторы добавлены
        assert!(index.find_constructor("Массив").is_some());
        assert!(index.find_constructor("Соответствие").is_some());
        assert!(index.find_constructor("ТаблицаЗначений").is_some());
        assert!(index.find_constructor("СписокЗначений").is_some());
        assert!(index.find_constructor("ФиксированныйМассив").is_some());
    }

    #[test]
    fn test_method_overloads_are_kept_and_validated() {
        use crate::domain::signature_registry::SignatureDataSource;
        use crate::domain::types::{RawMethodData, RawParamData, RawTypeData};
        use crate::domain::SignatureSourceRegistry;

        struct TestSource;
        impl SignatureDataSource for TestSource {
            fn name(&self) -> &str {
                "TestSource"
            }
            fn priority(&self) -> u32 {
                10
            }
            fn load(&self) -> Vec<RawTypeData> {
                vec![RawTypeData {
                    name: "ТабличнаяЧасть".to_string(),
                    methods: vec![
                        // overload 1: Выгрузить(Строки?: Массив, Колонки?: Строка)
                        RawMethodData {
                            name: "Выгрузить".to_string(),
                            return_type: "ТаблицаЗначений".to_string(),
                            params: vec![
                                RawParamData {
                                    name: "Строки".to_string(),
                                    param_type: "Массив".to_string(),
                                    is_optional: true,
                                    ..Default::default()
                                },
                                RawParamData {
                                    name: "Колонки".to_string(),
                                    param_type: "Строка".to_string(),
                                    is_optional: true,
                                    ..Default::default()
                                },
                            ],
                            ..Default::default()
                        },
                        // overload 2: Выгрузить(ПараметрыОтбора?: Структура, Колонки?: Строка)
                        RawMethodData {
                            name: "Выгрузить".to_string(),
                            return_type: "ТаблицаЗначений".to_string(),
                            params: vec![
                                RawParamData {
                                    name: "ПараметрыОтбора".to_string(),
                                    param_type: "Структура".to_string(),
                                    is_optional: true,
                                    ..Default::default()
                                },
                                RawParamData {
                                    name: "Колонки".to_string(),
                                    param_type: "Строка".to_string(),
                                    is_optional: true,
                                    ..Default::default()
                                },
                            ],
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                }]
            }
        }

        let index = SignatureSourceRegistry::new().register(TestSource).build();

        let overloads = index.find_methods("ТабличнаяЧасть", "Выгрузить");
        assert_eq!(overloads.len(), 2);

        // actual: Выгрузить(Массив, Строка)
        let actual = MethodSignature::new(
            "Выгрузить".to_string(),
            Some("ТабличнаяЧасть".to_string()),
            vec![
                ParameterInfo {
                    name: "arg0".to_string(),
                    type_name: Some("Массив".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                },
                ParameterInfo {
                    name: "arg1".to_string(),
                    type_name: Some("Строка".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                },
            ],
            Some("ТаблицаЗначений".to_string()),
            None,
            None,
            SignatureSource::UserCode,
            None,
            ContextRequirements::default(),
        );

        assert!(
            matches!(
                index.validate_overloaded_signature(&overloads, &actual),
                crate::domain::signature_index::SignatureValidationResult::Valid
            ),
            "Вызов должен пройти валидацию по одному из overload'ов"
        );
    }

    // ================= Milestone 3.11 Phase 2: Faceted Type Support =================

    #[test]
    fn test_extract_base_facet_type_catalog() {
        // Справочники
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникМенеджер.Контрагенты"),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникОбъект.Номенклатура"),
            Some("СправочникОбъект")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникСсылка.Валюты"),
            Some("СправочникСсылка")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникВыборка.Контрагенты"),
            Some("СправочникВыборка")
        );
    }

    #[test]
    fn test_extract_base_facet_type_document() {
        // Документы
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ДокументМенеджер.ЗаказКлиента"),
            Some("ДокументМенеджер")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ДокументОбъект.РасходнаяНакладная"),
            Some("ДокументОбъект")
        );
    }

    #[test]
    fn test_extract_base_facet_type_registers() {
        // Регистры сведений
        assert_eq!(
            SignatureIndex::extract_base_facet_type("РегистрСведенийМенеджер.КурсыВалют"),
            Some("РегистрСведенийМенеджер")
        );
        // Регистры накопления
        assert_eq!(
            SignatureIndex::extract_base_facet_type("РегистрНакопленияМенеджер.ОстаткиТоваров"),
            Some("РегистрНакопленияМенеджер")
        );
    }

    #[test]
    fn test_extract_base_facet_type_non_faceted() {
        // Не-фасетные типы должны возвращать None
        assert_eq!(SignatureIndex::extract_base_facet_type("Массив"), None);
        assert_eq!(SignatureIndex::extract_base_facet_type("Строка"), None);
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ТаблицаЗначений"),
            None
        );
        // Базовые фасетные типы без имени тоже None
        assert_eq!(
            SignatureIndex::extract_base_facet_type("СправочникМенеджер"),
            None
        );
    }

    #[test]
    fn test_substitute_type_name() {
        // Базовые фасетные типы
        assert_eq!(
            SignatureIndex::substitute_type_name("СправочникОбъект", "Контрагенты"),
            "СправочникОбъект.Контрагенты"
        );
        assert_eq!(
            SignatureIndex::substitute_type_name("СправочникСсылка", "Номенклатура"),
            "СправочникСсылка.Номенклатура"
        );
        assert_eq!(
            SignatureIndex::substitute_type_name("ДокументОбъект", "ЗаказКлиента"),
            "ДокументОбъект.ЗаказКлиента"
        );

        // Не-фасетные типы остаются как есть
        assert_eq!(
            SignatureIndex::substitute_type_name("Неопределено", "Контрагенты"),
            "Неопределено"
        );
        assert_eq!(
            SignatureIndex::substitute_type_name("Строка", "Контрагенты"),
            "Строка"
        );
    }

    #[test]
    fn test_extract_metadata_name() {
        assert_eq!(
            SignatureIndex::extract_metadata_name("СправочникМенеджер.Контрагенты"),
            Some("Контрагенты")
        );
        assert_eq!(
            SignatureIndex::extract_metadata_name("ДокументОбъект.ЗаказКлиента"),
            Some("ЗаказКлиента")
        );
        // Не-фасетные типы
        assert_eq!(SignatureIndex::extract_metadata_name("Массив"), None);
        assert_eq!(
            SignatureIndex::extract_metadata_name("Файл.Существует"),
            None
        ); // Файл - не фасетный тип
    }

    #[test]
    fn test_find_method_faceted_fallback() {
        let mut index = SignatureIndex::new();

        // Добавляем метод под базовым типом (как в platform_types.rs)
        let sig = MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig);

        // Поиск по точному имени (базовый тип) - должен найти
        let found_exact = index.find_method("СправочникМенеджер", "СоздатьЭлемент");
        assert!(found_exact.is_some());

        // Поиск по конкретизированному типу (fallback к базовому) - тоже должен найти
        let found_fallback = index.find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент");
        assert!(found_fallback.is_some());
        assert_eq!(found_fallback.unwrap().name, "СоздатьЭлемент");
        assert_eq!(
            found_fallback.unwrap().return_type,
            Some("СправочникОбъект".to_string())
        );
    }

    #[test]
    fn test_find_method_document_faceted() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Провести".to_string(),
            Some("ДокументОбъект".to_string()),
            vec![],
            Some("Неопределено".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("ДокументОбъект"), sig);

        // Поиск через конкретизированный тип
        let found = index.find_method("ДокументОбъект.ЗаказКлиента", "Провести");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Провести");
    }

    #[test]
    fn test_find_method_non_faceted_still_works() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method(TypeId::new("Массив"), sig);

        // Обычный поиск по не-фасетному типу должен работать как раньше
        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Добавить");
    }

    // ================= MILESTONE 3.11: Pattern-Matching Functions =================

    #[test]
    fn test_get_facet_kind_from_prefix_manager() {
        // Все менеджеры
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникМенеджер"),
            Some(FacetKind::Manager)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументМенеджер"),
            Some(FacetKind::Manager)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийМенеджер"),
            Some(FacetKind::Manager)
        );
        // МенеджерЗаписи тоже Manager
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийМенеджерЗаписи"),
            Some(FacetKind::Manager)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_object() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникОбъект"),
            Some(FacetKind::Object)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументОбъект"),
            Some(FacetKind::Object)
        );
        // Запись регистра - тоже Object
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийЗапись"),
            Some(FacetKind::Object)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_reference() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникСсылка"),
            Some(FacetKind::Reference)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументСсылка"),
            Some(FacetKind::Reference)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ПеречислениеСсылка"),
            Some(FacetKind::Reference)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_selection() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникВыборка"),
            Some(FacetKind::Selection)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументВыборка"),
            Some(FacetKind::Selection)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_list() {
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("СправочникСписок"),
            Some(FacetKind::List)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ДокументСписок"),
            Some(FacetKind::List)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_collection() {
        // Наборы записей - Collection
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрСведенийНаборЗаписей"),
            Some(FacetKind::Collection)
        );
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("РегистрНакопленияНаборЗаписей"),
            Some(FacetKind::Collection)
        );
    }

    #[test]
    fn test_get_facet_kind_from_prefix_non_faceted() {
        // Не-фасетные типы -> None
        assert_eq!(SignatureIndex::get_facet_kind_from_prefix("Массив"), None);
        assert_eq!(SignatureIndex::get_facet_kind_from_prefix("Строка"), None);
        assert_eq!(
            SignatureIndex::get_facet_kind_from_prefix("ТаблицаЗначений"),
            None
        );
    }

    #[test]
    fn test_resolve_metadata_kind_instance_method() {
        use crate::domain::metadata_patterns::ExtractedPattern;

        let mut index = SignatureIndex::new();

        index.update_metadata_patterns(vec![
            ExtractedPattern {
                prefix: "Справочник".to_string(),
                kind: MetadataKind::Catalog,
                placeholder_suffix: Some("справочника".to_string()),
            },
            ExtractedPattern {
                prefix: "Документ".to_string(),
                kind: MetadataKind::Document,
                placeholder_suffix: Some("документа".to_string()),
            },
            ExtractedPattern {
                prefix: "ПланОбмена".to_string(),
                kind: MetadataKind::ExchangePlan,
                placeholder_suffix: Some("плана обмена".to_string()),
            },
        ]);

        assert_eq!(
            index.resolve_metadata_kind("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            index.resolve_metadata_kind("ДокументОбъект"),
            Some(MetadataKind::Document)
        );
        assert_eq!(
            index.resolve_metadata_kind("ПланОбменаСсылка"),
            Some(MetadataKind::ExchangePlan)
        );
    }

    #[test]
    fn test_update_metadata_patterns() {
        use crate::domain::metadata_patterns::ExtractedPattern;

        let mut index = SignatureIndex::new();

        // Изначально нет извлечённых паттернов
        assert!(!index.has_extracted_metadata_patterns());
        assert_eq!(index.extracted_metadata_patterns_count(), 0);

        // Добавляем паттерны
        index.update_metadata_patterns(vec![
            ExtractedPattern {
                prefix: "Справочник".to_string(),
                kind: MetadataKind::Catalog,
                placeholder_suffix: Some("справочника".to_string()),
            },
            ExtractedPattern {
                prefix: "Документ".to_string(),
                kind: MetadataKind::Document,
                placeholder_suffix: Some("документа".to_string()),
            },
        ]);

        // Теперь есть извлечённые паттерны
        assert!(index.has_extracted_metadata_patterns());
        assert_eq!(index.extracted_metadata_patterns_count(), 2);

        // Резолвинг использует извлечённые паттерны
        assert_eq!(
            index.resolve_metadata_kind("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
    }

    #[test]
    fn test_metadata_patterns_accessor() {
        let index = SignatureIndex::new();

        // Можно получить ссылку на реестр
        let patterns = index.metadata_patterns();
        assert!(!patterns.has_extracted_patterns());
    }

    #[test]
    fn test_extract_base_facet_type_exchange_plan() {
        // Планы обмена должны распознаваться
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ПланОбменаМенеджер.РаспределённаяБаза"),
            Some("ПланОбменаМенеджер")
        );
        assert_eq!(
            SignatureIndex::extract_base_facet_type("ПланОбменаОбъект.РаспределённаяБаза"),
            Some("ПланОбменаОбъект")
        );
    }

    // ================= MILESTONE 3.15: Lazy Resolution Tests =================

    #[test]
    fn test_lazy_return_type_caching() {
        let method = MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Число".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        // Изначально кэш пустой
        assert!(!method.has_cached_return_type());

        // Первый вызов заполняет кэш
        let result1 = method.get_resolved_return_type(|_| TypeResolution::unknown());
        assert!(method.has_cached_return_type());
        assert!(result1.is_some());

        // Второй вызов использует кэш (closure не вызывается)
        let result2 = method.get_resolved_return_type(|_| panic!("Should use cache!"));
        assert_eq!(result1.is_some(), result2.is_some());
    }

    #[test]
    fn test_lazy_return_type_none() {
        // Процедура без возвращаемого значения
        let method = MethodSignature::new(
            "Сообщить".to_string(),
            None,
            vec![],
            None, // return_type = None
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        assert!(!method.has_cached_return_type());

        // Для None return_type результат тоже None
        let result = method.get_resolved_return_type(|_| panic!("Should not be called!"));
        assert!(result.is_none());

        // Кэш всё равно заполняется
        assert!(method.has_cached_return_type());
    }

    #[test]
    fn test_lazy_params_caching() {
        let method = MethodSignature::new(
            "Вставить".to_string(),
            Some("Массив".to_string()),
            vec![
                ParameterInfo {
                    name: "Индекс".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                },
                ParameterInfo {
                    name: "Значение".to_string(),
                    type_name: Some("Произвольный".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                },
            ],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        assert!(!method.has_cached_params());

        // Первый вызов заполняет кэш
        let params = method.get_resolved_params(|_| TypeResolution::unknown());
        assert!(method.has_cached_params());
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].0, "Индекс");
        assert_eq!(params[1].0, "Значение");

        // Второй вызов использует кэш
        let params2 = method.get_resolved_params(|_| panic!("Should use cache!"));
        assert_eq!(params.len(), params2.len());
    }

    #[test]
    fn test_clone_shares_cache() {
        let method1 = MethodSignature::new(
            "Тест".to_string(),
            Some("Тип".to_string()),
            vec![],
            Some("Строка".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        // Заполняем кэш в method1
        let _ = method1.get_resolved_return_type(|_| TypeResolution::unknown());
        assert!(method1.has_cached_return_type());

        // Клонируем - кэш должен разделяться
        let method2 = method1.clone();
        assert!(method2.has_cached_return_type()); // Кэш уже заполнен!

        // Closure не вызывается т.к. кэш общий
        let _ = method2.get_resolved_return_type(|_| panic!("Should use shared cache!"));
    }

    #[test]
    fn test_reset_cache() {
        let mut method = MethodSignature::new(
            "Тест".to_string(),
            Some("Тип".to_string()),
            vec![],
            Some("Строка".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        // Заполняем кэш
        let _ = method.get_resolved_return_type(|_| TypeResolution::unknown());
        let _ = method.get_resolved_params(|_| TypeResolution::unknown());
        assert!(method.has_cached_return_type());
        assert!(method.has_cached_params());

        // Сбрасываем кэш
        method.reset_cache();
        assert!(!method.has_cached_return_type());
        assert!(!method.has_cached_params());
    }

    #[test]
    fn test_param_with_no_type() {
        let method = MethodSignature::new(
            "МетодБезТипов".to_string(),
            None,
            vec![ParameterInfo {
                name: "Параметр".to_string(),
                type_name: None, // Тип не указан
                is_optional: false,
                default_value: None,
                description: None,
            }],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        // Для параметра без типа должен вернуться unknown
        let params = method.get_resolved_params(|_| panic!("Should not be called for None type"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "Параметр");
        // TypeResolution.unknown() возвращается для параметров без типа
    }

    // ================= MERGE LOGIC TESTS =================

    /// Тест: При добавлении метода с return_type обновляется существующий метод без return_type
    #[test]
    fn test_add_platform_method_merges_return_type() {
        let mut index = SignatureIndex::new();

        // Добавляем метод БЕЗ return_type (как из syntax_helper)
        let sig_no_return = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            None, // Нет return type
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig_no_return);

        // Добавляем метод С return_type (как из platform_types.rs)
        let sig_with_return = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникСсылка".to_string()), // Есть return type
            None,
            None,
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::ServerOnly,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig_with_return);

        // Проверяем что метод обновился
        let found = index.find_method("СправочникМенеджер", "НайтиПоКоду");
        assert!(found.is_some());

        let method = found.unwrap();
        assert_eq!(method.return_type, Some("СправочникСсылка".to_string()));
        assert_eq!(method.return_facet, Some(FacetKind::Reference));
        assert_eq!(method.context_requirements, ContextRequirements::ServerOnly);
    }

    /// Тест: Существующий return_type НЕ перезаписывается
    #[test]
    fn test_add_platform_method_preserves_existing_return_type() {
        let mut index = SignatureIndex::new();

        // Добавляем метод С return_type
        let sig_with_return = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Число".to_string()), // Есть return type
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_with_return);

        // Добавляем метод с ДРУГИМ return_type
        let sig_different_return = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Строка".to_string()), // Другой return type
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_different_return);

        // Проверяем что оригинальный return_type сохранился
        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().return_type, Some("Число".to_string()));
    }

    /// Тест: Параметры обновляются если у существующего метода их нет
    #[test]
    fn test_add_platform_method_merges_params() {
        let mut index = SignatureIndex::new();

        // Добавляем метод с параметром без типа (как будто источник не знал тип)
        let param_unknown = ParameterInfo {
            name: "Индекс".to_string(),
            type_name: None,
            is_optional: false,
            default_value: None,
            description: None,
        };
        let sig_unknown_param_type = MethodSignature::new(
            "Вставить".to_string(),
            Some("Массив".to_string()),
            vec![param_unknown],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_unknown_param_type);

        // Добавляем тот же overload, но уже с известным типом параметра
        let param_typed = ParameterInfo {
            name: "Индекс".to_string(),
            type_name: Some("Число".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        };
        let sig_with_params = MethodSignature::new(
            "Вставить".to_string(),
            Some("Массив".to_string()),
            vec![param_typed],
            None,
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_with_params);

        // Проверяем что параметры "обогатились"
        let found = index.find_method("Массив", "Вставить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().params.len(), 1);
        assert_eq!(found.unwrap().params[0].name, "Индекс");
        assert_eq!(
            found.unwrap().params[0].type_name,
            Some("Число".to_string())
        );
    }

    // ================= PHASE 3: Enhanced Merge Tests =================

    /// Тест: syntax_helper (без return) + platform_types (с return) = merged result
    #[test]
    fn test_merge_syntax_helper_then_platform_types() {
        let mut index = SignatureIndex::new();

        // Шаг 1: syntax_helper добавляет метод БЕЗ return_type
        let sig_syntax_helper = MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![], // Нет параметров
            None,   // Нет return_type (syntax_helper не парсит return types)
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig_syntax_helper);

        // Шаг 2: platform_types добавляет тот же метод С return_type
        let sig_platform_types = MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект.<Имя справочника>".to_string()), // С return_type
            None,
            None,
            SignatureSource::Platform,
            Some(FacetKind::Object),
            ContextRequirements::ServerOnly,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig_platform_types);

        // Проверяем результат merge
        let found = index.find_method("СправочникМенеджер", "СоздатьЭлемент");
        assert!(found.is_some(), "Метод должен быть найден");

        let method = found.unwrap();
        assert_eq!(
            method.return_type,
            Some("СправочникОбъект.<Имя справочника>".to_string()),
            "return_type должен быть заполнен из platform_types"
        );
        assert_eq!(
            method.return_facet,
            Some(FacetKind::Object),
            "return_facet должен быть Object"
        );
        assert_eq!(
            method.context_requirements,
            ContextRequirements::ServerOnly,
            "context_requirements должен быть ServerOnly"
        );
    }

    /// Тест: порядок merge не влияет на результат (platform_types первый)
    #[test]
    fn test_merge_order_independence_platform_first() {
        let mut index = SignatureIndex::new();

        // Шаг 1: platform_types ПЕРВЫМ добавляет метод С return_type
        let sig_platform_types = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникСсылка".to_string()),
            None,
            None,
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::ServerOnly,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig_platform_types);

        // Шаг 2: syntax_helper ВТОРЫМ добавляет тот же метод БЕЗ return_type
        let sig_syntax_helper = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            None, // Нет return_type
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("СправочникМенеджер"), sig_syntax_helper);

        // Проверяем что оригинальный return_type сохранился
        let found = index.find_method("СправочникМенеджер", "НайтиПоКоду");
        assert!(found.is_some());

        let method = found.unwrap();
        assert_eq!(
            method.return_type,
            Some("СправочникСсылка".to_string()),
            "return_type должен сохраниться от platform_types"
        );
        assert_eq!(
            method.return_facet,
            Some(FacetKind::Reference),
            "return_facet должен сохраниться"
        );
        assert_eq!(
            method.context_requirements,
            ContextRequirements::ServerOnly,
            "context_requirements должен сохраниться"
        );
    }

    /// Тест: конфликт return_type - первый побеждает (логируется warning)
    #[test]
    fn test_merge_conflict_return_type_keeps_first() {
        let mut index = SignatureIndex::new();

        // Первый источник: return_type = "Число"
        let sig_first = MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Число".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_first);

        // Второй источник: return_type = "Строка" (конфликт!)
        let sig_second = MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Строка".to_string()), // Другой return_type
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_second);

        // Первый return_type должен сохраниться
        let found = index.find_method("Массив", "Количество");
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().return_type,
            Some("Число".to_string()),
            "При конфликте первый return_type должен сохраниться"
        );
    }

    /// Тест: регистронезависимость при merge
    #[test]
    fn test_merge_case_insensitive() {
        let mut index = SignatureIndex::new();

        // Первый: имя в нижнем регистре
        let sig_lower = MethodSignature::new(
            "добавить".to_string(), // lower case
            Some("Массив".to_string()),
            vec![],
            None, // Нет return_type
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_lower);

        // Второй: имя в смешанном регистре с return_type
        let sig_mixed = MethodSignature::new(
            "Добавить".to_string(), // Mixed case
            Some("Массив".to_string()),
            vec![],
            Some("Неопределено".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method(TypeId::new("Массив"), sig_mixed);

        // Проверяем что merge произошёл (поиск в любом регистре)
        let found_lower = index.find_method("Массив", "добавить");
        let found_upper = index.find_method("Массив", "ДОБАВИТЬ");
        let found_mixed = index.find_method("Массив", "Добавить");

        assert!(found_lower.is_some(), "Должен найти в lower case");
        assert!(found_upper.is_some(), "Должен найти в UPPER case");
        assert!(found_mixed.is_some(), "Должен найти в Mixed case");

        // Все должны указывать на один и тот же метод с merged return_type
        assert_eq!(
            found_lower.unwrap().return_type,
            Some("Неопределено".to_string()),
            "return_type должен быть merged"
        );
        assert_eq!(
            found_upper.unwrap().return_type,
            Some("Неопределено".to_string())
        );
        assert_eq!(
            found_mixed.unwrap().return_type,
            Some("Неопределено".to_string())
        );
    }
}
