//! Generic Type Inference
//!
//! Автоматический вывод типов Generic коллекций из операций:
//! - `arr.Добавить("текст")` → `Массив<Строка>`
//! - `map.Вставить("ключ", 123)` → `Соответствие<Строка, Число>`

use crate::domain::types::*;
use std::collections::HashMap;

/// Generic type inference engine
pub struct GenericInference {
    /// Отслеживание типов для переменных
    variable_types: HashMap<String, GenericTypeInfo>,
}

/// Информация о выведенном Generic типе
#[derive(Debug, Clone)]
pub struct GenericTypeInfo {
    pub base_type: String,
    pub inferred_params: Vec<ConcreteType>,
    pub confidence: f32,
}

impl GenericInference {
    pub fn new() -> Self {
        Self {
            variable_types: HashMap::new(),
        }
    }

    /// Вывести тип из вызова метода
    ///
    /// # Примеры
    ///
    /// ```ignore
    /// // arr.Добавить("текст") → Массив<Строка>
    /// inference.infer_from_method_call("arr", "Добавить", &[ConcreteType::string()]);
    ///
    /// // map.Вставить("ключ", 123) → Соответствие<Строка, Число>
    /// inference.infer_from_method_call("map", "Вставить", &[
    ///     ConcreteType::string(),
    ///     ConcreteType::number()
    /// ]);
    /// ```
    pub fn infer_from_method_call(
        &mut self,
        variable_name: &str,
        method_name: &str,
        arguments: &[ConcreteType],
    ) -> Option<GenericType> {
        // Определяем базовый тип по имени метода и типам аргументов
        let base_type = self.detect_collection_type_with_args_internal(method_name, arguments)?;

        // Выводим параметры типа из аргументов
        let type_params = match base_type.as_str() {
            "Массив" => self.infer_array_element_type(method_name, arguments)?,
            "Список" => self.infer_list_element_type(method_name, arguments)?,
            "Соответствие" => self.infer_map_types(method_name, arguments)?,
            "ФиксированныйМассив" => {
                self.infer_array_element_type(method_name, arguments)?
            }
            _ => return None,
        };

        // Сохраняем информацию о типе
        self.variable_types.insert(
            variable_name.to_string(),
            GenericTypeInfo {
                base_type: base_type.clone(),
                inferred_params: type_params.clone(),
                confidence: 0.9,
            },
        );

        Some(GenericType {
            base_type,
            type_params,
        })
    }

    /// Получить выведенный тип переменной
    pub fn get_variable_type(&self, variable_name: &str) -> Option<&GenericTypeInfo> {
        self.variable_types.get(variable_name)
    }

    /// Обновить вывод типа на основе новой информации
    pub fn refine_inference(
        &mut self,
        variable_name: &str,
        new_param: ConcreteType,
        param_index: usize,
    ) {
        if let Some(info) = self.variable_types.get_mut(variable_name) {
            if param_index < info.inferred_params.len() {
                // Если типы разные, создаём Union
                let existing = &info.inferred_params[param_index];
                if existing != &new_param {
                    // Для простоты пока просто обновляем
                    // TODO: Создавать Union тип
                    info.inferred_params[param_index] = new_param;
                    info.confidence *= 0.8; // Снижаем уверенность при противоречиях
                }
            }
        }
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /// Определить тип коллекции по имени метода, количеству и типам аргументов
    fn detect_collection_type_with_args_internal(
        &self,
        method_name: &str,
        arguments: &[ConcreteType],
    ) -> Option<String> {
        match method_name {
            // Вставить - нужно различать по типу первого аргумента
            "Вставить" | "Insert" => {
                if arguments.len() == 2 {
                    // Если первый аргумент - Number, это индекс массива
                    if matches!(arguments[0], ConcreteType::Primitive(PrimitiveType::Number)) {
                        Some("Массив".to_string())
                    } else {
                        // Иначе это ключ соответствия
                        Some("Соответствие".to_string())
                    }
                } else {
                    // По умолчанию для других случаев - Массив
                    Some("Массив".to_string())
                }
            }

            // Методы Массива
            "Добавить" | "Add" | "Удалить" | "Delete" | "Найти" | "Find" | "ВГраницах"
            | "InBounds" | "Очистить" | "Clear" => Some("Массив".to_string()),

            // Методы Списка (ValueList)
            "ЗагрузитьЗначения" | "LoadValues" | "НайтиПоЗначению" | "FindByValue" => {
                Some("Список".to_string())
            }

            // Методы Соответствия (Map)
            "Получить" | "Get" | "Установить" | "Set" | "Количество" | "Count" => {
                Some("Соответствие".to_string())
            }

            _ => None,
        }
    }

    /// Определить тип коллекции по имени метода и количеству аргументов (обратная совместимость)
    #[allow(dead_code)]
    fn detect_collection_type_with_args(
        &self,
        method_name: &str,
        arg_count: usize,
    ) -> Option<String> {
        // Создаём пустой массив аргументов для обратной совместимости
        let dummy_args = vec![
            ConcreteType::Platform(PlatformType {
                name: "Произвольный".to_string()
            });
            arg_count
        ];
        self.detect_collection_type_with_args_internal(method_name, &dummy_args)
    }

    /// Определить тип коллекции по имени метода (устаревший метод)
    #[allow(dead_code)]
    fn detect_collection_type(&self, method_name: &str) -> Option<String> {
        self.detect_collection_type_with_args(method_name, 0)
    }

    /// Вывести тип элемента массива из аргументов
    fn infer_array_element_type(
        &self,
        method_name: &str,
        arguments: &[ConcreteType],
    ) -> Option<Vec<ConcreteType>> {
        match method_name {
            // arr.Добавить(элемент) - первый аргумент это тип элемента
            "Добавить" | "Add" => arguments.first().cloned().map(|t| vec![t]),

            // arr.Вставить(индекс, элемент) - второй аргумент это тип элемента
            "Вставить" | "Insert" => arguments.get(1).cloned().map(|t| vec![t]),

            // arr.Найти(значение) - первый аргумент это тип элемента
            "Найти" | "Find" => arguments.first().cloned().map(|t| vec![t]),

            _ => None,
        }
    }

    /// Вывести тип элемента списка из аргументов
    fn infer_list_element_type(
        &self,
        method_name: &str,
        arguments: &[ConcreteType],
    ) -> Option<Vec<ConcreteType>> {
        match method_name {
            "ЗагрузитьЗначения" | "LoadValues" => {
                // Предполагаем, что загружается массив элементов определённого типа
                arguments.first().cloned().map(|t| vec![t])
            }

            "НайтиПоЗначению" | "FindByValue" => {
                arguments.first().cloned().map(|t| vec![t])
            }

            _ => None,
        }
    }

    /// Вывести типы ключа и значения соответствия
    fn infer_map_types(
        &self,
        method_name: &str,
        arguments: &[ConcreteType],
    ) -> Option<Vec<ConcreteType>> {
        match method_name {
            // map.Вставить(ключ, значение)
            "Вставить" | "Insert" | "Установить" | "Set" => {
                if arguments.len() >= 2 {
                    Some(vec![arguments[0].clone(), arguments[1].clone()])
                } else {
                    None
                }
            }

            // map.Получить(ключ) - можем вывести только тип ключа
            "Получить" | "Get" => {
                arguments.first().cloned().map(|key_type| {
                    // Тип значения неизвестен, используем Dynamic
                    vec![key_type, ConcreteType::undefined()]
                })
            }

            _ => None,
        }
    }
}

impl Default for GenericInference {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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

        let result = inference.infer_from_method_call(
            "list",
            "ЗагрузитьЗначения",
            &[ConcreteType::boolean()],
        );

        assert!(result.is_some());
        let generic = result.unwrap();
        assert_eq!(generic.base_type, "Список");
        assert!(matches!(
            generic.type_params[0],
            ConcreteType::Primitive(PrimitiveType::Boolean)
        ));
    }
}
