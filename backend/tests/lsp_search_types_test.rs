//! Unit тесты для LSP Custom Command: bsl.searchTypes
//!
//! Проверяет интеграцию Quick Actions Webview с TypeRepository
//! через LSP Server для поиска типов.
//!
//! ЦЕЛЬ ТЕСТОВ:
//! - Проверить корректность фильтрации типов по query
//! - Проверить лимиты результатов
//! - Проверить поиск по русским/английским именам
//! - Проверить граничные случаи (пустой репозиторий, пустой query)
//! - Проверить production типы из Syntax Helper

use bsl_backend::system::SystemCoordinator;
use bsl_shared::domain::types::{FacetKind, RawTypeData};
use std::sync::Arc;

/// Вспомогательная структура для эмуляции handle_search_types логики
struct SearchTypesRequest {
    query: String,
    limit: usize,
}

#[derive(Debug, PartialEq)]
struct TypeSearchResult {
    name: String,
    english_name: Option<String>,
    facet: String,
    certainty: String,
}

/// Эмуляция handle_search_types из lsp_server.rs (изолированно для unit тестирования)
fn search_types_in_repository(
    coordinator: &SystemCoordinator,
    params: SearchTypesRequest,
) -> Vec<TypeSearchResult> {
    let analysis_engine = match coordinator.get_analysis_engine() {
        Some(engine) => engine,
        None => return vec![],
    };

    let repo = analysis_engine.get_repository();
    let all_types = repo.get_all_types();

    if all_types.is_empty() {
        return vec![];
    }

    let query_lower = params.query.to_lowercase();
    all_types
        .iter()
        .filter(|t| {
            t.name.to_lowercase().contains(&query_lower)
                || (!t.english_name.is_empty()
                    && t.english_name.to_lowercase().contains(&query_lower))
        })
        .take(params.limit)
        .map(|t| {
            let facet = if t.facets.is_empty() {
                "Object".to_string()
            } else {
                format!("{:?}", t.facets[0])
            };

            TypeSearchResult {
                name: t.name.clone(),
                english_name: if t.english_name.is_empty() {
                    None
                } else {
                    Some(t.english_name.clone())
                },
                facet,
                certainty: "Known (100%)".to_string(),
            }
        })
        .collect()
}

/// Вспомогательная функция: создать SystemCoordinator с базовыми типами
fn create_coordinator_with_basic_types() -> Arc<SystemCoordinator> {
    let coordinator = Arc::new(SystemCoordinator::new());

    // Инициализируем coordinator (это создаёт AnalysisEngine)
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        coordinator.start_with_paths(None, None).await
    }).expect("Failed to start coordinator");

    // Получаем TypeRepository для добавления тестовых типов
    let engine = coordinator.get_analysis_engine().unwrap();
    let repo = engine.get_repository();

    // Добавляем базовые типы для тестирования (в дополнение к fallback типам)
    let test_types = vec![
        RawTypeData {
            name: "Массив".to_string(),
            english_name: "Array".to_string(),
            facets: vec![FacetKind::Object],
            properties: vec![],
            methods: vec![],
            description: "Динамический массив".to_string(),
            ..Default::default()
        },
        RawTypeData {
            name: "Структура".to_string(),
            english_name: "Structure".to_string(),
            facets: vec![FacetKind::Object],
            properties: vec![],
            methods: vec![],
            description: "Ассоциативная структура".to_string(),
            ..Default::default()
        },
        RawTypeData {
            name: "ТаблицаЗначений".to_string(),
            english_name: "ValueTable".to_string(),
            facets: vec![FacetKind::Object],
            properties: vec![],
            methods: vec![],
            description: "Таблица со строками и колонками".to_string(),
            ..Default::default()
        },
        RawTypeData {
            name: "Строка".to_string(),
            english_name: "String".to_string(),
            facets: vec![FacetKind::Object],
            properties: vec![],
            methods: vec![],
            description: "Строковый тип".to_string(),
            ..Default::default()
        },
        RawTypeData {
            name: "Число".to_string(),
            english_name: "Number".to_string(),
            facets: vec![FacetKind::Object],
            properties: vec![],
            methods: vec![],
            description: "Числовой тип".to_string(),
            ..Default::default()
        },
    ];

    repo.load_types(test_types).expect("Failed to load test types");

    coordinator
}

// ============================================================================
// Тесты: Основная функциональность
// ============================================================================

#[test]
fn test_search_types_basic_functionality() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Массив".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(!results.is_empty(), "Должен найти тип 'Массив'");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Массив");
    assert_eq!(results[0].english_name, Some("Array".to_string()));
    assert_eq!(results[0].certainty, "Known (100%)");
}

#[test]
fn test_search_types_case_insensitive() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();

    // ACT - поиск в разных регистрах
    let results_lower = search_types_in_repository(
        &coordinator,
        SearchTypesRequest {
            query: "массив".to_string(),
            limit: 15,
        },
    );
    let results_upper = search_types_in_repository(
        &coordinator,
        SearchTypesRequest {
            query: "МАССИВ".to_string(),
            limit: 15,
        },
    );
    let results_mixed = search_types_in_repository(
        &coordinator,
        SearchTypesRequest {
            query: "МаСсИв".to_string(),
            limit: 15,
        },
    );

    // ASSERT - все варианты должны находить тип
    assert_eq!(results_lower.len(), 1);
    assert_eq!(results_upper.len(), 1);
    assert_eq!(results_mixed.len(), 1);
    assert_eq!(results_lower[0].name, "Массив");
}

#[test]
fn test_search_types_by_english_name() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Array".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Массив");
    assert_eq!(results[0].english_name, Some("Array".to_string()));
}

#[test]
fn test_search_types_partial_match() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Табл".to_string(), // Частичное совпадение
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "ТаблицаЗначений");
}

#[test]
fn test_search_types_limit_respected() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "".to_string(), // Пустой запрос → все типы
        limit: 2, // Лимит = 2
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(
        results.len() <= 2,
        "Результатов должно быть не больше лимита"
    );
}

#[test]
fn test_search_types_no_match() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "НеСуществующийТип123".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(
        results.is_empty(),
        "Несуществующий тип не должен находиться"
    );
}

#[test]
fn test_search_types_empty_query() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    // Пустой запрос → все типы (до лимита)
    assert!(results.len() <= 15);
    assert!(!results.is_empty(), "Должны вернуться все типы");
}

#[test]
fn test_search_types_empty_repository() {
    // ARRANGE - coordinator БЕЗ инициализации (AnalysisEngine не создан)
    let coordinator = Arc::new(SystemCoordinator::new());
    let request = SearchTypesRequest {
        query: "Массив".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(
        results.is_empty(),
        "Неинициализированный репозиторий → пустой результат"
    );
}

// ============================================================================
// Тесты: Регрессионные проверки
// ============================================================================

#[test]
fn test_search_types_regression_table_value() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "ТаблицаЗначений".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(
        !results.is_empty(),
        "⚠️ РЕГРЕСС: 'ТаблицаЗначений' раньше не находилась в mock данных!"
    );
    assert_eq!(results[0].name, "ТаблицаЗначений");
}

#[test]
fn test_search_types_multiple_matches() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Стр".to_string(), // Частичное совпадение → "Структура", "Строка" (минимум)
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(
        results.len() >= 2,
        "Должно найтись минимум 2 типа (может быть больше из fallback типов)"
    );
    let names: Vec<String> = results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"Структура".to_string()));
    assert!(names.contains(&"Строка".to_string()));
}

// ============================================================================
// Тесты: Performance и граничные случаи
// ============================================================================

#[test]
fn test_search_types_large_query() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "A".repeat(1000), // Очень длинный запрос
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(results.is_empty(), "Длинный запрос → пустой результат");
}

#[test]
fn test_search_types_limit_zero() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Массив".to_string(),
        limit: 0, // Лимит = 0
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    assert!(results.is_empty(), "Лимит 0 → пустой результат");
}

#[test]
fn test_search_types_unicode_query() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Массив 🚀".to_string(), // Unicode символы
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT
    // Должен корректно обработать unicode (но ничего не найти)
    assert!(results.is_empty());
}

// ============================================================================
// Интеграционный тест: Production типы из Syntax Helper
// ============================================================================

#[test]
#[ignore] // Требует наличия Syntax Helper архива
fn test_search_types_production_syntax_helper() {
    // ARRANGE - загружаем РЕАЛЬНЫЕ типы из Syntax Helper
    use std::path::PathBuf;

    let syntax_helper_path = PathBuf::from("examples/syntax_helper");
    if !syntax_helper_path.exists() {
        println!("⚠️ Syntax Helper не найден в examples/syntax_helper");
        return;
    }

    let coordinator = Arc::new(SystemCoordinator::new());

    // Загружаем типы платформы
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        coordinator.start_with_paths(Some(&syntax_helper_path), None).await
    }).expect("Failed to start coordinator with platform types");

    // ACT - поиск типов платформы
    let test_queries = vec!["Массив", "Структура", "ТаблицаЗначений", "Array"];

    for query in test_queries {
        let results = search_types_in_repository(
            &coordinator,
            SearchTypesRequest {
                query: query.to_string(),
                limit: 15,
            },
        );

        // ASSERT
        println!("Query '{}' → {} results", query, results.len());
        assert!(
            !results.is_empty(),
            "Production типы должны находиться: {}",
            query
        );
    }
}

// ============================================================================
// Тест: Корректность формата ответа
// ============================================================================

#[test]
fn test_search_types_response_format() {
    // ARRANGE
    let coordinator = create_coordinator_with_basic_types();
    let request = SearchTypesRequest {
        query: "Массив".to_string(),
        limit: 15,
    };

    // ACT
    let results = search_types_in_repository(&coordinator, request);

    // ASSERT - проверяем формат TypeSearchResult
    assert_eq!(results.len(), 1);
    let result = &results[0];

    // Обязательные поля
    assert!(!result.name.is_empty(), "name не должен быть пустым");
    assert!(
        !result.certainty.is_empty(),
        "certainty не должен быть пустым"
    );
    assert!(!result.facet.is_empty(), "facet не должен быть пустым");

    // Проверка формата certainty
    assert_eq!(result.certainty, "Known (100%)");

    // english_name должен быть Some для типов с переводом
    assert!(result.english_name.is_some());
}
