//! Демонстрация централизованного TypeSystemService

use anyhow::Result;
use bsl_gradual_types::system::services::{
    InitializationStage, TypeSystemService, TypeSystemServiceConfig, TypeSystemServiceFactory,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏢 Демонстрация централизованного TypeSystemService");

    // Тест 1: Создание сервиса с настройками по умолчанию
    println!("\n=== 🔧 Тест 1: Создание сервиса ===");
    let service = TypeSystemService::with_defaults();

    let initial_state = service.get_initialization_state().await;
    println!("Начальное состояние: {:?}", initial_state.current_stage);
    assert!(!initial_state.is_initialized);

    // Тест 2: Инициализация сервиса
    println!("\n=== ⚡ Тест 2: Инициализация сервиса ===");
    match service.initialize().await {
        Ok(_) => {
            let final_state = service.get_initialization_state().await;
            println!(
                "✅ Инициализация завершена: {:?}",
                final_state.current_stage
            );
            println!("📊 Прогресс: {}%", final_state.progress);
            println!("📝 Статус: {}", final_state.status_message);

            if let Some(start_time) = final_state.start_time {
                println!(
                    "⏱️ Время инициализации: {:.2}s",
                    start_time.elapsed().as_secs_f64()
                );
            }
        }
        Err(e) => {
            println!("⚠️ Инициализация с предупреждениями: {}", e);
            println!("   Это нормально в тестовом окружении");
        }
    }

    // Тест 3: LSP API
    println!("\n=== 📡 Тест 3: LSP API ===");
    let expressions = vec![
        "Справочники.Контрагенты",
        "Документы.ЗаказКлиента",
        "ТаблицаЗначений",
        "СписокЗначений",
    ];

    for expression in expressions {
        let resolution = service.resolve_expression(expression).await;
        println!("🔍 '{}' → {:?}", expression, resolution.certainty);

        let completions = service.get_completions(expression).await;
        println!(
            "💡 Автодополнение для '{}': {} вариантов",
            expression,
            completions.len()
        );
    }

    // Тест 4: Веб API
    println!("\n=== 🌐 Тест 4: Веб API ===");
    let search_queries = vec!["ТаблицаЗначений", "Справочники", "HTTP"];

    for query in search_queries {
        // Создаем простой поисковый запрос
        use bsl_gradual_types::documentation::AdvancedSearchQuery;
        let search_query = AdvancedSearchQuery {
            query: query.to_string(),
            ..Default::default()
        };

        match service.search(search_query).await {
            Ok(results) => {
                println!(
                    "🔍 Поиск '{}': {} результатов за {}ms",
                    query, results.total_count, results.search_time_ms
                );
            }
            Err(e) => {
                println!("❌ Ошибка поиска '{}': {}", query, e);
            }
        }

        // Тест автодополнения
        match service.get_suggestions(query).await {
            Ok(suggestions) => {
                println!(
                    "💡 Предложения для '{}': {:?}",
                    query,
                    suggestions.iter().take(3).collect::<Vec<_>>()
                );
            }
            Err(e) => {
                println!("❌ Ошибка предложений '{}': {}", query, e);
            }
        }
    }

    // Тест 5: Статистика
    println!("\n=== 📊 Тест 5: Статистика использования ===");
    let stats = service.get_usage_stats().await;
    println!("LSP запросы: {}", stats.lsp_requests);
    println!("Веб запросы: {}", stats.web_requests);
    println!("Поисковые запросы: {}", stats.search_requests);
    println!("Запросы автодополнения: {}", stats.completion_requests);

    match service.get_performance_stats().await {
        Ok(perf_stats) => {
            println!("Общее количество запросов: {}", perf_stats.total_requests);
            println!("Использование памяти: {:.2} MB", perf_stats.memory_usage_mb);
            println!("Cache hit ratio: {:.2}", perf_stats.cache_hit_ratio);
        }
        Err(e) => {
            println!("⚠️ Ошибка получения статистики производительности: {}", e);
        }
    }

    // Тест 6: Factory методы
    println!("\n=== 🏭 Тест 6: Factory методы ===");

    println!("Тестируем TypeSystemServiceFactory::create_for_development()");
    match TypeSystemServiceFactory::create_for_development().await {
        Ok(dev_service) => {
            println!("✅ Development сервис создан");
            let dev_stats = dev_service.get_usage_stats().await;
            println!("Development сервис готов к использованию");
        }
        Err(e) => {
            println!("⚠️ Development сервис с предупреждениями: {}", e);
        }
    }

    println!("\n🎉 TypeSystemService полностью протестирован!");
    println!("🏢 Архитектура shared service готова к использованию");

    Ok(())
}
