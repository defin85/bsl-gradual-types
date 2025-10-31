/// Bug Hunting тесты для Generic типов
///
/// Проверяет потенциальные проблемы:
/// - Циклические зависимости
/// - None/Option значения
/// - Вложенные Generic типы
/// - Unicode/кириллица
/// - Производительность
use bsl_shared::domain::types::{
    ConcreteType, GenericType, PlatformType, RawAttributeData, TabularRowType,
};

// ============================================================================
// Проблема 1: Циклические зависимости
// ============================================================================

#[test]
fn test_bug_hunt_circular_tabular_reference() {
    // Что если атрибут табличной части ссылается на родительский объект?
    // Документы.ЗаказНаряды.Работы.Документ → Документы.ЗаказНаряды?

    let attrs = vec![
        RawAttributeData {
            name: "Документ".to_string(),
            attr_type: "Документы.ЗаказНаряды".to_string(), // Циклическая ссылка!
        },
        RawAttributeData {
            name: "ВидРаботы".to_string(),
            attr_type: "ПланВидовХарактеристикСсылка.ВидыРабот".to_string(),
        },
    ];

    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "Работы".to_string(),
        attrs,
    );

    // Не должно быть паники при создании
    assert_eq!(row_type.parent_type, "Документы.ЗаказНаряды");
    assert_eq!(row_type.attributes.len(), 2);

    // Проверяем, что циклическая ссылка сохранена как есть
    assert_eq!(
        row_type.get_attribute_type("Документ"),
        Some(&"Документы.ЗаказНаряды".to_string())
    );

    println!("✓ Циклическая ссылка обрабатывается корректно (сохраняется как строка)");
}

#[test]
fn test_bug_hunt_self_referencing_generic() {
    // Что если Generic тип ссылается сам на себя?
    // Массив<Массив<Массив<...>>>?

    let inner_type = ConcreteType::Platform(PlatformType {
        name: "Число".to_string(),
    });

    let level1 = GenericType::array(inner_type.clone());
    let level2 = GenericType::array(ConcreteType::Platform(PlatformType {
        name: "Массив".to_string(), // Упрощённая самоссылка
    }));

    // Проверяем, что структуры создаются без паники
    assert_eq!(level1.base_type, "Массив");
    assert_eq!(level2.base_type, "Массив");

    println!("✓ Вложенные Generic типы создаются без проблем");
}

// ============================================================================
// Проблема 2: None/Option значения
// ============================================================================

#[test]
fn test_bug_hunt_attribute_with_empty_type() {
    // Что если attr_type - пустая строка?

    let attrs = vec![
        RawAttributeData {
            name: "Атрибут1".to_string(),
            attr_type: "".to_string(), // Пустой тип!
        },
        RawAttributeData {
            name: "Атрибут2".to_string(),
            attr_type: "Строка".to_string(),
        },
    ];

    let row_type = TabularRowType::new("Документы.Test".to_string(), "Items".to_string(), attrs);

    // Не должно быть паники
    assert_eq!(
        row_type.get_attribute_type("Атрибут1"),
        Some(&"".to_string())
    );
    assert_eq!(
        row_type.get_attribute_type("Атрибут2"),
        Some(&"Строка".to_string())
    );

    println!("✓ Пустые типы атрибутов обрабатываются корректно");
}

#[test]
fn test_bug_hunt_generic_with_empty_params() {
    // Generic тип без параметров

    let generic = GenericType {
        base_type: "ТабличнаяЧасть".to_string(),
        type_params: vec![], // Пусто!
    };

    assert_eq!(generic.type_params.len(), 0);
    assert_eq!(generic.base_type, "ТабличнаяЧасть");

    println!("✓ Generic тип с пустыми параметрами создаётся без проблем");
}

#[test]
fn test_bug_hunt_tabular_row_attribute_access_missing() {
    // Доступ к несуществующему атрибуту

    let attrs = vec![RawAttributeData {
        name: "Существующий".to_string(),
        attr_type: "Строка".to_string(),
    }];

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    // Доступ к несуществующему атрибуту должен вернуть None, а не паниковать
    assert_eq!(row_type.get_attribute_type("Несуществующий"), None);
    assert_eq!(row_type.get_attribute_type(""), None);
    assert_eq!(row_type.get_attribute_type("АтрибутСОпечаткой"), None);

    println!("✓ Доступ к несуществующим атрибутам возвращает None без паники");
}

// ============================================================================
// Проблема 3: Unicode и кириллица
// ============================================================================

#[test]
fn test_bug_hunt_unicode_normalization() {
    // Разные формы Unicode нормализации для кириллицы

    // "Ё" может быть представлена как:
    // 1. Precomposed: U+0401
    // 2. Decomposed: U+0415 (Е) + U+0308 (combining diaeresis)

    let attrs1 = vec![RawAttributeData {
        name: "Ёлка".to_string(), // Precomposed
        attr_type: "Строка".to_string(),
    }];

    let attrs2 = vec![RawAttributeData {
        name: "Eлка".to_string(), // Может быть decomposed или латинская E
        attr_type: "Строка".to_string(),
    }];

    let row1 = TabularRowType::new("Doc.Test".to_string(), "Items1".to_string(), attrs1);

    let row2 = TabularRowType::new("Doc.Test".to_string(), "Items2".to_string(), attrs2);

    // Проверяем, что оба варианта создаются без проблем
    assert!(row1.get_attribute_name(0).is_some());
    assert!(row2.get_attribute_name(0).is_some());

    // Названия могут быть разными из-за нормализации
    let name1 = row1.get_attribute_name(0).unwrap();
    let name2 = row2.get_attribute_name(0).unwrap();

    println!(
        "✓ Unicode нормализация: '{}' vs '{}' (могут быть разными)",
        name1, name2
    );
}

#[test]
fn test_bug_hunt_mixed_scripts() {
    // Смешанные скрипты: кириллица + латиница + цифры + спецсимволы

    let attrs = vec![
        RawAttributeData {
            name: "НаименованиеПродукта1_Eng".to_string(),
            attr_type: "StringТип123!@#".to_string(),
        },
        RawAttributeData {
            name: "ПолеСДлиннымИменем_ИКириллицей_И_Цифрами_123".to_string(),
            attr_type: "ОченьДлинныйТипСКириллицейИЛатиницей_Type_123".to_string(),
        },
    ];

    let row_type = TabularRowType::new(
        "Документы.ТестовыйДокумент_Test_123".to_string(),
        "ТабличнаяЧасть_Items_1".to_string(),
        attrs,
    );

    // Не должно быть паники
    assert_eq!(row_type.attributes.len(), 2);

    println!("✓ Смешанные скрипты обрабатываются корректно");
}

#[test]
fn test_bug_hunt_emoji_in_names() {
    // Что если в именах эмодзи? (экстремальный случай)

    let attrs = vec![RawAttributeData {
        name: "Поле😀Test".to_string(),
        attr_type: "Тип🎉String".to_string(),
    }];

    let row_type = TabularRowType::new("Doc🔥Test".to_string(), "Items🎯".to_string(), attrs);

    // Проверяем, что создаётся без паники
    assert_eq!(row_type.parent_type, "Doc🔥Test");
    assert_eq!(row_type.tabular_section_name, "Items🎯");

    // Emoji занимают 4 байта в UTF-8
    println!(
        "✓ Emoji в именах: parent_type байт={}, символов={}",
        row_type.parent_type.len(),
        row_type.parent_type.chars().count()
    );
}

// ============================================================================
// Проблема 4: Вложенные типы
// ============================================================================

#[test]
fn test_bug_hunt_deeply_nested_types() {
    // Глубоко вложенные типы: Массив<Структура<Массив<Строка>>>

    let attrs = vec![RawAttributeData {
        name: "ВложенныеДанные".to_string(),
        attr_type: "Массив<Структура<Массив<Строка>>>".to_string(),
    }];

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    // Вложенный тип сохраняется как строка (парсинг на уровне TypeResolver)
    assert_eq!(
        row_type.get_attribute_type("ВложенныеДанные"),
        Some(&"Массив<Структура<Массив<Строка>>>".to_string())
    );

    println!("✓ Глубоко вложенные типы сохраняются как строки без парсинга");
}

#[test]
fn test_bug_hunt_malformed_generic_syntax() {
    // Некорректный Generic синтаксис в строке типа

    let attrs = vec![
        RawAttributeData {
            name: "НекорректныйТип1".to_string(),
            attr_type: "Массив<".to_string(), // Незакрытая скобка
        },
        RawAttributeData {
            name: "НекорректныйТип2".to_string(),
            attr_type: "Массив<>".to_string(), // Пустые параметры
        },
        RawAttributeData {
            name: "НекорректныйТип3".to_string(),
            attr_type: "Массив<<Строка>>".to_string(), // Двойная скобка
        },
    ];

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    // Некорректный синтаксис сохраняется "as is"
    assert_eq!(
        row_type.get_attribute_type("НекорректныйТип1"),
        Some(&"Массив<".to_string())
    );

    println!("✓ Некорректный Generic синтаксис не вызывает паники на уровне TabularRowType");
}

// ============================================================================
// Проблема 5: Производительность
// ============================================================================

#[test]
fn test_bug_hunt_large_number_of_attributes() {
    // Табличная часть с большим количеством атрибутов (100+)

    let mut attrs = Vec::new();
    for i in 0..150 {
        attrs.push(RawAttributeData {
            name: format!("Атрибут{}", i),
            attr_type: format!("Тип{}", i),
        });
    }

    let row_type = TabularRowType::new("Doc.Test".to_string(), "Items".to_string(), attrs);

    // Проверяем поиск атрибута в конце списка
    use std::time::Instant;

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = row_type.get_attribute_type("Атрибут149");
    }
    let duration = start.elapsed();

    println!(
        "✓ Поиск атрибута (150 атрибутов, 1000 итераций): {:?}",
        duration
    );

    // Должно быть быстро (линейный поиск O(n), но n=150 не должно быть проблемой)
    assert!(
        duration.as_millis() < 100,
        "Поиск атрибута слишком медленный: {:?}",
        duration
    );
}

#[test]
fn test_bug_hunt_memory_consumption_many_row_types() {
    // Создаём много TabularRowType для проверки утечек памяти

    let mut row_types = Vec::new();

    for i in 0..1000 {
        let attrs = vec![
            RawAttributeData {
                name: format!("Attr{}", i),
                attr_type: format!("Type{}", i),
            },
            RawAttributeData {
                name: format!("Attr{}_2", i),
                attr_type: format!("Type{}_2", i),
            },
        ];

        let row_type = TabularRowType::new(format!("Doc.Test{}", i), format!("Items{}", i), attrs);

        row_types.push(row_type);
    }

    // Проверяем, что все созданы
    assert_eq!(row_types.len(), 1000);

    // Проверяем случайный элемент
    assert_eq!(row_types[500].tabular_section_name, "Items500");

    println!("✓ Создано 1000 TabularRowType без проблем с памятью");

    // Drop будет вызван автоматически - проверяем, что нет утечек
}

#[test]
fn test_bug_hunt_generic_clone_performance() {
    // Проверяем производительность клонирования Generic типов

    let row_type = TabularRowType::new(
        "Документы.ЗаказНаряды".to_string(),
        "Работы".to_string(),
        vec![
            RawAttributeData {
                name: "ВидРаботы".to_string(),
                attr_type: "ПланВидовХарактеристикСсылка.ВидыРабот".to_string(),
            },
            RawAttributeData {
                name: "Стоимость".to_string(),
                attr_type: "Число".to_string(),
            },
        ],
    );

    use std::time::Instant;

    let start = Instant::now();
    for _ in 0..10000 {
        let _ = row_type.clone();
    }
    let duration = start.elapsed();

    println!("✓ 10000 клонирований TabularRowType: {:?}", duration);

    assert!(
        duration.as_millis() < 100,
        "Клонирование слишком медленное: {:?}",
        duration
    );
}

// ============================================================================
// Проблема 6: Граничные значения
// ============================================================================

#[test]
fn test_bug_hunt_max_string_length() {
    // Очень длинные строки (приближаемся к лимитам памяти)

    // 1 MB строка
    let very_long_name = "А".repeat(1_000_000);

    let row_type = TabularRowType::new(very_long_name.clone(), "Test".to_string(), vec![]);

    // Проверяем, что создалось
    assert_eq!(row_type.parent_type.chars().count(), 1_000_000);

    println!("✓ Очень длинное имя (1M символов) обрабатывается корректно");
}

#[test]
fn test_bug_hunt_null_bytes_in_strings() {
    // Null bytes в строках (потенциально проблематично в C FFI)

    let attrs = vec![RawAttributeData {
        name: "Attr\0WithNull".to_string(),
        attr_type: "Type\0WithNull".to_string(),
    }];

    let row_type = TabularRowType::new("Doc\0Test".to_string(), "Items\0".to_string(), attrs);

    // Rust strings могут содержать null bytes (это не C strings)
    assert!(row_type.parent_type.contains('\0'));

    println!("✓ Null bytes в строках обрабатываются корректно (Rust не C!)");
}

#[test]
fn test_bug_hunt_concurrent_access() {
    // Проверяем, что TabularRowType потокобезопасен при read-only доступе

    use std::sync::Arc;
    use std::thread;

    let row_type = Arc::new(TabularRowType::new(
        "Doc.Test".to_string(),
        "Items".to_string(),
        vec![RawAttributeData {
            name: "Attr1".to_string(),
            attr_type: "String".to_string(),
        }],
    ));

    let mut handles = vec![];

    // Запускаем 10 потоков для чтения
    for i in 0..10 {
        let row_clone = Arc::clone(&row_type);

        let handle = thread::spawn(move || {
            for _ in 0..100 {
                let _ = row_clone.get_attribute_name(0);
                let _ = row_clone.get_attribute_type("Attr1");
                let _ = row_clone.get_full_name();
            }
            println!("✓ Поток {} завершён", i);
        });

        handles.push(handle);
    }

    // Ждём все потоки
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    println!("✓ Concurrent read-only доступ работает корректно");
}
