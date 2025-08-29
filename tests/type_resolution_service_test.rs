use bsl_gradual_types::domain::{InMemoryTypeRepository, TypeResolutionService};
use std::sync::Arc;

#[tokio::test]
async fn test_type_resolution_service_methods() {
    // Создаём TypeResolutionService как это делает CentralTypeSystem
    let repository: Arc<dyn bsl_gradual_types::domain::TypeRepository> =
        Arc::new(InMemoryTypeRepository::new());
    let resolution_service = Arc::new(TypeResolutionService::new(repository));

    // Инициализируем сервис (загружает PlatformTypeResolver)
    resolution_service
        .initialize()
        .await
        .expect("Failed to initialize TypeResolutionService");

    // Тест 1: get_all_platform_globals
    let platform_globals = resolution_service.get_all_platform_globals();
    println!("Platform globals count: {}", platform_globals.len());
    assert!(platform_globals.len() > 0, "Should have platform globals");

    // Тест 2: get_completions
    let completions = resolution_service.get_completions("Справ");
    println!("Completions for 'Справ': {}", completions.len());
    assert!(completions.len() > 0, "Should have completions");

    // Тест 3: search_types
    let search_results = resolution_service.search_types("Справочники");
    println!("Search results for 'Справочники': {:?}", search_results);
    assert!(search_results.len() > 0, "Should have search results");

    // Тест 4: resolve_expression_async
    let resolution = resolution_service
        .resolve_expression_async("Справочники")
        .await;
    println!("Resolution for 'Справочники': {:?}", resolution);
    assert!(
        !matches!(
            resolution.certainty,
            bsl_gradual_types::domain::types::Certainty::Unknown
        ),
        "Should resolve 'Справочники' successfully"
    );

    // Тест 5: get_type_info
    if let Some(type_info) = resolution_service.get_type_info("Справочники") {
        println!("Type info for 'Справочники': {:?}", type_info);
        assert_eq!(type_info.name, "Справочники");
    } else {
        panic!("Should get type info for 'Справочники'");
    }

    println!("✅ Все тесты TypeResolutionService прошли успешно!");
}
