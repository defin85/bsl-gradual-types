//! Platform types loader
//!
//! Создание и загрузка базовых платформенных типов 1С:Предприятие

use bsl_shared::domain::types::{
    FacetKind, GenericInfo, InferenceMethodInfo, RawDataSource, RawMethodData, RawParamData,
    RawPropertyData, RawTypeData,
};

/// Создаёт тип "ТабличнаяЧасть" с Generic методами
///
/// # Generic параметр T
/// Все методы используют параметр "T", который будет заменён на конкретный
/// тип строки табличной части (например, СтрокаРаботы).
///
/// # Примеры методов
/// - `Добавить(): T` - возвращает новую строку типа T
/// - `Получить(индекс: Число): T` - возвращает строку типа T
/// - `Количество(): Число` - возвращает число строк
///
/// # Фасеты
/// - `Collection` - коллекция строк
pub fn create_tabular_section_type() -> RawTypeData {
    RawTypeData {
        name: "ТабличнаяЧасть".to_string(),
        english_name: "TabularSection".to_string(),
        description: "Табличная часть - коллекция строк конфигурационного объекта с методами управления".to_string(),
        category: "PlatformType".to_string(),
        source: RawDataSource::Platform,

        // ✅ МЕТОДЫ С GENERIC ПАРАМЕТРОМ "T"
        methods: vec![
            // 1. Добавить() - создаёт новую строку и возвращает её
            RawMethodData {
                name: "Добавить".to_string(),
                english_name: "Add".to_string(),
                return_type: "T".to_string(),  // ← Generic!
                params: vec![],
            },

            // 2. Вставить(индекс: Число) - вставляет строку в указанную позицию
            RawMethodData {
                name: "Вставить".to_string(),
                english_name: "Insert".to_string(),
                return_type: "T".to_string(),  // ← Generic!
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 3. Получить(индекс: Число): T - возвращает строку по индексу
            RawMethodData {
                name: "Получить".to_string(),
                english_name: "Get".to_string(),
                return_type: "T".to_string(),  // ← Generic!
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 4. Удалить(индекс: Число) - удаляет строку по индексу
            RawMethodData {
                name: "Удалить".to_string(),
                english_name: "Delete".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 5. Количество(): Число - возвращает количество строк
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(),
                params: vec![],
            },

            // 6. Очистить() - удаляет все строки
            RawMethodData {
                name: "Очистить".to_string(),
                english_name: "Clear".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![],
            },

            // 7. Индекс(строка: T): Число - возвращает индекс строки
            RawMethodData {
                name: "Индекс".to_string(),
                english_name: "IndexOf".to_string(),
                return_type: "Число".to_string(),
                params: vec![
                    RawParamData {
                        name: "Строка".to_string(),
                        param_type: "T".to_string(),  // ← Generic!
                        is_optional: false,
                    },
                ],
            },

            // 8. Найти(значение: Произвольный, имяКолонки: Строка): T
            RawMethodData {
                name: "Найти".to_string(),
                english_name: "Find".to_string(),
                return_type: "T".to_string(),  // ← Generic!
                params: vec![
                    RawParamData {
                        name: "Значение".to_string(),
                        param_type: "Произвольный".to_string(),
                        is_optional: false,
                    },
                    RawParamData {
                        name: "ИмяКолонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },

            // 9. Сдвинуть(строка: T, смещение: Число)
            RawMethodData {
                name: "Сдвинуть".to_string(),
                english_name: "Move".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Строка".to_string(),
                        param_type: "T".to_string(),  // ← Generic!
                        is_optional: false,
                    },
                    RawParamData {
                        name: "Смещение".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 10. ВыгрузитьКолонку(имяКолонки: Строка): Массив
            RawMethodData {
                name: "ВыгрузитьКолонку".to_string(),
                english_name: "UnloadColumn".to_string(),
                return_type: "Массив".to_string(),
                params: vec![
                    RawParamData {
                        name: "ИмяКолонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 11. ЗагрузитьКолонку(массив: Массив, имяКолонки: Строка)
            RawMethodData {
                name: "ЗагрузитьКолонку".to_string(),
                english_name: "LoadColumn".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Массив".to_string(),
                        param_type: "Массив".to_string(),
                        is_optional: false,
                    },
                    RawParamData {
                        name: "ИмяКолонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 12. Свернуть(имяКолонокГруппировок: Строка, имяКолонокСуммирования: Строка)
            RawMethodData {
                name: "Свернуть".to_string(),
                english_name: "GroupBy".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "ИменаКолонокГруппировок".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                    RawParamData {
                        name: "ИменаКолонокСуммирования".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },

            // 13. Скопировать(параметры: Структура): ТабличнаяЧасть<T>
            RawMethodData {
                name: "Скопировать".to_string(),
                english_name: "Copy".to_string(),
                return_type: "ТабличнаяЧасть<T>".to_string(),  // ← Generic тип!
                params: vec![
                    RawParamData {
                        name: "Параметры".to_string(),
                        param_type: "Структура".to_string(),
                        is_optional: true,
                    },
                ],
            },

            // 14. Итог(имяКолонки: Строка): Число
            RawMethodData {
                name: "Итог".to_string(),
                english_name: "Total".to_string(),
                return_type: "Число".to_string(),
                params: vec![
                    RawParamData {
                        name: "ИмяКолонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: false,
                    },
                ],
            },

            // 15. Заполнить(значение: Произвольный, имяКолонки: Строка)
            RawMethodData {
                name: "Заполнить".to_string(),
                english_name: "Fill".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Значение".to_string(),
                        param_type: "Произвольный".to_string(),
                        is_optional: false,
                    },
                    RawParamData {
                        name: "ИмяКолонки".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },

            // 16. Сортировать(имяКолонок: Строка, направление: Строка)
            RawMethodData {
                name: "Сортировать".to_string(),
                english_name: "Sort".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "ИменаКолонок".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                    RawParamData {
                        name: "Направление".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },
        ],

        // Свойства (количество доступно как свойство для чтения)
        properties: vec![
            RawPropertyData {
                name: "Количество".to_string(),
                prop_type: "Число".to_string(),
                is_readonly: true,
            },
        ],

        facets: vec![FacetKind::Collection],
        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: Some(GenericInfo {
            base_type: "ТабличнаяЧасть".to_string(),
            type_param_count: 1,
            inference_methods: vec![
                InferenceMethodInfo {
                    method_name: "Добавить".to_string(),
                    param_indices: vec![],
                    inferred_type_params: vec![0],
                },
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![],
                    inferred_type_params: vec![0],
                },
                InferenceMethodInfo {
                    method_name: "Получить".to_string(),
                    param_indices: vec![],
                    inferred_type_params: vec![0],
                },
                InferenceMethodInfo {
                    method_name: "Найти".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
                InferenceMethodInfo {
                    method_name: "Индекс".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
                InferenceMethodInfo {
                    method_name: "Сдвинуть".to_string(),
                    param_indices: vec![0],
                    inferred_type_params: vec![0],
                },
            ],
        }),
    }
}

/// Создаёт тип "Массив" с Generic метаданными
///
/// # Generic параметр T
/// Все методы используют параметр "T", который будет заменён на конкретный
/// тип элемента (например, Строка, Число).
///
/// # Примеры методов
/// - `Добавить(Значение: T)` - добавляет элемент типа T
/// - `Получить(Индекс): T` - возвращает элемент типа T
/// - `Количество()` - возвращает количество элементов
///
/// # Фасеты
/// - `Collection` - коллекция элементов
pub fn create_array_type() -> RawTypeData {
    RawTypeData {
        name: "Массив".to_string(),
        english_name: "Array".to_string(),
        description: "Динамический массив произвольных значений".to_string(),
        category: "PlatformType".to_string(),
        source: RawDataSource::Platform,

        // ✅ МЕТОДЫ С GENERIC ПАРАМЕТРОМ "T"
        methods: vec![
            RawMethodData {
                name: "Добавить".to_string(),
                english_name: "Add".to_string(),
                return_type: "Число".to_string(), // индекс добавленного элемента
                params: vec![RawParamData {
                    name: "Значение".to_string(),
                    param_type: "T".to_string(), // ← Generic!
                    is_optional: false,
                }],
            },
            RawMethodData {
                name: "Вставить".to_string(),
                english_name: "Insert".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Индекс".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                    },
                    RawParamData {
                        name: "Значение".to_string(),
                        param_type: "T".to_string(), // ← Generic!
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Получить".to_string(),
                english_name: "Get".to_string(),
                return_type: "T".to_string(), // ← Generic!
                params: vec![RawParamData {
                    name: "Индекс".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: false,
                }],
            },
            RawMethodData {
                name: "Удалить".to_string(),
                english_name: "Delete".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![RawParamData {
                    name: "Индекс".to_string(),
                    param_type: "Число".to_string(),
                    is_optional: false,
                }],
            },
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Очистить".to_string(),
                english_name: "Clear".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Найти".to_string(),
                english_name: "Find".to_string(),
                return_type: "Число".to_string(), // индекс найденного элемента или -1
                params: vec![RawParamData {
                    name: "Значение".to_string(),
                    param_type: "T".to_string(), // ← Generic!
                    is_optional: false,
                }],
            },
        ],

        properties: vec![RawPropertyData {
            name: "Количество".to_string(),
            prop_type: "Число".to_string(),
            is_readonly: true,
        }],

        facets: vec![FacetKind::Collection],

        // ✅ Generic метаданные для Массив<T>
        generic_info: Some(GenericInfo {
            base_type: "Массив".to_string(),
            type_param_count: 1, // только T
            inference_methods: vec![
                InferenceMethodInfo {
                    method_name: "Добавить".to_string(),
                    param_indices: vec![0], // первый параметр (Значение)
                    inferred_type_params: vec![0], // выводим T (параметр 0)
                },
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![1], // второй параметр (Значение)
                    inferred_type_params: vec![0], // выводим T
                },
                InferenceMethodInfo {
                    method_name: "Найти".to_string(),
                    param_indices: vec![0], // первый параметр (Значение)
                    inferred_type_params: vec![0], // выводим T
                },
            ],
        }),

        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
    }
}

/// Создаёт тип "Соответствие" с Generic метаданными (2 параметра!)
///
/// # Generic параметры K, V
/// K — тип ключа, V — тип значения
///
/// # Примеры методов
/// - `Вставить(Ключ: K, Значение: V)` - вставляет пару ключ-значение
/// - `Получить(Ключ: K): V` - получает значение по ключу
/// - `Количество()` - возвращает количество пар ключ-значение
///
/// # Фасеты
/// - `Collection` - коллекция пар ключ-значение
pub fn create_map_type() -> RawTypeData {
    RawTypeData {
        name: "Соответствие".to_string(),
        english_name: "Map".to_string(),
        description: "Ассоциативный массив (словарь) ключ-значение".to_string(),
        category: "PlatformType".to_string(),
        source: RawDataSource::Platform,

        // ✅ МЕТОДЫ С GENERIC ПАРАМЕТРАМИ "K" И "V"
        methods: vec![
            RawMethodData {
                name: "Вставить".to_string(),
                english_name: "Insert".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Ключ".to_string(),
                        param_type: "K".to_string(), // ← Generic параметр K
                        is_optional: false,
                    },
                    RawParamData {
                        name: "Значение".to_string(),
                        param_type: "V".to_string(), // ← Generic параметр V
                        is_optional: false,
                    },
                ],
            },
            RawMethodData {
                name: "Получить".to_string(),
                english_name: "Get".to_string(),
                return_type: "V".to_string(), // ← Generic параметр V
                params: vec![RawParamData {
                    name: "Ключ".to_string(),
                    param_type: "K".to_string(), // ← Generic параметр K
                    is_optional: false,
                }],
            },
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(),
                params: vec![],
            },
            RawMethodData {
                name: "Удалить".to_string(),
                english_name: "Delete".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![RawParamData {
                    name: "Ключ".to_string(),
                    param_type: "K".to_string(), // ← Generic параметр K
                    is_optional: false,
                }],
            },
            RawMethodData {
                name: "Очистить".to_string(),
                english_name: "Clear".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![],
            },
        ],

        properties: vec![RawPropertyData {
            name: "Количество".to_string(),
            prop_type: "Число".to_string(),
            is_readonly: true,
        }],

        facets: vec![FacetKind::Collection],

        // ✅ Generic метаданные для Соответствие<K, V>
        generic_info: Some(GenericInfo {
            base_type: "Соответствие".to_string(),
            type_param_count: 2, // K и V
            inference_methods: vec![
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![0], // первый параметр (Ключ)
                    inferred_type_params: vec![0], // выводим K (параметр 0)
                },
                InferenceMethodInfo {
                    method_name: "Вставить".to_string(),
                    param_indices: vec![1], // второй параметр (Значение)
                    inferred_type_params: vec![1], // выводим V (параметр 1)
                },
                InferenceMethodInfo {
                    method_name: "Получить".to_string(),
                    param_indices: vec![0], // первый параметр (Ключ)
                    inferred_type_params: vec![0], // выводим K
                },
                InferenceMethodInfo {
                    method_name: "Удалить".to_string(),
                    param_indices: vec![0], // первый параметр (Ключ)
                    inferred_type_params: vec![0], // выводим K
                },
            ],
        }),

        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
    }
}

/// Создаёт тип "СписокЗначений" с Generic метаданными
///
/// # Generic параметр T
/// СписокЗначений<T> — список значений с представлением
///
/// # Примеры методов
/// - `Добавить(Значение: T, Представление?: Строка)` - добавляет элемент
/// - `Количество()` - возвращает количество элементов
///
/// # Фасеты
/// - `Collection` - коллекция значений
pub fn create_value_list_type() -> RawTypeData {
    RawTypeData {
        name: "СписокЗначений".to_string(),
        english_name: "ValueList".to_string(),
        description: "Список значений с возможностью хранения представления".to_string(),
        category: "PlatformType".to_string(),
        source: RawDataSource::Platform,

        methods: vec![
            RawMethodData {
                name: "Добавить".to_string(),
                english_name: "Add".to_string(),
                return_type: "Неопределено".to_string(),
                params: vec![
                    RawParamData {
                        name: "Значение".to_string(),
                        param_type: "T".to_string(), // ← Generic!
                        is_optional: false,
                    },
                    RawParamData {
                        name: "Представление".to_string(),
                        param_type: "Строка".to_string(),
                        is_optional: true,
                    },
                ],
            },
            RawMethodData {
                name: "Количество".to_string(),
                english_name: "Count".to_string(),
                return_type: "Число".to_string(),
                params: vec![],
            },
        ],

        properties: vec![RawPropertyData {
            name: "Количество".to_string(),
            prop_type: "Число".to_string(),
            is_readonly: true,
        }],

        facets: vec![FacetKind::Collection],

        // ✅ Generic метаданные для СписокЗначений<T>
        generic_info: Some(GenericInfo {
            base_type: "СписокЗначений".to_string(),
            type_param_count: 1,
            inference_methods: vec![InferenceMethodInfo {
                method_name: "Добавить".to_string(),
                param_indices: vec![0], // первый параметр (Значение)
                inferred_type_params: vec![0], // выводим T
            }],
        }),

        kind: None,
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
    }
}

/// Загружает все платформенные типы в репозиторий
pub fn load_all_platform_types() -> Vec<RawTypeData> {
    vec![
        create_tabular_section_type(),
        create_array_type(),
        create_map_type(),
        create_value_list_type(),
        // Здесь будут добавлены другие платформенные типы:
        // create_string_type(),
        // create_number_type(),
        // etc.
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tabular_section_type_exists() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .expect("ТабличнаяЧасть должен быть в платформенных типах");

        assert_eq!(tabular_type.category, "PlatformType");
        assert_eq!(tabular_type.english_name, "TabularSection");
    }

    #[test]
    fn test_tabular_section_has_collection_facet() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .unwrap();

        assert!(tabular_type.facets.contains(&FacetKind::Collection));
    }

    #[test]
    fn test_tabular_section_has_generic_methods() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .unwrap();

        // Проверяем методы с Generic параметром "T"
        let add_method = tabular_type.methods.iter().find(|m| m.name == "Добавить").unwrap();
        assert_eq!(add_method.return_type, "T");
        assert_eq!(add_method.params.len(), 0);

        let get_method = tabular_type.methods.iter().find(|m| m.name == "Получить").unwrap();
        assert_eq!(get_method.return_type, "T");
        assert_eq!(get_method.params.len(), 1);
        assert_eq!(get_method.params[0].name, "Индекс");

        let insert_method = tabular_type.methods.iter().find(|m| m.name == "Вставить").unwrap();
        assert_eq!(insert_method.return_type, "T");
    }

    #[test]
    fn test_tabular_section_has_non_generic_methods() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .unwrap();

        // Проверяем методы БЕЗ Generic параметра
        let count_method = tabular_type.methods.iter().find(|m| m.name == "Количество").unwrap();
        assert_eq!(count_method.return_type, "Число");

        let clear_method = tabular_type.methods.iter().find(|m| m.name == "Очистить").unwrap();
        assert_eq!(clear_method.return_type, "Неопределено");

        let delete_method = tabular_type.methods.iter().find(|m| m.name == "Удалить").unwrap();
        assert_eq!(delete_method.return_type, "Неопределено");
    }

    #[test]
    fn test_tabular_section_method_count() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .unwrap();

        // Должно быть 16 методов
        assert_eq!(tabular_type.methods.len(), 16);
    }

    #[test]
    fn test_tabular_section_has_count_property() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .unwrap();

        // Проверяем свойство Количество
        let count_property = tabular_type.properties.iter().find(|p| p.name == "Количество").unwrap();
        assert_eq!(count_property.prop_type, "Число");
        assert_eq!(count_property.is_readonly, true);
    }

    #[test]
    fn test_tabular_section_find_method_params() {
        let platform_types = load_all_platform_types();

        let tabular_type = platform_types
            .iter()
            .find(|t| t.name == "ТабличнаяЧасть")
            .unwrap();

        // Проверяем метод Найти с параметрами
        let find_method = tabular_type.methods.iter().find(|m| m.name == "Найти").unwrap();
        assert_eq!(find_method.return_type, "T");
        assert_eq!(find_method.params.len(), 2);
        assert_eq!(find_method.params[0].name, "Значение");
        assert_eq!(find_method.params[0].param_type, "Произвольный");
        assert_eq!(find_method.params[1].name, "ИмяКолонки");
        assert_eq!(find_method.params[1].param_type, "Строка");
    }

    // --- Тесты для Generic коллекций (Этап 1.2) ---

    #[test]
    fn test_array_type_exists() {
        let platform_types = load_all_platform_types();

        let array_type = platform_types
            .iter()
            .find(|t| t.name == "Массив")
            .expect("Массив должен быть в платформенных типах");

        assert_eq!(array_type.category, "PlatformType");
        assert_eq!(array_type.english_name, "Array");
    }

    #[test]
    fn test_array_has_generic_info() {
        let platform_types = load_all_platform_types();

        let array_type = platform_types
            .iter()
            .find(|t| t.name == "Массив")
            .unwrap();

        assert!(array_type.generic_info.is_some());
        let generic_info = array_type.generic_info.as_ref().unwrap();

        assert_eq!(generic_info.base_type, "Массив");
        assert_eq!(generic_info.type_param_count, 1); // только T
        assert_eq!(generic_info.inference_methods.len(), 3); // Добавить, Вставить, Найти
    }

    #[test]
    fn test_array_add_method_inference() {
        let platform_types = load_all_platform_types();

        let array_type = platform_types
            .iter()
            .find(|t| t.name == "Массив")
            .unwrap();

        let generic_info = array_type.generic_info.as_ref().unwrap();
        let add_inference = generic_info
            .inference_methods
            .iter()
            .find(|m| m.method_name == "Добавить")
            .unwrap();

        assert_eq!(add_inference.param_indices, vec![0]); // первый параметр
        assert_eq!(add_inference.inferred_type_params, vec![0]); // выводим T
    }

    #[test]
    fn test_array_has_generic_methods() {
        let platform_types = load_all_platform_types();

        let array_type = platform_types
            .iter()
            .find(|t| t.name == "Массив")
            .unwrap();

        // Проверяем методы с Generic параметром "T"
        let add_method = array_type
            .methods
            .iter()
            .find(|m| m.name == "Добавить")
            .unwrap();
        assert_eq!(add_method.params[0].param_type, "T");

        let get_method = array_type
            .methods
            .iter()
            .find(|m| m.name == "Получить")
            .unwrap();
        assert_eq!(get_method.return_type, "T");

        let find_method = array_type
            .methods
            .iter()
            .find(|m| m.name == "Найти")
            .unwrap();
        assert_eq!(find_method.params[0].param_type, "T");
    }

    #[test]
    fn test_map_type_exists() {
        let platform_types = load_all_platform_types();

        let map_type = platform_types
            .iter()
            .find(|t| t.name == "Соответствие")
            .expect("Соответствие должен быть в платформенных типах");

        assert_eq!(map_type.category, "PlatformType");
        assert_eq!(map_type.english_name, "Map");
    }

    #[test]
    fn test_map_has_generic_info() {
        let platform_types = load_all_platform_types();

        let map_type = platform_types
            .iter()
            .find(|t| t.name == "Соответствие")
            .unwrap();

        assert!(map_type.generic_info.is_some());
        let generic_info = map_type.generic_info.as_ref().unwrap();

        assert_eq!(generic_info.base_type, "Соответствие");
        assert_eq!(generic_info.type_param_count, 2); // K и V
        assert_eq!(generic_info.inference_methods.len(), 4); // Вставить (2), Получить, Удалить
    }

    #[test]
    fn test_map_two_type_params() {
        let platform_types = load_all_platform_types();

        let map_type = platform_types
            .iter()
            .find(|t| t.name == "Соответствие")
            .unwrap();

        let generic_info = map_type.generic_info.as_ref().unwrap();

        // Методы Вставить должны иметь разные inferred_type_params
        let insert_methods: Vec<_> = generic_info
            .inference_methods
            .iter()
            .filter(|m| m.method_name == "Вставить")
            .collect();

        assert_eq!(insert_methods.len(), 2);

        // Первый Вставить для ключа
        assert_eq!(insert_methods[0].inferred_type_params, vec![0]);
        // Второй Вставить для значения
        assert_eq!(insert_methods[1].inferred_type_params, vec![1]);
    }

    #[test]
    fn test_map_has_generic_methods() {
        let platform_types = load_all_platform_types();

        let map_type = platform_types
            .iter()
            .find(|t| t.name == "Соответствие")
            .unwrap();

        // Проверяем методы с Generic параметрами K и V
        let insert_method = map_type
            .methods
            .iter()
            .find(|m| m.name == "Вставить")
            .unwrap();
        assert_eq!(insert_method.params[0].param_type, "K");
        assert_eq!(insert_method.params[1].param_type, "V");

        let get_method = map_type
            .methods
            .iter()
            .find(|m| m.name == "Получить")
            .unwrap();
        assert_eq!(get_method.params[0].param_type, "K");
        assert_eq!(get_method.return_type, "V");
    }

    #[test]
    fn test_value_list_type_exists() {
        let platform_types = load_all_platform_types();

        let value_list_type = platform_types
            .iter()
            .find(|t| t.name == "СписокЗначений")
            .expect("СписокЗначений должен быть в платформенных типах");

        assert_eq!(value_list_type.category, "PlatformType");
        assert_eq!(value_list_type.english_name, "ValueList");
    }

    #[test]
    fn test_value_list_has_generic_info() {
        let platform_types = load_all_platform_types();

        let value_list_type = platform_types
            .iter()
            .find(|t| t.name == "СписокЗначений")
            .unwrap();

        assert!(value_list_type.generic_info.is_some());
        let generic_info = value_list_type.generic_info.as_ref().unwrap();

        assert_eq!(generic_info.base_type, "СписокЗначений");
        assert_eq!(generic_info.type_param_count, 1); // только T
        assert_eq!(generic_info.inference_methods.len(), 1); // Добавить
    }

    #[test]
    fn test_all_collection_types_have_collection_facet() {
        let platform_types = load_all_platform_types();

        let collection_types = vec!["Массив", "Соответствие", "СписокЗначений", "ТабличнаяЧасть"];

        for type_name in collection_types {
            let collection_type = platform_types
                .iter()
                .find(|t| t.name == type_name)
                .unwrap();

            assert!(
                collection_type.facets.contains(&FacetKind::Collection),
                "{} должен иметь Collection фасет",
                type_name
            );
        }
    }
}
