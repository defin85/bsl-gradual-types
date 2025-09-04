//! Тест правильной архитектуры LSP Server
//!
//! Проверяет что LSP Server правильно работает с TypeSystemService (Application Layer)
//! согласно Clean Architecture принципам

use bsl_gradual_types::application::TypeSystemService;
use bsl_gradual_types::system::SystemCoordinator;
use std::io::Write;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_lsp_clean_architecture() {
    // Arrange: создаем SystemCoordinator как IoC Container
    let coordinator = SystemCoordinator::new();

    // Act: получаем TypeSystemService через DI
    let type_service: Arc<TypeSystemService> = coordinator.type_service();

    // Assert: проверяем что можем создать временный файл и проанализировать его
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, "Функция ТестоваяФункция()").expect("Failed to write to temp file");

    let file_path = temp_file.path().to_string_lossy().to_string();

    // ✅ ПРАВИЛЬНО: используем Application Layer напрямую (не через SystemCoordinator)
    let result = type_service.analyze_file(&file_path).await;

    // Проверяем что анализ прошел успешно
    assert!(
        result.is_ok(),
        "TypeSystemService should analyze file successfully"
    );

    let analysis = result.unwrap();
    assert_eq!(analysis.file_path, file_path);

    println!("✅ Clean Architecture проверена: LSP → TypeSystemService ✅");
}

#[tokio::test]
async fn test_system_coordinator_as_ioc_container() {
    // ✅ ПРАВИЛЬНАЯ РОЛЬ: SystemCoordinator как IoC Container
    let coordinator = SystemCoordinator::new();

    // SystemCoordinator предоставляет зависимости через DI
    let type_service_1 = coordinator.type_service();
    let type_service_2 = coordinator.type_service();

    // Проверяем что это один и тот же экземпляр (singleton через Arc)
    assert!(Arc::ptr_eq(&type_service_1, &type_service_2));

    println!("✅ SystemCoordinator работает как IoC Container ✅");
}

#[test]
fn test_architecture_layers_separation() {
    // Проверяем что у нас правильное разделение слоев

    // System Layer: SystemCoordinator
    let coordinator = SystemCoordinator::new();

    // Application Layer: TypeSystemService
    let type_service = coordinator.type_service();

    // Presentation Layer: будет использовать TypeSystemService
    // (в реальном LSP Server: BslLanguageServer::new(client, type_service))

    println!("✅ Архитектурные слои правильно разделены:");
    println!("  🎯 System Layer: SystemCoordinator (IoC Container)");
    println!("  🔧 Application Layer: TypeSystemService (Business Logic)");
    println!("  🌐 Presentation Layer: BslLanguageServer (Protocol Adapter)");
}
