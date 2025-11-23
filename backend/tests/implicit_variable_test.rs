//! Тест для неявно объявленных переменных
//! Проблема: переменные без `Перем` должны корректно получать тип через type inference

use bsl_backend::application::TypeSystemService;
use bsl_backend::helpers::hover_formatter::HoverFormatConfig;
use bsl_backend::system::{AnalysisCache, IrCache, ParserCoordinator};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::engine::AnalysisEngine;
use std::sync::Arc;

#[tokio::test]
async fn test_implicit_variable_type_inference() {
    // Тест кейс из файла test_hover_milestone_2_11.bsl
    let code = r#"
ТаблицаЗначенийТип = Новый ТаблицаЗначений;
Кол = ТаблицаЗначенийТип.Количество();
"#;

    // Создаём необходимые компоненты
    let repository = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));
    let cache = Arc::new(AnalysisCache::new(100));
    let ir_cache = Arc::new(IrCache::new(50));
    let parser = Arc::new(ParserCoordinator::new(repository.clone()));

    let service = TypeSystemService::new(analysis_engine, cache, parser, ir_cache);

    // Строка 3 (индекс 2), колонка 0 - переменная "Кол"
    // Помним что нумерация строк с 0 в коде
    let hover_config = HoverFormatConfig {
        show_certainty: true,
        ..Default::default()
    };

    let hover_info = service
        .get_hover_info(code, 2, 0, Some(hover_config))
        .await
        .unwrap();

    assert!(hover_info.is_some(), "Должна быть информация о hover для переменной Кол");

    let info = hover_info.unwrap();
    println!("Hover info: {}", info);

    // Проверяем, что тип НЕ "Неопределено"
    assert!(!info.contains("Тип: Неопределено"),
        "Переменная Кол не должна иметь тип Неопределено. Актуальная информация: {}", info);

    // Проверяем, что тип "Число" (результат метода Количество)
    assert!(info.contains("Число"),
        "Переменная Кол должна иметь тип Число (от метода Количество). Актуальная информация: {}", info);
}

#[tokio::test]
async fn test_explicit_vs_implicit_variables() {
    // Сравнение явно объявленных и неявных переменных
    let code = r#"
Перем ЯвнаяПеременная;
ЯвнаяПеременная = 42;

НеявнаяПеременная = 100;
"#;

    // Создаём необходимые компоненты
    let repository = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));
    let cache = Arc::new(AnalysisCache::new(100));
    let ir_cache = Arc::new(IrCache::new(50));
    let parser = Arc::new(ParserCoordinator::new(repository.clone()));

    let service = TypeSystemService::new(analysis_engine, cache, parser, ir_cache);

    // Проверка явной переменной (строка 2, колонка 0)
    let explicit_hover = service
        .get_hover_info(code, 2, 0, None)
        .await
        .unwrap();

    assert!(explicit_hover.is_some());
    let explicit_info = explicit_hover.unwrap();
    assert!(explicit_info.contains("Число"), "Явная переменная должна получить тип Число");

    // Проверка неявной переменной (строка 4, колонка 0)
    let implicit_hover = service
        .get_hover_info(code, 4, 0, None)
        .await
        .unwrap();

    assert!(implicit_hover.is_some());
    let implicit_info = implicit_hover.unwrap();
    assert!(implicit_info.contains("Число"),
        "Неявная переменная должна получить тип Число. Актуально: {}", implicit_info);
}

#[tokio::test]
async fn test_implicit_variable_with_method_chain() {
    // Более сложный случай с цепочкой вызовов методов
    let code = r#"
Массив = Новый Массив;
Результат = Массив.Количество();
СтроковыйРезультат = Результат.Строка();
"#;

    // Создаём необходимые компоненты
    let repository = Arc::new(InMemoryTypeRepository::new());
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));
    let cache = Arc::new(AnalysisCache::new(100));
    let ir_cache = Arc::new(IrCache::new(50));
    let parser = Arc::new(ParserCoordinator::new(repository.clone()));

    let service = TypeSystemService::new(analysis_engine, cache, parser, ir_cache);

    // Проверка переменной "Результат" (строка 2, колонка 0)
    let result_hover = service
        .get_hover_info(code, 2, 0, None)
        .await
        .unwrap();

    assert!(result_hover.is_some());
    let result_info = result_hover.unwrap();
    assert!(result_info.contains("Число"),
        "Переменная Результат должна иметь тип Число. Актуально: {}", result_info);
}