//! Демонстрация новой системы типов платформы с syntax helper

use bsl_gradual_types::adapters::platform_types_v2::PlatformTypesResolverV2;

fn main() -> anyhow::Result<()> {
    println!("=== Демонстрация PlatformTypesResolverV2 ===\n");

    // Создаём резолвер
    let mut resolver = PlatformTypesResolverV2::new();

    // Загружаем syntax helper если доступен
    let _ = resolver.load_from_file("examples/syntax_helper/syntax_database.json");

    // Показываем статистику
    println!("📊 Статистика загруженных данных:");
    let stats = resolver.get_stats();
    for (key, value) in &stats {
        println!("  {}: {}", key, value);
    }

    // Проверяем статус загрузки
    if resolver.is_loaded() {
        println!("\n✅ Данные syntax helper загружены успешно!");
    } else {
        println!("\n⚠️ Используются только hardcoded типы (syntax helper не загружен)");
    }

    // Получаем глобальные функции
    let global_functions = resolver.get_global_functions();
    println!(
        "\n🔧 Примеры глобальных функций (всего {}):",
        global_functions.len()
    );
    for (name, _) in global_functions.iter().take(15) {
        println!("  - {}", name);
    }

    // Получаем все типы платформы
    let platform_globals = resolver.get_platform_globals();
    println!(
        "\n📦 Все глобальные объекты и функции (всего {}):",
        platform_globals.len()
    );
    for (name, _) in platform_globals.iter().take(20) {
        println!("  - {}", name);
    }

    // Получаем примитивные типы
    let primitive_types = resolver.get_primitive_types();
    println!("\n🧱 Примитивные типы (всего {}):", primitive_types.len());
    for (name, _) in primitive_types.iter().take(10) {
        println!("  - {}", name);
    }

    // Получаем ключевые слова
    let keywords = resolver.get_keywords();
    println!("\n🔤 Ключевые слова (всего {}):", keywords.len());
    for keyword in keywords.iter().take(15) {
        print!("{}, ", keyword);
    }
    println!();

    // Получаем операторы
    let operators = resolver.get_operators();
    println!("\n➕ Операторы (всего {}):", operators.len());
    for op in operators.iter().take(10) {
        print!("{}, ", op);
    }
    println!();

    // Тестируем поиск конкретных функций
    println!("\n🔍 Тестирование поиска конкретных функций:");

    let test_functions = vec![
        "Сообщить",
        "Message",
        "Тип",
        "Type",
        "Строка",
        "String",
        "ЗначениеЗаполнено",
        "ValueIsFilled",
    ];

    for func_name in test_functions {
        if platform_globals.contains_key(func_name) {
            println!("  ✅ Найдена функция: {}", func_name);
        } else {
            println!("  ❌ Не найдена функция: {}", func_name);
        }
    }

    println!("\n✅ Демонстрация завершена!");

    Ok(())
}
