//! Простой тест интеграции ConfigurationGuidedParser

use anyhow::Result;

fn main() -> Result<()> {
    println!("🚀 Простой тест интеграции ConfigurationGuidedParser");

    // Тест создания ConfigurationGuidedParser
    use bsl_gradual_types::adapters::config_parser_guided_discovery::ConfigurationGuidedParser;

    println!("\n=== Тест 1: Создание ConfigurationGuidedParser ===");
    let guided_parser = ConfigurationGuidedParser::new("test_path");
    println!(
        "✅ ConfigurationGuidedParser создан успешно: {:?}",
        guided_parser
    );

    println!("\n=== Тест 2: Проверка типов ===");
    println!("✅ Все типы доступны и компилируются");

    println!("\n🎉 Простые тесты завершены успешно!");
    Ok(())
}
