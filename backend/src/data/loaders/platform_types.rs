//! Platform types loader
//!
//! Определения GenericInfo для типов-коллекций платформы 1С.
//!
//! ## Архитектура (Milestone 3.x: Унификация источников данных)
//!
//! **Данные о методах** (параметры, описания, return_type) — из syntax_helper.
//! **GenericInfo** (inference rules) — из этого модуля.
//!
//! GenericInfo содержит InferenceMethodInfo — правила для вывода Generic типов
//! на основе аргументов методов во время анализа кода.

use std::collections::HashMap;

use bsl_shared::domain::types::{GenericInfo, InferenceMethodInfo};

/// Возвращает реестр GenericInfo для типов-коллекций платформы
///
/// Эти метаданные используются для вывода Generic параметров (T, K, V)
/// на основе вызовов методов во время анализа кода.
///
/// # Пример использования
///
/// ```ignore
/// let registry = get_generic_info_registry();
/// if let Some(info) = registry.get("Массив") {
///     repository.set_generic_info("Массив", info.clone());
/// }
/// ```
///
/// # Поддерживаемые типы
///
/// - `Массив<T>` — динамический массив
/// - `Соответствие<K, V>` — ассоциативный массив
/// - `СписокЗначений<T>` — список значений с представлениями
/// - `ТабличнаяЧасть<T>` — табличная часть объекта
pub fn get_generic_info_registry() -> HashMap<String, GenericInfo> {
    let mut registry = HashMap::new();

    // ==================== Массив<T> ====================
    registry.insert(
        "Массив".to_string(),
        GenericInfo {
            base_type: "Массив".to_string(),
            type_param_count: 1, // только T
            inference_methods: vec![
                // Массив.Добавить(Значение: T) → выводим T из первого параметра
                InferenceMethodInfo {
                    method_name: "Добавить".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
                // Массив.Вставить(Индекс, Значение: T) → выводим T из второго параметра
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![1],
                    inferred_type_params: vec![0],
                },
                // Массив.Найти(Значение: T) → выводим T из первого параметра
                InferenceMethodInfo {
                    method_name: "Найти".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
            ],
        },
    );

    // ==================== Соответствие<K, V> ====================
    registry.insert(
        "Соответствие".to_string(),
        GenericInfo {
            base_type: "Соответствие".to_string(),
            type_param_count: 2, // K и V
            inference_methods: vec![
                // Соответствие.Вставить(Ключ: K, Значение: V) → выводим K из param[0]
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0], // K
                },
                // Соответствие.Вставить(Ключ: K, Значение: V) → выводим V из param[1]
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![1],
                    inferred_type_params: vec![1], // V
                },
                // Соответствие.Получить(Ключ: K) → выводим K из первого параметра
                InferenceMethodInfo {
                    method_name: "Получить".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0], // K
                },
                // Соответствие.Удалить(Ключ: K) → выводим K из первого параметра
                InferenceMethodInfo {
                    method_name: "Удалить".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0], // K
                },
            ],
        },
    );

    // ==================== СписокЗначений<T> ====================
    registry.insert(
        "СписокЗначений".to_string(),
        GenericInfo {
            base_type: "СписокЗначений".to_string(),
            type_param_count: 1, // только T
            inference_methods: vec![
                // СписокЗначений.Добавить(Значение: T, Представление?) → выводим T из первого параметра
                InferenceMethodInfo {
                    method_name: "Добавить".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
            ],
        },
    );

    // ==================== ТабличнаяЧасть<T> ====================
    registry.insert(
        "ТабличнаяЧасть".to_string(),
        GenericInfo {
            base_type: "ТабличнаяЧасть".to_string(),
            type_param_count: 1, // T — тип строки табличной части
            inference_methods: vec![
                // ТабличнаяЧасть.Добавить() → возвращает T (новую строку)
                InferenceMethodInfo {
                    method_name: "Добавить".to_string(),
                    param_indices: vec![],
                    inferred_type_params: vec![0],
                },
                // ТабличнаяЧасть.Вставить(Индекс) → возвращает T
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![],
                    inferred_type_params: vec![0],
                },
                // ТабличнаяЧасть.Получить(Индекс) → возвращает T
                InferenceMethodInfo {
                    method_name: "Получить".to_string(),
                    param_indices: vec![],
                    inferred_type_params: vec![0],
                },
                // ТабличнаяЧасть.Найти(Значение, ИмяКолонки?) → возвращает T
                InferenceMethodInfo {
                    method_name: "Найти".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
                // ТабличнаяЧасть.Индекс(Строка: T) → выводим T из первого параметра
                InferenceMethodInfo {
                    method_name: "Индекс".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
                // ТабличнаяЧасть.Сдвинуть(Строка: T, Смещение) → выводим T из первого параметра
                InferenceMethodInfo {
                    method_name: "Сдвинуть".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
            ],
        },
    );

    registry
}

/// Применяет GenericInfo из реестра к типам в TypeRepository
///
/// Вызывается после загрузки типов из syntax_helper для добавления
/// inference metadata к типам-коллекциям.
///
/// # Arguments
///
/// * `repository` - репозиторий типов
///
/// # Returns
///
/// Количество успешно применённых GenericInfo
pub fn apply_generic_info_to_repository<R: bsl_shared::domain::repository::TypeRepository + ?Sized>(
    repository: &R,
) -> usize {
    let registry = get_generic_info_registry();
    let mut applied_count = 0;

    for (type_name, generic_info) in registry {
        if repository.set_generic_info(&type_name, generic_info) {
            applied_count += 1;
            tracing::info!(
                "✅ GenericInfo применён к типу '{}'",
                type_name
            );
        } else {
            tracing::warn!(
                "⚠️ Тип '{}' не найден в репозитории, GenericInfo не применён",
                type_name
            );
        }
    }

    applied_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_info_registry_contains_array() {
        let registry = get_generic_info_registry();

        let array_info = registry.get("Массив").expect("Массив должен быть в реестре");
        assert_eq!(array_info.base_type, "Массив");
        assert_eq!(array_info.type_param_count, 1);
        assert_eq!(array_info.inference_methods.len(), 3); // Добавить, Вставить, Найти
    }

    #[test]
    fn test_generic_info_registry_contains_map() {
        let registry = get_generic_info_registry();

        let map_info = registry.get("Соответствие").expect("Соответствие должен быть в реестре");
        assert_eq!(map_info.base_type, "Соответствие");
        assert_eq!(map_info.type_param_count, 2); // K и V
        assert_eq!(map_info.inference_methods.len(), 4);
    }

    #[test]
    fn test_generic_info_registry_contains_value_list() {
        let registry = get_generic_info_registry();

        let list_info = registry.get("СписокЗначений").expect("СписокЗначений должен быть в реестре");
        assert_eq!(list_info.base_type, "СписокЗначений");
        assert_eq!(list_info.type_param_count, 1);
        assert_eq!(list_info.inference_methods.len(), 1); // Добавить
    }

    #[test]
    fn test_generic_info_registry_contains_tabular_section() {
        let registry = get_generic_info_registry();

        let tab_info = registry.get("ТабличнаяЧасть").expect("ТабличнаяЧасть должен быть в реестре");
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
}
