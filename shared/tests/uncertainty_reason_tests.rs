//! Unit тесты для трёхуровневой логики Certainty и UncertaintyReason
//!
//! MILESTONE 3.16: Three-level resolution logic
//!
//! Тестируемые сценарии:
//! 1. Known (100%) - конфигурация загружена, тип найден в метаданных
//! 2. Inferred (50%) - конфигурация НЕ загружена (graceful degradation)
//! 3. Unknown - конфигурация загружена, но тип НЕ найден (ошибка)
//!
//! Тестируемые модули:
//! - shared/src/domain/types.rs (Certainty, UncertaintyReason)
//! - shared/src/domain/resolver.rs (resolve_member_access)
//! - shared/src/domain/validators.rs (validate_from_resolution)

#![allow(clippy::vec_init_then_push)]

use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::{
    Certainty, MetadataKind, RawDataSource, RawMethodData, RawTypeData, UncertaintyReason,
};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeErrorKind;
use bsl_shared::TypeRepository;
use std::sync::Arc;

// === Helper функции ===

/// Создает конфигурационный тип (документ)
fn create_document_type(name: &str) -> RawTypeData {
    RawTypeData {
        name: format!("Документы.{}", name),
        english_name: format!("Documents.Document_{}", name),
        description: format!("Документ {}", name),
        category: "Document".to_string(),
        source: RawDataSource::Configuration,
        methods: vec![
            RawMethodData {
                name: "Записать".to_string(),
                english_name: "Write".to_string(),
                return_type: "".to_string(),
                params: vec![],
                description: None,
                is_deprecated: false,
                is_constructor: false,
                context_requirements: None,
                return_facet: None,
            },
        ],
        properties: vec![],
        facets: vec![],
        kind: Some(MetadataKind::Document),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    }
}

/// Создает пустой репозиторий (только платформенные типы)
fn create_empty_repository() -> Arc<InMemoryTypeRepository> {
    Arc::new(InMemoryTypeRepository::new())
}

/// Создает репозиторий с конфигурационным типом
fn create_repository_with_document(name: &str) -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let doc_type = create_document_type(name);
    repo.load_types(vec![doc_type]).unwrap();
    repo
}

/// Создает репозиторий с другими конфигурационными типами
fn create_repository_with_other_documents() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());
    let types = vec![
        create_document_type("ПриказОтОтпуске"),
        create_document_type("РасходнаяНакладная"),
    ];
    repo.load_types(types).unwrap();
    repo
}

// === ТЕСТЫ ===

/// TEST 1: Конфигурация загружена, тип найден → Known
///
/// Сценарий:
/// - Репозиторий содержит документ "Документы.СуществующийДок"
/// - Резолвим "Документы.СуществующийДок"
/// - Проверяем: Certainty::Known, uncertainty_reason = None
#[test]
fn test_certainty_config_loaded_type_found_is_known() {
    let repo = create_repository_with_document("СуществующийДок");
    let resolver = TypeResolver::new(repo.clone());
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let validator = TypeValidator::new(&metadata_lookup);

    // Резолвим существующий тип
    let resolution = resolver.resolve_expression_sync("Документы.СуществующийДок");

    // Проверяем Certainty::Known
    assert_eq!(
        resolution.certainty, Certainty::Known,
        "Expected Certainty::Known for existing type in loaded configuration"
    );

    // Проверяем что нет uncertainty_reason
    assert_eq!(
        resolution.metadata.uncertainty_reason, None,
        "Expected no uncertainty_reason for Known certainty"
    );

    // Проверяем что validate_from_resolution возвращает None (нет ошибок)
    let validation_error = validator.validate_from_resolution(&resolution);
    assert_eq!(
        validation_error, None,
        "Expected no validation error for Known certainty"
    );
}

/// TEST 2: Конфигурация НЕ загружена → Inferred (graceful degradation)
///
/// Сценарий:
/// - Репозиторий ПУСТ (только платформенные типы)
/// - Резолвим "Документы.ЛюбойДок"
/// - Проверяем: Certainty::InferredWeak, uncertainty_reason = ConfigurationNotLoaded
#[test]
fn test_certainty_config_not_loaded_is_inferred() {
    let repo = create_empty_repository();
    let resolver = TypeResolver::new(repo.clone());
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let validator = TypeValidator::new(&metadata_lookup);

    // Резолвим несуществующий тип (но конфигурация не загружена)
    let resolution = resolver.resolve_expression_sync("Документы.ЛюбойДок");

    // Проверяем Certainty::InferredWeak
    assert_eq!(
        resolution.certainty, Certainty::InferredWeak,
        "Expected Certainty::InferredWeak when configuration is not loaded"
    );

    // Проверяем uncertainty_reason = ConfigurationNotLoaded
    assert_eq!(
        resolution.metadata.uncertainty_reason,
        Some(UncertaintyReason::ConfigurationNotLoaded),
        "Expected ConfigurationNotLoaded uncertainty reason"
    );

    // Проверяем что validate_from_resolution возвращает None (graceful degradation, нет ошибок)
    let validation_error = validator.validate_from_resolution(&resolution);
    assert_eq!(
        validation_error, None,
        "Expected no validation error for ConfigurationNotLoaded (graceful degradation)"
    );
}

/// TEST 3: Конфигурация загружена, тип НЕ найден → Unknown (ошибка)
///
/// Сценарий:
/// - Репозиторий содержит другие документы
/// - Резолвим "Документы.НесуществующийДок"
/// - Проверяем: Certainty::Unknown, uncertainty_reason = MetadataObjectNotFound
#[test]
fn test_certainty_config_loaded_type_not_found_is_unknown() {
    let repo = create_repository_with_other_documents();
    let resolver = TypeResolver::new(repo.clone());
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let validator = TypeValidator::new(&metadata_lookup);

    // Резолвим несуществующий тип (конфигурация загружена)
    let resolution = resolver.resolve_expression_sync("Документы.НесуществующийДок");

    // Проверяем Certainty::Unknown
    assert_eq!(
        resolution.certainty, Certainty::Unknown,
        "Expected Certainty::Unknown when configuration is loaded but type not found"
    );

    // Проверяем uncertainty_reason = MetadataObjectNotFound
    match &resolution.metadata.uncertainty_reason {
        Some(UncertaintyReason::MetadataObjectNotFound { kind, name }) => {
            assert_eq!(*kind, MetadataKind::Document, "Expected Document kind");
            assert_eq!(name, "НесуществующийДок", "Expected the missing type name");
        }
        other => {
            panic!(
                "Expected MetadataObjectNotFound uncertainty reason, got: {:?}",
                other
            );
        }
    }

    // Проверяем что validate_from_resolution возвращает ошибку
    let validation_error = validator.validate_from_resolution(&resolution);
    assert!(
        validation_error.is_some(),
        "Expected validation error for Unknown certainty with MetadataObjectNotFound"
    );
}

/// TEST 4: Валидация из resolution генерирует ошибку
///
/// Сценарий:
/// - Создаем resolution с UncertaintyReason::MetadataObjectNotFound
/// - Вызываем validate_from_resolution()
/// - Проверяем: возвращает TypeErrorKind::UnknownMetadataObject
#[test]
fn test_validate_from_resolution_generates_error_for_unknown_metadata() {
    let repo = create_repository_with_other_documents();
    let resolver = TypeResolver::new(repo.clone());
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let validator = TypeValidator::new(&metadata_lookup);

    // Резолвим несуществующий тип
    let resolution = resolver.resolve_expression_sync("Документы.НесуществующийДок");

    // Вызываем validate_from_resolution
    let error = validator.validate_from_resolution(&resolution);

    // Проверяем что вернулась ошибка
    assert!(
        error.is_some(),
        "Expected validation error for MetadataObjectNotFound"
    );

    // Проверяем тип ошибки
    if let Some(error_kind) = error {
        match error_kind {
            TypeErrorKind::UnknownMetadataObject {
                kind,
                name,
                ..
            } => {
                assert_eq!(kind, MetadataKind::Document);
                assert_eq!(name, "НесуществующийДок");
            }
            other => {
                panic!("Expected UnknownMetadataObject error, got: {:?}", other);
            }
        }
    }
}

/// TEST 5: Graceful degradation - конфигурация не загружена
///
/// Сценарий:
/// - Пустой репозиторий
/// - Резолвим различные конфигурационные типы
/// - Все должны быть Inferred с ConfigurationNotLoaded
#[test]
fn test_graceful_degradation_multiple_types() {
    let repo = create_empty_repository();
    let resolver = TypeResolver::new(repo.clone());
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let validator = TypeValidator::new(&metadata_lookup);

    let test_cases = vec![
        "Документы.ПроизвольныйДок",
        "Справочники.ПроизвольныйСправочник",
        "Перечисления.ПроизвольноеПеречисление",
    ];

    for type_expr in test_cases {
        let resolution = resolver.resolve_expression_sync(type_expr);

        // Все должны быть InferredWeak
        assert_eq!(
            resolution.certainty, Certainty::InferredWeak,
            "Expected InferredWeak for {} when config not loaded",
            type_expr
        );

        // Все должны иметь ConfigurationNotLoaded
        assert_eq!(
            resolution.metadata.uncertainty_reason,
            Some(UncertaintyReason::ConfigurationNotLoaded),
            "Expected ConfigurationNotLoaded for {}",
            type_expr
        );

        // Валидация не должна генерировать ошибки
        let validation_error = validator.validate_from_resolution(&resolution);
        assert_eq!(
            validation_error, None,
            "Expected no validation error for {} (graceful degradation)",
            type_expr
        );
    }
}

/// TEST 6: Отличие между Inferred и Unknown
///
/// Сценарий:
/// - Создаём два репозитория
/// - Репозиторий 1: пустой (config not loaded)
/// - Репозиторий 2: с конфигурацией (config loaded)
/// - Резолвим один и тот же тип в обоих
/// - Проверяем что результаты разные
#[test]
fn test_difference_between_inferred_and_unknown() {
    let repo_empty = create_empty_repository();
    let repo_with_config = create_repository_with_other_documents();

    let resolver_empty = TypeResolver::new(repo_empty.clone());
    let resolver_with_config = TypeResolver::new(repo_with_config.clone());

    let type_expr = "Документы.НесуществующийДок";

    // Резолвим в пустом репозитории
    let resolution_empty = resolver_empty.resolve_expression_sync(type_expr);

    // Резолвим в репозитории с конфигурацией
    let resolution_with_config = resolver_with_config.resolve_expression_sync(type_expr);

    // Проверяем что certainty отличается
    assert_eq!(
        resolution_empty.certainty, Certainty::InferredWeak,
        "Empty repo should produce InferredWeak"
    );
    assert_eq!(
        resolution_with_config.certainty, Certainty::Unknown,
        "Loaded repo should produce Unknown"
    );

    // Проверяем что uncertainty_reason отличается
    assert_eq!(
        resolution_empty.metadata.uncertainty_reason,
        Some(UncertaintyReason::ConfigurationNotLoaded)
    );
    match resolution_with_config.metadata.uncertainty_reason {
        Some(UncertaintyReason::MetadataObjectNotFound { .. }) => {
            // OK
        }
        other => {
            panic!("Expected MetadataObjectNotFound, got: {:?}", other);
        }
    }

    // Проверяем что валидация ведёт себя по-разному
    let metadata_lookup_empty = TypeMetadataLookup::new(repo_empty);
    let metadata_lookup_with_config = TypeMetadataLookup::new(repo_with_config);
    let validator_empty = TypeValidator::new(&metadata_lookup_empty);
    let validator_with_config = TypeValidator::new(&metadata_lookup_with_config);

    let error_empty = validator_empty.validate_from_resolution(&resolution_empty);
    let error_with_config = validator_with_config.validate_from_resolution(&resolution_with_config);

    assert_eq!(
        error_empty, None,
        "Empty repo should not generate validation error (graceful degradation)"
    );
    assert!(
        error_with_config.is_some(),
        "Loaded repo should generate validation error"
    );
}

/// TEST 7: ResolutionMetadata содержит корректные данные
///
/// Сценарий:
/// - Резолвим существующий и несуществующий типы
/// - Проверяем что ResolutionMetadata содержит корректные certainty и uncertainty_reason
#[test]
fn test_resolution_metadata_contains_correct_data() {
    let repo_with_config = create_repository_with_document("СуществующийДок");
    let resolver = TypeResolver::new(repo_with_config);

    let resolution_found = resolver.resolve_expression_sync("Документы.СуществующийДок");
    let resolution_not_found = resolver.resolve_expression_sync("Документы.НесуществующийДок");

    // Проверяем certainty для найденного типа
    assert_eq!(
        resolution_found.certainty, Certainty::Known,
        "Expected Known certainty for found type"
    );
    assert_eq!(
        resolution_found.metadata.uncertainty_reason, None,
        "Expected no uncertainty_reason for found type"
    );

    // Проверяем certainty для ненайденного типа
    assert_eq!(
        resolution_not_found.certainty, Certainty::Unknown,
        "Expected Unknown certainty for not found type"
    );
    assert!(
        matches!(
            resolution_not_found.metadata.uncertainty_reason,
            Some(UncertaintyReason::MetadataObjectNotFound { .. })
        ),
        "Expected MetadataObjectNotFound uncertainty reason for not found type. Got: {:?}",
        resolution_not_found.metadata.uncertainty_reason
    );
}

/// TEST 8: Inferred для платформенных типов при пустом репозитории
///
/// Сценарий:
/// - Пустой репозиторий
/// - Резолвим стандартные типы (Строка, Число, Булево)
/// - Эти типы должны быть Known (не зависят от конфигурации)
#[test]
fn test_primitive_types_always_known() {
    let repo = create_empty_repository();
    let resolver = TypeResolver::new(repo);

    let primitive_types = vec!["Строка", "Число", "Булево"];

    for type_name in primitive_types {
        let resolution = resolver.resolve_expression_sync(type_name);

        // Примитивные типы всегда Known
        assert_eq!(
            resolution.certainty, Certainty::Known,
            "Expected Known for primitive type {}",
            type_name
        );

        // Нет uncertainty_reason
        assert_eq!(
            resolution.metadata.uncertainty_reason, None,
            "Expected no uncertainty_reason for primitive type {}",
            type_name
        );
    }
}

/// TEST 9: Смешанные сценарии - существующие и несуществующие типы
///
/// Сценарий:
/// - Репозиторий с двумя документами
/// - Резолвим существующие, несуществующие и примитивные типы
/// - Проверяем соответствующие Certainty уровни
#[test]
fn test_mixed_scenarios_with_repository() {
    let repo = create_repository_with_document("ЗаказОтПокупателя");
    let resolver = TypeResolver::new(repo.clone());
    let metadata_lookup = TypeMetadataLookup::new(repo);
    let validator = TypeValidator::new(&metadata_lookup);

    // Case 1: Существующий тип → Known
    let resolution_found = resolver.resolve_expression_sync("Документы.ЗаказОтПокупателя");
    assert_eq!(resolution_found.certainty, Certainty::Known);
    assert_eq!(resolution_found.metadata.uncertainty_reason, None);
    assert_eq!(validator.validate_from_resolution(&resolution_found), None);

    // Case 2: Несуществующий тип → Unknown
    let resolution_not_found = resolver.resolve_expression_sync("Документы.НесуществующийДок");
    assert_eq!(resolution_not_found.certainty, Certainty::Unknown);
    assert!(resolution_not_found.metadata.uncertainty_reason.is_some());
    assert!(validator.validate_from_resolution(&resolution_not_found).is_some());

    // Case 3: Примитивный тип → Known
    let resolution_primitive = resolver.resolve_expression_sync("Строка");
    assert_eq!(resolution_primitive.certainty, Certainty::Known);
    assert_eq!(resolution_primitive.metadata.uncertainty_reason, None);
}
