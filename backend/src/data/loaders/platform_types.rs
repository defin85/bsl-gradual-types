//! Platform types loader
//!
//! Создание и загрузка базовых платформенных типов 1С:Предприятие

use bsl_shared::domain::types::{
    FacetKind, RawDataSource, RawMethodData, RawParamData, RawPropertyData, RawTypeData,
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
    }
}

/// Загружает все платформенные типы в репозиторий
pub fn load_all_platform_types() -> Vec<RawTypeData> {
    vec![
        create_tabular_section_type(),
        // Здесь будут добавлены другие платформенные типы:
        // create_array_type(),
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
}
