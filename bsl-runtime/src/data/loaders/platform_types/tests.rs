use super::*;

#[test]
fn test_generic_info_registry_contains_array() {
    let registry = get_generic_info_registry();

    let array_info = registry
        .get("Массив")
        .expect("Массив должен быть в реестре");
    assert_eq!(array_info.base_type, "Массив");
    assert_eq!(array_info.type_param_count, 1);
    assert_eq!(array_info.inference_methods.len(), 3); // Добавить, Вставить, Найти
}

#[test]
fn test_generic_info_registry_contains_map() {
    let registry = get_generic_info_registry();

    let map_info = registry
        .get("Соответствие")
        .expect("Соответствие должен быть в реестре");
    assert_eq!(map_info.base_type, "Соответствие");
    assert_eq!(map_info.type_param_count, 2); // K и V
    assert_eq!(map_info.inference_methods.len(), 4);
}

#[test]
fn test_generic_info_registry_contains_value_list() {
    let registry = get_generic_info_registry();

    let list_info = registry
        .get("СписокЗначений")
        .expect("СписокЗначений должен быть в реестре");
    assert_eq!(list_info.base_type, "СписокЗначений");
    assert_eq!(list_info.type_param_count, 1);
    assert_eq!(list_info.inference_methods.len(), 1); // Добавить
}

#[test]
fn test_generic_info_registry_contains_tabular_section() {
    let registry = get_generic_info_registry();

    let tab_info = registry
        .get("ТабличнаяЧасть")
        .expect("ТабличнаяЧасть должен быть в реестре");
    assert_eq!(tab_info.base_type, "ТабличнаяЧасть");
    assert_eq!(tab_info.type_param_count, 1);
    assert_eq!(tab_info.inference_methods.len(), 6);
}

#[test]
fn test_array_add_method_inference() {
    let registry = get_generic_info_registry();
    let array_info = registry.get("Массив").unwrap();

    let add_inference = array_info
        .inference_methods
        .iter()
        .find(|m| m.method_name == "Добавить")
        .expect("Добавить должен быть в inference_methods");

    assert_eq!(add_inference.param_indices, vec![0]); // первый параметр
    assert_eq!(add_inference.inferred_type_params, vec![0]); // выводим T
}

#[test]
fn test_map_insert_infers_both_params() {
    let registry = get_generic_info_registry();
    let map_info = registry.get("Соответствие").unwrap();

    let insert_methods: Vec<_> = map_info
        .inference_methods
        .iter()
        .filter(|m| m.method_name == "Вставить")
        .collect();

    // Должно быть 2 записи для Вставить: одна для K, одна для V
    assert_eq!(insert_methods.len(), 2);

    // Первая для ключа (K)
    assert_eq!(insert_methods[0].param_indices, vec![0]);
    assert_eq!(insert_methods[0].inferred_type_params, vec![0]);

    // Вторая для значения (V)
    assert_eq!(insert_methods[1].param_indices, vec![1]);
    assert_eq!(insert_methods[1].inferred_type_params, vec![1]);
}

#[test]
fn test_registry_size() {
    let registry = get_generic_info_registry();

    // Должно быть 4 типа: Массив, Соответствие, СписокЗначений, ТабличнаяЧасть
    assert_eq!(registry.len(), 4);
}
