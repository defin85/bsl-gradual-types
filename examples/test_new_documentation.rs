//! Тест новой системы документации

use anyhow::Result;
use bsl_gradual_types::documentation::{DocumentationConfig, PlatformConfig};
use bsl_gradual_types::documentation::BslDocumentationSystem;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Тестирование новой системы документации BSL...\n");

    // Создаем систему документации
    let doc_system = BslDocumentationSystem::new();

    // Настройки
    let config = DocumentationConfig {
        platform_config: PlatformConfig {
            syntax_helper_path: "examples/syntax_helper/rebuilt.shcntx_ru".to_string(),
            platform_version: "8.3.23".to_string(),
            show_progress: true,
            worker_threads: 4,
        },
        configuration_path: Some("tests/fixtures/xml_full".to_string()), // Полная конфигурация
        ..Default::default()
    };

    println!("📚 Инициализация системы документации...");

    // Инициализируем систему
    doc_system.initialize(config).await?;

    println!("✅ Система документации инициализирована!\n");

    // Получаем статистику
    println!("📊 Получение статистики...");
    let stats = doc_system.get_statistics().await?;

    println!("📈 Статистика платформенных типов:");
    println!("   • Типов: {}", stats.platform.total_types);
    println!("   • Методов: {}", stats.platform.total_methods);
    println!("   • Свойств: {}", stats.platform.total_properties);
    println!("   • Память: {:.1} MB", stats.platform.memory_usage_mb);

    // Получаем иерархию
    println!("\n🌲 Получение иерархии типов...");
    let hierarchy = doc_system.get_type_hierarchy().await?;

    println!("📊 Статистика иерархии:");
    println!("   • Всего узлов: {}", hierarchy.statistics.total_nodes);
    println!(
        "   • Максимальная глубина: {}",
        hierarchy.statistics.max_depth
    );
    println!(
        "   • Корневых категорий: {}",
        hierarchy.root_categories.len()
    );

    // Показываем корневые категории
    println!("\n📁 Корневые категории:");
    for category in &hierarchy.root_categories {
        println!("   🏢 {} ({})", category.name, category.description);
        println!("      └─ Дочерних узлов: {}", category.children.len());
        println!("      └─ Типов: {}", category.statistics.child_types_count);
        println!(
            "      └─ Методов: {}",
            category.statistics.total_methods_count
        );
        println!(
            "      └─ Свойств: {}",
            category.statistics.total_properties_count
        );
    }

    // Тестируем поиск типа
    println!("\n🔍 Тестирование поиска типа 'ТаблицаЗначений'...");
    if let Some(type_details) = doc_system.get_type_details("ТаблицаЗначений").await?
    {
        println!("✅ Найден тип: {}", type_details.russian_name);
        println!("   • Английское название: {}", type_details.english_name);
        println!("   • Описание: {}", type_details.description);
        println!("   • Методов: {}", type_details.methods.len());
        println!("   • Свойств: {}", type_details.properties.len());
        println!("   • Фасеты: {:?}", type_details.available_facets);
        println!("   • Активный фасет: {:?}", type_details.active_facet);

        // Показываем первые несколько методов
        if !type_details.methods.is_empty() {
            println!("   📋 Методы (первые 5):");
            for method in type_details.methods.iter().take(5) {
                println!("      🔧 {} / {}", method.russian_name, method.english_name);
            }
        }

        // Показываем свойства
        if !type_details.properties.is_empty() {
            println!("   📊 Свойства:");
            for property in &type_details.properties {
                println!(
                    "      📋 {} / {}",
                    property.russian_name, property.english_name
                );
            }
        }
    } else {
        println!("❌ Тип 'ТаблицаЗначений' не найден");
    }

    println!("\n🎉 Тест завершен успешно!");

    Ok(())
}
