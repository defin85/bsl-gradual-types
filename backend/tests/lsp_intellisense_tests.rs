//! Интеграционные тесты LSP IntelliSense (completion/resolve/signatureHelp).

mod intellisense_testkit;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

mod converters {
    pub use crate::position;
}

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

#[path = "../src/bin/lsp_server/handlers/signature_help.rs"]
mod signature_help_handler;

use std::sync::Arc;

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::{AnalysisCache, IndexItem, IndexItemKind, IndexKind, IntellisenseIndexStore, IrCache, ParserCoordinator, TypeKind};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{ParameterInfo, RawDataSource, RawMethodData, RawParamData, RawTypeData};
use bsl_shared::engine::AnalysisEngine;
use bsl_shared::TypeResolver;
use tower_lsp::lsp_types::{CompletionResponse, InsertTextFormat, Position, Url};

struct TestEnv {
    service: Arc<TypeSystemService>,
    engine: Arc<AnalysisEngine>,
}

fn position_at_marker(content: &str, marker: &str) -> Position {
    let (line, column) = intellisense_testkit::find_marker_position(content, marker);
    Position {
        line,
        character: column,
    }
}

fn build_index_with_keywords() -> Arc<IntellisenseIndexStore> {
    let index = Arc::new(IntellisenseIndexStore::new("m8", "platform"));
    index.set_keywords(vec![IndexItem::new(
        "Процедура",
        IndexItemKind::Keyword,
        IndexKind::Keyword,
    )]);
    index.upsert_type(IndexItem::new(
        "Массив",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    index
}

fn build_repository_with_array() -> Arc<InMemoryTypeRepository> {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![RawTypeData {
            name: "Массив".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Добавить".to_string(),
                return_type: "Булево".to_string(),
                params: vec![
                    RawParamData {
                        name: "Элемент".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: false,
                        default_value: None,
                    },
                    RawParamData {
                        name: "Позиция".to_string(),
                        param_type: "Число".to_string(),
                        is_optional: true,
                        default_value: None,
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }])
        .expect("load types");

    let mut signatures = SignatureIndex::new();
    signatures.add_platform_method(
        TypeId::new("Массив"),
        MethodSignature::new(
            "Добавить".to_string(),
            Some("Массив".to_string()),
            vec![
                ParameterInfo {
                    name: "Элемент".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: false,
                    default_value: None,
                    description: None,
                },
                ParameterInfo {
                    name: "Позиция".to_string(),
                    type_name: Some("Число".to_string()),
                    is_optional: true,
                    default_value: None,
                    description: None,
                },
            ],
            Some("Булево".to_string()),
            None,
            None,
            SignatureSource::Platform,
            None,
            Default::default(),
        ),
    );
    signatures.add_constructor(
        TypeId::new("Массив"),
        ConstructorSignature {
            type_name: "Массив".to_string(),
            params: vec![ParameterInfo {
                name: "Размер".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: true,
                default_value: None,
                description: None,
            }],
            facet: None,
            source: SignatureSource::Platform,
            is_collection: true,
            generic_params_count: 1,
        },
    );
    repository.set_signature_index(signatures);

    repository
}

fn build_engine_and_service(
    repository: Arc<InMemoryTypeRepository>,
    index: Arc<IntellisenseIndexStore>,
) -> (Arc<AnalysisEngine>, Arc<TypeSystemService>) {
    let repo = repository as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repo.clone()));
    let engine = Arc::new(AnalysisEngine::new(resolver.clone(), repo.clone()));
    let cache = Arc::new(AnalysisCache::new(8));
    let ir_cache = Arc::new(IrCache::new(8));
    let parser = Arc::new(ParserCoordinator::new_with_resolver(repo, resolver));
    let service = Arc::new(TypeSystemService::new(
        engine.clone(),
        cache,
        parser,
        ir_cache,
        index,
    ));
    service.initialize().expect("initialize service");
    (engine, service)
}

fn build_env() -> TestEnv {
    let repository = build_repository_with_array();
    let index = build_index_with_keywords();
    let (engine, service) = build_engine_and_service(repository, index);
    TestEnv { service, engine }
}

#[tokio::test]
async fn lsp_completion_returns_items_and_stats() {
    let env = build_env();
    let content = "Массив.";
    let position = position_at_marker(content, "Массив.");
    let uri = Url::parse("file:///m8_lsp_completion.bsl").expect("url");

    let response = completion_handler::handle_completion(
        content,
        position,
        &uri,
        Some(env.service.clone()),
        false,
    )
    .await
    .expect("completion response");

    assert!(!response.had_error);
    assert!(response.stats.is_some());

    let items = match response.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };

    assert!(items.iter().any(|item| item.label == "Добавить"));
}

#[tokio::test]
async fn lsp_completion_resolve_respects_snippet_support() {
    let env = build_env();
    let content = "Массив.";
    let position = position_at_marker(content, "Массив.");
    let uri = Url::parse("file:///m8_lsp_completion.bsl").expect("url");

    let response = completion_handler::handle_completion(
        content,
        position,
        &uri,
        Some(env.service.clone()),
        false,
    )
    .await
    .expect("completion response");

    let items = match response.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };
    let item = items
        .into_iter()
        .find(|entry| entry.label == "Добавить")
        .expect("Добавить completion");

    let resolved_snippet =
        completion_handler::handle_completion_resolve(item.clone(), Some(env.service.clone()), true)
            .await;
    let resolved_plain =
        completion_handler::handle_completion_resolve(item, Some(env.service), false).await;

    assert_eq!(resolved_snippet.insert_text_format, Some(InsertTextFormat::SNIPPET));
    assert!(resolved_snippet
        .insert_text
        .as_deref()
        .unwrap_or("")
        .contains("${1:"));
    assert_eq!(resolved_plain.insert_text_format, None);
    assert_eq!(resolved_plain.detail.as_deref(), Some("Булево"));
}

#[tokio::test]
async fn lsp_signature_help_returns_method_and_constructor() {
    let env = build_env();
    let content = r#"Процедура Тест()
    Новый Массив(1, )
    Массив.Добавить(1, )
КонецПроцедуры"#;

    let constructor_pos = position_at_marker(content, "Новый Массив(1, ");
    let method_pos = position_at_marker(content, "Массив.Добавить(1, ");

    let constructor = signature_help_handler::handle_signature_help(
        content,
        constructor_pos,
        Some(env.engine.clone()),
    )
    .await
    .expect("constructor signature help");
    let method =
        signature_help_handler::handle_signature_help(content, method_pos, Some(env.engine))
            .await
            .expect("method signature help");

    let constructor_label = constructor
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(constructor_label.starts_with("Новый Массив("));
    assert_eq!(constructor.active_parameter, Some(1));

    let method_label = method
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(method_label.contains("Добавить("));
    assert_eq!(method.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_completion_with_empty_index_returns_default_keywords() {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let index = Arc::new(IntellisenseIndexStore::new("m8", "platform"));
    let (_engine, service) = build_engine_and_service(repository, index);
    let uri = Url::parse("file:///m8_lsp_empty.bsl").expect("url");

    let response = completion_handler::handle_completion(
        "",
        Position::new(0, 0),
        &uri,
        Some(service),
        false,
    )
    .await
    .expect("completion response");

    let items = match response.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };

    assert!(items.iter().any(|item| item.label == "Процедура"));
}
