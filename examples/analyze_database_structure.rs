//! Анализ структуры базы данных синтакс-помощника

use bsl_gradual_types::adapters::syntax_helper_parser::SyntaxHelperParser;
use std::collections::HashSet;

fn main() {
    println!("=== Анализ структуры базы данных ===\n");
    
    // Загружаем базу данных из JSON
    let database = match SyntaxHelperParser::load_from_file("examples/syntax_helper/syntax_database.json") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Ошибка загрузки базы данных: {}", e);
            return;
        }
    };
    
    println!("СТАТИСТИКА:");
    println!("  Глобальных функций: {}", database.global_functions.len());
    println!("  Глобальных объектов: {}", database.global_objects.len());
    println!("  Методов объектов: {}", database.object_methods.len());
    println!("  Свойств объектов: {}", database.object_properties.len());
    println!("  Системных перечислений: {}", database.system_enums.len());
    println!("  Ключевых слов: {}", database.keywords.len());
    println!("  Операторов: {}", database.operators.len());
    
    // Анализируем префиксы в ключах методов
    println!("\nПРЕФИКСЫ В КЛЮЧАХ МЕТОДОВ:");
    let mut prefixes = HashSet::new();
    for key in database.object_methods.keys() {
        if let Some(dot_pos) = key.find('.') {
            let prefix = &key[..dot_pos];
            prefixes.insert(prefix.to_string());
        }
    }
    
    let mut sorted_prefixes: Vec<_> = prefixes.into_iter().collect();
    sorted_prefixes.sort();
    
    println!("Найдено {} уникальных префиксов", sorted_prefixes.len());
    
    // Показываем префиксы с catalog
    println!("\nПрефиксы с 'catalog':");
    for prefix in &sorted_prefixes {
        if prefix.contains("catalog") {
            // Считаем количество методов с этим префиксом
            let count = database.object_methods.keys()
                .filter(|k| k.starts_with(&format!("{}.", prefix)))
                .count();
            println!("  {} ({} методов)", prefix, count);
        }
    }
    
    // Проверяем ValueTable в глобальных объектах
    println!("\nОБЪЕКТЫ С ValueTable:");
    for (key, obj) in &database.global_objects {
        if key.contains("ValueTable") {
            println!("  {} -> {} методов, {} свойств", 
                key, obj.methods.len(), obj.properties.len());
            
            // Проверяем, есть ли методы в базе
            if !obj.methods.is_empty() {
                println!("    Первые 5 методов:");
                for method_name in obj.methods.iter().take(5) {
                    // Пробуем найти метод с разными вариантами ключей
                    let possible_keys = vec![
                        format!("{}.{}", key, method_name),
                        format!("catalog236.{}", method_name),
                        format!("ValueTable.{}", method_name),
                    ];
                    
                    let mut found = false;
                    for possible_key in &possible_keys {
                        if database.object_methods.contains_key(possible_key) {
                            println!("      {} -> найден под ключом {}", method_name, possible_key);
                            found = true;
                            break;
                        }
                    }
                    
                    if !found {
                        println!("      {} -> НЕ НАЙДЕН в базе методов", method_name);
                    }
                }
            }
        }
    }
}