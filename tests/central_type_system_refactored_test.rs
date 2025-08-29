use bsl_gradual_types::system::{
    CacheSettings, CentralSystemConfig, CentralTypeSystem, PerformanceSettings,
};
use std::sync::Arc;

#[tokio::test]
async fn test_central_type_system_refactored() {
    // Создаём конфигурацию для CentralTypeSystem
    let config = CentralSystemConfig {
        html_path: "examples/syntax_helper/rebuilt.shcntx_ru".to_string(),
        configuration_path: None,
        verbose_logging: false,
        cache_settings: CacheSettings {
            enable_repository_cache: true,
            enable_resolution_cache: true,
            enable_lsp_cache: true,
            cache_ttl_seconds: 3600,
            max_cache_size: 10000,
        },
        performance_settings: PerformanceSettings {
            enable_parallel_parsing: false,
            max_parser_threads: 1,
            lsp_response_timeout_ms: 5000,
            web_request_timeout_ms: 30000,
        },
    };

    // Создаём и инициализируем CentralTypeSystem
    let system = CentralTypeSystem::new(config);
    system
        .initialize()
        .await
        .expect("Failed to initialize CentralTypeSystem");

    // Тест 1: get_all_types_with_resolutions (теперь через resolution_service)
    let all_types = system.get_all_types_with_resolutions().await;
    println!("All types count: {}", all_types.len());
    assert!(all_types.len() > 0, "Should have types");

    // Тест 2: search_types (теперь через resolution_service)
    let search_results = system.search_types("Справочники").await;
    println!("Search results for 'Справочники': {:?}", search_results);
    assert!(search_results.len() > 0, "Should have search results");

    // Тест 3: resolve_expression (теперь через resolution_service)
    let resolution = system.resolve_expression("Справочники").await;
    println!("Resolution for 'Справочники': {:?}", resolution);
    assert!(
        !matches!(
            resolution.certainty,
            bsl_gradual_types::domain::types::Certainty::Unknown
        ),
        "Should resolve 'Справочники' successfully"
    );

    // Тест 4: get_type_info (теперь через resolution_service)
    if let Some(type_info) = system.get_type_info("Справочники").await {
        println!("Type info for 'Справочники': {:?}", type_info);
        assert_eq!(type_info.name, "Справочники");
    } else {
        panic!("Should get type info for 'Справочники'");
    }

    // Тест 5: get_variable_type (теперь через resolution_service)
    let var_type = system.get_variable_type("Справочники", "").await;
    println!("Variable type for 'Справочники': {:?}", var_type);
    assert!(
        !matches!(
            var_type.certainty,
            bsl_gradual_types::domain::types::Certainty::Unknown
        ),
        "Should resolve variable type successfully"
    );

    // Тест 6: get_all_types
    let all_type_names = system.get_all_types().await;
    println!("All type names count: {}", all_type_names.len());
    assert!(all_type_names.len() > 0, "Should have type names");
    assert!(
        all_type_names.contains(&"Справочники".to_string()),
        "Should contain 'Справочники'"
    );

    println!("✅ Все тесты рефакторинга CentralTypeSystem прошли успешно!");
}
