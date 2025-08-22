//! Простой тест HTML рендеринга

use anyhow::Result;
use bsl_gradual_types::documentation::core::hierarchy::DocumentationSourceType;
use bsl_gradual_types::documentation::render::HtmlDocumentationRenderer;
use bsl_gradual_types::documentation::search::{
    FacetValue, HighlightFragment, PaginationInfo, SearchFacet, SearchResultItem, SearchResults,
};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎨 Простой тест HTML рендеринга");

    // Создаем HTML рендерер
    let html_renderer = HtmlDocumentationRenderer::new();
    println!("✅ HtmlDocumentationRenderer создан");

    // Создаем тестовые результаты поиска
    let test_results = create_test_search_results();
    println!(
        "✅ Тестовые результаты созданы: {} элементов",
        test_results.items.len()
    );

    // Рендеринг результатов поиска
    println!("\n=== 🔍 Рендеринг результатов поиска ===");
    let search_html = html_renderer.render_search_results(&test_results).await?;

    // Создаем полную HTML страницу
    let full_page = format!(
        "<!DOCTYPE html>\n\
         <html lang='ru'>\n\
         <head>\n\
         <meta charset='UTF-8'>\n\
         <meta name='viewport' content='width=device-width, initial-scale=1.0'>\n\
         <title>BSL Search Results</title>\n\
         {}\n\
         </head>\n\
         <body class='theme-dark'>\n\
         <div class='container'>\n\
         {}\n\
         </div>\n\
         {}\n\
         </body>\n\
         </html>",
        html_renderer.render_css(),
        search_html,
        html_renderer.render_javascript()
    );

    // Сохраняем в файл
    let output_file = "bsl_search_demo.html";
    fs::write(output_file, &full_page)?;

    println!("✅ HTML страница сохранена: {}", output_file);
    println!("📄 Размер HTML: {} символов", full_page.len());
    println!("🌐 Откройте файл в браузере для просмотра результата");

    // Тест разных тем
    println!("\n=== 🎨 Тест переключения тем ===");
    let mut renderer_copy = html_renderer;

    // Тест темной темы
    renderer_copy.set_theme("dark").await?;
    println!("✅ Темная тема установлена");

    // Тест светлой темы
    renderer_copy.set_theme("light").await?;
    println!("✅ Светлая тема установлена");

    // Тест VSCode темы
    renderer_copy.set_theme("vscode").await?;
    println!("✅ VSCode тема установлена");

    println!("\n🎉 HTML рендеринг работает отлично!");

    Ok(())
}

/// Создать тестовые результаты поиска
fn create_test_search_results() -> SearchResults {
    SearchResults {
        items: vec![
            SearchResultItem {
                type_id: "platform_1".to_string(),
                display_name: "ТаблицаЗначений".to_string(),
                description: "Универсальная коллекция для работы с табличными данными. Поддерживает добавление строк, колонок, сортировку и фильтрацию.".to_string(),
                category: "Global context/Universal collections".to_string(),
                source_type: DocumentationSourceType::Platform { version: "8.3".to_string() },
                relevance_score: 1.0,
                highlights: vec![
                    HighlightFragment {
                        field: "name".to_string(),
                        highlighted_text: "<mark>ТаблицаЗначений</mark>".to_string(),
                    }
                ],
                breadcrumb: vec!["Global context".to_string(), "Universal collections".to_string()],
            },
            SearchResultItem {
                type_id: "platform_2".to_string(),
                display_name: "СписокЗначений".to_string(),
                description: "Упорядоченная коллекция уникальных значений с возможностью быстрого поиска.".to_string(),
                category: "Global context/Universal collections".to_string(),
                source_type: DocumentationSourceType::Platform { version: "8.3".to_string() },
                relevance_score: 0.8,
                highlights: vec![],
                breadcrumb: vec!["Global context".to_string(), "Universal collections".to_string()],
            },
            SearchResultItem {
                type_id: "platform_3".to_string(),
                display_name: "ДеревоЗначений".to_string(),
                description: "Иерархическая коллекция для представления древовидных структур данных.".to_string(),
                category: "Global context/Universal collections".to_string(),
                source_type: DocumentationSourceType::Platform { version: "8.3".to_string() },
                relevance_score: 0.7,
                highlights: vec![],
                breadcrumb: vec!["Global context".to_string(), "Universal collections".to_string()],
            },
        ],
        total_count: 3,
        facets: vec![
            SearchFacet {
                name: "Категории".to_string(),
                values: vec![
                    FacetValue {
                        value: "Universal collections".to_string(),
                        count: 15,
                        selected: false,
                    },
                    FacetValue {
                        value: "System types".to_string(),
                        count: 8,
                        selected: false,
                    },
                ],
            },
            SearchFacet {
                name: "Источник".to_string(),
                values: vec![
                    FacetValue {
                        value: "Platform".to_string(),
                        count: 23,
                        selected: true,
                    },
                ],
            },
        ],
        search_time_ms: 25,
        suggestions: vec!["ТаблицаЗначений".to_string(), "СписокЗначений".to_string()],
        related_queries: vec!["СписокЗначений".to_string(), "ДеревоЗначений".to_string()],
        pagination_info: PaginationInfo {
            current_page: 0,
            total_pages: 1,
            has_next: false,
            has_previous: false,
            page_size: 10,
        },
    }
}
