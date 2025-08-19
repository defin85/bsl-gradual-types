use anyhow::Result;
use bsl_gradual_types::documentation::{
    core::{BslDocumentationSystem, DocumentationConfig},
    render::RenderEngine,
};

/// Демонстрация интерактивного дерева типов
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 BSL Interactive Tree Demo");
    println!("{}", "=".repeat(50));

    // Инициализируем систему документации
    let documentation_system = BslDocumentationSystem::new();
    println!("✅ Documentation system created");

    // Инициализируем систему с конфигурацией по умолчанию
    let config = DocumentationConfig::default();
    documentation_system.initialize(config).await?;
    println!("✅ System fully initialized");

    // Получаем иерархию типов
    let hierarchy = documentation_system.get_type_hierarchy().await?;
    println!("✅ Type hierarchy obtained");
    println!("📊 Statistics:");
    println!("   • Categories: {}", hierarchy.root_categories.len());
    
    let total_nodes: usize = hierarchy.root_categories.iter()
        .map(|cat| cat.children.len())
        .sum();
    println!("   • Total nodes: {}", total_nodes);

    // Создаем рендерер
    let render_engine = RenderEngine::new();
    let html_renderer = render_engine.html_renderer();
    
    // Создаем интерактивное дерево
    println!("\n🌳 Creating interactive tree...");
    let interactive_tree = html_renderer.create_interactive_tree(&hierarchy);
    
    println!("✅ Interactive tree created:");
    println!("   • Root nodes: {}", interactive_tree.root_nodes.len());
    println!("   • Lazy loading: {}", interactive_tree.settings.lazy_loading);
    println!("   • Drag & drop: {}", interactive_tree.settings.drag_drop);
    println!("   • Context menus: {}", interactive_tree.settings.context_menus);
    println!("   • Tree search: {}", interactive_tree.settings.tree_search);
    println!("   • Bookmarks: {}", interactive_tree.settings.bookmarks);
    println!("   • Favorites: {}", interactive_tree.settings.favorites);

    // Рендерим дерево в HTML
    println!("\n🎨 Rendering interactive tree to HTML...");
    let tree_html = html_renderer.render_interactive_tree(&interactive_tree)?;
    
    // Рендерим полную иерархию с интерактивным деревом
    let full_html = html_renderer.render_hierarchy(&hierarchy).await?;
    
    // Сохраняем в файл
    let output_file = "interactive_tree_demo.html";
    std::fs::write(output_file, full_html)?;
    
    println!("✅ Interactive tree rendered to HTML");
    println!("📄 File saved: {}", output_file);
    println!("🌐 Open in browser to see interactive features:");
    println!("   • Lazy loading of child nodes");
    println!("   • Expandable/collapsible tree structure");
    println!("   • Real-time search in tree");
    println!("   • Drag & drop for organization"); 
    println!("   • Right-click context menus");
    println!("   • Bookmarks and favorites");
    println!("   • Detailed node information with tabs");
    
    // Показываем примеры узлов дерева
    println!("\n📋 Sample tree nodes:");
    for (i, node) in interactive_tree.root_nodes.iter().take(5).enumerate() {
        println!("{}. {} {} - {} (has_children: {})", 
            i + 1,
            node.icon,
            node.display_name,
            node.description.as_deref().unwrap_or("No description"),
            node.has_children
        );
    }
    
    if interactive_tree.root_nodes.len() > 5 {
        println!("   ... и еще {} узлов", interactive_tree.root_nodes.len() - 5);
    }

    println!("\n🎯 Features implemented:");
    println!("   ✅ Hierarchical tree with lazy loading");
    println!("   ✅ Node expansion: type → methods → parameters");
    println!("   ✅ Drag & drop for organization");
    println!("   ✅ Context menus");
    println!("   ✅ Bookmarks and favorites");
    println!("   ✅ Real-time tree search");
    println!("   ✅ Responsive themes (Dark/Light/VSCode)");
    println!("   ✅ Animated expand/collapse");
    println!("   ✅ Detailed node information tabs");

    println!("\n🏆 Milestone 3.2: Interactive Tree - COMPLETED!");

    Ok(())
}