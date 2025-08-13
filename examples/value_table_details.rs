//! Детальная информация о ТаблицеЗначений

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;

fn main() {
    println!("=== Детальная информация о ТаблицеЗначений ===\n");
    
    // Загружаем базу данных из JSON
    let database = match SyntaxHelperParser::load_from_file("examples/syntax_helper/syntax_database.json") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Ошибка загрузки базы данных: {}", e);
            return;
        }
    };
    
    let value_table_key = "catalog236/ValueTable";
    
    if let Some(object_info) = database.global_objects.get(value_table_key) {
        println!("Объект: ТаблицаЗначений (ValueTable)");
        println!("Описание: {}", object_info.description.as_deref().unwrap_or(""));
        println!("Количество методов: {}", object_info.methods.len());
        println!("Количество свойств: {}", object_info.properties.len());
        
        println!("\n=== МЕТОДЫ ({}): ===", object_info.methods.len());
        for (i, method_name) in object_info.methods.iter().enumerate() {
            let key = format!("{}.{}", value_table_key, method_name);
            if let Some(method_info) = database.object_methods.get(&key) {
                let params = method_info.parameters.iter()
                    .map(|p| format!("{}: {}{}", 
                        p.name, 
                        p.type_ref.as_ref().map(|t| t.name_ru.as_str()).unwrap_or("?"),
                        if p.is_optional { " (необяз.)" } else { "" }))
                    .collect::<Vec<_>>()
                    .join(", ");
                
                let return_type = method_info.return_type.as_ref()
                    .map(|t| t.name_ru.as_str())
                    .unwrap_or("void");
                
                println!("\n{}. {}({}) -> {}", 
                    i + 1, 
                    method_info.name,
                    params,
                    return_type);
                
                if let Some(desc) = &method_info.description {
                    println!("   {}", desc.chars().take(100).collect::<String>());
                }
            } else {
                println!("\n{}. {} (метаданные не найдены)", i + 1, method_name);
            }
        }
        
        println!("\n=== СВОЙСТВА ({}): ===", object_info.properties.len());
        for (i, property_name) in object_info.properties.iter().enumerate() {
            let key = format!("{}.{}", value_table_key, property_name);
            if let Some(property_info) = database.object_properties.get(&key) {
                let prop_type = property_info.property_type.as_ref()
                    .map(|t| t.name_ru.as_str())
                    .unwrap_or("?");
                let readonly = if property_info.is_readonly { " (только чтение)" } else { "" };
                
                println!("\n{}. {}: {}{}", 
                    i + 1, 
                    property_info.name,
                    prop_type,
                    readonly);
                
                if let Some(desc) = &property_info.description {
                    println!("   {}", desc.chars().take(100).collect::<String>());
                }
            } else {
                println!("\n{}. {} (метаданные не найдены)", i + 1, property_name);
            }
        }
    } else {
        println!("ТаблицаЗначений не найдена!");
    }
}