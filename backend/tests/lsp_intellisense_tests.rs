//! Интеграционные тесты LSP IntelliSense (completion/resolve/signatureHelp).

mod intellisense_testkit;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

#[path = "../src/bin/lsp_server/handlers/signature_help.rs"]
mod signature_help_handler;

use std::sync::Arc;

use bsl_analysis_v2::{
    AnalysisHostV2, Change as ChangeV2, DepsSnapshotId, FileId as V2FileId,
    SemanticDiagnosticsMaterializationPath, SettingsId,
};
use bsl_backend::system::{
    IndexItem, IndexItemKind, IndexKind, IndexSnapshot, IntellisenseIndexStore, TypeKind,
};
use bsl_runtime::application::type_system::signature_help_query;
use bsl_runtime::system::LineIndex;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::signature_index::{
    ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::types::{
    ParameterInfo, RawDataSource, RawMethodData, RawParamData, RawTypeData, TypeResolution,
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
        platform_signatures_loaded: false,
        common_module_factory_registry: Default::default(),
        global_context_index: Default::default(),
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

fn build_v2_analysis_ir(
    content: &str,
    uri: &Url,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> (
    bsl_analysis_v2::AnalysisV2,
    V2FileId,
    Arc<str>,
    Arc<SemanticProgram>,
) {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("test-analysis"),
        deps,
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("test-analysis"),
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
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let file_content = analysis
        .file_text(file_id)
        .ok()
        .flatten()
        .expect("file_text");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

    (analysis, file_id, file_content, ir_program)
}

#[tokio::test]
async fn lsp_completion_static_receiver_without_owner_hint_returns_items_and_stats() {
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

    assert!(
        items.iter().any(|item| item.label == "Добавить"),
        "items: {:?}",
        items
    );
}

#[tokio::test]
async fn lsp_completion_unknown_member_access_without_owner_hint_is_fail_closed() {
    let env = build_env();
    let content = "МойМассив.";
    let position = position_at_marker(content, "МойМассив.");
    let uri = Url::parse("file:///m8_lsp_completion_unknown_owner.bsl").expect("url");

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
        false,
    )
    .await
    .expect("completion response");

    assert!(!response.had_error);
    assert!(response.stats.is_none());

    let items = match response.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };

    assert!(items.is_empty(), "items: {:?}", items);
}

#[tokio::test]
async fn lsp_completion_returns_items_and_stats_with_owner_hint() {
    let env = build_env();
    let content = "Массив.";
    let position = position_at_marker(content, "Массив.");
    let uri = Url::parse("file:///m8_lsp_completion.bsl").expect("url");

    let (file_content, file_path, ir_program) = build_v2_ir(content, &uri, env.deps.clone());
    let response = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        Some(TypeResolution::explicit("Массив")),
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
        Some(TypeResolution::explicit("Массив")),
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
async fn lsp_signature_help_keeps_method_semantic_facts_and_fail_closes_constructor_without_canonical_fact(
) {
    let env = build_env();
    let content = r#"Процедура Тест()
    Новый Массив(1, )
    МойМассив = Новый Массив
    МойМассив.Добавить(1, )
КонецПроцедуры"#;
    let uri = Url::parse("file:///test_signature_help_v2.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, env.deps.clone());

    let constructor_pos = position_at_marker(content, "Новый Массив(1, ");
    let method_pos = position_at_marker(content, "МойМассив.Добавить(1, ");

    let constructor = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content.clone(),
        constructor_pos,
        ir_program.clone(),
        env.deps.clone(),
        None,
    );
    let method = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        method_pos,
        ir_program,
        env.deps,
        None,
    )
    .expect("method signature help");

    assert!(
        constructor.is_none(),
        "constructor signature help must stay fail-closed when canonical semantic facts are absent"
    );

    let method_label = method
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(method_label.contains("Добавить("));
    assert_eq!(method.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_keeps_method_semantic_facts_with_empty_request_time_repository() {
    let env = build_env();
    let content = r#"Процедура Тест()
    Новый Массив(1, )
    МойМассив = Новый Массив
    МойМассив.Добавить(1, )
КонецПроцедуры"#;
    let uri = Url::parse("file:///test_signature_help_v2_empty_repo.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, env.deps.clone());
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));
    let constructor_pos = position_at_marker(content, "Новый Массив(1, ");
    let method_pos = position_at_marker(content, "МойМассив.Добавить(1, ");
    let constructor_query =
        signature_help_query(content, constructor_pos.line, constructor_pos.character)
            .expect("constructor query");
    let constructor_call_offset = LineIndex::new(content)
        .utf16_position_to_byte_offset(
            content,
            constructor_query.call_start_line,
            constructor_query.call_start_character,
        )
        .min(u32::MAX as usize) as u32;
    let constructor_fact = ir_program
        .semantic_facts
        .constructor_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(constructor_call_offset)
                && target.type_name.eq_ignore_ascii_case("Массив")
        });
    assert!(
        constructor_fact.is_none(),
        "canonical IR must not materialize constructor semantic fact from incomplete recovery; offset={constructor_call_offset}; spans={:?}",
        ir_program
            .semantic_facts
            .constructor_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );
    let method_query =
        signature_help_query(content, method_pos.line, method_pos.character).expect("method query");
    let method_call_offset = LineIndex::new(content)
        .utf16_position_to_byte_offset(
            content,
            method_query.call_start_line,
            method_query.call_start_character,
        )
        .min(u32::MAX as usize) as u32;
    let method_fact = ir_program
        .semantic_facts
        .call_method_targets_by_span
        .iter()
        .find(|(span, target)| {
            [method_call_offset.saturating_sub(1), method_call_offset]
                .into_iter()
                .any(|offset| span.contains(offset))
                && target.method_name.eq_ignore_ascii_case("Добавить")
                && target.signature.is_some()
        });
    assert!(
        method_fact.is_some(),
        "missing method semantic fact at offset {method_call_offset}; spans={:?}",
        ir_program
            .semantic_facts
            .call_method_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );

    let constructor = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content.clone(),
        constructor_pos,
        ir_program.clone(),
        stripped_deps.clone(),
        None,
    );
    assert!(
        constructor.is_none(),
        "without request-time deps, incomplete constructor signature help must stay unavailable because canonical IR no longer synthesizes recovery target"
    );

    let method = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        method_pos,
        ir_program,
        stripped_deps,
        None,
    )
    .expect("method signature help from semantic facts");

    let method_label = method
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(method_label.contains("Добавить("));
    assert_eq!(method.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_uses_exact_semantic_index_when_runtime_ir_facts_are_missing() {
    let env = build_env();
    let content = r#"Процедура Тест()
    МойМассив = Новый Массив
    МойМассив.Добавить(1, )
КонецПроцедуры"#;
    let uri = Url::parse("file:///test_signature_help_v2_exact_index.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, env.deps.clone());
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));
    let method_pos = position_at_marker(content, "МойМассив.Добавить(1, ");

    let mut poisoned_program = ir_program.as_ref().clone();
    poisoned_program.semantic_facts = Default::default();
    let poisoned_ir = Arc::new(poisoned_program);

    let method = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        method_pos,
        poisoned_ir,
        stripped_deps,
        None,
    )
    .expect("method signature help from exact semantic index");

    let method_label = method
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(method_label.contains("Добавить("), "label={method_label}");
    assert_eq!(method.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_uses_exact_semantic_index_after_diagnostics_only_query() {
    let env = build_env();
    let content = r#"Процедура Тест()
    МойМассив = Новый Массив();
    МойМассив.Добавить(1, 2);
КонецПроцедуры"#;
    let uri =
        Url::parse("file:///test_signature_help_v2_after_diagnostics_only.bsl").expect("test uri");

    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("signature-help-after-diagnostics-only"),
        deps: env.deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("signature-help-after-diagnostics-only"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from(
            uri.to_file_path()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| uri.to_string()),
        ),
    });

    let analysis = host.analysis();
    let file_id = V2FileId(1);
    let profiled = analysis
        .semantic_diagnostics_profiled(file_id)
        .expect("semantic diagnostics profile")
        .expect("semantic diagnostics result");
    assert_eq!(
        profiled.profile.materialization_path,
        Some(SemanticDiagnosticsMaterializationPath::DiagnosticsOnly)
    );

    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let file_content = analysis
        .file_text(file_id)
        .ok()
        .flatten()
        .expect("file_text");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));
    let method_pos = position_at_marker(content, "МойМассив.Добавить(1, ");

    let mut poisoned_program = ir_program.as_ref().clone();
    poisoned_program.semantic_facts = Default::default();
    let poisoned_ir = Arc::new(poisoned_program);

    let method = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        method_pos,
        poisoned_ir,
        stripped_deps,
        None,
    )
    .expect("method signature help from exact semantic index after diagnostics-only query");

    let method_label = method
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(method_label.contains("Добавить("), "label={method_label}");
    assert_eq!(method.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_uses_semantic_facts_for_local_function_with_empty_request_time_repository(
) {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let deps = build_deps(repository);
    let content = concat!(
        "Функция Локальная(Аргумент, Доп = Неопределено)\n",
        "    Возврат Аргумент;\n",
        "КонецФункции\n",
        "\n",
        "Процедура Тест()\n",
        "    Локальная(1, );\n",
        "КонецПроцедуры\n"
    );
    let uri = Url::parse("file:///test_signature_help_local_empty_repo.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, deps.clone());
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));

    let local_pos = position_at_marker(content, "Локальная(1, ");
    let local_query =
        signature_help_query(content, local_pos.line, local_pos.character).expect("local query");
    let local_call_offset = LineIndex::new(content)
        .utf16_position_to_byte_offset(
            content,
            local_query.call_start_line,
            local_query.call_start_character,
        )
        .min(u32::MAX as usize) as u32;
    let local_fact = ir_program
        .semantic_facts
        .call_method_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(local_call_offset)
                && target.method_name.eq_ignore_ascii_case("Локальная")
                && target.signature.is_some()
        });
    assert!(
        local_fact.is_some(),
        "missing local callable semantic fact at offset {local_call_offset}; spans={:?}",
        ir_program
            .semantic_facts
            .call_method_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );

    let local = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        local_pos,
        ir_program,
        stripped_deps,
        None,
    )
    .expect("local signature help from semantic facts");

    let local_label = local
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(local_label.contains("Локальная("), "label={local_label}");
    assert!(local_label.contains("Аргумент"), "label={local_label}");
    assert!(local_label.contains("Доп"), "label={local_label}");
    assert_eq!(local.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_uses_exact_semantic_index_for_local_function_when_runtime_ir_facts_are_missing(
) {
    let repository = Arc::new(InMemoryTypeRepository::new());
    let deps = build_deps(repository);
    let content = concat!(
        "Функция Локальная(Аргумент, Доп = Неопределено)\n",
        "    Возврат Аргумент;\n",
        "КонецФункции\n",
        "\n",
        "Процедура Тест()\n",
        "    Локальная(1, );\n",
        "КонецПроцедуры\n"
    );
    let uri = Url::parse("file:///test_signature_help_local_exact_index.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, deps.clone());
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));
    let local_pos = position_at_marker(content, "Локальная(1, ");

    let mut poisoned_program = ir_program.as_ref().clone();
    poisoned_program.semantic_facts = Default::default();
    let poisoned_ir = Arc::new(poisoned_program);

    let local = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        local_pos,
        poisoned_ir,
        stripped_deps,
        None,
    )
    .expect("local signature help from exact semantic index");

    let local_label = local
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(local_label.contains("Локальная("), "label={local_label}");
    assert!(local_label.contains("Аргумент"), "label={local_label}");
    assert!(local_label.contains("Доп"), "label={local_label}");
    assert_eq!(local.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_uses_semantic_facts_for_global_function_with_empty_request_time_repository(
) {
    let repository = build_repository_with_array();
    let global_signature = MethodSignature::new(
        "ГлобальнаяФункция".to_string(),
        None,
        vec![
            ParameterInfo {
                name: "Первый".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            },
            ParameterInfo {
                name: "Второй".to_string(),
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
    );
    repository.add_global_function_signature("ГлобальнаяФункция", global_signature);
    let deps = build_deps(repository);
    let content = concat!(
        "Процедура Тест()\n",
        "    ГлобальнаяФункция(1, );\n",
        "КонецПроцедуры\n"
    );
    let uri = Url::parse("file:///test_signature_help_global_empty_repo.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, deps.clone());
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));

    let global_pos = position_at_marker(content, "ГлобальнаяФункция(1, ");
    let global_query =
        signature_help_query(content, global_pos.line, global_pos.character).expect("global query");
    let global_call_offset = LineIndex::new(content)
        .utf16_position_to_byte_offset(
            content,
            global_query.call_start_line,
            global_query.call_start_character,
        )
        .min(u32::MAX as usize) as u32;
    let global_fact = ir_program
        .semantic_facts
        .call_method_targets_by_span
        .iter()
        .find(|(span, target)| {
            span.contains(global_call_offset)
                && target.method_name.eq_ignore_ascii_case("ГлобальнаяФункция")
                && target.signature.is_some()
        });
    assert!(
        global_fact.is_some(),
        "missing global callable semantic fact at offset {global_call_offset}; spans={:?}",
        ir_program
            .semantic_facts
            .call_method_targets_by_span
            .keys()
            .collect::<Vec<_>>()
    );

    let global = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        global_pos,
        ir_program,
        stripped_deps,
        None,
    )
    .expect("global signature help from semantic facts");

    let global_label = global
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(
        global_label.contains("ГлобальнаяФункция("),
        "label={global_label}"
    );
    assert!(global_label.contains("Первый"), "label={global_label}");
    assert!(global_label.contains("Второй"), "label={global_label}");
    assert_eq!(global.active_parameter, Some(1));
}

#[tokio::test]
async fn lsp_signature_help_uses_exact_semantic_index_for_global_function_when_runtime_ir_facts_are_missing(
) {
    let repository = build_repository_with_array();
    let global_signature = MethodSignature::new(
        "ГлобальнаяФункция".to_string(),
        None,
        vec![
            ParameterInfo {
                name: "Первый".to_string(),
                type_name: Some("Число".to_string()),
                is_optional: false,
                default_value: None,
                description: None,
            },
            ParameterInfo {
                name: "Второй".to_string(),
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
    );
    repository.add_global_function_signature("ГлобальнаяФункция", global_signature);
    let deps = build_deps(repository);
    let content = concat!(
        "Процедура Тест()\n",
        "    ГлобальнаяФункция(1, );\n",
        "КонецПроцедуры\n"
    );
    let uri = Url::parse("file:///test_signature_help_global_exact_index.bsl").expect("test uri");
    let (analysis, file_id, file_content, ir_program) =
        build_v2_analysis_ir(content, &uri, deps.clone());
    let stripped_deps = build_deps(Arc::new(InMemoryTypeRepository::new()));
    let global_pos = position_at_marker(content, "ГлобальнаяФункция(1, ");

    let mut poisoned_program = ir_program.as_ref().clone();
    poisoned_program.semantic_facts = Default::default();
    let poisoned_ir = Arc::new(poisoned_program);

    let global = signature_help_handler::handle_signature_help_v2(
        &analysis,
        file_id,
        file_content,
        global_pos,
        poisoned_ir,
        stripped_deps,
        None,
    )
    .expect("global signature help from exact semantic index");

    let global_label = global
        .signatures
        .first()
        .map(|sig| sig.label.as_str())
        .unwrap_or("");
    assert!(
        global_label.contains("ГлобальнаяФункция("),
        "label={global_label}"
    );
    assert!(global_label.contains("Первый"), "label={global_label}");
    assert!(global_label.contains("Второй"), "label={global_label}");
    assert_eq!(global.active_parameter, Some(1));
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

#[tokio::test]
async fn lsp_completion_non_member_excludes_locals_from_other_procedure() {
    let env = build_env();
    let content = concat!(
        "Процедура Первая()\n",
        "    ЛокалПервая = 1;\n",
        "    Лок\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Вторая()\n",
        "    ЛокалВторая = 2;\n",
        "КонецПроцедуры\n"
    );
    let uri = Url::parse("file:///m8_lsp_local_scope_split.bsl").expect("url");

    let position = Position {
        line: 2,
        character: "    Лок".chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32,
    };

    let (file_content, file_path, ir_program) = build_v2_ir(content, &uri, env.deps.clone());
    let response = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        None,
        env.deps,
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
    let labels: Vec<String> = items.into_iter().map(|item| item.label).collect();

    assert!(
        labels.iter().any(|label| label == "ЛокалПервая"),
        "labels: {:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ЛокалВторая"),
        "labels: {:?}",
        labels
    );
}
