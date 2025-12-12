//! Тесты для SignatureIndex
//!
//! Все тесты из оригинального модуля сохранены без изменений.

#[cfg(test)]
mod tests {
    use crate::domain::signature_index::{
        ConstructorSignature, ContextRequirements, MethodSignature, SignatureIndex,
        SignatureSource,
    };
    use crate::domain::types::{FacetKind, MetadataKind, ParameterInfo, TypeResolution};

    #[test]
    fn test_signature_index_basic() {
        let mut index = SignatureIndex::new();

        let sig = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method("Массив".to_string(), sig);

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
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method("Массив".to_string(), sig);

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

        index.add_constructor("Массив".to_string(), constructor);

        let found = index.find_constructor("Массив");
        assert!(found.is_some());
        assert_eq!(found.unwrap().type_name, "Массив");
    }

    #[test]
    fn test_find_constructor_case_insensitive() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        // Поиск в разных регистрах
        assert!(index.find_constructor("Массив").is_some());
        assert!(index.find_constructor("массив").is_some());
        assert!(index.find_constructor("МАССИВ").is_some());
    }

    #[test]
    fn test_is_collection_type() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        assert!(index.is_collection_type("Массив"));
        assert!(index.is_collection_type("Соответствие"));
        assert!(!index.is_collection_type("ТаблицаЗначений"));
    }

    #[test]
    fn test_get_generic_params_count() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        assert_eq!(index.get_generic_params_count("Массив"), Some(1));
        assert_eq!(index.get_generic_params_count("Соответствие"), Some(2));
        assert_eq!(index.get_generic_params_count("ТаблицаЗначений"), Some(0));
    }

    #[test]
    fn test_builtin_constructors() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_constructors();

        // Проверяем что все встроенные конструкторы добавлены
        assert!(index.find_constructor("Массив").is_some());
        assert!(index.find_constructor("Соответствие").is_some());
        assert!(index.find_constructor("ТаблицаЗначений").is_some());
        assert!(index.find_constructor("СписокЗначений").is_some());
        assert!(index.find_constructor("ФиксированныйМассив").is_some());
    }

    #[test]
    fn test_builtin_methods_tabular_section() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_methods();

        // Проверяем что методы ТабличнаяЧасть добавлены
        let vygruzit = index.find_method("ТабличнаяЧасть", "Выгрузить");
        assert!(vygruzit.is_some(), "Метод Выгрузить должен быть добавлен");
        assert_eq!(
            vygruzit.unwrap().return_type,
            Some("ТаблицаЗначений".to_string())
        );

        let dobavit = index.find_method("ТабличнаяЧасть", "Добавить");
        assert!(dobavit.is_some(), "Метод Добавить должен быть добавлен");
        assert_eq!(
            dobavit.unwrap().return_type,
            Some("СтрокаТабличнойЧасти".to_string())
        );

        let kolichestvo = index.find_method("ТабличнаяЧасть", "Количество");
        assert!(
            kolichestvo.is_some(),
            "Метод Количество должен быть добавлен"
        );
        assert_eq!(kolichestvo.unwrap().return_type, Some("Число".to_string()));

        let ochistit = index.find_method("ТабличнаяЧасть", "Очистить");
        assert!(ochistit.is_some(), "Метод Очистить должен быть добавлен");
        assert_eq!(ochistit.unwrap().return_type, None); // void

        let udalit = index.find_method("ТабличнаяЧасть", "Удалить");
        assert!(udalit.is_some(), "Метод Удалить должен быть добавлен");

        let naiti = index.find_method("ТабличнаяЧасть", "Найти");
        assert!(naiti.is_some(), "Метод Найти должен быть добавлен");
        assert_eq!(
            naiti.unwrap().return_type,
            Some("СтрокаТабличнойЧасти".to_string())
        );

        let naiti_stroki = index.find_method("ТабличнаяЧасть", "НайтиСтроки");
        assert!(
            naiti_stroki.is_some(),
            "Метод НайтиСтроки должен быть добавлен"
        );
        assert_eq!(
            naiti_stroki.unwrap().return_type,
            Some("Массив".to_string())
        );

        let poluchit = index.find_method("ТабличнаяЧасть", "Получить");
        assert!(poluchit.is_some(), "Метод Получить должен быть добавлен");
        assert_eq!(
            poluchit.unwrap().return_type,
            Some("СтрокаТабличнойЧасти".to_string())
        );

        let indeks = index.find_method("ТабличнаяЧасть", "Индекс");
        assert!(indeks.is_some(), "Метод Индекс должен быть добавлен");
        assert_eq!(indeks.unwrap().return_type, Some("Число".to_string()));

        let itogo = index.find_method("ТабличнаяЧасть", "Итого");
        assert!(itogo.is_some(), "Метод Итого должен быть добавлен");
        assert_eq!(itogo.unwrap().return_type, Some("Число".to_string()));

        let sdvinut = index.find_method("ТабличнаяЧасть", "Сдвинуть");
        assert!(sdvinut.is_some(), "Метод Сдвинуть должен быть добавлен");

        let zagruzit = index.find_method("ТабличнаяЧасть", "Загрузить");
        assert!(zagruzit.is_some(), "Метод Загрузить должен быть добавлен");

        let sortirovat = index.find_method("ТабличнаяЧасть", "Сортировать");
        assert!(
            sortirovat.is_some(),
            "Метод Сортировать должен быть добавлен"
        );

        let vygruzit_kolonku = index.find_method("ТабличнаяЧасть", "ВыгрузитьКолонку");
        assert!(
            vygruzit_kolonku.is_some(),
            "Метод ВыгрузитьКолонку должен быть добавлен"
        );
        assert_eq!(
            vygruzit_kolonku.unwrap().return_type,
            Some("Массив".to_string())
        );

        let zagruzit_kolonku = index.find_method("ТабличнаяЧасть", "ЗагрузитьКолонку");
        assert!(
            zagruzit_kolonku.is_some(),
            "Метод ЗагрузитьКолонку должен быть добавлен"
        );

        let vstavit = index.find_method("ТабличнаяЧасть", "Вставить");
        assert!(vstavit.is_some(), "Метод Вставить должен быть добавлен");
        assert_eq!(
            vstavit.unwrap().return_type,
            Some("СтрокаТабличнойЧасти".to_string())
        );
    }

    #[test]
    fn test_tabular_section_method_params() {
        let mut index = SignatureIndex::new();
        index.initialize_builtin_methods();

        // Проверяем параметры метода Выгрузить
        let vygruzit = index
            .find_method("ТабличнаяЧасть", "Выгрузить")
            .expect("Метод Выгрузить должен существовать");
        assert_eq!(vygruzit.params.len(), 2);
        assert!(vygruzit.params[0].is_optional);
        assert!(vygruzit.params[1].is_optional);

        // Проверяем параметры метода Найти
        let naiti = index
            .find_method("ТабличнаяЧасть", "Найти")
            .expect("Метод Найти должен существовать");
        assert_eq!(naiti.params.len(), 2);
        assert!(!naiti.params[0].is_optional); // Значение - обязательный
        assert!(naiti.params[1].is_optional); // Колонки - опциональный

        // Проверяем параметры метода Сдвинуть
        let sdvinut = index
            .find_method("ТабличнаяЧасть", "Сдвинуть")
            .expect("Метод Сдвинуть должен существовать");
        assert_eq!(sdvinut.params.len(), 2);
        assert!(!sdvinut.params[0].is_optional);
        assert!(!sdvinut.params[1].is_optional);
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
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method("СправочникМенеджер".to_string(), sig);

        // Поиск по точному имени (базовый тип) - должен найти
        let found_exact = index.find_method("СправочникМенеджер", "СоздатьЭлемент");
        assert!(found_exact.is_some());

        // Поиск по конкретизированному типу (fallback к базовому) - тоже должен найти
        let found_fallback =
            index.find_method("СправочникМенеджер.Контрагенты", "СоздатьЭлемент");
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
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method("ДокументОбъект".to_string(), sig);

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
            SignatureSource::Platform,
            None,
            ContextRequirements::default(),
        );

        index.add_platform_method("Массив".to_string(), sig);

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
    fn test_get_metadata_kind_from_prefix_catalog() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("СправочникМенеджер"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("СправочникОбъект"),
            Some(MetadataKind::Catalog)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("СправочникСсылка"),
            Some(MetadataKind::Catalog)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_document() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ДокументМенеджер"),
            Some(MetadataKind::Document)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ДокументОбъект"),
            Some(MetadataKind::Document)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_registers() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрСведенийМенеджер"),
            Some(MetadataKind::InformationRegister)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрНакопленияНаборЗаписей"),
            Some(MetadataKind::AccumulationRegister)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрБухгалтерииВыборка"),
            Some(MetadataKind::AccountingRegister)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("РегистрРасчетаЗапись"),
            Some(MetadataKind::CalculationRegister)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_plans() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланСчетовМенеджер"),
            Some(MetadataKind::ChartOfAccounts)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланВидовХарактеристикОбъект"),
            Some(MetadataKind::ChartOfCharacteristicTypes)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланВидовРасчетаСсылка"),
            Some(MetadataKind::ChartOfCalculationTypes)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_other() {
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("БизнесПроцессМенеджер"),
            Some(MetadataKind::BusinessProcess)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ЗадачаОбъект"),
            Some(MetadataKind::Task)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПеречислениеМенеджер"),
            Some(MetadataKind::Enum)
        );
    }

    #[test]
    fn test_get_metadata_kind_from_prefix_non_faceted() {
        // Не-фасетные типы -> None
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("Массив"),
            None
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ТаблицаЗначений"),
            None
        );
    }

    // ================= MILESTONE 3.13: MetadataPatternRegistry Tests =================

    #[test]
    fn test_get_metadata_kind_from_prefix_exchange_plan() {
        // Планы обмена
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланОбменаМенеджер"),
            Some(MetadataKind::ExchangePlan)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланОбменаОбъект"),
            Some(MetadataKind::ExchangePlan)
        );
        assert_eq!(
            SignatureIndex::get_metadata_kind_from_prefix("ПланОбменаСсылка"),
            Some(MetadataKind::ExchangePlan)
        );
    }

    #[test]
    fn test_resolve_metadata_kind_instance_method() {
        let index = SignatureIndex::new();

        // Instance method должен работать как статический (fallback)
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
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("СправочникМенеджер".to_string(), sig_no_return);

        // Добавляем метод С return_type (как из platform_types.rs)
        let sig_with_return = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникСсылка".to_string()), // Есть return type
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::ServerOnly,
        );
        index.add_platform_method("СправочникМенеджер".to_string(), sig_with_return);

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
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_with_return);

        // Добавляем метод с ДРУГИМ return_type
        let sig_different_return = MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Строка".to_string()), // Другой return type
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_different_return);

        // Проверяем что оригинальный return_type сохранился
        let found = index.find_method("Массив", "Добавить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().return_type, Some("Число".to_string()));
    }

    /// Тест: Параметры обновляются если у существующего метода их нет
    #[test]
    fn test_add_platform_method_merges_params() {
        let mut index = SignatureIndex::new();

        // Добавляем метод БЕЗ параметров
        let sig_no_params = MethodSignature::new(
            "Вставить".to_string(),
            Some("Массив".to_string()),
            vec![], // Нет параметров
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_no_params);

        // Добавляем метод С параметрами
        let param = ParameterInfo {
            name: "Индекс".to_string(),
            type_name: Some("Число".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        };
        let sig_with_params = MethodSignature::new(
            "Вставить".to_string(),
            Some("Массив".to_string()),
            vec![param],
            None,
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_with_params);

        // Проверяем что параметры обновились
        let found = index.find_method("Массив", "Вставить");
        assert!(found.is_some());
        assert_eq!(found.unwrap().params.len(), 1);
        assert_eq!(found.unwrap().params[0].name, "Индекс");
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
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("СправочникМенеджер".to_string(), sig_syntax_helper);

        // Шаг 2: platform_types добавляет тот же метод С return_type
        let sig_platform_types = MethodSignature::new(
            "СоздатьЭлемент".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            Some("СправочникОбъект.<Имя справочника>".to_string()), // С return_type
            SignatureSource::Platform,
            Some(FacetKind::Object),
            ContextRequirements::ServerOnly,
        );
        index.add_platform_method("СправочникМенеджер".to_string(), sig_platform_types);

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
            SignatureSource::Platform,
            Some(FacetKind::Reference),
            ContextRequirements::ServerOnly,
        );
        index.add_platform_method("СправочникМенеджер".to_string(), sig_platform_types);

        // Шаг 2: syntax_helper ВТОРЫМ добавляет тот же метод БЕЗ return_type
        let sig_syntax_helper = MethodSignature::new(
            "НайтиПоКоду".to_string(),
            Some("СправочникМенеджер".to_string()),
            vec![],
            None, // Нет return_type
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("СправочникМенеджер".to_string(), sig_syntax_helper);

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
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_first);

        // Второй источник: return_type = "Строка" (конфликт!)
        let sig_second = MethodSignature::new(
            "Количество".to_string(),
            Some("Массив".to_string()),
            vec![],
            Some("Строка".to_string()), // Другой return_type
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_second);

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
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_lower);

        // Второй: имя в смешанном регистре с return_type
        let sig_mixed = MethodSignature::new(
            "Добавить".to_string(), // Mixed case
            Some("Массив".to_string()),
            vec![],
            Some("Неопределено".to_string()),
            SignatureSource::Platform,
            None,
            ContextRequirements::Universal,
        );
        index.add_platform_method("Массив".to_string(), sig_mixed);

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
