//! Интеграционные тесты LSP IntelliSense (completion/resolve/signatureHelp).

mod intellisense_testkit;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

#[path = "../src/bin/lsp_server/handlers/definition.rs"]
mod definition_handler;

#[path = "../src/bin/lsp_server/handlers/signature_help.rs"]
mod signature_help_handler;

use std::sync::Arc;

use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId, SettingsId,
};
use bsl_backend::system::{
    IndexItem, IndexItemKind, IndexKind, IndexSnapshot, IntellisenseIndexStore, TypeKind,
};
use bsl_shared::domain::type_definition_location::TypeDefinitionLocation;
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
use tempfile::TempDir;
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
    index.upsert_type(IndexItem::new(
        "Строка",
        IndexItemKind::Type(TypeKind::Platform),
        IndexKind::Type,
    ));
    index.snapshot()
}

fn build_repository_with_array() -> Arc<InMemoryTypeRepository> {
    let repository = Arc::new(InMemoryTypeRepository::new());
    repository
        .load_types(vec![
            RawTypeData {
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
            },
            RawTypeData {
                name: "Строка".to_string(),
                source: RawDataSource::Platform,
                methods: vec![RawMethodData {
                    name: "Длина".to_string(),
                    return_type: "Число".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        ])
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
    signatures.add_platform_method(
        TypeId::new("Строка"),
        MethodSignature::new(
            "Длина".to_string(),
            Some("Строка".to_string()),
            vec![],
            Some("Число".to_string()),
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
        platform_signatures_loaded: false,
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

fn build_v2_ir_with_parse_result(
    content: &str,
    uri: &Url,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> (
    Arc<str>,
    Arc<str>,
    Arc<SemanticProgram>,
    Arc<bsl_syntax::ast::ParseResult>,
) {
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
    let parse_result = analysis
        .parse_result(file_id)
        .ok()
        .flatten()
        .expect("parse_result");

    (file_content, file_path, ir_program, parse_result)
}

#[tokio::test]
async fn lsp_definition_flow_sensitive_receiver_hint_changes_result() {
    let tmp = TempDir::new().expect("temp dir");
    let method_file = tmp.path().join("string_methods.bsl");
    let method_text = "Процедура Длина() Экспорт\nКонецПроцедуры\n";
    std::fs::write(&method_file, method_text).expect("write method file");

    let start = method_text
        .find("Длина")
        .map(|idx| idx as u32)
        .expect("method name offset");
    let end = start + ("Длина".len() as u32);

    let repository = build_repository_with_array();
    repository.add_config_method_definition_location(
        "Строка",
        "Длина",
        TypeDefinitionLocation::user_defined(method_file.clone(), start, end),
    );

    let deps = build_deps(repository);

    let content = "Процедура Test()\n\
                   x = 0;\n\
                   Если ТипЗнч(x) = Тип(\"Строка\") Тогда\n\
                       x.Длина();\n\
                   КонецЕсли;\n\
                   КонецПроцедуры\n";
    let uri = Url::parse("file:///flow_sensitive_definition_gate.bsl").expect("url");
    let mut position = position_at_marker(content, "Длина");
    // find_marker_position возвращает позицию *после* маркера; для корректного попадания в span
    // метода смещаемся на 1 UTF-16 unit влево.
    position.character = position.character.saturating_sub(1);

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("test"),
        deps: deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("test"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let file_id = V2FileId(1);
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("flow_sensitive_definition_gate.bsl"),
    });

    let analysis = host.snapshot();
    let file_content = analysis.file_text(file_id).ok().flatten().expect("file_text");
    let file_path = analysis.file_path(file_id).ok().flatten().expect("file_path");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

    // Считаем тип receiver'а (`x`) на старте выражения `x.Длина()`:
    // - базовый v2 тип: Число (из `x = 0`)
    // - flow-sensitive тип в then-ветке: Строка (из type guard)
    let receiver_offset = file_content
        .find("x.Длина")
        .map(|idx| idx as u32)
        .expect("receiver offset");

    let base_receiver_type = analysis
        .type_at_byte_offset(file_id, receiver_offset)
        .ok()
        .flatten();
    let flow_receiver_type = analysis
        .flow_type_at_byte_offset(file_id, receiver_offset)
        .ok()
        .flatten();

    assert_eq!(
        base_receiver_type
            .as_ref()
            .map(|value| value.type_name())
            .as_deref(),
        Some("Число")
    );
    assert_eq!(
        flow_receiver_type
            .as_ref()
            .map(|value| value.type_name())
            .as_deref(),
        Some("Строка")
    );

    let disabled = definition_handler::handle_goto_definition_v2(
        file_path.clone(),
        file_content.clone(),
        ir_program.clone(),
        None,
        base_receiver_type,
        deps.clone(),
        position,
        &uri,
    )
    .await;
    assert!(
        disabled.is_none(),
        "expected no definition without flow-sensitive receiver hint"
    );

    let enabled = definition_handler::handle_goto_definition_v2(
        file_path,
        file_content,
        ir_program,
        None,
        flow_receiver_type,
        deps,
        position,
        &uri,
    )
    .await
    .expect("definition (flow-sensitive)");

    let expected_uri = Url::from_file_path(&method_file).expect("expected method uri");
    match enabled {
        tower_lsp::lsp_types::GotoDefinitionResponse::Scalar(location) => {
            assert_eq!(location.uri, expected_uri);
        }
        other => panic!("expected scalar location, got {:?}", other),
    }
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
        None,
        env.deps.clone(),
        position,
        &uri,
        &env.index_snapshot,
        false,
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
async fn lsp_completion_flow_sensitive_is_gated_by_flag() {
    let env = build_env();
    let content = "Процедура Тест()\n\
                   Перем x;\n\
                   Если ТипЗнч(x) = Тип(\"Строка\") Тогда\n\
                       x.\n\
                   КонецЕсли;\n\
                   КонецПроцедуры\n";
    let position = position_at_marker(content, "x.");
    let uri = Url::parse("file:///flow_sensitive_completion_gate.bsl").expect("url");

    let (file_content, file_path, ir_program, parse_result) =
        build_v2_ir_with_parse_result(content, &uri, env.deps.clone());

    let disabled = completion_handler::handle_completion_v2(
        file_content.clone(),
        file_path.clone(),
        ir_program.clone(),
        Some(parse_result.clone()),
        None,
        env.deps.clone(),
        position,
        &uri,
        &env.index_snapshot,
        false,
        false,
    )
    .await
    .expect("completion (flow-sensitive disabled)");

    let disabled_items = match disabled.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };
    assert!(
        !disabled_items.iter().any(|item| item.label == "Длина"),
        "expected completion to NOT include flow-sensitive method when disabled, got {:?}",
        disabled_items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>()
    );

    let enabled = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        Some(parse_result),
        None,
        env.deps,
        position,
        &uri,
        &env.index_snapshot,
        false,
        true,
    )
    .await
    .expect("completion (flow-sensitive enabled)");

    let enabled_items = match enabled.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };
    assert!(
        enabled_items.iter().any(|item| item.label == "Длина"),
        "expected completion to include flow-sensitive method when enabled, got {:?}",
        enabled_items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>()
    );
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
        None,
        env.deps.clone(),
        position,
        &uri,
        &env.index_snapshot,
        false,
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
        None,
        env.deps.clone(),
    )
    .await
    .expect("constructor signature help");

    let receiver_type_hint = {
        let query = bsl_backend::application::type_system::signature_help_query(
            content,
            method_pos.line,
            method_pos.character,
        )
        .expect("signature help query");

        let mut host = AnalysisHostV2::default();
        host.apply_change(ChangeV2::SetDepsSnapshot {
            deps_id: DepsSnapshotId::from_hash("test"),
            deps: env.deps.clone(),
        });
        host.apply_change(ChangeV2::SetSettingsSnapshot {
            settings_id: SettingsId::from_hash("test"),
            diagnostics_detail_level: DetailLevel::Full,
        });
        host.apply_change(ChangeV2::SetFile {
            file_id: V2FileId(1),
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("test.bsl"),
        });

        query
            .receiver_end_character
            .and_then(|receiver_end_character| {
                let analysis = host.snapshot();
                analysis
                    .utf16_position_to_byte_offset(
                        V2FileId(1),
                        query.call_start_line,
                        receiver_end_character,
                    )
                    .ok()
                    .flatten()
                    .and_then(|byte_offset| {
                        analysis
                            .type_at_byte_offset(V2FileId(1), byte_offset as u32)
                            .ok()
                            .flatten()
                    })
            })
    };
    let method = signature_help_handler::handle_signature_help_v2(
        Arc::from(content.to_string()),
        method_pos,
        receiver_type_hint,
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
async fn lsp_signature_help_flow_sensitive_receiver_hint_is_gated_by_flag() {
    let env = build_env();
    let content = "Процедура Тест()\n\
                   Перем x;\n\
                   Если ТипЗнч(x) = Тип(\"Строка\") Тогда\n\
                       x.Длина(\n\
                   КонецЕсли;\n\
                   КонецПроцедуры\n";
    let position = position_at_marker(content, "x.Длина(");

    let query = bsl_backend::application::type_system::signature_help_query(
        content,
        position.line,
        position.character,
    )
    .expect("signature help query");

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("test"),
        deps: env.deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("test"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("flow_sensitive_signature_help_gate.bsl"),
    });
    let analysis = host.snapshot();

    let receiver_end_character = query.receiver_end_character.expect("receiver end");
    let receiver_offset = analysis
        .utf16_position_to_byte_offset(V2FileId(1), query.call_start_line, receiver_end_character)
        .ok()
        .flatten()
        .expect("receiver offset");
    let receiver_offset = receiver_offset.min(u32::MAX as usize) as u32;

    let base_receiver_type_hint = analysis
        .type_at_byte_offset(V2FileId(1), receiver_offset)
        .ok()
        .flatten();
    let flow_receiver_type_hint = analysis
        .flow_type_at_byte_offset(V2FileId(1), receiver_offset)
        .ok()
        .flatten();

    let base = signature_help_handler::handle_signature_help_v2(
        Arc::from(content.to_string()),
        position,
        base_receiver_type_hint,
        env.deps.clone(),
    )
    .await;
    assert!(
        base.is_none(),
        "expected signature help to be absent without flow-sensitive receiver hint, got {:?}",
        base.as_ref()
            .and_then(|value| value.signatures.first().map(|sig| sig.label.as_str()))
    );

    let flow = signature_help_handler::handle_signature_help_v2(
        Arc::from(content.to_string()),
        position,
        flow_receiver_type_hint,
        env.deps,
    )
    .await
    .expect("signature help (flow-sensitive)");

    let label = flow
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(
        label.contains("Длина("),
        "expected flow-sensitive signature help to resolve method, got {}",
        label
    );
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
        None,
        deps,
        Position::new(0, 0),
        &uri,
        &index_snapshot,
        false,
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
