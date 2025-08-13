//! Простой пример для тестирования автодополнения

use bsl_gradual_types::core::platform_resolver::PlatformTypeResolver;

fn main() {
    println!("=== Тестирование автодополнения ===\n");
    
    let resolver = PlatformTypeResolver::new();
    
    // Тест 1: Автодополнение после "Массив."
    println!("1. Автодополнение для 'Массив.':");
    let completions = resolver.get_completions("Массив.");
    if completions.is_empty() {
        println!("   Нет автодополнений");
    } else {
        for item in completions.iter().take(10) {
            println!("   - {}: {}", item.label, item.detail.as_deref().unwrap_or(""));
        }
    }
    
    // Тест 2: Автодополнение для частичного ввода
    println!("\n2. Автодополнение для 'Сооб':");
    let completions = resolver.get_completions("Сооб");
    if completions.is_empty() {
        println!("   Нет автодополнений");
    } else {
        for item in completions.iter().take(5) {
            println!("   - {}: {}", item.label, item.detail.as_deref().unwrap_or(""));
        }
    }
    
    // Тест 3: Автодополнение после "Справочники."
    println!("\n3. Автодополнение для 'Справочники.':");
    let completions = resolver.get_completions("Справочники.");
    if completions.is_empty() {
        println!("   Нет автодополнений");
    } else {
        for item in completions.iter().take(5) {
            println!("   - {}: {}", item.label, item.detail.as_deref().unwrap_or(""));
        }
    }
    
    // Тест 4: Автодополнение для "Строка."
    println!("\n4. Автодополнение для 'Строка.':");
    let completions = resolver.get_completions("Строка.");
    if completions.is_empty() {
        println!("   Нет автодополнений");
    } else {
        for item in completions.iter().take(10) {
            println!("   - {}: {}", item.label, item.detail.as_deref().unwrap_or(""));
        }
    }
    
    // Тест 5: Все глобальные функции
    println!("\n5. Глобальные функции (показываем первые 10):");
    let completions = resolver.get_completions("");
    let global_functions: Vec<_> = completions.iter()
        .filter(|c| c.detail.as_deref() == Some("Глобальная функция"))
        .take(10)
        .collect();
    
    if global_functions.is_empty() {
        println!("   Нет глобальных функций");
    } else {
        for item in global_functions {
            println!("   - {}", item.label);
        }
    }
    
    println!("\n=== Тест завершён ===");
}