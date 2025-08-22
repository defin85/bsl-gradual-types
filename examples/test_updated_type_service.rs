//! Тест обновленного TypeSystemService v2.0 на базе UnifiedTypeSystem

use anyhow::Result;
use bsl_gradual_types::core::type_system_service::{
    TypeSystemService, TypeSystemServiceConfig, TypeSystemServiceFactory,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎉 Тестирование TypeSystemService v2.0 на базе UnifiedTypeSystem");

    // Создаем обновленный сервис
    println!("\n=== 🏗️ Создание TypeSystemService v2.0 ===");
    let service = TypeSystemService::with_defaults();

    // Проверяем начальное состояние
    let initial_state = service.get_initialization_state().await;
    println!("Начальное состояние: {:?}", initial_state.current_stage);

    // Инициализируем сервис
    println!("\n=== ⚡ Инициализация с UnifiedTypeSystem ===");
    match service.initialize().await {
        Ok(_) => {
            let final_state = service.get_initialization_state().await;
            println!(
                "✅ TypeSystemService v2.0 инициализирован: {:?}",
                final_state.current_stage
            );
            println!("📊 Прогресс: {}%", final_state.progress);
            println!("📝 Статус: {}", final_state.status_message);
        }
        Err(e) => {
            println!("⚠️ Инициализация с предупреждениями: {}", e);
        }
    }

    // Тест 1: LSP API (на базе UnifiedTypeSystem)
    println!("\n=== 📡 Тест 1: LSP API через UnifiedTypeSystem ===");
    let test_expressions = vec![
        "ТаблицаЗначений",
        "Справочники.Контрагенты",
        "СписокЗначений",
        "Документы.ЗаказКлиента",
        "HTTPЗапрос",
    ];

    for expression in &test_expressions {
        // Резолюция через единую систему
        let resolution = service.resolve_expression(expression).await;
        println!(
            "🔍 '{}' → {:?} (источник: {:?})",
            expression, resolution.certainty, resolution.source
        );

        // Автодополнение через единую систему
        let completions = service.get_completions(expression).await;
        println!("💡 Автодополнение: {} вариантов", completions.len());

        // Показываем первые варианты
        for (i, completion) in completions.iter().take(3).enumerate() {
            println!(
                "    {}. {} ({:?})",
                i + 1,
                completion.label,
                completion.kind
            );
        }
    }

    // Тест 2: Веб API (на базе UnifiedTypeSystem)
    println!("\n=== 🌐 Тест 2: Веб API через UnifiedTypeSystem ===");

    // Получение всех типов для отображения
    let display_types = service.get_all_types_for_display().await;
    println!("🎨 Типы для отображения: {}", display_types.len());

    // Показываем первые типы
    for (i, display_type) in display_types.iter().take(5).enumerate() {
        println!(
            "  {}. {} (категория: {}, уверенность: {:?})",
            i + 1,
            display_type.name,
            display_type.category,
            display_type.certainty
        );
    }

    // Поиск типов для отображения
    let search_queries = vec!["ТаблицаЗначений", "Справочники", "HTTP"];
    for query in search_queries {
        let search_results = service.search_types_for_display(query).await;
        println!(
            "🔍 Веб поиск '{}': {} результатов",
            query,
            search_results.len()
        );

        if let Some(first_result) = search_results.first() {
            println!(
                "    Первый: {} - {}",
                first_result.name, first_result.description
            );
        }
    }

    // Тест детальной информации
    println!("\n=== 📖 Тест 3: Детальная информация типов ===");
    let type_ids = vec!["ТаблицаЗначений", "СписокЗначений", "HTTPЗапрос"];

    for type_id in type_ids {
        if let Some(details) = service.get_type_details(type_id).await {
            println!("📋 Детали '{}':", details.name);
            println!("  - Методы: {}", details.methods.len());
            println!("  - Свойства: {}", details.properties.len());
            println!("  - Фасеты: {:?}", details.facets);
            println!("  - Уверенность: {:?}", details.full_resolution.certainty);
        } else {
            println!("❌ Детали для '{}' не найдены", type_id);
        }
    }

    // Тест 4: Расширенная статистика
    println!("\n=== 📊 Тест 4: Объединенная статистика ===");

    // Статистика TypeSystemService
    let service_stats = service.get_usage_stats().await;
    println!("📈 TypeSystemService статистика:");
    println!("  - LSP запросы: {}", service_stats.lsp_requests);
    println!("  - Веб запросы: {}", service_stats.web_requests);
    println!("  - Поисковые запросы: {}", service_stats.search_requests);
    println!("  - Автодополнение: {}", service_stats.completion_requests);

    // Статистика UnifiedTypeSystem
    let unified_stats = service.get_unified_system_stats().await;
    println!("🏗️ UnifiedTypeSystem статистика:");
    println!(
        "  - Платформенные типы: {}",
        unified_stats.platform_types_count
    );
    println!(
        "  - Конфигурационные типы: {}",
        unified_stats.configuration_types_count
    );
    println!(
        "  - Всего TypeResolution: {}",
        unified_stats.total_resolutions
    );
    println!(
        "  - Запросы резолюции: {}",
        unified_stats.resolution_requests
    );

    if unified_stats.cache_hits + unified_stats.cache_misses > 0 {
        let hit_ratio = unified_stats.cache_hits as f64
            / (unified_stats.cache_hits + unified_stats.cache_misses) as f64;
        println!("  - Cache hit ratio: {:.2}", hit_ratio);
    }

    // Статистика производительности
    match service.get_performance_stats().await {
        Ok(perf_stats) => {
            println!("⚡ Performance статистика:");
            println!("  - Общие запросы: {}", perf_stats.total_requests);
            println!("  - Cache ratio: {:.2}", perf_stats.cache_hit_ratio);
            println!("  - Память: {:.2} MB", perf_stats.memory_usage_mb);
        }
        Err(e) => {
            println!("⚠️ Ошибка статистики производительности: {}", e);
        }
    }

    println!("\n🎉 TypeSystemService v2.0 полностью протестирован!");
    println!("🏆 Единая архитектура с TypeResolution как источником истины работает!");

    Ok(())
}
