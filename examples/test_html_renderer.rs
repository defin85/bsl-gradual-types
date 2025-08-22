//! Демонстрация HTML рендеринга документации

use anyhow::Result;
use bsl_gradual_types::documentation::core::hierarchy::{CategoryNode, TypeHierarchy};
use bsl_gradual_types::documentation::{
    AdvancedSearchQuery, DocumentationSearchEngine, RenderEngine,
};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎨 Демонстрация HTML рендеринга BSL документации");

    // Создаем рендер-движок
    let render_engine = RenderEngine::new();
    println!("✅ RenderEngine создан");

    // Создаем тестовую иерархию
    let test_hierarchy = create_test_hierarchy();
    println!(
        "✅ Тестовая иерархия создана: {} категорий",
        test_hierarchy.root_categories.len()
    );

    // Рендеринг иерархии в HTML
    println!("\n=== 🏗️ Рендеринг иерархии ===");
    let hierarchy_html = render_engine.render_hierarchy_html(&test_hierarchy).await?;

    // Сохраняем в файл
    let output_file = "type_hierarchy_rendered.html";
    fs::write(output_file, &hierarchy_html)?;
    println!("✅ HTML иерархия сохранена в: {}", output_file);
    println!("📄 Размер HTML: {} символов", hierarchy_html.len());

    // Тест поиска и рендеринга результатов
    println!("\n=== 🔍 Рендеринг результатов поиска ===");
    let search_engine = DocumentationSearchEngine::new();

    // Создаем тестовый запрос
    let test_query = AdvancedSearchQuery {
        query: "ТаблицаЗначений".to_string(),
        ..Default::default()
    };

    let search_results = search_engine.search(test_query).await?;
    println!(
        "🔍 Результаты поиска: {} найдено",
        search_results.total_count
    );

    // Рендеринг результатов поиска
    let search_html = render_engine
        .html_renderer
        .render_search_results(&search_results)
        .await?;

    // Создаем полную страницу с результатами поиска
    let full_search_page = format!(
        "<!DOCTYPE html>\n\
         <html>\n\
         <head>\n\
         <meta charset='UTF-8'>\n\
         <title>BSL Search Results</title>\n\
         {}\n\
         </head>\n\
         <body>\n\
         {}\n\
         </body>\n\
         </html>",
        render_engine.html_renderer.render_css(),
        search_html
    );

    let search_output_file = "search_results_rendered.html";
    fs::write(search_output_file, &full_search_page)?;
    println!("✅ Результаты поиска сохранены в: {}", search_output_file);

    // Тест разных тем
    println!("\n=== 🎨 Тест тем ===");
    let themes = render_engine.get_available_themes();
    println!("Доступные темы: {:?}", themes);

    for theme_name in themes {
        println!("  ✅ Тема '{}' доступна", theme_name);
    }

    println!("\n🎉 HTML рендеринг готов!");
    println!("🌐 Откройте файлы в браузере для просмотра:");
    println!("  - {}", output_file);
    println!("  - {}", search_output_file);

    Ok(())
}

/// Создать тестовую иерархию для демонстрации
fn create_test_hierarchy() -> TypeHierarchy {
    use bsl_gradual_types::documentation::core::hierarchy::{
        DocumentationNode, HierarchyStatistics, NavigationIndex, RootCategoryNode, SubCategoryNode,
    };
    use std::collections::HashMap;

    // Создаем корневую категорию
    let root_category = RootCategoryNode {
        id: "global_context".to_string(),
        name: "Global Context".to_string(),
        description: "Глобальный контекст 1С:Предприятие".to_string(),
        children: vec![
            DocumentationNode::SubCategory(SubCategoryNode {
                id: "universal_collections".to_string(),
                name: "Универсальные коллекции".to_string(),
                description: "Коллекции для работы с данными".to_string(),
                hierarchy_path: vec![
                    "Global Context".to_string(),
                    "Universal collections".to_string(),
                ],
            }),
            DocumentationNode::SubCategory(SubCategoryNode {
                id: "system_types".to_string(),
                name: "Системные типы".to_string(),
                description: "Встроенные типы платформы".to_string(),
                hierarchy_path: vec!["Global Context".to_string(), "System types".to_string()],
            }),
        ],
        ui_metadata: Default::default(),
        statistics: Default::default(),
    };

    TypeHierarchy {
        root_categories: vec![root_category],
        statistics: HierarchyStatistics {
            total_nodes: 3,
            node_counts: HashMap::new(),
            max_depth: 2,
            average_children_per_node: 2.0,
            most_populated_category: "Universal collections".to_string(),
        },
        navigation_index: NavigationIndex {
            by_id: HashMap::new(),
            by_russian_name: HashMap::new(),
            by_english_name: HashMap::new(),
        },
    }
}
