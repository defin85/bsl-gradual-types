//! Unit-тесты для TypeSystemService (Phase 5)
//!
//! Тестирование новых методов:
//! - get_all_types_as_dto() - DTO конверсия для Web API
//! - get_metrics_summary() - подсчет метрик типов

use bsl_backend::system::SystemCoordinator;

#[tokio::test]
async fn test_get_all_types_as_dto_basic() {
    // Arrange: инициализируем систему
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");

    let type_service = coordinator.type_service()
        .expect("TypeSystemService should be available");

    // Act: получаем все типы с пагинацией
    let limit = 10;
    let offset = 0;
    let result = type_service.get_all_types_as_dto(limit, offset, None, None, false);

    // Assert: проверяем структуру результата
    assert!(!result.types.is_empty(), "Should return some types");
    assert!(result.types.len() <= limit, "Should respect limit");

    // Проверяем наличие метрик
    assert!(result.metrics.total_types > 0, "Should have total_types metric");

    // Проверяем пагинацию
    assert!(result.pagination.is_some(), "Should have pagination info");
    let pagination = result.pagination.unwrap();
    assert_eq!(pagination.page_size, limit);
    assert_eq!(pagination.current_page, 1);
}

#[tokio::test]
async fn test_get_all_types_as_dto_pagination() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act: получаем первую страницу
    let page1 = type_service.get_all_types_as_dto(5, 0, None, None, false);

    // Act: получаем вторую страницу
    let page2 = type_service.get_all_types_as_dto(5, 5, None, None, false);

    // Assert: страницы должны быть разными
    if page1.types.len() == 5 && page2.types.len() > 0 {
        let first_page_names: Vec<_> = page1.types.iter().map(|t| &t.name).collect();
        let second_page_names: Vec<_> = page2.types.iter().map(|t| &t.name).collect();

        // Первые элементы не должны совпадать
        assert_ne!(first_page_names, second_page_names, "Pages should have different types");
    }

    // Проверяем информацию о пагинации
    let pag1 = page1.pagination.unwrap();
    let pag2 = page2.pagination.unwrap();

    assert_eq!(pag1.current_page, 1);
    assert_eq!(pag2.current_page, 2);
}

#[tokio::test]
async fn test_get_all_types_as_dto_certainty_levels() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act: получаем типы
    let result = type_service.get_all_types_as_dto(50, 0, None, None, false);

    // Assert: проверяем что у типов есть certainty levels
    for type_dto in &result.types {
        assert!(type_dto.certainty <= 100, "Certainty should be 0-100%");
        assert!(!type_dto.certainty_text.is_empty(), "Should have certainty text");
    }

    // Проверяем метрики по certainty
    let metrics = &result.metrics;
    let total_categorized = metrics.certainty_high + metrics.certainty_medium + metrics.certainty_low;
    assert!(total_categorized > 0, "Should categorize types by certainty");
}

#[tokio::test]
async fn test_get_all_types_as_dto_categories() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act
    let result = type_service.get_all_types_as_dto(50, 0, None, None, false);

    // Assert: проверяем категории
    assert!(!result.categories.is_empty(), "Should have categories");

    for (category_name, category_dto) in &result.categories {
        assert!(!category_name.is_empty(), "Category name should not be empty");
        assert!(!category_dto.color.is_empty(), "Category should have color");
        assert!(!category_dto.icon.is_empty(), "Category should have icon");
        // count может быть любым неотрицательным числом (usize)
    }
}

#[tokio::test]
async fn test_get_metrics_summary_basic() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act: получаем метрики
    let metrics = type_service.get_metrics_summary();

    // Assert: проверяем структуру
    assert!(metrics.is_object(), "Metrics should be JSON object");

    let total = metrics.get("total_types").and_then(|v| v.as_u64());
    assert!(total.is_some(), "Should have total_types");
    assert!(total.unwrap() > 0, "Should have at least some types");

    let known = metrics.get("known_types").and_then(|v| v.as_u64());
    let inferred = metrics.get("inferred_types").and_then(|v| v.as_u64());
    let unknown = metrics.get("unknown_types").and_then(|v| v.as_u64());

    assert!(known.is_some(), "Should have known_types");
    assert!(inferred.is_some(), "Should have inferred_types");
    assert!(unknown.is_some(), "Should have unknown_types");
}

#[tokio::test]
async fn test_get_metrics_summary_consistency() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act
    let metrics = type_service.get_metrics_summary();

    // Assert: проверяем что сумма категорий = total
    let total = metrics.get("total_types").and_then(|v| v.as_u64()).unwrap();
    let known = metrics.get("known_types").and_then(|v| v.as_u64()).unwrap();
    let inferred = metrics.get("inferred_types").and_then(|v| v.as_u64()).unwrap();
    let unknown = metrics.get("unknown_types").and_then(|v| v.as_u64()).unwrap();

    assert_eq!(
        total,
        known + inferred + unknown,
        "Sum of categorized types should equal total"
    );
}

#[tokio::test]
async fn test_dto_type_fields_completeness() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act
    let result = type_service.get_all_types_as_dto(10, 0, None, None, false);

    // Assert: проверяем что все поля TypeDto заполнены
    for type_dto in &result.types {
        assert!(!type_dto.id.is_empty(), "ID should not be empty");
        assert!(!type_dto.name.is_empty(), "Name should not be empty");
        assert!(!type_dto.category.is_empty(), "Category should not be empty");
        assert!(!type_dto.source.is_empty(), "Source should not be empty");
        assert!(!type_dto.description.is_empty(), "Description should not be empty");
        // facets могут быть пустыми для некоторых типов
        // union_types опциональны
    }
}

#[tokio::test]
async fn test_large_pagination_request() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.expect("Should start");
    let type_service = coordinator.type_service().expect("Should have service");

    // Act: запрашиваем большое количество типов
    let result = type_service.get_all_types_as_dto(1000, 0, None, None, false);

    // Assert: должно вернуть не больше чем есть типов
    assert!(result.types.len() <= 1000, "Should not exceed limit");
    assert_eq!(
        result.types.len(),
        result.metrics.total_types.min(1000),
        "Should return min(total, limit)"
    );
}