use super::*;

#[test]
fn test_simple_method_without_params() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("Метод1")
        .returns("Строка")
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Метод1");
    assert!(method.is_some(), "Метод должен быть найден");

    let found = method.unwrap();
    assert_eq!(found.name, "Метод1");
    assert_eq!(found.return_type, Some("Строка".to_string()));
    assert!(found.params.is_empty());
}

#[test]
fn test_void_method() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("Очистить")
        .void()
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Очистить");
    assert!(method.is_some());

    let found = method.unwrap();
    assert_eq!(found.name, "Очистить");
    assert_eq!(found.return_type, None);
}

#[test]
fn test_method_with_required_param() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("Получить")
        .returns("Значение")
        .param("Индекс", "Число")
        .required()
        .desc("Индекс элемента")
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Получить").unwrap();
    assert_eq!(method.params.len(), 1);

    let param = &method.params[0];
    assert_eq!(param.name, "Индекс");
    assert_eq!(param.type_name, Some("Число".to_string()));
    assert!(!param.is_optional);
    assert_eq!(param.description, Some("Индекс элемента".to_string()));
}

#[test]
fn test_method_with_optional_params() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("Выгрузить")
        .returns("ТаблицаЗначений")
        .param("Колонки", "Строка")
        .optional()
        .desc("Список колонок")
        .param("Отбор", "Структура")
        .optional()
        .desc("Условия отбора")
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Выгрузить").unwrap();
    assert_eq!(method.params.len(), 2);
    assert_eq!(method.return_type, Some("ТаблицаЗначений".to_string()));

    assert!(method.params[0].is_optional);
    assert!(method.params[1].is_optional);
    assert_eq!(method.params[0].name, "Колонки");
    assert_eq!(method.params[1].name, "Отбор");
}

#[test]
fn test_method_with_mixed_params() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("Найти")
        .returns("СтрокаТаблицы")
        .param_any("Значение")
        .required()
        .desc("Искомое значение")
        .param("Колонки", "Строка")
        .optional()
        .desc("Колонки для поиска")
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Найти").unwrap();
    assert_eq!(method.params.len(), 2);

    // Первый параметр - произвольный тип, обязательный
    assert_eq!(method.params[0].type_name, None);
    assert!(!method.params[0].is_optional);

    // Второй параметр - строка, опциональный
    assert_eq!(method.params[1].type_name, Some("Строка".to_string()));
    assert!(method.params[1].is_optional);
}

#[test]
fn test_method_with_facet() {
    let type_id = TypeId::new("СправочникМенеджер");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("СоздатьЭлемент")
        .returns("СправочникОбъект")
        .facet(FacetKind::Object)
        .context(ContextRequirements::ServerOnly)
        .add_to(&mut index);

    let method = index
        .find_method("СправочникМенеджер", "СоздатьЭлемент")
        .unwrap();
    assert_eq!(method.return_facet, Some(FacetKind::Object));
    assert_eq!(method.context_requirements, ContextRequirements::ServerOnly);
}

#[test]
fn test_build_without_adding_to_index() {
    let type_id = TypeId::new("ТестТип");

    let (built_type_id, signature) = MethodBuilder::for_type(&type_id)
        .method("ТестМетод")
        .returns("Число")
        .build();

    assert_eq!(built_type_id, type_id);
    assert_eq!(signature.name, "ТестМетод");
    assert_eq!(signature.return_type, Some("Число".to_string()));
}

#[test]
#[should_panic(expected = "Method name must be set")]
fn test_panic_without_method_name() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .returns("Строка")
        .add_to(&mut index);
}

#[test]
fn test_default_values() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    MethodBuilder::for_type(&type_id)
        .method("Метод")
        .returns("Число")
        .param("Размер", "Число")
        .default_value("10")
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Метод").unwrap();
    assert_eq!(method.params[0].default_value, Some("10".to_string()));
}

#[test]
fn test_chained_param_modifiers() {
    let type_id = TypeId::new("ТестТип");
    let mut index = SignatureIndex::new();

    // Проверяем что можно вызывать несколько модификаторов подряд
    MethodBuilder::for_type(&type_id)
        .method("Тест")
        .returns("Число")
        .param("П1", "Строка")
        .optional()
        .desc("Описание 1")
        .default_value("значение")
        .param("П2", "Число")
        .required()
        .desc("Описание 2")
        .add_to(&mut index);

    let method = index.find_method("ТестТип", "Тест").unwrap();
    assert_eq!(method.params.len(), 2);

    let p1 = &method.params[0];
    assert!(p1.is_optional);
    assert_eq!(p1.description, Some("Описание 1".to_string()));
    assert_eq!(p1.default_value, Some("значение".to_string()));

    let p2 = &method.params[1];
    assert!(!p2.is_optional);
    assert_eq!(p2.description, Some("Описание 2".to_string()));
}
