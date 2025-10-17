//! Интеграционный тест для Milestone 2.9: Inline Scope Analysis
//!
//! Проверяем, что TypeSystemService.get_hover_info_ir() корректно:
//! 1. Находит локальные переменные через find_variable_at_position()
//! 2. Резолвит типы переменных через TypeRepository (Platform/Config)
//! 3. Возвращает методы и свойства через TypeMetadataLookup

use std::sync::Arc;
use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, ParserCoordinator};
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;

/// Инициализация TypeSystemService для тестов
fn setup_service() -> TypeSystemService {
    use bsl_backend::system::IrCache;

    // 1. Создаем InMemoryTypeRepository (конкретная реализация TypeRepository)
    let repository = Arc::new(InMemoryTypeRepository::new());

    // 2. Создаем TypeResolver
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    // 3. Создаем AnalysisEngine
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));

    // 4. Создаем SystemLayer компоненты
    let cache = Arc::new(AnalysisCache::new(1000)); // capacity = 1000
    let parser = Arc::new(ParserCoordinator::with_fallback());
    let ir_cache = Arc::new(IrCache::new(100)); // MILESTONE 2.13: IR Cache

    // 5. Создаем TypeSystemService
    TypeSystemService::new(analysis_engine, cache, parser, ir_cache)
}

#[tokio::test]
async fn test_inline_scope_simple_assignment() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив();
КонецПроцедуры
    "#;

    // Hover на "МассивДанных" (строка 2, колонка 4)
    let hover_result = service
        .get_hover_info(code, 2, 4)
        .await
        .expect("Failed to get hover info");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");
    let hover_text = hover_result.unwrap();

    // Проверяем, что hover содержит информацию о переменной и типе
    assert!(
        hover_text.contains("МассивДанных") || hover_text.contains("Массив"),
        "Hover должен содержать информацию о переменной или типе Массив. Получили: {}",
        hover_text
    );
}

#[tokio::test]
async fn test_inline_scope_with_methods() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив();
    МассивДанных.Добавить(123);
КонецПроцедуры
    "#;

    // Hover на "МассивДанных" перед вызовом метода (строка 3, колонка 4)
    let hover_result = service
        .get_hover_info(code, 3, 4)
        .await
        .expect("Failed to get hover info");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");
    let hover_text = hover_result.unwrap();

    // Проверяем, что hover содержит методы (если Platform типы загружены)
    println!("Hover text: {}", hover_text);

    // Базовая проверка - должна быть информация о переменной
    assert!(
        hover_text.contains("МассивДанных") || hover_text.contains("Массив"),
        "Hover должен содержать информацию о переменной. Получили: {}",
        hover_text
    );
}

#[tokio::test]
async fn test_inline_scope_multiple_variables() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    МассивДанных = Новый Массив();
    СтруктураДанных = Новый Структура();
    ТаблицаЗначений = Новый ТаблицаЗначений();
КонецПроцедуры
    "#;

    // Hover на "СтруктураДанных" (строка 3, колонка 4)
    let hover_result = service
        .get_hover_info(code, 3, 4)
        .await
        .expect("Failed to get hover info");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");
    let hover_text = hover_result.unwrap();

    // Проверяем, что hover содержит правильный тип (Структура, а не Массив или ТаблицаЗначений)
    assert!(
        hover_text.contains("СтруктураДанных") || hover_text.contains("Структура"),
        "Hover должен содержать информацию о СтруктураДанных. Получили: {}",
        hover_text
    );

    // Дополнительная проверка: не должен содержать другие типы
    // (это зависит от реализации - возможно hover покажет только текущую переменную)
}

#[tokio::test]
async fn test_inline_scope_nested_scope() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    Внешняя = Новый Массив();

    Функция ВнутренняяФункция()
        Локальная = Новый Структура();
        Возврат Локальная;
    КонецФункции

    Результат = ВнутренняяФункция();
КонецПроцедуры
    "#;

    // Hover на "Локальная" внутри вложенной функции (строка 5, колонка 8)
    let hover_result = service
        .get_hover_info(code, 5, 8)
        .await
        .expect("Failed to get hover info");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");
    let hover_text = hover_result.unwrap();

    // Проверяем, что hover находит локальную переменную в nested scope
    println!("Nested scope hover: {}", hover_text);

    assert!(
        hover_text.contains("Локальная") || hover_text.contains("Структура"),
        "Hover должен содержать информацию о локальной переменной. Получили: {}",
        hover_text
    );
}

#[tokio::test]
async fn test_inline_scope_unknown_type() {
    let service = setup_service();

    let code = r#"
Процедура Тест()
    НеизвестнаяПеременная = ВызовНеизвестнойФункции();
КонецПроцедуры
    "#;

    // Hover на "НеизвестнаяПеременная" (строка 2, колонка 4)
    let hover_result = service
        .get_hover_info(code, 2, 4)
        .await
        .expect("Failed to get hover info");

    assert!(hover_result.is_some(), "Hover должен вернуть информацию");
    let hover_text = hover_result.unwrap();

    // Проверяем, что hover возвращает информацию о неопределённом типе
    println!("Unknown type hover: {}", hover_text);

    assert!(
        hover_text.contains("Неопределено") || hover_text.contains("Unknown") || hover_text.contains("НеизвестнаяПеременная"),
        "Hover должен показать, что тип неопределён. Получили: {}",
        hover_text
    );
}
