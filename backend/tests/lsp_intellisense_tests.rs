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

use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
};
use bsl_backend::system::{
    IndexItem, IndexItemKind, IndexKind, IndexSnapshot, IntellisenseIndexStore, TypeKind,
};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{
    ParameterInfo, RawDataSource, RawMethodData, RawParamData, RawTypeData,
};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;
use bsl_shared::TypeResolver;
use tower_lsp::lsp_types::{CompletionResponse, InsertTextFormat, Position, Url};

struct TestEnv {
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    index_snapshot: IndexSnapshot,
}

fn position_at_marker(content: &str, marker: &str) -> Position {
    let (line, column) = intellisense_testkit::find_marker_position(content, marker);
    Position {
        line,
        character: column,
    }
}

fn build_index_with_keywords() -> IndexSnapshot {
    let index = IntellisenseIndexStore::new("m8", "platform");
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
    index.snapshot()
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

fn build_deps(repository_impl: Arc<InMemoryTypeRepository>) -> Arc<bsl_analysis_v2::SemanticDeps> {
    let repository = repository_impl as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));

    Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repository.get_signature_index_clone(),
        resolver: Some(resolver),
        repository,
    })
}

fn build_env() -> TestEnv {
    let repository = build_repository_with_array();
    let deps = build_deps(repository);
    let index_snapshot = build_index_with_keywords();
    TestEnv {
        deps,
        index_snapshot,
    }
}

fn build_v2_ir(
    content: &str,
    uri: &Url,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> (Arc<str>, Arc<str>, Arc<SemanticProgram>) {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("test"),
        deps: deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("test"),
        diagnostics_detail_level: DetailLevel::Full,
    });

    let path = uri
        .to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| uri.to_string());
    let file_id = V2FileId(1);
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(path),
    });

    let analysis = host.analysis();
    let file_content = analysis
        .file_text(file_id)
        .ok()
        .flatten()
        .expect("file_text");
    let file_path = analysis
        .file_path(file_id)
        .ok()
        .flatten()
        .expect("file_path");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

    (file_content, file_path, ir_program)
}

#[tokio::test]
async fn lsp_completion_returns_items_and_stats() {
    let env = build_env();
    let content = "Массив.";
    let position = position_at_marker(content, "Массив.");
    let uri = Url::parse("file:///m8_lsp_completion.bsl").expect("url");

    let (file_content, file_path, ir_program) = build_v2_ir(content, &uri, env.deps.clone());
    let response = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        None,
        env.deps.clone(),
        position,
        &uri,
        &env.index_snapshot,
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

    let (file_content, file_path, ir_program) = build_v2_ir(content, &uri, env.deps.clone());
    let response = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        None,
        env.deps.clone(),
        position,
        &uri,
        &env.index_snapshot,
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
        completion_handler::handle_completion_resolve(item.clone(), Some(env.deps.clone()), true)
            .await;
    let resolved_plain =
        completion_handler::handle_completion_resolve(item, Some(env.deps), false).await;

    assert_eq!(
        resolved_snippet.insert_text_format,
        Some(InsertTextFormat::SNIPPET)
    );
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

    let constructor = signature_help_handler::handle_signature_help_v2(
        Arc::from(content.to_string()),
        constructor_pos,
        env.deps.clone(),
    )
    .await
    .expect("constructor signature help");
    let method = signature_help_handler::handle_signature_help_v2(
        Arc::from(content.to_string()),
        method_pos,
        env.deps,
    )
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
    let deps = build_deps(repository);
    let index = IntellisenseIndexStore::new("m8", "platform");
    let index_snapshot = index.snapshot();
    let uri = Url::parse("file:///m8_lsp_empty.bsl").expect("url");

    let (file_content, file_path, ir_program) = build_v2_ir("", &uri, deps.clone());
    let response = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        None,
        deps,
        Position::new(0, 0),
        &uri,
        &index_snapshot,
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
