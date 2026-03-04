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
/// ```text
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
pub fn apply_generic_info_to_repository<
    R: bsl_shared::domain::repository::TypeRepository + ?Sized,
>(
    repository: &R,
) -> usize {
    let registry = get_generic_info_registry();
    let mut applied_count = 0;

    for (type_name, generic_info) in registry {
        if repository.set_generic_info(&type_name, generic_info) {
            applied_count += 1;
            tracing::info!("✅ GenericInfo применён к типу '{}'", type_name);
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
#[path = "platform_types/tests.rs"]
mod tests;
