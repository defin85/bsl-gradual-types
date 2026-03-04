use super::*;

#[test]
fn test_infer_array_from_add() {
    let mut inference = GenericInference::new();

    // arr.Добавить("текст")
    let result = inference.infer_from_method_call("arr", "Добавить", &[ConcreteType::string()]);

    assert!(result.is_some());
    let generic = result.unwrap();
    assert_eq!(generic.base_type, "Массив");
    assert_eq!(generic.type_params.len(), 1);
    assert!(matches!(
        generic.type_params[0],
        ConcreteType::Primitive(PrimitiveType::String)
    ));
}

#[test]
fn test_infer_array_from_insert() {
    let mut inference = GenericInference::new();

    // arr.Вставить(0, 123)
    let result = inference.infer_from_method_call(
        "arr",
        "Вставить",
        &[ConcreteType::number(), ConcreteType::number()],
    );

    assert!(result.is_some());
    let generic = result.unwrap();
    assert_eq!(generic.base_type, "Массив");
    assert!(matches!(
        generic.type_params[0],
        ConcreteType::Primitive(PrimitiveType::Number)
    ));
}

#[test]
fn test_infer_map_from_insert() {
    let mut inference = GenericInference::new();

    // map.Вставить("ключ", 123)
    let result = inference.infer_from_method_call(
        "map",
        "Вставить",
        &[ConcreteType::string(), ConcreteType::number()],
    );

    assert!(result.is_some());
    let generic = result.unwrap();
    assert_eq!(generic.base_type, "Соответствие");
    assert_eq!(generic.type_params.len(), 2);
    assert!(matches!(
        generic.type_params[0],
        ConcreteType::Primitive(PrimitiveType::String)
    ));
    assert!(matches!(
        generic.type_params[1],
        ConcreteType::Primitive(PrimitiveType::Number)
    ));
}

#[test]
fn test_variable_type_tracking() {
    let mut inference = GenericInference::new();

    inference.infer_from_method_call("arr", "Добавить", &[ConcreteType::string()]);

    let var_type = inference.get_variable_type("arr");
    assert!(var_type.is_some());

    let info = var_type.unwrap();
    assert_eq!(info.base_type, "Массив");
    assert_eq!(info.confidence, 0.9);
}

#[test]
fn test_refine_inference() {
    let mut inference = GenericInference::new();

    // Первый вывод: Массив<Строка>
    inference.infer_from_method_call("arr", "Добавить", &[ConcreteType::string()]);

    // Уточнение: добавляем число
    inference.refine_inference("arr", ConcreteType::number(), 0);

    let info = inference.get_variable_type("arr").unwrap();
    // Уверенность должна снизиться из-за противоречия
    assert!(info.confidence < 0.9);
}

#[test]
fn test_unknown_method_returns_none() {
    let mut inference = GenericInference::new();

    let result = inference.infer_from_method_call("obj", "НеизвестныйМетод", &[]);

    assert!(result.is_none());
}

#[test]
fn test_list_inference() {
    let mut inference = GenericInference::new();

    let result =
        inference.infer_from_method_call("list", "ЗагрузитьЗначения", &[ConcreteType::boolean()]);

    assert!(result.is_some());
    let generic = result.unwrap();
    assert_eq!(generic.base_type, "Список");
    assert!(matches!(
        generic.type_params[0],
        ConcreteType::Primitive(PrimitiveType::Boolean)
    ));
}
