//! Интеграционные тесты для GenericInfo типов-коллекций
//!
//! Тестирует что GenericInfo корректно настроен для типов:
//! - Массив<T>
//! - Соответствие<K, V>
//! - СписокЗначений<T>
//! - ТабличнаяЧасть<T>

use bsl_backend::data::loaders::get_generic_info_registry;

#[test]
fn test_generic_info_registry_has_array() {
    let registry = get_generic_info_registry();

    let array_info = registry
        .get("Массив")
        .expect("Массив должен быть в реестре GenericInfo");

    assert_eq!(array_info.base_type, "Массив");
    assert_eq!(array_info.type_param_count, 1);
    assert!(!array_info.inference_methods.is_empty());
}

#[test]
fn test_generic_info_registry_has_map() {
    let registry = get_generic_info_registry();

    let map_info = registry
        .get("Соответствие")
        .expect("Соответствие должен быть в реестре GenericInfo");

    assert_eq!(map_info.base_type, "Соответствие");
    assert_eq!(map_info.type_param_count, 2); // K и V
    assert!(!map_info.inference_methods.is_empty());
}

#[test]
fn test_generic_info_registry_has_value_list() {
    let registry = get_generic_info_registry();

    let list_info = registry
        .get("СписокЗначений")
        .expect("СписокЗначений должен быть в реестре GenericInfo");

    assert_eq!(list_info.base_type, "СписокЗначений");
    assert_eq!(list_info.type_param_count, 1);
}

#[test]
fn test_generic_info_registry_has_tabular_section() {
    let registry = get_generic_info_registry();

    let tab_info = registry
        .get("ТабличнаяЧасть")
        .expect("ТабличнаяЧасть должен быть в реестре GenericInfo");

    assert_eq!(tab_info.base_type, "ТабличнаяЧасть");
    assert_eq!(tab_info.type_param_count, 1);
    assert!(!tab_info.inference_methods.is_empty());
}

#[test]
fn test_array_add_inference_method() {
    let registry = get_generic_info_registry();
    let array_info = registry.get("Массив").unwrap();

    let add_method = array_info
        .inference_methods
        .iter()
        .find(|m| m.method_name == "Добавить")
        .expect("Метод Добавить должен быть в inference_methods");

    // Добавить(Значение) — первый параметр определяет T
    assert_eq!(add_method.param_indices, vec![0]);
    assert_eq!(add_method.inferred_type_params, vec![0]); // T
}

#[test]
fn test_map_insert_inference_method() {
    let registry = get_generic_info_registry();
    let map_info = registry.get("Соответствие").unwrap();

    let insert_methods: Vec<_> = map_info
        .inference_methods
        .iter()
        .filter(|m| m.method_name == "Вставить")
        .collect();

    // Вставить(Ключ, Значение) — два inference rules для K и V
    assert_eq!(insert_methods.len(), 2);
}

#[test]
fn test_tabular_section_add_inference() {
    let registry = get_generic_info_registry();
    let tab_info = registry.get("ТабличнаяЧасть").unwrap();

    let add_method = tab_info
        .inference_methods
        .iter()
        .find(|m| m.method_name == "Добавить")
        .expect("Метод Добавить должен быть в inference_methods");

    // ТабличнаяЧасть.Добавить() — возвращает T (новую строку)
    assert_eq!(add_method.inferred_type_params, vec![0]);
}

#[test]
fn test_apply_generic_info_to_repository() {
    use bsl_backend::data::loaders::apply_generic_info_to_repository;
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::types::{RawDataSource, RawTypeData};

    let repository = InMemoryTypeRepository::new();

    // Добавляем типы-коллекции (без GenericInfo)
    let types = vec![
        RawTypeData {
            name: "Массив".to_string(),
            english_name: "Array".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        },
        RawTypeData {
            name: "Соответствие".to_string(),
            english_name: "Map".to_string(),
            source: RawDataSource::Platform,
            ..Default::default()
        },
    ];

    repository.load_types(types).unwrap();

    // Проверяем что GenericInfo НЕТ
    let arr_before = repository.find_type("Массив").unwrap();
    assert!(arr_before.generic_info.is_none());

    // Применяем GenericInfo
    let applied = apply_generic_info_to_repository(&repository);
    assert!(applied >= 2); // Массив и Соответствие

    // Проверяем что GenericInfo ЕСТЬ
    let arr_after = repository.find_type("Массив").unwrap();
    assert!(arr_after.generic_info.is_some());
    assert_eq!(arr_after.generic_info.as_ref().unwrap().base_type, "Массив");
}
