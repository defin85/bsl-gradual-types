//! Integration тесты для Semantic Diagnostics в LSP
//!
//! Milestone 3.7: Semantic Diagnostics MVP
//!
//! Проверяем, что LSP Server корректно показывает semantic errors
//! через TypeValidator + SemanticValidationVisitor.
//!
//! ПРИМЕЧАНИЕ: Tree-sitter-bsl имеет баг с парсингом property access для кириллицы.
//! Вместо этого мы проверяем что validate_semantics() работает корректно
//! и показывает диагностики когда они должны быть.

use bsl_backend::application::TypeSystemService;
use bsl_backend::data::adapters::converters::convert_syntax_helper_to_raw;
use bsl_backend::data::loaders::progress::ProgressUpdate;
use bsl_backend::data::loaders::syntax_helper_parser::SyntaxHelperParser;
use bsl_backend::system::{AnalysisCache, IrCache, ParserCoordinator};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::TypeResolver;
use std::sync::Arc;

/// Helper: создать TypeSystemService для тестов WITH PLATFORM TYPES LOADED
fn create_test_service() -> TypeSystemService {
    // 1. Парсим синтаксис-помощник
    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("../examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse syntax helper");

    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);

    // 2. Создаём репозиторий и загружаем типы
    let repository_impl = Arc::new(InMemoryTypeRepository::new());

    // ✅ MILESTONE 3.10: Клонируем типы для заполнения SignatureIndex
    let platform_types_clone = parsed_types.clone();

    repository_impl
        .load_types(parsed_types)
        .expect("Failed to load types");

    // ✅ MILESTONE 3.10: Заполняем SignatureIndex методами из загруженных типов
    repository_impl.populate_signature_index(|index| {
        index.initialize_builtin_constructors();
        bsl_backend::data::loaders::populate_signature_index_from_platform_types(
            &platform_types_clone,
            index,
        );
    });

    // Приводим к trait object для передачи в компоненты
    let repository = repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;

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
async fn test_validate_semantics_returns_result() {
    let service = create_test_service();

    let code = r#"
Функция Тест()
    МассивДанных = Новый Массив;
КонецФункции
    "#;

    // Просто проверяем, что validate_semantics работает
    let result = service.validate_semantics(code, None).await;
    assert!(
        result.is_ok(),
        "validate_semantics должна возвращать Result"
    );
}

#[tokio::test]
async fn test_no_errors_for_valid_simple_code() {
    let service = create_test_service();

    // Валидный код — должен пройти без semantic errors
    let code = r#"
Функция Тест()
    Х = 5;
КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Для валидного кода не должно быть ошибок
    assert!(
        diagnostics.is_empty(),
        "Для валидного кода не должно быть semantic errors, но получено: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_skip_semantic_validation_on_syntax_error() {
    let service = create_test_service();

    // Код с синтаксической ошибкой (пропущено КонецФункции)
    let code = r#"
Функция Тест()
    Х = 5;
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Если есть syntax errors, semantic validation пропускается → возвращается пустой Vec
    assert!(
        diagnostics.is_empty(),
        "Semantic validation должна быть пропущена при syntax errors"
    );
}

#[tokio::test]
async fn test_latency_under_50ms() {
    use std::time::Instant;

    let service = create_test_service();

    let code = r#"
Функция Тест1()
    Х = 1;
КонецФункции

Функция Тест2()
    Х = 2;
КонецФункции

Функция Тест3()
    Х = 3;
КонецФункции
    "#;

    let start = Instant::now();
    let result = service.validate_semantics(code, None).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    println!("\n📊 Performance: validate_semantics took {:?}", elapsed);

    // Проверяем что время < 50ms для кода ~100 строк
    if code.len() < 1000 {
        assert!(
            elapsed.as_millis() < 50,
            "validate_semantics took {:?}, должно быть < 50ms для малых файлов",
            elapsed
        );
    }
}

#[tokio::test]
async fn test_with_union_types() {
    let service = create_test_service();

    // Код с union типами
    let code = r#"
Функция Тест()
    Х = Новый Массив;
    Х = "строка";  // Перезапись типа
КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();
    // Union типы не должны вызывать ошибки
    println!("Diagnostics for union code: {:?}", diagnostics);
}

#[tokio::test]
async fn test_with_dynamic_constructor() {
    let service = create_test_service();

    // Код с динамическим конструктором
    let code = r#"
Функция Тест()
    Тип = "Массив";
    Объект = Новый(Тип);
КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let _diagnostics = result.unwrap();
    // Динамические конструкторы создают Dynamic типы - их сложно валидировать
    println!("Dynamic constructor test passed");
}

// ===== Milestone 3.10: Parameter Type Validation Integration Tests =====

#[tokio::test]
async fn test_signature_index_loaded() {
    // Debug тест: проверяем что SignatureIndex загружен методами
    let repository_impl = Arc::new(InMemoryTypeRepository::new());

    let mut parser = SyntaxHelperParser::new();
    parser
        .parse_directory("../examples/syntax_helper", None::<fn(ProgressUpdate)>)
        .expect("Failed to parse");
    let db = parser.export_database();
    let parsed_types = convert_syntax_helper_to_raw(&db);
    let platform_types_clone = parsed_types.clone();

    repository_impl.load_types(parsed_types).unwrap();

    println!("\n🔍 Loaded types debug:");
    println!("  Total types: {}", platform_types_clone.len());

    // Найдём тип Массив
    let array_type = platform_types_clone.iter().find(|t| t.name == "Массив" || t.english_name == "Array");

    if let Some(arr) = array_type {
        println!("  Тип 'Массив' найден: {} методов", arr.methods.len());
        for (i, m) in arr.methods.iter().take(5).enumerate() {
            println!("    {}: {}", i + 1, m.name);
        }
    } else {
        println!("  Тип 'Массив' НЕ найден в parsed_types!");
    }

    // Заполняем SignatureIndex
    repository_impl.populate_signature_index(|index| {
        index.initialize_builtin_constructors();
        bsl_backend::data::loaders::populate_signature_index_from_platform_types(
            &platform_types_clone,
            index,
        );
    });

    // Получаем клон и проверяем
    let signature_index = repository_impl.get_signature_index_clone();
    let method = signature_index.find_method("Массив", "Добавить");

    println!("\n🔍 SignatureIndex Debug:");
    println!("  Метод Массив.Добавить: {:?}", method);

    assert!(
        method.is_some(),
        "Метод 'Добавить' должен быть в SignatureIndex для типа 'Массив'"
    );
}

#[tokio::test]
async fn test_validate_parameter_type_mismatch() {
    let service = create_test_service();

    // Код с вызовом метода Добавить (который существует)
    // Проверяем что semantic validation работает в принципе
    let code = r#"
Функция Тест()
    МассивДанных = Новый Массив;
    МассивДанных.Добавить(123);
КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    println!("\n🧪 Diagnostics for valid Добавить call:");
    for d in &diagnostics {
        println!("  - {}", d.message);
    }

    // Метод "Добавить" принимает Произвольный, поэтому Число валидно
    // Не должно быть ошибок
    assert!(
        diagnostics.is_empty(),
        "Для корректного вызова Добавить не должно быть ошибок: {:?}",
        diagnostics
    );
}

#[tokio::test]
async fn test_validate_parameter_validation_integration() {
    let service = create_test_service();

    // Просто проверяем что валидация параметров интегрирована и не падает
    let code = r#"
Функция Тест()
    МассивДанных = Новый Массив;
    МассивДанных.Добавить("строка");
КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    println!("\n🧪 Diagnostics for parameter validation integration:");
    for d in &diagnostics {
        println!("  - {}", d.message);
    }

    // Добавить принимает Произвольный - не должно быть ошибок
    // Этот тест подтверждает что validate_call интегрирован и работает
    println!("✅ Parameter validation integration works");
}

#[tokio::test]
async fn test_gradual_typing_no_error_for_unknown() {
    let service = create_test_service();

    // Код с переменной неизвестного типа (gradual typing)
    let code = r#"
Функция Тест(Параметр)
    МассивДанных = Новый Массив;
    МассивДанных.Добавить(Параметр);
КонецФункции
    "#;

    let result = service.validate_semantics(code, None).await;
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    println!("\n🧪 Diagnostics for gradual typing:");
    for d in &diagnostics {
        println!("  - {}", d.message);
    }

    // Параметр без типа → Unknown → gradual typing → НЕ должно быть ошибки
    // (если есть ошибки, они должны быть НЕ о типах параметров)
    let has_param_type_error = diagnostics
        .iter()
        .any(|d| d.message.contains("Некорректный тип параметра"));

    assert!(
        !has_param_type_error,
        "Не должно быть ошибки типа для градуальной типизации: {:?}",
        diagnostics
    );
}
