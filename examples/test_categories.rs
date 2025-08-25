//! Тестирование парсинга категорий и группировки типов

use anyhow::Result;
use bsl_gradual_types::data::loaders::syntax_helper_parser::{
    OptimizationSettings, SyntaxHelperParser, SyntaxNode,
};
use std::path::Path;

fn main() -> Result<()> {
    println!("=== Тестирование парсинга категорий ===\n");

    let syntax_helper_path = Path::new("examples/syntax_helper/rebuilt.shcntx_ru/objects");

    // Создаём парсер
    let settings = OptimizationSettings {
        show_progress: false,
        ..Default::default()
    };
    let mut parser = SyntaxHelperParser::with_settings(settings);

    // Парсим директорию
    println!("📂 Парсинг директории: {}", syntax_helper_path.display());
    parser.parse_directory(syntax_helper_path)?;

    // Экспортируем данные
    let database = parser.export_database();

    // Ищем категорию catalog234
    println!("\n🔍 Поиск категории catalog234...");

    for (id, category) in &database.categories {
        if id == "catalog234" {
            println!("\n✅ Найдена категория:");
            println!("   ID: {}", id);
            println!("   Название: {}", category.name);
            println!("   Описание: {}", category.description);

            // Ищем типы в этой категории
            println!("\n📋 Типы в категории '{}':", category.name);

            let mut types_in_category = Vec::new();
            for (path, node) in &database.nodes {
                if let SyntaxNode::Type(type_info) = node {
                    if type_info.identity.category_path == category.name {
                        types_in_category.push(&type_info.identity.russian_name);
                    }
                }
            }

            if types_in_category.is_empty() {
                // Если не нашли по имени категории, ищем по пути
                for (path, node) in &database.nodes {
                    if path.contains("/catalog234/") {
                        if let SyntaxNode::Type(type_info) = node {
                            types_in_category.push(&type_info.identity.russian_name);
                        }
                    }
                }
            }

            types_in_category.sort();
            for (i, type_name) in types_in_category.iter().enumerate() {
                println!("   {}. {}", i + 1, type_name);
                if i >= 9 {
                    println!("   ... и еще {} типов", types_in_category.len() - 10);
                    break;
                }
            }

            println!("\n   Всего типов в категории: {}", types_in_category.len());
        }
    }

    // Проверяем другие категории
    println!("\n📊 Первые 10 категорий:");
    for (i, (id, category)) in database.categories.iter().enumerate() {
        if i >= 10 {
            break;
        }

        // Считаем типы в категории
        let types_count = database
            .nodes
            .values()
            .filter(|node| {
                if let SyntaxNode::Type(type_info) = node {
                    type_info.identity.category_path == category.name
                } else {
                    false
                }
            })
            .count();

        println!("   {} -> {} ({} типов)", id, category.name, types_count);
    }

    // Статистика по категориям
    println!("\n📈 Статистика:");
    println!("   Всего категорий: {}", database.categories.len());

    // Находим категории с наибольшим количеством типов
    let mut category_stats: Vec<(String, usize)> = Vec::new();

    for (id, category) in &database.categories {
        let count = database
            .nodes
            .values()
            .filter(|node| {
                if let SyntaxNode::Type(type_info) = node {
                    type_info.identity.category_path == category.name
                        || type_info
                            .identity
                            .catalog_path
                            .contains(&format!("/{}/", id))
                } else {
                    false
                }
            })
            .count();

        if count > 0 {
            category_stats.push((category.name.clone(), count));
        }
    }

    category_stats.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\n🏆 Топ категорий по количеству типов:");
    for (i, (name, count)) in category_stats.iter().enumerate() {
        if i >= 5 {
            break;
        }
        println!("   {}. {} - {} типов", i + 1, name, count);
    }

    Ok(())
}
