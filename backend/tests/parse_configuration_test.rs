//! Интеграционные тесты для Milestone 2.17: Configuration Parser
//!
//! Тестирует загрузку конфигурационных типов через TypeSystemService

use bsl_backend::system::SystemCoordinator;
use std::path::Path;

#[tokio::test]
async fn test_load_configuration_types_success() {
    // Arrange: Создаем SystemCoordinator и TypeSystemService
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService should be available");

    let config_path = Path::new("../examples/conf/conf_test");

    // Пропускаем тест если конфигурация отсутствует
    if !config_path.exists() {
        eprintln!(
            "⚠️ Skipping test: test configuration not found at {:?}",
            config_path
        );
        return;
    }

    // Act: Загружаем типы из конфигурации
    let result = type_service.load_configuration_types(config_path);

    // Assert: Проверяем успешную загрузку
    assert!(
        result.is_ok(),
        "Failed to load configuration types: {:?}",
        result.err()
    );

    let loaded_count = result.unwrap();
    assert!(
        loaded_count >= 4,
        "Expected at least 4 types (2 catalogs, 1 document, 1 register), got {}",
        loaded_count
    );

    println!(
        "✅ Successfully loaded {} configuration types",
        loaded_count
    );
}

#[tokio::test]
async fn test_load_configuration_types_missing_path() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService should be available");

    let config_path = Path::new("/nonexistent/path");

    // Act
    let result = type_service.load_configuration_types(config_path);

    // Assert: Ошибка при отсутствующем пути
    assert!(result.is_err(), "Should fail with nonexistent path");

    let error = result.unwrap_err();
    let error_msg = error.to_string();

    // ✅ ИСПРАВЛЕНО (2025-01-18): Гибкая проверка для Windows/Linux
    // На Windows /nonexistent/path может резолвиться как C:\nonexistent\path
    // и проходить exists(), но падать на Configuration.xml
    let is_valid_error = error_msg.contains("does not exist")
        || error_msg.contains("Configuration.xml not found")
        || error_msg.contains("is not a directory");

    assert!(
        is_valid_error,
        "Error message should mention missing path or Configuration.xml, got: {}",
        error_msg
    );

    println!("✅ Correctly rejected nonexistent path: {}", error_msg);
}

#[tokio::test]
async fn test_load_configuration_types_missing_xml() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService should be available");

    // Используем существующую директорию, но без Configuration.xml
    let config_path = Path::new("../examples");

    if !config_path.exists() {
        eprintln!("⚠️ Skipping test: examples directory not found");
        return;
    }

    // Act
    let result = type_service.load_configuration_types(config_path);

    // Assert: Ошибка при отсутствии Configuration.xml
    assert!(result.is_err(), "Should fail without Configuration.xml");

    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("Configuration.xml not found"),
        "Error message should mention missing Configuration.xml, got: {}",
        error_msg
    );

    println!("✅ Correctly rejected directory without Configuration.xml");
}

#[tokio::test]
async fn test_load_configuration_types_batch_processing() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService should be available");

    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Skipping test: test configuration not found");
        return;
    }

    // Act: Загружаем типы (batch size = 100 в TypeSystemService)
    let result = type_service.load_configuration_types(config_path);

    // Assert: Проверяем что batch loading работает корректно
    assert!(result.is_ok(), "Batch loading failed: {:?}", result.err());

    let loaded_count = result.unwrap();
    assert!(loaded_count > 0, "Should load at least one type");

    // Проверяем что типы действительно загружены в TypeRepository
    let engine = coordinator
        .get_analysis_engine()
        .expect("AnalysisEngine should be available");

    let repo = engine.get_repository();

    // Ищем один из известных типов из test configuration
    let catalog_type = repo.find_type("Справочники.Контрагенты");
    assert!(
        catalog_type.is_some(),
        "Catalog type 'Справочники.Контрагенты' should be loaded into repository"
    );

    println!(
        "✅ Batch processing works correctly, {} types loaded and accessible",
        loaded_count
    );
}

#[tokio::test]
async fn test_load_configuration_types_integration_with_hover() {
    // Arrange: Проверяем интеграцию с hover после загрузки типов
    let coordinator = SystemCoordinator::new();
    coordinator.start().await.unwrap();

    let type_service = coordinator
        .type_service()
        .expect("TypeSystemService should be available");

    let config_path = Path::new("../examples/conf/conf_test");

    if !config_path.exists() {
        eprintln!("⚠️ Skipping test: test configuration not found");
        return;
    }

    // Act: Загружаем типы
    let _ = type_service.load_configuration_types(config_path).unwrap();

    // Проверяем что типы доступны через hover API
    let test_code = r#"
Процедура Тест()
    СправочникКонтрагенты = Справочники.Контрагенты;
КонецПроцедуры
"#;

    let hover_result = type_service
        .get_hover_info(test_code, 2, 30)
        .await;

    // Assert: Hover должен найти информацию о типе Справочники.Контрагенты
    assert!(
        hover_result.is_ok(),
        "Hover failed: {:?}",
        hover_result.err()
    );

    let hover_text_opt = hover_result.unwrap();

    if let Some(hover_text) = hover_text_opt {
        assert!(
            hover_text.contains("Справочники.Контрагенты") || hover_text.contains("Контрагенты"),
            "Hover should contain information about catalog type, got: {}",
            hover_text
        );

        println!("✅ Integration with hover works correctly");
        println!("Hover text: {}", hover_text);
    } else {
        panic!("Hover returned None, expected Some(hover_text)");
    }
}
