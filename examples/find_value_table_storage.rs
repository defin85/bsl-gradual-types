//! Поиск всех упоминаний ТаблицыЗначений в базе данных

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;

fn main() {
    println!("=== Поиск хранения ТаблицыЗначений ===\n");
    
    // Загружаем базу данных из JSON
    let database = match SyntaxHelperParser::load_from_file("examples/syntax_helper/syntax_database.json") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Ошибка загрузки базы данных: {}", e);
            return;
        }
    };
    
    // 1. Поиск в глобальных объектах
    println!("1. ГЛОБАЛЬНЫЕ ОБЪЕКТЫ с 'ValueTable' или 'ТаблицаЗначений':");
    for (key, info) in &database.global_objects {
        if key.contains("ValueTable") || key.contains("ТаблицаЗначений") {
            println!("  - {} (методов: {}, свойств: {})", 
                key, info.methods.len(), info.properties.len());
        }
    }
    
    // 2. Поиск в методах объектов
    println!("\n2. МЕТОДЫ ОБЪЕКТОВ с 'ValueTable' или 'ТаблицаЗначений':");
    let mut count = 0;
    for key in database.object_methods.keys() {
        if key.contains("ValueTable") || key.contains("ТаблицаЗначений") {
            println!("  - {}", key);
            count += 1;
            if count >= 10 {
                println!("  ... и ещё {}", 
                    database.object_methods.keys()
                        .filter(|k| k.contains("ValueTable") || k.contains("ТаблицаЗначений"))
                        .count() - 10);
                break;
            }
        }
    }
    
    // 3. Поиск в свойствах объектов
    println!("\n3. СВОЙСТВА ОБЪЕКТОВ с 'ValueTable' или 'ТаблицаЗначений':");
    count = 0;
    for key in database.object_properties.keys() {
        if key.contains("ValueTable") || key.contains("ТаблицаЗначений") {
            println!("  - {}", key);
            count += 1;
            if count >= 10 {
                println!("  ... и ещё {}", 
                    database.object_properties.keys()
                        .filter(|k| k.contains("ValueTable") || k.contains("ТаблицаЗначений"))
                        .count() - 10);
                break;
            }
        }
    }
    
    // 4. Проверим конкретно catalog236
    println!("\n4. ОБЪЕКТЫ В CATALOG236:");
    for (key, _) in &database.global_objects {
        if key.starts_with("catalog236") {
            println!("  - {}", key);
        }
    }
    
    // 5. Давайте проверим методы с префиксом catalog236
    println!("\n5. МЕТОДЫ С ПРЕФИКСОМ catalog236:");
    let catalog236_methods: Vec<_> = database.object_methods.keys()
        .filter(|k| k.starts_with("catalog236"))
        .collect();
    
    println!("Найдено {} методов", catalog236_methods.len());
    for (i, key) in catalog236_methods.iter().take(20).enumerate() {
        if let Some(method) = database.object_methods.get(*key) {
            println!("  {}. {} -> {}", i + 1, key, method.name);
        }
    }
    
    // 6. Проверим, может быть методы хранятся без префикса catalog
    println!("\n6. ПРЯМОЙ ПОИСК ValueTable.Add:");
    if let Some(method) = database.object_methods.get("ValueTable.Add") {
        println!("  Найден: {}", method.name);
    } else {
        println!("  Не найден под ключом 'ValueTable.Add'");
    }
    
    if let Some(method) = database.object_methods.get("catalog236/ValueTable.Add") {
        println!("  Найден: {}", method.name);
    } else {
        println!("  Не найден под ключом 'catalog236/ValueTable.Add'");
    }
    
    // 7. Давайте посмотрим ключи методов, содержащие "Add"
    println!("\n7. МЕТОДЫ С 'Add' или 'Добавить':");
    let add_methods: Vec<_> = database.object_methods.iter()
        .filter(|(k, v)| k.contains("Add") || v.name == "Добавить" || v.name == "Add")
        .take(10)
        .collect();
    
    for (key, method) in add_methods {
        println!("  {} -> {}", key, method.name);
    }
}