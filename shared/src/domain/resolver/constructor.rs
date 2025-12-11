//! Constructor Resolution
//!
//! Milestone 2.21: Constructor Resolution

use super::result_types::ConstructorResolution;
use super::strategies::GenericStrategy;
use super::type_resolver::TypeResolver;
use crate::domain::signature_index::SignatureIndex;

impl TypeResolver {
    /// Резолвить конструктор
    pub fn resolve_constructor(
        &self,
        type_name: &str,
        arg_types: &[String],
        signature_index: &SignatureIndex,
    ) -> ConstructorResolution {
        // 1. Проверка на динамический конструктор
        if type_name.is_empty() || type_name == "?" {
            return ConstructorResolution::Dynamic {
                reason: "Динамический конструктор через строку - тип определяется в runtime"
                    .to_string(),
            };
        }

        // 2. Поиск сигнатуры конструктора
        let constructor = match signature_index.find_constructor(type_name) {
            Some(c) => c,
            None => {
                return ConstructorResolution::NotFound {
                    type_name: type_name.to_string(),
                    hint: format!(
                        "Конструктор для типа '{}' не найден в SignatureIndex",
                        type_name
                    ),
                };
            }
        };

        // 3. Валидация параметров
        let validation_errors = self.validate_constructor_params(&constructor.params, arg_types);

        // 4. Generic inference для коллекций
        let generic_params = if constructor.is_collection {
            self.infer_generic_params(type_name, arg_types, constructor.generic_params_count)
        } else {
            None
        };

        // 5. Формирование результата
        ConstructorResolution::Resolved {
            type_name: type_name.to_string(),
            facet: constructor.facet.clone(),
            generic_params,
            validation_errors,
        }
    }

    /// Валидировать параметры конструктора
    fn validate_constructor_params(
        &self,
        expected_params: &[crate::domain::types::ParameterInfo],
        actual_arg_types: &[String],
    ) -> Vec<String> {
        let mut errors = Vec::new();

        // Проверка количества параметров
        let required_count = expected_params.iter().filter(|p| !p.is_optional).count();

        if actual_arg_types.len() < required_count {
            errors.push(format!(
                "Недостаточно аргументов: ожидается минимум {}, передано {}",
                required_count,
                actual_arg_types.len()
            ));
        }

        if actual_arg_types.len() > expected_params.len() {
            errors.push(format!(
                "Слишком много аргументов: ожидается максимум {}, передано {}",
                expected_params.len(),
                actual_arg_types.len()
            ));
        }

        // Проверка типов параметров
        for (i, (param, arg_type)) in expected_params
            .iter()
            .zip(actual_arg_types.iter())
            .enumerate()
        {
            if let Some(expected_type) = &param.type_name {
                // TODO: добавить более сложную проверку совместимости типов
                // Пока простая проверка на точное соответствие
                if expected_type != "Произвольный" && expected_type != arg_type {
                    errors.push(format!(
                        "Параметр {} '{}': ожидается тип {}, передан {}",
                        i + 1,
                        param.name,
                        expected_type,
                        arg_type
                    ));
                }
            }
        }

        errors
    }

    /// Вывести generic параметры для коллекций
    fn infer_generic_params(
        &self,
        type_name: &str,
        arg_types: &[String],
        generic_count: usize,
    ) -> Option<Vec<String>> {
        if generic_count == 0 {
            return None;
        }

        match type_name {
            "Массив" | "Array" => {
                // Массив может быть создан с начальным размером
                // Новый Массив(10) → Массив<?>
                Some(vec!["?".to_string()])
            }

            "ФиксированныйМассив" | "FixedArray" => {
                // Новый ФиксированныйМассив(ИсходныйМассив)
                if !arg_types.is_empty() {
                    let generic = GenericStrategy::extract_from_type(&arg_types[0])
                        .unwrap_or_else(|| "?".to_string());
                    Some(vec![generic])
                } else {
                    Some(vec!["?".to_string()])
                }
            }

            "Соответствие" | "Map" => {
                // Соответствие<K, V>
                Some(vec!["?".to_string(), "?".to_string()])
            }

            "СписокЗначений" | "ValueList" => {
                // СписокЗначений<T>
                Some(vec!["?".to_string()])
            }

            _ => {
                // Для неизвестных коллекций возвращаем "?" для каждого generic
                Some(vec!["?".to_string(); generic_count])
            }
        }
    }
}
