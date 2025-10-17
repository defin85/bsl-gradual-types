//! Интеграционный тест для IR-based hover (Milestone 2.8: Task 5.4)
//!
//! Проверяет работу нового flow:
//! Code → Parser → IR → TypeResolution → Hover Info

use anyhow::Result;
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, ParserCoordinator};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::engine::AnalysisEngine;
use std::sync::Arc;

/// Создание тестового TypeSystemService
fn create_test_service() -> TypeSystemService {
    use bsl_backend::system::IrCache;

    let repo = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let engine = Arc::new(AnalysisEngine::new(resolver, repo));
    let cache = Arc::new(AnalysisCache::new(100));
    let parser = Arc::new(ParserCoordinator::with_fallback());
    let ir_cache = Arc::new(IrCache::new(100)); // MILESTONE 2.13: IR Cache

    TypeSystemService::new(engine, cache, parser, ir_cache)
}

#[tokio::test]
async fn test_ir_hover_variable_declaration() -> Result<()> {
    let service = create_test_service();

    let code = r#"
Перем x: Число;
Перем y: Строка;
"#;

    // Hover на "Число" (строка 1, после ":")
    let hover_result = service
        .get_hover_info(code, 1, 12)
        .await?;

    assert!(hover_result.is_some());
    let hover_text = hover_result.unwrap();

    println!("Hover result:\n{}", hover_text);

    // Проверяем что в hover есть информация о переменной или типе
    assert!(
        hover_text.contains("Переменная") || hover_text.contains("Объявление") || hover_text.contains("x"),
        "Hover должен содержать информацию о переменной или объявлении. Got: {}", hover_text
    );

    Ok(())
}

#[tokio::test]
async fn test_ir_hover_function_declaration() -> Result<()> {
    let service = create_test_service();

    let code = r#"
Функция ПолучитьСумму(a: Число, b: Число) Экспорт
    Возврат a + b;
КонецФункции
"#;

    // Hover на имени функции
    let hover_result = service
        .get_hover_info(code, 1, 12)
        .await?;

    assert!(hover_result.is_some());
    let hover_text = hover_result.unwrap();

    println!("Hover result:\n{}", hover_text);

    // Проверяем что в hover есть информация о функции
    assert!(
        hover_text.contains("Функция") || hover_text.contains("ПолучитьСумму"),
        "Hover должен содержать информацию о функции"
    );

    Ok(())
}

#[tokio::test]
async fn test_ir_hover_assignment() -> Result<()> {
    let service = create_test_service();

    let code = r#"
Перем результат;
результат = 42;
"#;

    // Hover на "результат" в присваивании
    let hover_result = service
        .get_hover_info(code, 2, 5)
        .await?;

    assert!(hover_result.is_some());
    let hover_text = hover_result.unwrap();

    println!("Hover result:\n{}", hover_text);

    // Проверяем что в hover есть информация о переменной
    assert!(
        hover_text.contains("результат") || hover_text.contains("Переменная") || hover_text.contains("Присваивание"),
        "Hover должен содержать информацию о переменной или присваивании"
    );

    Ok(())
}

#[tokio::test]
async fn test_ir_hover_platform_type() -> Result<()> {
    let service = create_test_service();

    let code = r#"
Перем массив: Массив;
"#;

    // Hover на "Массив"
    let hover_result = service
        .get_hover_info(code, 1, 16)
        .await?;

    assert!(hover_result.is_some());
    let hover_text = hover_result.unwrap();

    println!("Hover result:\n{}", hover_text);

    // Проверяем что в hover есть какая-то информация
    assert!(
        hover_text.contains("Переменная") || hover_text.contains("Объявление") || hover_text.contains("массив"),
        "Hover должен содержать информацию о переменной. Got: {}", hover_text
    );

    Ok(())
}

#[tokio::test]
async fn test_ir_hover_fallback_for_unknown() -> Result<()> {
    let service = create_test_service();

    let code = r#"
Перем неизвестная;
"#;

    // Hover на переменной без типа
    let hover_result = service
        .get_hover_info(code, 1, 8)
        .await?;

    assert!(hover_result.is_some());
    let hover_text = hover_result.unwrap();

    println!("Hover result:\n{}", hover_text);

    // Должно быть хоть что-то (даже если тип неизвестен)
    assert!(!hover_text.is_empty(), "Hover должен возвращать хоть какую-то информацию");

    Ok(())
}
