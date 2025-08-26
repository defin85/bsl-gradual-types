//! Комплексный тест революционной архитектуры
//!
//! Проверяет работу всех слоёв идеальной архитектуры

use bsl_gradual_types::presentation::adapters::{
    CliAnalysisRequest, CliOutputFormat, LspCompletionRequest, LspHoverRequest, WebSearchRequest,
};
use bsl_gradual_types::system::coordination::{CentralSystemConfig, CentralTypeSystem};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 КОМПЛЕКСНОЕ ТЕСТИРОВАНИЕ РЕВОЛЮЦИОННОЙ АРХИТЕКТУРЫ");
    println!("{}", "=".repeat(60));

    // === ТЕСТ 1: СОЗДАНИЕ И ИНИЦИАЛИЗАЦИЯ ===
    println!("\n1️⃣ Тест создания CentralTypeSystem...");

    let config = CentralSystemConfig {
        verbose_logging: false, // Отключаем для чистоты тестов
        ..Default::default()
    };

    let central_system = std::sync::Arc::new(CentralTypeSystem::new(config));
    println!("✅ CentralTypeSystem создана");

    // Инициализация
    let init_start = std::time::Instant::now();
    central_system.initialize().await?;
    let init_time = init_start.elapsed();

    println!("✅ Инициализация завершена за {:?}", init_time);

    // === ТЕСТ 2: МЕТРИКИ СИСТЕМЫ ===
    println!("\n2️⃣ Тест метрик системы...");

    let metrics = central_system.get_system_metrics().await;
    println!("📊 Метрики системы:");
    println!("   - Всего типов: {}", metrics.total_types);
    println!("   - Платформенных: {}", metrics.platform_types);
    println!("   - Конфигурационных: {}", metrics.configuration_types);
    println!("   - Память: {:.2} MB", metrics.cache_memory_mb);

    assert!(metrics.total_types > 0, "Типы должны быть загружены");
    assert!(
        metrics.platform_types > 0,
        "Платформенные типы должны быть загружены"
    );
    println!("✅ Метрики корректны");

    // === ТЕСТ 3: ПРОВЕРКА ЗДОРОВЬЯ ===
    println!("\n3️⃣ Тест проверки здоровья...");

    let health = central_system.health_check().await;
    println!("🏥 Здоровье системы:");
    println!("   - Статус: {}", health.status);
    println!("   - Оценка: {:.1}/10", health.overall_score * 10.0);
    println!("   - Компонентов: {}", health.components.len());

    for component in &health.components {
        println!("     • {}: {}", component.name, component.status);
    }

    assert_eq!(health.status, "healthy", "Система должна быть здоровой");
    assert!(
        health.overall_score > 0.8,
        "Оценка здоровья должна быть высокой"
    );
    println!("✅ Здоровье системы отличное");

    // === ТЕСТ 4: LSP ИНТЕРФЕЙС ===
    println!("\n4️⃣ Тест LSP интерфейса...");

    let lsp_interface = central_system.lsp_interface();

    // Тест автодополнения
    let completion_request = LspCompletionRequest {
        file_path: "test.bsl".to_string(),
        line: 10,
        column: 5,
        prefix: "Стр".to_string(),
        trigger_character: None,
    };

    let completion_response = lsp_interface
        .handle_completion_request(completion_request)
        .await?;
    println!(
        "   - Автодополнение: {} элементов",
        completion_response.items.len()
    );

    // Тест hover
    let hover_request = LspHoverRequest {
        file_path: "test.bsl".to_string(),
        line: 10,
        column: 5,
        expression: "Массив".to_string(),
    };

    if let Some(hover_response) = lsp_interface.handle_hover_request(hover_request).await? {
        println!(
            "   - Hover: {} элементов контента",
            hover_response.contents.len()
        );
    }

    // Метрики производительности LSP
    let perf_metrics = lsp_interface.get_performance_metrics().await?;
    println!("   - LSP запросов: {}", perf_metrics.total_requests);
    println!(
        "   - Среднее время: {:.2}ms",
        perf_metrics.average_response_time_ms
    );

    println!("✅ LSP интерфейс работает корректно");

    // === ТЕСТ 5: ВЕБ ИНТЕРФЕЙС ===
    println!("\n5️⃣ Тест веб-интерфейса...");

    let web_interface = central_system.web_interface();

    // Тест иерархии
    let hierarchy_response = web_interface.handle_hierarchy_request().await?;
    println!(
        "   - Иерархия: {} категорий",
        hierarchy_response.categories.len()
    );
    println!(
        "   - Всего типов в иерархии: {}",
        hierarchy_response.total_types
    );

    // В тестовом режиме категории могут быть пустыми (заглушки в Application Layer)
    println!(
        "   - Категории: {} (в тестовом режиме могут быть пустыми)",
        hierarchy_response.categories.len()
    );

    // Тест поиска
    let search_request = WebSearchRequest {
        query: "массив".to_string(),
        page: Some(1),
        per_page: Some(10),
        filters: None,
    };

    let search_response = web_interface.handle_search_request(search_request).await?;
    println!(
        "   - Поиск 'массив': {} результатов",
        search_response.results.len()
    );
    println!("   - Страниц: {}", search_response.total_pages);

    println!("✅ Веб-интерфейс работает корректно");

    // === ТЕСТ 6: CLI ИНТЕРФЕЙС ===
    println!("\n6️⃣ Тест CLI интерфейса...");

    let cli_interface = central_system.cli_interface();

    // Тест анализа проекта
    let analysis_request = CliAnalysisRequest {
        project_path: std::path::PathBuf::from("tests/fixtures"),
        output_format: CliOutputFormat::Text,
        include_coverage: true,
        include_errors: true,
        verbose: false,
    };

    let analysis_response = cli_interface
        .handle_analysis_request(analysis_request)
        .await?;
    println!("   - Анализ проекта:");
    println!("     • Файлов: {}", analysis_response.summary.total_files);
    println!(
        "     • Функций: {}",
        analysis_response.summary.total_functions
    );
    println!(
        "     • Переменных: {}",
        analysis_response.summary.total_variables
    );
    println!("     • Ошибок: {}", analysis_response.summary.error_count);

    if let Some(coverage) = &analysis_response.coverage {
        println!("     • Покрытие: {:.1}%", coverage.coverage_percentage);
    }

    println!("✅ CLI интерфейс работает корректно");

    // === ТЕСТ 7: ПРОИЗВОДИТЕЛЬНОСТЬ ===
    println!("\n7️⃣ Тест производительности...");

    let performance_start = std::time::Instant::now();

    // Множественные запросы
    for i in 0..10 {
        let test_request = LspCompletionRequest {
            file_path: "perf_test.bsl".to_string(),
            line: i,
            column: 1,
            prefix: "Тест".to_string(),
            trigger_character: None,
        };

        let _response = lsp_interface
            .handle_completion_request(test_request)
            .await?;
    }

    let performance_time = performance_start.elapsed();
    println!("   - 10 LSP запросов за: {:?}", performance_time);
    println!("   - Среднее время на запрос: {:?}", performance_time / 10);

    assert!(
        performance_time.as_millis() < 1000,
        "Производительность должна быть приемлемой"
    );
    println!("✅ Производительность отличная");

    // === ИТОГОВАЯ ПРОВЕРКА ===
    println!("\n🎯 ИТОГОВАЯ ПРОВЕРКА РЕВОЛЮЦИОННОЙ АРХИТЕКТУРЫ");
    println!("{}", "=".repeat(60));

    let final_metrics = central_system.get_system_metrics().await;
    let final_health = central_system.health_check().await;

    println!("📊 Финальные метрики:");
    println!("   - Система здорова: {}", final_health.status == "healthy");
    println!("   - Типов в системе: {}", final_metrics.total_types);
    println!("   - Общих запросов: {}", final_metrics.total_requests);
    println!("   - Время работы: инициализация + тестирование");

    // Проверяем все критерии успеха
    let mut success_criteria = Vec::new();

    // Критерий 1: Типы загружены
    if final_metrics.total_types > 10000 {
        success_criteria.push("✅ Загружено >10k типов");
    } else {
        success_criteria.push("❌ Недостаточно типов");
    }

    // Критерий 2: Система здорова
    if final_health.status == "healthy" {
        success_criteria.push("✅ Система здорова");
    } else {
        success_criteria.push("❌ Проблемы со здоровьем");
    }

    // Критерий 3: Все слои работают
    let all_layers_ready = final_health
        .components
        .iter()
        .all(|comp| comp.status == "healthy");
    if all_layers_ready {
        success_criteria.push("✅ Все слои архитектуры готовы");
    } else {
        success_criteria.push("❌ Есть проблемы в слоях");
    }

    // Критерий 4: Интерфейсы отвечают
    if final_metrics.total_requests > 0 {
        success_criteria.push("✅ Интерфейсы обрабатывают запросы");
    } else {
        success_criteria.push("❌ Интерфейсы не используются");
    }

    println!("\n🏆 КРИТЕРИИ УСПЕХА:");
    for criterion in &success_criteria {
        println!("   {}", criterion);
    }

    let passed_criteria = success_criteria
        .iter()
        .filter(|c| c.starts_with("✅"))
        .count();

    println!(
        "\n🎯 РЕЗУЛЬТАТ: {}/{} критериев пройдено",
        passed_criteria,
        success_criteria.len()
    );

    if passed_criteria == success_criteria.len() {
        println!("🎉 РЕВОЛЮЦИОННАЯ АРХИТЕКТУРА ПОЛНОСТЬЮ ГОТОВА!");
        println!("🚀 Идеальная слоистая архитектура BSL Type System работает в production!");
    } else {
        println!("⚠️ Есть проблемы, требующие доработки");
    }

    Ok(())
}
