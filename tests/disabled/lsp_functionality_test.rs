//! Тест функциональности LSP Server
//!
//! Проверяет работу hover, completion и диагностик

use bsl_gradual_types::system::SystemCoordinator;

#[tokio::test]
async fn test_lsp_hover_functionality() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    let type_service = coordinator.type_service();

    let bsl_content = r#"
Функция ТестоваяФункция()
    Переменная = "Привет мир";
    Возврат Переменная;
КонецФункции
"#;

    // Act - запрашиваем hover для строки с переменной (строка 2, позиция 10)
    let hover_result = type_service.get_hover_info(bsl_content, 2, 10, None).await;

    // Assert
    assert!(hover_result.is_ok(), "Hover должен работать без ошибок");

    let hover_info = hover_result.unwrap();
    assert!(hover_info.is_some(), "Hover должен возвращать информацию");

    let info = hover_info.unwrap();
    println!("🔍 Полученная hover info: '{}'", info);
    // Проверяем что есть информация о символе
    assert!(
        info.contains("Переменная") || info.contains("Строка") || info.contains("BSL символ"),
        "Hover должен содержать информацию о символе"
    );

    println!("✅ Hover работает: {}", info);
}

#[tokio::test]
async fn test_lsp_completion_functionality() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    let type_service = coordinator.type_service();

    let bsl_content = r#"
Функция НоваяФункция()
    // здесь будем запрашивать автодополнение
"#;

    // Act - запрашиваем completion для строки 2, позиция 4
    let completion_result = type_service.get_completion(bsl_content, 2, 4).await;

    // Assert
    assert!(
        completion_result.is_ok(),
        "Completion должен работать без ошибок"
    );

    let completions = completion_result.unwrap();
    assert!(
        !completions.is_empty(),
        "Должны быть доступны варианты автодополнения"
    );

    // Проверяем что есть базовые BSL конструкции
    let labels: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    assert!(
        labels.contains(&"Функция".to_string()),
        "Должна быть доступна конструкция 'Функция'"
    );
    assert!(
        labels.contains(&"Если".to_string()),
        "Должна быть доступна конструкция 'Если'"
    );
    assert!(
        labels.contains(&"Строка".to_string()),
        "Должен быть доступен тип 'Строка'"
    );

    println!(
        "✅ Completion работает, доступно {} вариантов:",
        completions.len()
    );
    for completion in &completions[..5] {
        // Показываем первые 5
        println!("  - {}: {:?}", completion.label, completion.detail);
    }
}

#[tokio::test]
async fn test_lsp_file_analysis() {
    // Arrange
    let coordinator = SystemCoordinator::new();
    let type_service = coordinator.type_service();

    // Создаём временный BSL файл
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(
        temp_file,
        r#"
Функция ПримерФункции(Параметр1, Параметр2)
    Результат = Параметр1 + Параметр2;
    Возврат Результат;
КонецФункции

Процедура ПримерПроцедуры()
    Сообщить("Привет из BSL!");
КонецПроцедуры
"#
    )
    .expect("Failed to write to temp file");

    let file_path = temp_file.path().to_string_lossy().to_string();

    // Act - анализируем файл
    let analysis_result = type_service.analyze_file(&file_path).await;

    // Assert
    assert!(
        analysis_result.is_ok(),
        "Анализ файла должен пройти успешно"
    );

    let analysis = analysis_result.unwrap();
    assert_eq!(analysis.file_path, file_path);

    println!("✅ Анализ файла успешно: {}", analysis.file_path);
}

#[test]
fn test_completion_items_structure() {
    // Тест структуры элементов автодополнения
    use bsl_gradual_types::application::CompletionItem;

    let item = CompletionItem {
        label: "ТестоваяФункция".to_string(),
        detail: Some("Пользовательская функция".to_string()),
        insert_text: Some("ТестоваяФункция(${1:параметры})".to_string()),
        documentation: None,
        filter_text: None,
        kind: bsl_gradual_types::domain::CompletionKind::Function,
        sort_text: None,
    };

    assert_eq!(item.label, "ТестоваяФункция");
    assert_eq!(item.detail, Some("Пользовательская функция".to_string()));
    assert!(item.insert_text.unwrap().contains("${1:параметры}"));

    println!("✅ Структура CompletionItem работает корректно");
}
