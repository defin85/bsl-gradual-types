//! Интеграционные тесты для платформенного типа ТабличнаяЧасть

use bsl_backend::data::loaders::platform_types::load_all_platform_types;
use bsl_shared::domain::types::FacetKind;

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
    let add_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Добавить")
        .unwrap();
    assert_eq!(add_method.return_type, "T");
    assert_eq!(add_method.params.len(), 0);

    let get_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Получить")
        .unwrap();
    assert_eq!(get_method.return_type, "T");
    assert_eq!(get_method.params.len(), 1);
    assert_eq!(get_method.params[0].name, "Индекс");

    let insert_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Вставить")
        .unwrap();
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
    let count_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Количество")
        .unwrap();
    assert_eq!(count_method.return_type, "Число");

    let clear_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Очистить")
        .unwrap();
    assert_eq!(clear_method.return_type, "Неопределено");

    let delete_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Удалить")
        .unwrap();
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
    let count_property = tabular_type
        .properties
        .iter()
        .find(|p| p.name == "Количество")
        .unwrap();
    assert_eq!(count_property.prop_type, "Число");
    assert!(count_property.is_readonly);
}

#[test]
fn test_tabular_section_find_method_params() {
    let platform_types = load_all_platform_types();

    let tabular_type = platform_types
        .iter()
        .find(|t| t.name == "ТабличнаяЧасть")
        .unwrap();

    // Проверяем метод Найти с параметрами
    let find_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Найти")
        .unwrap();
    assert_eq!(find_method.return_type, "T");
    assert_eq!(find_method.params.len(), 2);
    assert_eq!(find_method.params[0].name, "Значение");
    assert_eq!(find_method.params[0].param_type, "Произвольный");
    assert_eq!(find_method.params[1].name, "ИмяКолонки");
    assert_eq!(find_method.params[1].param_type, "Строка");
}

#[test]
fn test_all_16_methods_present() {
    let platform_types = load_all_platform_types();

    let tabular_type = platform_types
        .iter()
        .find(|t| t.name == "ТабличнаяЧасть")
        .unwrap();

    // Проверяем наличие всех 16 методов по имени
    let expected_methods = vec![
        "Добавить",
        "Вставить",
        "Получить",
        "Удалить",
        "Количество",
        "Очистить",
        "Индекс",
        "Найти",
        "Сдвинуть",
        "ВыгрузитьКолонку",
        "ЗагрузитьКолонку",
        "Свернуть",
        "Скопировать",
        "Итог",
        "Заполнить",
        "Сортировать",
    ];

    for expected_method_name in expected_methods {
        assert!(
            tabular_type
                .methods
                .iter()
                .any(|m| m.name == expected_method_name),
            "Метод '{}' не найден",
            expected_method_name
        );
    }

    assert_eq!(tabular_type.methods.len(), 16);
}

#[test]
fn test_generic_parameter_in_copy_method() {
    let platform_types = load_all_platform_types();

    let tabular_type = platform_types
        .iter()
        .find(|t| t.name == "ТабличнаяЧасть")
        .unwrap();

    // Проверяем метод Скопировать с Generic типом в return_type
    let copy_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Скопировать")
        .unwrap();
    assert_eq!(copy_method.return_type, "ТабличнаяЧасть<T>");
}

#[test]
fn test_english_names_present() {
    let platform_types = load_all_platform_types();

    let tabular_type = platform_types
        .iter()
        .find(|t| t.name == "ТабличнаяЧасть")
        .unwrap();

    // Проверяем наличие английских имён у методов
    let add_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Добавить")
        .unwrap();
    assert_eq!(add_method.english_name, "Add");

    let count_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Количество")
        .unwrap();
    assert_eq!(count_method.english_name, "Count");

    let find_method = tabular_type
        .methods
        .iter()
        .find(|m| m.name == "Найти")
        .unwrap();
    assert_eq!(find_method.english_name, "Find");
}
