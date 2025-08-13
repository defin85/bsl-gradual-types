//! Тестирование парсинга catalog236 (ТаблицаЗначений)

use bsl_gradual_types::adapters::syntax_helper_parser_v2::SyntaxHelperParserV2;
use std::time::Instant;

fn main() {
    println!("=== Тестирование парсинга catalog236 ===\n");
    
    let start = Instant::now();
    
    // Создаём парсер
    let mut parser = SyntaxHelperParserV2::new("examples/syntax_helper/rebuilt.shcntx_ru");
    
    // Парсим только catalog234/catalog236
    let catalog236_path = std::path::Path::new("examples/syntax_helper/rebuilt.shcntx_ru/objects/catalog234/catalog236");
    
    if !catalog236_path.exists() {
        eprintln!("Путь не существует: {:?}", catalog236_path);
        return;
    }
    
    // Обходим catalog236
    match parser.discover_directory(catalog236_path, "catalog234/catalog236") {
        Ok(()) => {
            let database = parser.database();
            
            println!("\n=== РЕЗУЛЬТАТЫ ===");
            println!("Время: {:?}", start.elapsed());
            println!("\nНайдено:");
            println!("  Объектов: {}", database.global_objects.len());
            println!("  Методов: {}", database.object_methods.len());
            println!("  Свойств: {}", database.object_properties.len());
            
            // Проверяем ValueTable
            println!("\n=== ValueTable ===");
            for (key, obj) in database.global_objects.iter() {
                if key.contains("ValueTable") {
                    println!("\nОбъект: {}", key);
                    println!("  Методов в списке: {}", obj.methods.len());
                    println!("  Свойств в списке: {}", obj.properties.len());
                    
                    // Проверяем детали методов
                    let mut found_methods = 0;
                    for method in &obj.methods {
                        // Пробуем разные варианты ключей
                        let keys_to_try = vec![
                            format!("{}.{}", key, method),
                            format!("{}.{}", key, method.split(' ').next().unwrap_or(method)),
                            format!("{}.Add110", key),  // Пример конкретного метода
                        ];
                        
                        for test_key in &keys_to_try {
                            if database.object_methods.contains_key(test_key) {
                                found_methods += 1;
                                break;
                            }
                        }
                    }
                    println!("  Найдено деталей методов: {}", found_methods);
                }
            }
            
            // Показываем первые несколько методов
            println!("\n=== Первые 10 методов в базе ===");
            for (i, (key, method)) in database.object_methods.iter().take(10).enumerate() {
                println!("{}. {} -> {}", i + 1, key, method.name);
                if let Some(syntax) = method.syntax.first() {
                    println!("   Синтаксис: {}", syntax);
                }
            }
        }
        Err(e) => {
            eprintln!("Ошибка: {}", e);
        }
    }
}

// Метод discover_directory уже публичный, поэтому не нужно расширять