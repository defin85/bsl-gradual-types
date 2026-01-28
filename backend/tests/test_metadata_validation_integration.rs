//! Интеграционный тест для валидации объектов метаданных (Milestone 3.16)
//!
//! Проверяет, что semantic validation показывает ошибку когда:
//! 1. Конфигурация загружена
//! 2. Объект метаданных НЕ найден в конфигурации

use bsl_analysis_v2::AstToIrConverter;
use bsl_backend::application::semantic_validation_visitor::SemanticValidationVisitor;
use bsl_backend::system::tree_sitter_adapter::TreeSitterAdapter;
use bsl_shared::domain::metadata_lookup::TypeMetadataLookup;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::{FacetKind, MetadataKind, RawDataSource, RawTypeData};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::ir::walk_program;
use std::sync::Arc;
use tree_sitter::Parser;

/// Создаёт репозиторий с "загруженной конфигурацией"
/// (configuration_types > 0)
fn create_repository_with_config() -> Arc<InMemoryTypeRepository> {
    let repo = Arc::new(InMemoryTypeRepository::new());

    // Добавляем конфигурационный тип для имитации загруженной конфигурации
    // Например, Справочник "Партнеры" существует
    let config_type = RawTypeData {
        name: "Справочники.Партнеры".to_string(),
        english_name: "Catalogs.Partners".to_string(),
        description: "Тестовый справочник".to_string(),
        methods: vec![],
        properties: vec![],
        category: "Configuration".to_string(),
        source: RawDataSource::Configuration,
        facets: vec![FacetKind::Manager],
        kind: Some(MetadataKind::Catalog),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    // Добавляем ещё один для увеличения счётчика
    let doc_type = RawTypeData {
        name: "Документы.РеализацияТоваров".to_string(),
        english_name: "Documents.SalesInvoice".to_string(),
        description: "Тестовый документ".to_string(),
        methods: vec![],
        properties: vec![],
        category: "Configuration".to_string(),
        source: RawDataSource::Configuration,
        facets: vec![FacetKind::Manager],
        kind: Some(MetadataKind::Document),
        attributes: vec![],
        tabular_sections: vec![],
        enum_values: vec![],
        generic_info: None,
        collection_item_type: None,
        module_paths: None,
    };

    // Загружаем типы через load_types()
    repo.load_types(vec![config_type, doc_type]).unwrap();

    repo
}

#[test]
fn test_nonexistent_document_generates_error_when_config_loaded() {
    // Код с несуществующим документом
    let code = r#"x = Документы.ЗаказКлиента;"#;

    println!("\n=== Тест: несуществующий документ при загруженной конфигурации ===");
    println!("Код: {}", code);

    // 1. Парсинг
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    // 2. Создаём репозиторий С конфигурацией
    let repository = create_repository_with_config();

    // Проверяем, что конфигурация "загружена"
    let stats = repository.get_stats();
    println!("Configuration types: {}", stats.configuration_types);
    assert!(
        stats.configuration_types > 0,
        "Конфигурация должна быть загружена"
    );

    // 3. Конвертация в IR
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository.clone(),
        signature_index.clone(),
    )
    .unwrap();

    // Проверяем наличие MemberAccess в IR
    let has_member_access = ir.nodes.iter().any(|n| {
        matches!(
            n.kind,
            bsl_shared::ir::SemanticNodeKind::MemberAccess { .. }
        )
    });
    println!("IR has MemberAccess: {}", has_member_access);
    assert!(has_member_access, "IR должен содержать MemberAccess");

    // 4. Semantic Validation
    let metadata_lookup = TypeMetadataLookup::new(repository.clone());

    // Проверяем is_configuration_loaded
    println!(
        "is_configuration_loaded: {}",
        metadata_lookup.is_configuration_loaded()
    );
    assert!(
        metadata_lookup.is_configuration_loaded(),
        "Конфигурация должна быть загружена"
    );

    let validator = TypeValidator::new(&metadata_lookup);
    let resolver = bsl_shared::domain::resolver::TypeResolver::new(repository);

    let mut visitor = SemanticValidationVisitor::new(&validator, &ir, &resolver, &signature_index);
    walk_program(&ir, &mut visitor);

    let errors = visitor.into_errors();

    println!("Errors found: {}", errors.len());
    for err in &errors {
        println!("  - {}", err.message);
    }

    // Должна быть ошибка о несуществующем документе!
    assert!(
        !errors.is_empty(),
        "Должна быть ошибка для несуществующего документа 'ЗаказКлиента'"
    );

    // Проверяем, что ошибка содержит имя документа
    let has_doc_error = errors
        .iter()
        .any(|e| e.message.contains("ЗаказКлиента") || e.message.contains("Документы"));
    assert!(
        has_doc_error,
        "Ошибка должна упоминать 'ЗаказКлиента' или 'Документы'"
    );
}

#[test]
fn test_existing_document_no_error() {
    // Код с СУЩЕСТВУЮЩИМ документом
    let code = r#"x = Документы.РеализацияТоваров;"#;

    println!("\n=== Тест: существующий документ не генерирует ошибку ===");
    println!("Код: {}", code);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    let repository = create_repository_with_config();
    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository.clone(),
        signature_index.clone(),
    )
    .unwrap();

    let metadata_lookup = TypeMetadataLookup::new(repository.clone());
    let validator = TypeValidator::new(&metadata_lookup);
    let resolver = bsl_shared::domain::resolver::TypeResolver::new(repository);

    let mut visitor = SemanticValidationVisitor::new(&validator, &ir, &resolver, &signature_index);
    walk_program(&ir, &mut visitor);

    let errors = visitor.into_errors();

    println!("Errors found: {}", errors.len());
    for err in &errors {
        println!("  - {}", err.message);
    }

    // Не должно быть ошибки для существующего документа
    let has_doc_error = errors
        .iter()
        .any(|e| e.message.contains("РеализацияТоваров"));
    assert!(
        !has_doc_error,
        "НЕ должно быть ошибки для существующего документа"
    );
}

#[test]
fn test_graceful_degradation_no_config() {
    // Код с несуществующим документом, но конфигурация НЕ загружена
    let code = r#"x = Документы.ЗаказКлиента;"#;

    println!("\n=== Тест: graceful degradation без конфигурации ===");
    println!("Код: {}", code);

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();
    let ast_result = TreeSitterAdapter::convert_tree(&tree, code).unwrap();

    // Репозиторий БЕЗ конфигурации (только platform types)
    let repository = Arc::new(InMemoryTypeRepository::new());

    // Проверяем, что конфигурация НЕ загружена
    let stats = repository.get_stats();
    println!("Configuration types: {}", stats.configuration_types);
    assert!(
        stats.configuration_types == 0,
        "Конфигурация НЕ должна быть загружена"
    );

    let signature_index = SignatureIndex::new();
    let ir = AstToIrConverter::convert(
        ast_result.program,
        code.to_string(),
        "test.bsl".to_string(),
        repository.clone(),
        signature_index.clone(),
    )
    .unwrap();

    let metadata_lookup = TypeMetadataLookup::new(repository.clone());
    println!(
        "is_configuration_loaded: {}",
        metadata_lookup.is_configuration_loaded()
    );

    let validator = TypeValidator::new(&metadata_lookup);
    let resolver = bsl_shared::domain::resolver::TypeResolver::new(repository);

    let mut visitor = SemanticValidationVisitor::new(&validator, &ir, &resolver, &signature_index);
    walk_program(&ir, &mut visitor);

    let errors = visitor.into_errors();

    println!("Errors found: {}", errors.len());
    for err in &errors {
        println!("  - {}", err.message);
    }

    // НЕ должно быть ошибки (graceful degradation)
    let has_metadata_error = errors
        .iter()
        .any(|e| e.message.contains("ЗаказКлиента") || e.message.contains("не найден"));
    assert!(
        !has_metadata_error,
        "НЕ должно быть ошибки при отсутствии конфигурации (graceful degradation)"
    );
}
