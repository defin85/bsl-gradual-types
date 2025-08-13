//! Список всех глобальных объектов

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;

fn main() {
    println!("=== Список всех глобальных объектов ===\n");
    
    // Загружаем базу данных из JSON
    let database = match SyntaxHelperParser::load_from_file("examples/syntax_helper/syntax_database.json") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Ошибка загрузки базы данных: {}", e);
            return;
        }
    };
    
    println!("Всего глобальных объектов: {}\n", database.global_objects.len());
    
    // Сортируем по имени для удобства
    let mut objects: Vec<_> = database.global_objects.iter().collect();
    objects.sort_by_key(|(name, _)| name.as_str());
    
    for (name, info) in objects {
        println!("{}", name);
        println!("  Методов: {}", info.methods.len());
        println!("  Свойств: {}", info.properties.len());
        if let Some(desc) = &info.description {
            println!("  Описание: {}", desc.chars().take(80).collect::<String>());
        }
        println!();
    }
    
    // Ищем объекты со словом "Таблиц"
    println!("\n=== Объекты со словом 'Таблиц' ===");
    for (name, info) in &database.global_objects {
        if name.to_lowercase().contains("таблиц") || name.to_lowercase().contains("table") {
            println!("{}: {} методов, {} свойств", name, info.methods.len(), info.properties.len());
        }
    }
    
    // Проверяем методы с "ТаблицаЗначений" в ключе
    println!("\n=== Методы с 'ТаблицаЗначений' в имени ===");
    let table_methods: Vec<_> = database.object_methods.keys()
        .filter(|k| k.contains("ТаблицаЗначений") || k.contains("ValueTable"))
        .collect();
    
    println!("Найдено {} методов", table_methods.len());
    for (i, key) in table_methods.iter().take(10).enumerate() {
        println!("  {}. {}", i + 1, key);
    }
    if table_methods.len() > 10 {
        println!("  ... и ещё {}", table_methods.len() - 10);
    }
}