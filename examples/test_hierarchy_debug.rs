use anyhow::Result;
use bsl_gradual_types::documentation::{BslDocumentationSystem, DocumentationConfig};

/// Отладка структуры иерархии типов
#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 DEBUG: Анализ структуры иерархии типов");
    println!("{}", "=".repeat(60));

    // Инициализируем систему
    let documentation_system = BslDocumentationSystem::new();
    let config = DocumentationConfig::default();
    documentation_system.initialize(config).await?;

    // Получаем иерархию
    let hierarchy = documentation_system.get_type_hierarchy().await?;

    println!("📊 Общая статистика:");
    println!(
        "   • Корневых категорий: {}",
        hierarchy.root_categories.len()
    );

    for (i, category) in hierarchy.root_categories.iter().enumerate() {
        println!("\n📁 Категория {}: '{}'", i + 1, category.name);
        println!("   • Дочерних элементов: {}", category.children.len());

        // Показываем первые 10 дочерних элементов
        for (j, child) in category.children.iter().take(10).enumerate() {
            match child {
                bsl_gradual_types::documentation::hierarchy::DocumentationNode::SubCategory(sub_cat) => {
                    println!("   └─ [{}] 📂 Подкатегория: '{}' ({} элементов)", 
                        j + 1, sub_cat.name, sub_cat.children.len());
                },
                bsl_gradual_types::documentation::hierarchy::DocumentationNode::PlatformType(platform_type) => {
                    println!("   └─ [{}] 🔧 Платформенный тип: '{}' (методов: {}, свойств: {})", 
                        j + 1, 
                        platform_type.base_info.russian_name,
                        platform_type.base_info.methods.len(),
                        platform_type.base_info.properties.len()
                    );
                },
                bsl_gradual_types::documentation::hierarchy::DocumentationNode::ConfigurationType(config_type) => {
                    println!("   └─ [{}] ⚙️ Конфигурационный тип: '{}'", 
                        j + 1, config_type.base_info.russian_name);
                },
                _ => {
                    println!("   └─ [{}] ❓ Неизвестный тип узла", j + 1);
                }
            }
        }

        if category.children.len() > 10 {
            println!("   └─ ... и еще {} элементов", category.children.len() - 10);
        }
    }

    // Подсчет общего количества элементов
    let total_elements: usize = hierarchy
        .root_categories
        .iter()
        .map(|cat| cat.children.len())
        .sum();

    println!("\n📈 Итоговая статистика:");
    println!("   • Всего элементов в иерархии: {}", total_elements);
    println!(
        "   • Среднее количество элементов на категорию: {:.1}",
        total_elements as f32 / hierarchy.root_categories.len() as f32
    );

    Ok(())
}
