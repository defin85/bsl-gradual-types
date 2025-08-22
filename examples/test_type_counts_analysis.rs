use anyhow::Result;
use bsl_gradual_types::{
    core::platform_resolver::PlatformTypeResolver,
    documentation::{
        core::{
            providers::ProviderConfig, BslDocumentationSystem, DocumentationConfig,
            DocumentationProvider,
        },
        platform::PlatformDocumentationProvider,
    },
};

/// Анализ разных систем подсчета типов
#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 АНАЛИЗ: Разница в подсчете типов");
    println!("{}", "=".repeat(70));

    // 1. PlatformTypeResolver (используется в статистике веб-сервера)
    println!("\n📊 1. PlatformTypeResolver (веб-сервер статистика):");
    let platform_resolver = PlatformTypeResolver::new();
    let resolver_globals_count = platform_resolver.get_platform_globals_count();
    println!("   • Platform globals count: {}", resolver_globals_count);

    // 2. PlatformDocumentationProvider (используется для иерархии)
    println!("\n📚 2. PlatformDocumentationProvider (иерархия):");
    let platform_provider = PlatformDocumentationProvider::new();

    // Инициализируем провайдер
    let config = ProviderConfig::default();
    match platform_provider.initialize(&config).await {
        Ok(_) => {
            let provider_types = platform_provider.get_all_types().await?;
            println!(
                "   • Documentation provider types: {}",
                provider_types.len()
            );

            // Показываем первые 10 типов
            for (i, doc_node) in provider_types.iter().take(10).enumerate() {
                match doc_node {
                    bsl_gradual_types::documentation::core::hierarchy::DocumentationNode::PlatformType(pt) => {
                        println!("   └─ [{}] Platform: '{}'", i + 1, pt.base_info.russian_name);
                    },
                    bsl_gradual_types::documentation::core::hierarchy::DocumentationNode::SubCategory(sc) => {
                        println!("   └─ [{}] SubCategory: '{}' ({} дочерних)", i + 1, sc.name, sc.children.len());
                    },
                    _ => {
                        println!("   └─ [{}] Other type", i + 1);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ Ошибка инициализации: {}", e);
        }
    }

    // 3. BslDocumentationSystem (полная система)
    println!("\n🏗️ 3. BslDocumentationSystem (полная система):");
    let documentation_system = BslDocumentationSystem::new();
    let docs_config = DocumentationConfig::default();

    match documentation_system.initialize(docs_config).await {
        Ok(_) => {
            let hierarchy = documentation_system.get_type_hierarchy().await?;

            println!(
                "   • Корневых категорий: {}",
                hierarchy.root_categories.len()
            );

            let mut total_subcategories = 0;
            let mut total_types_in_hierarchy = 0;
            let mut non_empty_subcategories = 0;

            for category in &hierarchy.root_categories {
                println!(
                    "   📁 Категория: '{}' ({} дочерних)",
                    category.name,
                    category.children.len()
                );

                for child in &category.children {
                    match child {
                        bsl_gradual_types::documentation::core::hierarchy::DocumentationNode::SubCategory(sub_cat) => {
                            total_subcategories += 1;
                            if !sub_cat.children.is_empty() {
                                non_empty_subcategories += 1;
                                total_types_in_hierarchy += sub_cat.children.len();
                                
                                if non_empty_subcategories <= 5 { // Показываем только первые 5
                                    println!("      └─ 📂 '{}': {} типов", sub_cat.name, sub_cat.children.len());
                                }
                            }
                        },
                        bsl_gradual_types::documentation::core::hierarchy::DocumentationNode::PlatformType(_) => {
                            total_types_in_hierarchy += 1;
                        },
                        _ => {}
                    }
                }
            }

            println!("   • Всего подкатегорий: {}", total_subcategories);
            println!("   • Непустых подкатегорий: {}", non_empty_subcategories);
            println!("   • Типов в иерархии: {}", total_types_in_hierarchy);
        }
        Err(e) => {
            println!("   ❌ Ошибка инициализации системы: {}", e);
        }
    }

    println!("\n🎯 ВЫВОДЫ:");
    println!("   • 13,607 = PlatformTypeResolver.get_platform_globals() - ВСЕ глобальные объекты");
    println!("   • 3,884 = DocumentationProvider - типы для документации");
    println!("   • 195 = Подкатегории в иерархии (многие пустые)");
    println!("   • ? = Реальное количество типов для отображения в дереве");

    Ok(())
}
