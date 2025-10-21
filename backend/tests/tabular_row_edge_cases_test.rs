/// Дополнительные unit тесты для TabularRowType - граничные случаи
///
/// Проверяем:
/// - Пустой список атрибутов
/// - Очень длинные имена
/// - Кириллица
/// - Множественные табличные части
/// - Вложенные типы в атрибутах

use bsl_shared::domain::types::{RawAttributeData, TabularRowType};

#[test]
fn test_tabular_row_empty_attributes() {
    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "Работы".to_string(),
        vec![], // Пустой список атрибутов
    );

    assert_eq!(row_type.attributes.len(), 0);
    assert_eq!(row_type.get_attribute_name(0), None);
    assert_eq!(row_type.parent_type, "Документы.ЗаказНаряды");
    assert_eq!(row_type.tabular_section_name, "Работы");
}

#[test]
fn test_tabular_row_single_attribute() {
    let attrs = vec![RawAttributeData {
        name: "ВидРаботы".to_string(),
        attr_type: "ПланВидовХарактеристикСсылка.ВидыРабот".to_string(),
    }];

    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "Работы".to_string(),
        attrs,
    );

    assert_eq!(row_type.attributes.len(), 1);
    assert_eq!(row_type.get_attribute_name(0), Some("ВидРаботы"));
    assert_eq!(
        row_type.get_attribute_type("ВидРаботы"),
        Some(&"ПланВидовХарактеристикСсылка.ВидыРабот".to_string())
    );
}

#[test]
fn test_tabular_row_very_long_names() {
    // Проверяем работу с длинными именами (>100 символов)
    let long_name = "А".repeat(150);
    let long_section_name = "Б".repeat(120);

    let row_type = TabularRowType::new(long_name.clone(), long_section_name.clone(), vec![]);

    assert_eq!(row_type.parent_type, long_name);
    assert_eq!(row_type.tabular_section_name, long_section_name);
    // Проверяем количество символов (не байтов!)
    assert_eq!(row_type.parent_type.chars().count(), 150);
    assert_eq!(row_type.tabular_section_name.chars().count(), 120);
}

#[test]
fn test_tabular_row_cyrillic_names() {
    // Полностью кириллические имена
    let attrs = vec![
        RawAttributeData {
            name: "НаименованиеТовара".to_string(),
            attr_type: "Строка".to_string(),
        },
        RawAttributeData {
            name: "Количество".to_string(),
            attr_type: "Число".to_string(),
        },
        RawAttributeData {
            name: "Цена".to_string(),
            attr_type: "Число".to_string(),
        },
    ];

    let row_type = TabularRowType::new(
        "Документы.ПоступлениеТоваровИУслуг".to_string(),
        "Товары".to_string(),
        attrs,
    );

    assert_eq!(row_type.attributes.len(), 3);
    assert_eq!(row_type.get_attribute_name(0), Some("НаименованиеТовара"));
    assert_eq!(row_type.get_attribute_name(1), Some("Количество"));
    assert_eq!(row_type.get_attribute_name(2), Some("Цена"));
}

#[test]
fn test_tabular_row_mixed_latin_cyrillic() {
    // Смешанные латиница и кириллица
    let attrs = vec![RawAttributeData {
        name: "ProductНаименование".to_string(),
        attr_type: "StringСтрока".to_string(),
    }];

    let row_type = TabularRowType::new(
        "Documents.ЗаказНаряды".to_string(),
        "Items".to_string(),
        attrs,
    );

    assert_eq!(row_type.parent_type, "Documents.ЗаказНаряды");
    assert_eq!(row_type.get_attribute_name(0), Some("ProductНаименование"));
}

#[test]
fn test_tabular_row_nested_generic_types() {
    // Вложенные Generic типы в атрибутах
    let attrs = vec![
        RawAttributeData {
            name: "СписокДанных".to_string(),
            attr_type: "Массив<Структура>".to_string(),
        },
        RawAttributeData {
            name: "ТаблицаЗначений".to_string(),
            attr_type: "ТаблицаЗначений".to_string(),
        },
    ];

    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "ДополнительныеДанные".to_string(),
        attrs,
    );

    assert_eq!(row_type.attributes.len(), 2);
    assert_eq!(
        row_type.get_attribute_type("СписокДанных"),
        Some(&"Массив<Структура>".to_string())
    );
    assert_eq!(
        row_type.get_attribute_type("ТаблицаЗначений"),
        Some(&"ТаблицаЗначений".to_string())
    );
}

#[test]
fn test_tabular_row_full_name_generation() {
    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "Работы".to_string(),
        vec![],
    );

    let full_name = row_type.get_full_name();

    // full_name должен быть в формате "СтрокаРаботы" или "Документы.ЗаказНаряды.Работы.Строка"
    // В зависимости от реализации
    assert!(!full_name.is_empty());
    assert!(full_name.contains("Работы") || full_name.contains("Строка"));
}

#[test]
fn test_tabular_row_out_of_bounds_access() {
    let attrs = vec![RawAttributeData {
        name: "Attr1".to_string(),
        attr_type: "String".to_string(),
    }];

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    // Доступ за границами массива (get_attribute_name по индексу)
    assert_eq!(row_type.get_attribute_name(0), Some("Attr1"));
    assert_eq!(row_type.get_attribute_name(1), None);
    assert_eq!(row_type.get_attribute_name(100), None);

    // get_attribute_type работает по имени, а не по индексу
    assert_eq!(
        row_type.get_attribute_type("Attr1"),
        Some(&"String".to_string())
    );
    assert_eq!(row_type.get_attribute_type("NonExistent"), None);
    assert_eq!(row_type.get_attribute_type("Attr100"), None);
}

#[test]
fn test_tabular_row_many_attributes() {
    // Большое количество атрибутов (50+)
    let mut attrs = Vec::new();
    for i in 0..60 {
        attrs.push(RawAttributeData {
            name: format!("Attr{}", i),
            attr_type: format!("Type{}", i),
        });
    }

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    assert_eq!(row_type.attributes.len(), 60);
    assert_eq!(row_type.get_attribute_name(0), Some("Attr0"));
    assert_eq!(row_type.get_attribute_name(59), Some("Attr59"));
    assert_eq!(row_type.get_attribute_name(60), None);
}

#[test]
fn test_tabular_row_clone() {
    let attrs = vec![RawAttributeData {
        name: "Test".to_string(),
        attr_type: "String".to_string(),
    }];

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    let cloned = row_type.clone();

    assert_eq!(row_type.parent_type, cloned.parent_type);
    assert_eq!(
        row_type.tabular_section_name,
        cloned.tabular_section_name
    );
    assert_eq!(row_type.attributes.len(), cloned.attributes.len());
}

#[test]
fn test_tabular_row_eq() {
    let attrs1 = vec![RawAttributeData {
        name: "Test".to_string(),
        attr_type: "String".to_string(),
    }];

    let attrs2 = vec![RawAttributeData {
        name: "Test".to_string(),
        attr_type: "String".to_string(),
    }];

    let row1 = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs1);

    let row2 = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs2);

    assert_eq!(row1, row2);
}

#[test]
fn test_tabular_row_ne() {
    let attrs = vec![RawAttributeData {
        name: "Test".to_string(),
        attr_type: "String".to_string(),
    }];

    let row1 = TabularRowType::new("Doc.Test1".to_string(), "Items".to_string(), attrs.clone());

    let row2 = TabularRowType::new("Doc.Test2".to_string(), "Items".to_string(), attrs);

    assert_ne!(row1, row2);
}

#[test]
fn test_tabular_row_special_characters() {
    // Специальные символы в именах
    let attrs = vec![RawAttributeData {
        name: "Attr_1-2@3".to_string(),
        attr_type: "Type#1$2".to_string(),
    }];

    let row_type = TabularRowType::new(
        "Doc.Test!@#".to_string(),
        "Items_1-2".to_string(),
        attrs,
    );

    assert_eq!(row_type.parent_type, "Doc.Test!@#");
    assert_eq!(row_type.tabular_section_name, "Items_1-2");
    assert_eq!(row_type.get_attribute_name(0), Some("Attr_1-2@3"));
}

#[test]
fn test_tabular_row_empty_strings() {
    // Пустые строки в именах (edge case)
    let attrs = vec![RawAttributeData {
        name: "".to_string(),
        attr_type: "".to_string(),
    }];

    let row_type = TabularRowType::new("".to_string(), "".to_string(), attrs);

    assert_eq!(row_type.parent_type, "");
    assert_eq!(row_type.tabular_section_name, "");
    assert_eq!(row_type.get_attribute_name(0), Some(""));
    assert_eq!(row_type.get_attribute_type(""), Some(&"".to_string()));
}

#[test]
fn test_tabular_row_whitespace_names() {
    // Пробелы и табуляция в именах
    let attrs = vec![RawAttributeData {
        name: "  Attr  ".to_string(),
        attr_type: "\tType\t".to_string(),
    }];

    let row_type = TabularRowType::new(
        "  Doc.Test  ".to_string(),
        "  Items  ".to_string(),
        attrs,
    );

    // Проверяем, что пробелы сохраняются
    assert_eq!(row_type.parent_type, "  Doc.Test  ");
    assert_eq!(row_type.tabular_section_name, "  Items  ");
    assert_eq!(row_type.get_attribute_name(0), Some("  Attr  "));
}
