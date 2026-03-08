use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use bsl_backend::system::IntellisenseIndexStore;
use bsl_shared::domain::repository::InMemoryTypeRepository;
use bsl_shared::domain::signature_index::{
    ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource,
};
use bsl_shared::domain::types::{ParameterInfo, RawDataSource, RawPropertyData, RawTypeData};
use bsl_shared::TypeRepository;
use bsl_shared::TypeResolver;
use tower_lsp::lsp_types::Url;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).expect("fixture read")
}

fn find_position(content: &str, marker: &str) -> Position {
    let byte_index = content.find(marker).expect("marker not found");
    let before = &content[..byte_index + marker.len()];
    let line = before.lines().count() - 1;
    let last_line = before.lines().last().unwrap_or("");
    let character = last_line.chars().map(|ch| ch.len_utf16()).sum::<usize>();
    Position {
        line: line as u32,
        character: character as u32,
    }
}

struct TestEnv {
    index: Arc<IntellisenseIndexStore>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
}

fn create_test_env() -> TestEnv {
    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    let raw_type = RawTypeData {
        name: "Массив".to_string(),
        source: RawDataSource::Platform,
        properties: vec![RawPropertyData {
            name: "Длина".to_string(),
            prop_type: "Число".to_string(),
            is_readonly: true,
        }],
        ..Default::default()
    };
    let metadata_type = RawTypeData {
        name: "Документы.ТестДок".to_string(),
        description: "Описание тестового документа".to_string(),
        source: RawDataSource::Configuration,
        ..Default::default()
    };
    repository_impl
        .load_types(vec![raw_type, metadata_type])
        .expect("load types");

    let mut index = SignatureIndex::new();
    let method = MethodSignature::new(
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
    );
    index.add_platform_method(bsl_shared::domain::type_id::TypeId::new("Массив"), method);
    index.add_constructor(
        bsl_shared::domain::type_id::TypeId::new("Массив"),
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

    let global_function = MethodSignature::new(
        "Дубль".to_string(),
        None,
        vec![ParameterInfo {
            name: "Значение".to_string(),
            type_name: Some("Число".to_string()),
            is_optional: false,
            default_value: None,
            description: None,
        }],
        Some("Число".to_string()),
        None,
        None,
        SignatureSource::Platform,
        None,
        Default::default(),
    );
    index.add_global_function(
        bsl_shared::domain::type_id::TypeId::new("Дубль"),
        global_function,
    );
    repository_impl.set_signature_index(index);

    let repository =
        repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let index = Arc::new(IntellisenseIndexStore::new("test", "test"));
    let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
        signature_index: repository.get_signature_index_clone(),
        resolver: Some(resolver),
        repository,
        platform_signatures_loaded: false,
    });

    TestEnv { index, deps }
}

fn snapshot_path(name: &str) -> PathBuf {
    golden_path(name)
}

fn assert_snapshot(name: &str, value: &serde_json::Value) {
    let path = snapshot_path(name);
    let json = serde_json::to_string_pretty(value).expect("snapshot json");
    if std::env::var("UPDATE_GOLDEN").ok().as_deref() == Some("1") {
        fs::create_dir_all(path.parent().expect("golden dir")).expect("create golden dir");
        fs::write(&path, json).expect("write golden");
        return;
    }
    let expected = fs::read_to_string(&path).expect("read golden");
    assert_eq!(expected, json);
}

fn completion_kind(kind: Option<CompletionItemKind>) -> Option<&'static str> {
    match kind {
        Some(CompletionItemKind::METHOD) => Some("METHOD"),
        Some(CompletionItemKind::FUNCTION) => Some("FUNCTION"),
        Some(CompletionItemKind::CLASS) => Some("CLASS"),
        Some(CompletionItemKind::KEYWORD) => Some("KEYWORD"),
        Some(CompletionItemKind::PROPERTY) => Some("PROPERTY"),
        _ => None,
    }
}

fn insert_text_format(format: Option<InsertTextFormat>) -> Option<&'static str> {
    match format {
        Some(InsertTextFormat::SNIPPET) => Some("SNIPPET"),
        Some(InsertTextFormat::PLAIN_TEXT) => Some("PLAIN_TEXT"),
        _ => None,
    }
}

fn completion_items_snapshot(items: &[CompletionItem]) -> serde_json::Value {
    serde_json::Value::Array(
        items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "label": item.label,
                    "kind": completion_kind(item.kind),
                    "sortText": item.sort_text,
                    "filterText": item.filter_text,
                    "insertText": item.insert_text,
                    "insertTextFormat": insert_text_format(item.insert_text_format),
                    "data": item.data,
                })
            })
            .collect(),
    )
}

fn extract_items(response: CompletionResponse) -> Vec<CompletionItem> {
    match response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    }
}

#[test]
fn metadata_completion_kinds_have_unique_lsp_kinds() {
    use bsl_shared::domain::CompletionKind::*;

    let metadata_kinds = [
        MetadataUnknown,
        Catalog,
        Document,
        Register,
        Report,
        DataProcessor,
        Enum,
        ChartOfAccounts,
        ChartOfCharacteristicTypes,
        ChartOfCalculationTypes,
        InformationRegister,
        AccumulationRegister,
        AccountingRegister,
        CalculationRegister,
        BusinessProcess,
        Task,
        ExchangePlan,
        Constant,
        CommonModule,
        Role,
        Subsystem,
        Language,
    ];

    let mut seen: Vec<CompletionItemKind> = Vec::new();
    for kind in metadata_kinds {
        let mapped = map_completion_kind(kind).expect("metadata kind should map");
        assert!(
            !seen.contains(&mapped),
            "Duplicate LSP kind mapping for metadata completion kind: {:?} -> {:?}",
            kind,
            mapped
        );
        seen.push(mapped);
    }
}

#[test]
fn metadata_completion_items_have_granular_kind_in_data() {
    let item = bsl_shared::domain::CompletionItem::with_details(
        "Регистр".to_string(),
        bsl_shared::domain::CompletionKind::InformationRegister,
        Some("Регистр сведений".to_string()),
        None,
    );
    let lsp_item = to_lsp_completion(item, None, None, vec![], false, None);
    let kind = lsp_item
        .data
        .as_ref()
        .and_then(|value| value.get("kind"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(kind, "metadata.information_register");
}

#[test]
fn method_completion_items_keep_method_kind_in_data() {
    let item = bsl_shared::domain::CompletionItem::new(
        "Добавить".to_string(),
        bsl_shared::domain::CompletionKind::Method,
    );
    let lsp_item = to_lsp_completion(item, None, None, vec![], false, None);
    let kind = lsp_item
        .data
        .as_ref()
        .and_then(|value| value.get("kind"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(kind, "method");
}

#[test]
fn property_completion_items_keep_property_kind_in_data() {
    let item = bsl_shared::domain::CompletionItem::new(
        "Длина".to_string(),
        bsl_shared::domain::CompletionKind::Property,
    );
    let lsp_item = to_lsp_completion(item, Some("Массив".to_string()), None, vec![0], false, None);
    let kind = lsp_item
        .data
        .as_ref()
        .and_then(|value| value.get("kind"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(kind, "property");

    let candidate_id = parse_candidate_id(&lsp_item).expect("candidate_id");
    match candidate_id.payload {
        CompletionCandidateIdPayload::Property {
            owner_type,
            name,
            member_identity,
        } => {
            assert_eq!(owner_type, "Массив");
            assert_eq!(name, "Длина");
            assert_eq!(member_identity, None);
        }
        other => panic!("expected property candidate_id, got {:?}", other),
    }
}

#[test]
fn structural_property_completion_items_include_member_identity_in_data() {
    let item = bsl_shared::domain::CompletionItem::new(
        "Идентификатор".to_string(),
        bsl_shared::domain::CompletionKind::Property,
    );
    let lsp_item = to_lsp_completion(
        item,
        Some("Структура".to_string()),
        Some("struct:field:1".to_string()),
        vec![0],
        false,
        None,
    );

    let member_identity = lsp_item
        .data
        .as_ref()
        .and_then(|value| value.get("member_identity"))
        .and_then(|value| value.as_str());
    assert_eq!(member_identity, Some("struct:field:1"));

    let candidate_id = parse_candidate_id(&lsp_item).expect("candidate_id");
    match candidate_id.payload {
        CompletionCandidateIdPayload::Property {
            owner_type,
            name,
            member_identity,
        } => {
            assert_eq!(owner_type, "Структура");
            assert_eq!(name, "Идентификатор");
            assert_eq!(member_identity.as_deref(), Some("struct:field:1"));
        }
        other => panic!("expected property candidate_id, got {:?}", other),
    }
}

fn build_v2_ir(
    content: &str,
    uri: &Url,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> (Arc<str>, Arc<str>, Arc<SemanticProgram>) {
    let mut host = bsl_analysis_v2::AnalysisHostV2::default();
    host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
        deps_id: bsl_analysis_v2::DepsSnapshotId::from_hash("test"),
        deps: deps.clone(),
    });

    let path = uri
        .to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| uri.to_string());
    let file_id = bsl_analysis_v2::FileId(1);
    host.apply_change(bsl_analysis_v2::Change::SetFile {
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
async fn m5_completion_v2_is_deterministic() {
    let content = read_fixture("m5_snippets_resolve.bsl");
    let position = find_position(&content, "Массив.");
    let uri = Url::parse("file:///m5_snippets_resolve.bsl").expect("url");
    let env = create_test_env();
    let index = env.index.clone();
    let index_snapshot = index.snapshot();
    let deps = env.deps.clone();

    let (file_content, file_path, ir_program) = build_v2_ir(&content, &uri, deps.clone());
    let v2 = handle_completion_v2(
        file_content.clone(),
        file_path.clone(),
        ir_program.clone(),
        None,
        None,
        deps.clone(),
        position,
        &uri,
        &index_snapshot,
        true,
        false,
    )
    .await
    .expect("completion v2");
    let v2_items = extract_items(v2.response);
    assert!(
        v2_items.iter().any(|item| item.label == "Добавить"),
        "expected v2 completion to contain 'Добавить'"
    );

    let v2_snapshot = completion_items_snapshot(&v2_items);

    // Determinism smoke: same input -> same output twice.
    let v2_second = handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        None,
        None,
        deps,
        position,
        &uri,
        &index_snapshot,
        true,
        false,
    )
    .await
    .expect("completion v2 (second)");
    let v2_second_items = extract_items(v2_second.response);
    let v2_second_snapshot = completion_items_snapshot(&v2_second_items);
    assert_eq!(v2_snapshot, v2_second_snapshot);
}

#[tokio::test]
async fn m5_completion_resolve_snippets_snapshot() {
    let content = read_fixture("m5_snippets_resolve.bsl");
    let position = find_position(&content, "Массив.");
    let uri = Url::parse("file:///m5_snippets_resolve.bsl").expect("url");
    let env = create_test_env();
    let index_snapshot = env.index.snapshot();
    let deps = env.deps;

    let (file_content, file_path, ir_program) = build_v2_ir(&content, &uri, deps.clone());
    let response = handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        None,
        None,
        deps.clone(),
        position,
        &uri,
        &index_snapshot,
        true,
        false,
    )
    .await
    .expect("completion");

    let items = match response.response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(list) => list,
    };

    let item = items
        .into_iter()
        .find(|entry| entry.label == "Добавить")
        .expect("Добавить completion");

    let resolved_true = handle_completion_resolve(item.clone(), Some(deps.clone()), true).await;
    let resolved_false = handle_completion_resolve(item.clone(), Some(deps), false).await;

    let snapshot = serde_json::json!({
        "completion": {
            "label": item.label,
            "kind": completion_kind(item.kind),
            "insertText": item.insert_text,
            "insertTextFormat": insert_text_format(item.insert_text_format),
        },
        "resolveSnippetSupportTrue": {
            "label": resolved_true.label,
            "detail": resolved_true.detail,
            "hasDocumentation": resolved_true.documentation.is_some(),
            "insertText": resolved_true.insert_text,
            "insertTextFormat": insert_text_format(resolved_true.insert_text_format),
        },
        "resolveSnippetSupportFalse": {
            "label": resolved_false.label,
            "detail": resolved_false.detail,
            "hasDocumentation": resolved_false.documentation.is_some(),
            "insertText": resolved_false.insert_text,
            "insertTextFormat": insert_text_format(resolved_false.insert_text_format),
        },
    });

    assert_snapshot("m5_completion_resolve_snippets.json", &snapshot);
}

#[tokio::test]
async fn m6_completion_resolve_uses_candidate_id_for_function_origin() {
    let env = create_test_env();
    let deps = env.deps.clone();

    let file_symbol = to_lsp_completion(
        bsl_shared::domain::CompletionItem::new(
            "Дубль".to_string(),
            bsl_shared::domain::CompletionKind::Function,
        ),
        None,
        None,
        vec![0],
        false,
        Some(deps.as_ref()),
    );
    let module_symbol = to_lsp_completion(
        bsl_shared::domain::CompletionItem::new(
            "Дубль".to_string(),
            bsl_shared::domain::CompletionKind::Function,
        ),
        None,
        None,
        vec![1],
        false,
        Some(deps.as_ref()),
    );

    let file_resolved = handle_completion_resolve(file_symbol, Some(deps.clone()), false).await;
    let module_resolved = handle_completion_resolve(module_symbol, Some(deps), false).await;

    assert_eq!(
        file_resolved.detail, None,
        "file-level symbol should not resolve to global signature"
    );
    assert_eq!(
        module_resolved.detail.as_deref(),
        Some("Число"),
        "module/global function should resolve via SignatureIndex"
    );
}

#[tokio::test]
async fn m6_completion_resolve_uses_candidate_id_for_property() {
    let env = create_test_env();
    let deps = env.deps.clone();

    let item = to_lsp_completion(
        bsl_shared::domain::CompletionItem::new(
            "Длина".to_string(),
            bsl_shared::domain::CompletionKind::Property,
        ),
        Some("Массив".to_string()),
        None,
        vec![0],
        false,
        Some(deps.as_ref()),
    );

    let resolved = handle_completion_resolve(item, Some(deps), false).await;
    assert_eq!(resolved.detail.as_deref(), Some("Число"));
}

#[tokio::test]
async fn m6_completion_resolve_uses_candidate_id_for_metadata() {
    let env = create_test_env();
    let deps = env.deps.clone();

    let item = to_lsp_completion(
        bsl_shared::domain::CompletionItem::new(
            "ТестДок".to_string(),
            bsl_shared::domain::CompletionKind::Document,
        ),
        None,
        None,
        vec![2],
        false,
        Some(deps.as_ref()),
    );

    let resolved = handle_completion_resolve(item, Some(deps), false).await;
    assert_eq!(resolved.detail.as_deref(), Some("Документ"));
    assert!(resolved.documentation.is_some());
}

#[tokio::test]
async fn m6_completion_resolve_dedup_sources_prefers_local_function() {
    let env = create_test_env();
    let deps = env.deps.clone();

    let deduped = to_lsp_completion(
        bsl_shared::domain::CompletionItem::new(
            "Дубль".to_string(),
            bsl_shared::domain::CompletionKind::Function,
        ),
        None,
        None,
        vec![0, 1],
        false,
        Some(deps.as_ref()),
    );

    let resolved = handle_completion_resolve(deduped, Some(deps), false).await;
    assert_eq!(
        resolved.detail, None,
        "deduped local+module function should not resolve to global signature"
    );
}

#[tokio::test]
async fn m6_completion_resolve_legacy_fallback_works_without_candidate_id() {
    let env = create_test_env();
    let deps = env.deps.clone();

    let mut legacy = to_lsp_completion(
        bsl_shared::domain::CompletionItem::new(
            "Добавить".to_string(),
            bsl_shared::domain::CompletionKind::Method,
        ),
        Some("Массив".to_string()),
        None,
        vec![0],
        false,
        Some(deps.as_ref()),
    );
    if let Some(value) = legacy.data.as_mut() {
        if let Some(obj) = value.as_object_mut() {
            obj.remove("candidate_id");
        }
    }

    let resolved = handle_completion_resolve(legacy, Some(deps), false).await;
    assert_eq!(resolved.detail, None);
}
