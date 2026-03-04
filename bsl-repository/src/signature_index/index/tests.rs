use super::*;
use crate::signature_index::SignatureSource;

#[test]
fn test_type_id_normalization_fallback() {
    let mut index = SignatureIndex::new();

    // Добавим метод для типа "Табличная часть" (с пробелом)
    let method = MethodSignature::new(
        "Выгрузить".to_string(),
        Some("Табличная часть".to_string()),
        vec![],
        Some("Массив".to_string()),
        None,
        None,
        SignatureSource::Platform,
        None,
        ContextRequirements::Universal,
    );

    index.add_platform_method(TypeId::new("Табличная часть"), method);

    // Поиск по CamelCase варианту должен работать через TypeId нормализацию
    let result = index.find_method("ТабличнаяЧасть", "Выгрузить");
    assert!(
        result.is_some(),
        "Метод должен быть найден через TypeId нормализацию CamelCase -> с пробелами"
    );

    let found_method = result.unwrap();
    assert_eq!(found_method.name, "Выгрузить");
    assert_eq!(found_method.return_type.as_deref(), Some("Массив"));
}

#[test]
fn test_type_id_normalization() {
    // Проверяем что TypeId правильно нормализует имена
    // TypeId("ТабличнаяЧасть") == TypeId("Табличная часть")
    let id1 = TypeId::new("ТабличнаяЧасть");
    let id2 = TypeId::new("Табличная часть");
    assert_eq!(
        id1, id2,
        "TypeId должен нормализовать CamelCase и варианты с пробелами"
    );

    // Проверяем lowercase нормализацию
    let id3 = TypeId::new("МАССИВ");
    let id4 = TypeId::new("массив");
    let id5 = TypeId::new("Массив");
    assert_eq!(id3, id4);
    assert_eq!(id4, id5);
}
