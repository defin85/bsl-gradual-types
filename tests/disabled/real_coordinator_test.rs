//! Простой тест для проверки РЕАЛЬНОЙ работы SystemCoordinator

use bsl_gradual_types::system::SystemCoordinator;
use std::fs;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_real_analyze_file() {
    // Создаём временный BSL файл
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let bsl_content = r#"
    // Простой BSL код для тестирования
    Процедура ТестоваяПроцедура()
        Сообщить("Привет мир");
    КонецПроцедуры
    "#;

    fs::write(temp_file.path(), bsl_content).expect("Failed to write to temp file");

    // Создаём SystemCoordinator
    let coordinator = SystemCoordinator::new();

    // РЕАЛЬНО вызываем analyze_file
    let result = coordinator
        .type_service()
        .analyze_file(temp_file.path().to_str().unwrap())
        .await;

    // Проверяем что получили результат
    match result {
        Ok(analysis) => {
            println!("✅ Анализ успешен для файла: {}", analysis.file_path);
            println!("📊 Найдено типов: {}", analysis.type_resolutions.len());
            assert_eq!(analysis.file_path, temp_file.path().to_str().unwrap());
        }
        Err(e) => {
            panic!("❌ Анализ файла провалился: {}", e);
        }
    }
}

#[test]
fn test_coordinator_creation() {
    // Простейший тест - может ли координатор создаться
    let coordinator = SystemCoordinator::new();
    let _service = coordinator.type_service();

    // Если дошли сюда без panic - уже хорошо
    println!("✅ SystemCoordinator создан успешно");
    println!("✅ Type system API получен успешно");
}
