//! Тест интеграции ConfigurationGuidedParser с PlatformTypeResolver

use anyhow::Result;
use bsl_gradual_types::domain::resolvers::PlatformTypeResolver;

fn main() -> Result<()> {
    println!("🚀 Тестирование интеграции ConfigurationGuidedParser");

    // Тест 1: Создание resolver'а с обычным конфигом (fallback)
    println!("\n=== Тест 1: Обычный resolver ===");
    let normal_resolver = PlatformTypeResolver::new();
    println!(
        "✅ Обычный resolver создан, platform globals: {}",
        normal_resolver.get_platform_globals_count()
    );

    // Тест 2: Попытка создать guided resolver с несуществующим путем
    println!("\n=== Тест 2: Guided resolver с несуществующим путем ===");
    let fake_path = "non_existent_path";
    match PlatformTypeResolver::with_guided_config(fake_path) {
        Ok(_) => println!("❌ Неожиданно удалось создать resolver с несуществующим путем"),
        Err(e) => println!("✅ Ожидаемая ошибка: {}", e),
    }

    // Тест 3: Проверка наличия platform globals
    println!("\n=== Тест 3: Проверка platform globals ===");
    let resolver = PlatformTypeResolver::new();

    let globals_to_check = vec![
        "Справочники",
        "Документы",
        "Перечисления",
        "Catalogs",
        "Documents",
        "Enums",
    ];

    for global in &globals_to_check {
        if resolver.has_platform_global(global) {
            println!("✅ Найден platform global: {}", global);
        } else {
            println!("❌ Не найден platform global: {}", global);
        }
    }

    println!("\n🎉 Все тесты интеграции завершены!");
    Ok(())
}
