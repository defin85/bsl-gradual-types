/// Тест для проблемы #1: Новый ТаблицаЗначений vs Новый ТаблицаЗначений()
///
/// В 1С допустимы ОБА синтаксиса для создания объектов:
/// - `Новый ТаблицаЗначений` (без скобок)
/// - `Новый ТаблицаЗначений()` (со скобками)
///
/// Проверяем что оба варианта корректно парсятся и сохраняют TypeHint.

use bsl_backend::application::TypeSystemService;
use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::progress::ProgressUpdate;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_backend::system::{AnalysisCache, IrCache, ParserCoordinator};
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::TypeResolver;
use std::sync::Arc;

/// Helper: создать TypeSystemService для тестов WITH PLATFORM TYPES LOADED
fn create_test_service() -> TypeSystemService {
    // 1. Парсим синтаксис-помощник
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    // 2. Создаём репозиторий и загружаем типы
    let repository = Arc::new(InMemoryTypeRepository::new())
        as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    repository
        .load_types(parsed_types)
        .expect("Failed to load types");

    // 3. Создаём остальные компоненты
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let analysis_engine = Arc::new(AnalysisEngine::new(resolver, repository.clone()));
    let cache = Arc::new(AnalysisCache::new(100));
    let ir_cache = Arc::new(IrCache::new(50));
    let parser = Arc::new(ParserCoordinator::new(repository.clone()));

    let service = TypeSystemService::new(analysis_engine, cache, parser, ir_cache);

    // 4. Инициализируем сервис
    service
        .initialize()
        .expect("Failed to initialize TypeSystemService");

    service
}

#[tokio::test]
async fn test_constructor_without_parentheses() {
    // Arrange: создаём TypeSystemService с платформенными типами
    let service = create_test_service();

    // Act: парсим код БЕЗ скобок
    let code = r#"
Функция Тест()
    ТЗ = Новый ТаблицаЗначений;
    Возврат ТЗ.Количество();
КонецФункции
"#;

    let parse_result = service.parse_and_validate(code).unwrap();
    assert!(
        parse_result.is_empty(),
        "Не должно быть syntax errors для 'Новый ТаблицаЗначений'"
    );

    // Проверяем что semantic validation НЕ выдаёт ошибку
    let semantic_errors = service.validate_semantics(code, None).await.unwrap();

    // DEBUG: выводим ошибки если есть
    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprintln!("ERROR: {:?}", error);
        }
    }

    assert!(
        semantic_errors.is_empty(),
        "❌ Метод 'Количество' должен быть найден для 'ТаблицаЗначений'"
    );
}

#[tokio::test]
async fn test_constructor_with_parentheses() {
    // Arrange
    let service = create_test_service();

    // Act: парсим код СО скобками
    let code = r#"
Функция Тест()
    ТЗ = Новый ТаблицаЗначений();
    Возврат ТЗ.Количество();
КонецФункции
"#;

    let parse_result = service.parse_and_validate(code).unwrap();
    assert!(
        parse_result.is_empty(),
        "Не должно быть syntax errors для 'Новый ТаблицаЗначений()'"
    );

    // Проверяем что semantic validation НЕ выдаёт ошибку
    let semantic_errors = service.validate_semantics(code, None).await.unwrap();

    // DEBUG: выводим ошибки если есть
    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprintln!("ERROR: {:?}", error);
        }
    }

    assert!(
        semantic_errors.is_empty(),
        "✅ Метод 'Количество' должен быть найден для 'ТаблицаЗначений()'"
    );
}

#[tokio::test]
async fn test_both_constructors_in_one_function() {
    // Arrange
    let service = create_test_service();

    // Act: парсим код с ОБОИМИ вариантами
    let code = r#"
Функция Тест()
    ТЗ1 = Новый ТаблицаЗначений;     // БЕЗ скобок
    ТЗ2 = Новый ТаблицаЗначений();   // СО скобками

    Кол1 = ТЗ1.Количество();
    Кол2 = ТЗ2.Количество();

    Возврат Кол1 + Кол2;
КонецФункции
"#;

    let parse_result = service.parse_and_validate(code).unwrap();
    assert!(parse_result.is_empty(), "Не должно быть syntax errors");

    let semantic_errors = service.validate_semantics(code, None).await.unwrap();

    // DEBUG: выводим ошибки если есть
    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprintln!("ERROR: {:?}", error);
        }
    }

    assert!(
        semantic_errors.is_empty(),
        "❌ Оба конструктора должны работать одинаково!"
    );
}
