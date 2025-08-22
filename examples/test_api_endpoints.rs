//! Тест REST API endpoints для поисковой системы

use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🌐 Тестирование REST API endpoints");

    // Запускаем веб-сервер в фоне (потребуется отдельный процесс)
    println!("📋 Для полного тестирования запустите веб-сервер:");
    println!("   cargo run --bin bsl-web-server --port 8080");
    println!();

    // Примеры curl команд для тестирования API
    println!("🔧 Примеры использования API:");
    println!();

    // 1. Расширенный поиск
    println!("1️⃣ POST /api/v1/search - Расширенный поиск:");
    let search_payload = json!({
        "query": "ТаблицаЗначений",
        "filters": {
            "source_types": [],
            "categories": [],
            "facets": [],
            "availability": [],
            "version_range": null,
            "include_methods": true,
            "include_properties": true,
            "include_examples": false
        },
        "sort": {
            "field": "Relevance",
            "direction": "Descending",
            "secondary": null
        },
        "pagination": {
            "page_size": 10,
            "page_number": 0,
            "max_results": null
        },
        "options": {
            "fuzzy_search": true,
            "include_synonyms": true,
            "search_in_examples": false,
            "min_score": 0.5,
            "highlight_matches": true
        }
    });

    println!("curl -X POST http://localhost:8080/api/v1/search \\");
    println!("  -H \"Content-Type: application/json\" \\");
    println!("  -d '{}'", search_payload.to_string());
    println!();

    // 2. Автодополнение
    println!("2️⃣ GET /api/v1/suggestions - Автодополнение:");
    println!("curl \"http://localhost:8080/api/v1/suggestions?q=Табли&limit=5\"");
    println!();

    // 3. Статистика поиска
    println!("3️⃣ GET /api/v1/search-stats - Статистика поиска:");
    println!("curl \"http://localhost:8080/api/v1/search-stats\"");
    println!();

    // 4. Категории
    println!("4️⃣ GET /api/v1/categories - Список категорий:");
    println!("curl \"http://localhost:8080/api/v1/categories\"");
    println!();

    // 5. Legacy поиск (совместимость)
    println!("5️⃣ GET /api/types - Legacy поиск (обратная совместимость):");
    println!("curl \"http://localhost:8080/api/types?search=Таблица&page=0&per_page=10\"");
    println!();

    println!("🎯 Примеры ответов API:");
    println!();

    // Пример ответа поиска
    println!("📊 Пример ответа /api/v1/search:");
    let sample_search_response = json!({
        "items": [
            {
                "type_id": "platform_123",
                "display_name": "ТаблицаЗначений",
                "description": "Универсальная коллекция для работы с табличными данными",
                "category": "Global context/Universal collections",
                "source_type": {"Platform": {"version": "8.3"}},
                "relevance_score": 1.0,
                "highlights": [
                    {
                        "field": "content",
                        "highlighted_text": "<mark>ТаблицаЗначений</mark> универсальная коллекция"
                    }
                ],
                "breadcrumb": ["Global context", "Universal collections"]
            }
        ],
        "total_count": 1,
        "facets": [
            {
                "name": "Категории",
                "values": [
                    {"value": "Global context/Universal collections", "count": 15, "selected": false}
                ]
            }
        ],
        "search_time_ms": 25,
        "suggestions": ["ТаблицаЗначений", "СписокЗначений"],
        "related_queries": ["СписокЗначений", "ДеревоЗначений"],
        "pagination_info": {
            "current_page": 0,
            "total_pages": 1,
            "has_next": false,
            "has_previous": false,
            "page_size": 10
        }
    });
    println!("{}", serde_json::to_string_pretty(&sample_search_response)?);
    println!();

    // Пример ответа автодополнения
    println!("💡 Пример ответа /api/v1/suggestions:");
    let sample_suggestions_response = json!({
        "suggestions": ["ТаблицаЗначений", "ТаблицаЗначенийКолонка", "ТаблицаЗначенийСтрока"],
        "query": "Табли",
        "count": 3
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&sample_suggestions_response)?
    );

    println!("\n🎉 Документация API endpoints готова!");
    println!("🚀 Запустите веб-сервер для тестирования API");

    Ok(())
}
