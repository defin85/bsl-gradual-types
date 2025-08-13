//! Проверка методов ТаблицыЗначений

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;

fn main() {
    println!("=== Проверка методов ТаблицыЗначений ===\n");
    
    // Загружаем базу данных из JSON
    let database = match SyntaxHelperParser::load_from_file("examples/syntax_helper/syntax_database.json") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Ошибка загрузки базы данных: {}", e);
            return;
        }
    };
    
    // Ищем ТаблицуЗначений в глобальных объектах
    let value_table_names = ["ТаблицаЗначений", "ValueTable"];
    
    for name in &value_table_names {
        if let Some(object_info) = database.global_objects.get(*name) {
            println!("Найден объект: {}", name);
            println!("Описание: {}", object_info.description.as_deref().unwrap_or(""));
            println!("Количество методов: {}", object_info.methods.len());
            println!("Количество свойств: {}", object_info.properties.len());
            
            println!("\nМетоды:");
            for (i, method_name) in object_info.methods.iter().enumerate() {
                let key = format!("{}.{}", name, method_name);
                if let Some(method_info) = database.object_methods.get(&key) {
                    let params = method_info.parameters.iter()
                        .map(|p| format!("{}: {}", 
                            p.name, 
                            p.type_ref.as_ref().map(|t| t.name_ru.as_str()).unwrap_or("?")))
                        .collect::<Vec<_>>()
                        .join(", ");
                    
                    let return_type = method_info.return_type.as_ref()
                        .map(|t| t.name_ru.as_str())
                        .unwrap_or("void");
                    
                    println!("  {}. {}({}) -> {}", 
                        i + 1, 
                        method_info.name,
                        params,
                        return_type);
                } else {
                    println!("  {}. {} (метаданные не найдены)", i + 1, method_name);
                }
            }
            
            if !object_info.properties.is_empty() {
                println!("\nСвойства:");
                for (i, property_name) in object_info.properties.iter().enumerate() {
                    let key = format!("{}.{}", name, property_name);
                    if let Some(property_info) = database.object_properties.get(&key) {
                        let prop_type = property_info.property_type.as_ref()
                            .map(|t| t.name_ru.as_str())
                            .unwrap_or("?");
                        let readonly = if property_info.is_readonly { " (только чтение)" } else { "" };
                        
                        println!("  {}. {}: {}{}", 
                            i + 1, 
                            property_info.name,
                            prop_type,
                            readonly);
                    } else {
                        println!("  {}. {} (метаданные не найдены)", i + 1, property_name);
                    }
                }
            }
            
            println!("\n{}", "=".repeat(50));
        }
    }
    
    // Также проверим, есть ли методы с префиксом ТаблицаЗначений
    let value_table_methods: Vec<_> = database.object_methods.keys()
        .filter(|k| k.starts_with("ТаблицаЗначений.") || k.starts_with("ValueTable."))
        .collect();
    
    if !value_table_methods.is_empty() {
        println!("\nВсего найдено методов с префиксом ТаблицаЗначений/ValueTable: {}", value_table_methods.len());
    }
}