//! Тестирование discovery-based парсера

use bsl_gradual_types::adapters::syntax_helper_parser_v2::SyntaxHelperParserV2;
use std::time::Instant;

fn main() {
    println!("=== Тестирование Discovery-Based парсера ===\n");
    
    let start = Instant::now();
    
    // Создаём парсер с путём к распакованному архиву
    let mut parser = SyntaxHelperParserV2::new("examples/syntax_helper/rebuilt.shcntx_ru");
    
    // Запускаем парсинг
    match parser.parse() {
        Ok(()) => {
            let database = parser.database();
            
            println!("\n=== РЕЗУЛЬТАТЫ ПАРСИНГА ===");
            println!("Время выполнения: {:?}", start.elapsed());
            println!("\nСтатистика:");
            println!("  Глобальных объектов: {}", database.global_objects.len());
            println!("  Методов объектов: {}", database.object_methods.len());
            println!("  Свойств объектов: {}", database.object_properties.len());
            
            // Проверяем, нашли ли мы ValueTable
            println!("\n=== Проверка ValueTable ===");
            
            // Ищем объект ValueTable
            let value_table_keys: Vec<_> = database.global_objects.keys()
                .filter(|k| k.contains("ValueTable"))
                .collect();
            
            if value_table_keys.is_empty() {
                println!("ValueTable не найден!");
            } else {
                for key in &value_table_keys {
                    if let Some(obj) = database.global_objects.get(*key) {
                        println!("\nОбъект: {}", key);
                        println!("  Методов: {}", obj.methods.len());
                        println!("  Свойств: {}", obj.properties.len());
                        
                        // Проверяем методы
                        if obj.methods.len() > 0 {
                            println!("  Первые 5 методов:");
                            for method in obj.methods.iter().take(5) {
                                println!("    - {}", method);
                                
                                // Проверяем, есть ли детали метода
                                let method_key = format!("{}.{}", key, method);
                                if database.object_methods.contains_key(&method_key) {
                                    println!("      ✓ Детали метода найдены");
                                } else {
                                    // Пробуем альтернативный ключ
                                    let alt_key = format!("{}.{}", key, method.split(' ').next().unwrap_or(method));
                                    if database.object_methods.contains_key(&alt_key) {
                                        println!("      ✓ Детали найдены под ключом: {}", alt_key);
                                    } else {
                                        println!("      ✗ Детали метода НЕ найдены");
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Проверяем catalog236
            println!("\n=== Проверка catalog236 ===");
            let catalog236_objects: Vec<_> = database.global_objects.keys()
                .filter(|k| k.contains("catalog236"))
                .collect();
            
            println!("Найдено объектов с catalog236: {}", catalog236_objects.len());
            for key in catalog236_objects.iter().take(5) {
                println!("  - {}", key);
            }
            
            // Проверяем методы с catalog236
            let catalog236_methods: Vec<_> = database.object_methods.keys()
                .filter(|k| k.contains("catalog236"))
                .collect();
            
            println!("\nНайдено методов с catalog236: {}", catalog236_methods.len());
            for key in catalog236_methods.iter().take(5) {
                if let Some(method) = database.object_methods.get(*key) {
                    println!("  - {} -> {}", key, method.name);
                }
            }
        }
        Err(e) => {
            eprintln!("Ошибка при парсинге: {}", e);
        }
    }
}