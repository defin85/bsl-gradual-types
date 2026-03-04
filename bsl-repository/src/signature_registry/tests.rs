use super::*;

/// Тестовый источник данных
struct TestSource {
    name: String,
    priority: u32,
    types: Vec<RawTypeData>,
}

impl SignatureDataSource for TestSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        self.priority
    }

    fn load(&self) -> Vec<RawTypeData> {
        self.types.clone()
    }
}

#[test]
fn test_registry_empty() {
    let registry = SignatureSourceRegistry::new();
    assert_eq!(registry.source_count(), 0);

    let index = registry.build();
    assert!(index.find_constructor("Массив").is_none());
}

#[test]
fn test_registry_single_source() {
    use bsl_types::types::{RawDataSource, RawMethodData, RawParamData};

    let types = vec![RawTypeData {
        name: "Массив".to_string(),
        methods: vec![RawMethodData {
            name: "Добавить".to_string(),
            return_type: "".to_string(),
            params: vec![RawParamData {
                name: "Значение".to_string(),
                param_type: "Произвольный".to_string(),
                is_optional: false,
                default_value: None,
            }],
            ..Default::default()
        }],
        source: RawDataSource::Platform,
        ..Default::default()
    }];

    let source = TestSource {
        name: "Test".to_string(),
        priority: 10,
        types,
    };

    let index = SignatureSourceRegistry::new().register(source).build();

    // Проверяем что метод добавлен
    let method = index.find_method("Массив", "Добавить");
    assert!(method.is_some());
    assert_eq!(method.unwrap().name, "Добавить");
}

#[test]
fn test_registry_priority_order() {
    use bsl_types::types::{RawDataSource, RawMethodData};

    // Источник с приоритетом 20 (добавится позже)
    let types1 = vec![RawTypeData {
        name: "ТестТип".to_string(),
        methods: vec![RawMethodData {
            name: "Метод1".to_string(),
            return_type: "Строка".to_string(), // С return type
            ..Default::default()
        }],
        source: RawDataSource::Platform,
        ..Default::default()
    }];

    // Источник с приоритетом 10 (добавится раньше)
    let types2 = vec![RawTypeData {
        name: "ТестТип".to_string(),
        methods: vec![RawMethodData {
            name: "Метод1".to_string(),
            return_type: "".to_string(), // Без return type
            ..Default::default()
        }],
        source: RawDataSource::Platform,
        ..Default::default()
    }];

    let source1 = TestSource {
        name: "Later".to_string(),
        priority: 20,
        types: types1,
    };

    let source2 = TestSource {
        name: "Earlier".to_string(),
        priority: 10,
        types: types2,
    };

    // Регистрируем в обратном порядке (source1 первым)
    // но source2 должен обработаться раньше из-за приоритета
    let index = SignatureSourceRegistry::new()
        .register(source1)
        .register(source2)
        .build();

    let method = index.find_method("ТестТип", "Метод1");
    assert!(method.is_some());
    // Должен быть return_type из source1 (приоритет 20),
    // т.к. он обогатил метод из source2 (приоритет 10)
    assert_eq!(method.unwrap().return_type, Some("Строка".to_string()));
}

#[test]
fn test_extract_placeholder_base_type_via_facet_utils() {
    use bsl_types::facet_utils::extract_placeholder_base_type;

    // Стандартный формат
    assert_eq!(
        extract_placeholder_base_type("СправочникМенеджер.<Имя справочника>"),
        Some("СправочникМенеджер")
    );
    assert_eq!(
        extract_placeholder_base_type("ДокументОбъект.<Имя документа>"),
        Some("ДокументОбъект")
    );

    // HTML-encoded формат
    assert_eq!(
        extract_placeholder_base_type("СправочникМенеджер.&lt;Имя справочника&gt;"),
        Some("СправочникМенеджер")
    );
    assert_eq!(
        extract_placeholder_base_type("ДокументОбъект.&lt;Имя документа&gt;"),
        Some("ДокументОбъект")
    );

    // Не-фасетные типы
    assert_eq!(extract_placeholder_base_type("Массив"), None);
    assert_eq!(extract_placeholder_base_type("СправочникМенеджер"), None);
}

#[test]
fn test_infer_method_metadata_create() {
    let method = RawMethodData {
        name: "СоздатьЭлемент".to_string(),
        return_type: "СправочникОбъект".to_string(),
        ..Default::default()
    };

    let (facet, context) = infer_method_metadata(&method);
    assert_eq!(facet, Some(FacetKind::Object));
    assert_eq!(context, ContextRequirements::ServerOnly);
}

#[test]
fn test_infer_method_metadata_find() {
    let method = RawMethodData {
        name: "НайтиПоКоду".to_string(),
        return_type: "СправочникСсылка".to_string(),
        ..Default::default()
    };

    let (facet, context) = infer_method_metadata(&method);
    assert_eq!(facet, Some(FacetKind::Reference));
    assert_eq!(context, ContextRequirements::ServerOnly);
}

#[test]
fn test_infer_method_metadata_write() {
    let method = RawMethodData {
        name: "Записать".to_string(),
        return_type: "".to_string(),
        ..Default::default()
    };

    let (facet, context) = infer_method_metadata(&method);
    assert_eq!(facet, None);
    assert_eq!(context, ContextRequirements::ServerOnly);
}

#[test]
fn test_infer_method_metadata_context_override() {
    let method = RawMethodData {
        name: "Записать".to_string(),
        return_type: "".to_string(),
        context_requirements: Some(ContextRequirements::ClientOnly),
        ..Default::default()
    };

    let (facet, context) = infer_method_metadata(&method);
    assert_eq!(facet, None);
    assert_eq!(context, ContextRequirements::ClientOnly);
}
