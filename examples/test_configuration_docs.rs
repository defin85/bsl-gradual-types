//! Тест системы документации конфигурационных типов

use anyhow::Result;
use bsl_gradual_types::documentation::ConfigurationDocumentationProvider;
use bsl_gradual_types::documentation::core::{DocumentationProvider, ProviderConfig};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Тестирование документации конфигурационных типов...\n");
    
    // Создаем провайдер конфигурации
    let config_provider = ConfigurationDocumentationProvider::new();
    
    // Конфигурация с полными XML файлами (с namespace)
    let config = ProviderConfig {
        data_source: "tests/fixtures/xml_full".to_string(),
        ..Default::default()
    };
    
    println!("📁 Инициализация с путем: {}", config.data_source);
    
    // Инициализируем провайдер
    match config_provider.initialize(&config).await {
        Ok(_) => println!("✅ Провайдер конфигурации инициализирован"),
        Err(e) => println!("❌ Ошибка инициализации: {}", e),
    }
    
    // Получаем статистику
    match config_provider.get_statistics().await {
        Ok(stats) => {
            println!("\n📊 Статистика конфигурационных типов:");
            println!("   • Типов: {}", stats.total_types);
            println!("   • Методов: {}", stats.total_methods);
            println!("   • Свойств: {}", stats.total_properties);
            println!("   • Память: {:.1} MB", stats.memory_usage_mb);
        }
        Err(e) => println!("❌ Ошибка получения статистики: {}", e),
    }
    
    // Получаем корневую категорию
    match config_provider.get_root_category().await {
        Ok(root_category) => {
            println!("\n📁 Корневая категория конфигурации:");
            println!("   🏢 {} ({})", root_category.name, root_category.description);
            println!("      └─ Дочерних узлов: {}", root_category.children.len());
            println!("      └─ Типов: {}", root_category.statistics.child_types_count);
        }
        Err(e) => println!("❌ Ошибка получения категории: {}", e),
    }
    
    // Проверяем доступность
    match config_provider.check_availability().await {
        Ok(available) => println!("\n🔗 Доступность конфигурации: {}", if available { "✅ Доступна" } else { "❌ Недоступна" }),
        Err(e) => println!("❌ Ошибка проверки доступности: {}", e),
    }
    
    // Получаем все типы
    match config_provider.get_all_types().await {
        Ok(types) => {
            println!("\n📋 Все конфигурационные типы ({}):", types.len());
            for (i, type_doc) in types.iter().enumerate().take(5) {
                println!("   {}. {} / {}", i+1, type_doc.russian_name, type_doc.english_name);
                println!("      └─ Описание: {}", type_doc.description);
                println!("      └─ Источник: {:?}", type_doc.source_type);
            }
            if types.len() > 5 {
                println!("   ... и еще {} типов", types.len() - 5);
            }
        }
        Err(e) => println!("❌ Ошибка получения типов: {}", e),
    }
    
    println!("\n🎉 Тест завершен!");
    
    Ok(())
}