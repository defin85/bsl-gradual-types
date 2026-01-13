//! Completion handler for LSP
//!
//! Handles textDocument/completion requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use serde_json::json;
use tracing::error;

use bsl_backend::application::{
    get_completion_with_semantic_program_snapshot, get_completion_with_semantic_program_snapshot_v2,
};
use bsl_backend::application::CompletionStats;
use bsl_backend::application::type_system::{
    build_call_snippet, resolve_method_completion, resolve_type_details,
};
use bsl_backend::system::IndexSnapshot;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::ir::SemanticProgram;

pub struct CompletionResponseWithStats {
    pub response: CompletionResponse,
    pub stats: Option<CompletionStats>,
    pub had_error: bool,
}

pub async fn handle_completion_v2(
    file_content: Arc<str>,
    file_path: Arc<str>,
    ir_program: Arc<SemanticProgram>,
    parse_result: Option<Arc<bsl_syntax::ast::ParseResult>>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    position: Position,
    file_uri: &Url,
    index_snapshot: &IndexSnapshot,
    snippet_support: bool,
) -> Option<CompletionResponseWithStats> {
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let completion = match parse_result {
        Some(parse_result) => {
            get_completion_with_semantic_program_snapshot_v2(
                file_content.as_ref(),
                position.line,
                position.character,
                Some(file_uri.as_str()),
                index_snapshot,
                &metadata_lookup,
                file_path.as_ref(),
                resolver.as_ref(),
                ir_program,
                parse_result,
            )
            .await
        }
        None => {
            get_completion_with_semantic_program_snapshot(
                file_content.as_ref(),
                position.line,
                position.character,
                Some(file_uri.as_str()),
                index_snapshot,
                &metadata_lookup,
                file_path.as_ref(),
                resolver.as_ref(),
                ir_program,
            )
            .await
        }
    };

    match completion {
        Ok(result) => {
            let lsp_completions: Vec<CompletionItem> = result
                .items
                .into_iter()
                .map(|candidate| {
                    to_lsp_completion(
                        candidate.item,
                        candidate.owner_type,
                        candidate.origin_sources,
                        snippet_support,
                    )
                })
                .collect();
            Some(CompletionResponseWithStats {
                response: CompletionResponse::List(CompletionList {
                    is_incomplete: result.is_incomplete,
                    items: lsp_completions,
                }),
                stats: Some(result.stats),
                had_error: false,
            })
        }
        Err(e) => {
            error!("Failed to get completions (v2): {}", e);
            Some(CompletionResponseWithStats {
                response: CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: vec![],
                }),
                stats: None,
                had_error: true,
            })
        }
    }
}

pub async fn handle_completion_resolve(
    mut item: CompletionItem,
    deps: Option<Arc<bsl_analysis_v2::SemanticDeps>>,
    snippet_support: bool,
) -> CompletionItem {
    if item.detail.is_some() && item.documentation.is_some() && !snippet_support {
        return item;
    }

    let deps = match deps {
        Some(deps) => deps,
        None => return item,
    };

    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let (kind, owner_type) = parse_completion_data(&item);
    let resolved = match (kind.as_deref(), owner_type.as_deref()) {
        (Some("method"), Some(owner)) => {
            if let Some(signature) =
                deps.repository.find_method_signature(Some(owner), &item.label)
            {
                let detail = signature
                    .return_type
                    .clone()
                    .filter(|value| !value.is_empty());
                let documentation = signature.description.clone();
                let params: Vec<(String, bool)> = signature
                    .params
                    .iter()
                    .map(|param| (param.name.clone(), param.is_optional))
                    .collect();
                let insert_text = if snippet_support {
                    build_call_snippet(&signature.name, &params)
                } else {
                    None
                };

                Some((detail, documentation, insert_text))
            } else {
                resolve_method_completion(
                    owner,
                    &item.label,
                    &metadata_lookup,
                    snippet_support,
                )
                .map(|details| (details.detail, details.documentation, details.insert_text))
            }
        }
        (Some("function"), _) => {
            deps.repository
                .find_method_signature(None, &item.label)
                .map(|signature| {
                    let detail = signature
                        .return_type
                        .clone()
                        .filter(|value| !value.is_empty());
                    let documentation = signature.description.clone();
                    let params: Vec<(String, bool)> = signature
                        .params
                        .iter()
                        .map(|param| (param.name.clone(), param.is_optional))
                        .collect();
                    let insert_text = if snippet_support {
                        build_call_snippet(&signature.name, &params)
                    } else {
                        None
                    };

                    (detail, documentation, insert_text)
                })
        }
        (Some("type"), _) => resolve_type_details(&item.label, &metadata_lookup)
            .map(|(detail, documentation)| (detail, documentation, None)),
        _ => resolve_type_details(&item.label, &metadata_lookup)
            .map(|(detail, documentation)| (detail, documentation, None)),
    };

    if let Some((detail, documentation, insert_text)) = resolved {
        item.detail = detail;
        if let Some(doc) = documentation {
            item.documentation = Some(Documentation::String(doc));
        }
        if let Some(snippet) = insert_text {
            if snippet_support {
                item.insert_text = Some(snippet);
                item.insert_text_format = Some(InsertTextFormat::SNIPPET);
            }
        }
    }

    item
}

fn to_lsp_completion(
    item: bsl_shared::domain::CompletionItem,
    owner_type: Option<String>,
    origin_sources: Vec<u8>,
    snippet_support: bool,
) -> CompletionItem {
    let kind = map_completion_kind(item.kind);
    let mut insert_text = item.insert_text;
    let contains_snippet = insert_text
        .as_deref()
        .map(|text| text.contains("${"))
        .unwrap_or(false);
    if !snippet_support && contains_snippet {
        insert_text = Some(item.label.clone());
    }
    let insert_text_format = if snippet_support && contains_snippet {
        Some(InsertTextFormat::SNIPPET)
    } else {
        None
    };

    let data = Some(json!({
        "kind": completion_kind_tag(item.kind),
        "owner_type": owner_type,
        "origin_sources": origin_sources,
    }));

    CompletionItem {
        label: item.label,
        kind,
        detail: None,
        documentation: None,
        insert_text,
        insert_text_format,
        filter_text: item.filter_text,
        sort_text: item.sort_text,
        data,
        ..Default::default()
    }
}

fn completion_kind_tag(kind: bsl_shared::domain::CompletionKind) -> &'static str {
    use bsl_shared::domain::CompletionKind::*;
    match kind {
        Method => "method",
        Function => "function",
        Type | Class | Struct | Catalog | Document | Enum => "type",
        Keyword => "keyword",
        _ => "other",
    }
}

fn map_completion_kind(kind: bsl_shared::domain::CompletionKind) -> Option<CompletionItemKind> {
    use bsl_shared::domain::CompletionKind::*;
    Some(match kind {
        Method => CompletionItemKind::METHOD,
        Function => CompletionItemKind::FUNCTION,
        Constructor => CompletionItemKind::CONSTRUCTOR,
        Field => CompletionItemKind::FIELD,
        Variable => CompletionItemKind::VARIABLE,
        Class | Type => CompletionItemKind::CLASS,
        Interface => CompletionItemKind::INTERFACE,
        Module | Global => CompletionItemKind::MODULE,
        Property => CompletionItemKind::PROPERTY,
        Unit => CompletionItemKind::UNIT,
        Value => CompletionItemKind::VALUE,
        Enum => CompletionItemKind::ENUM,
        EnumMember => CompletionItemKind::ENUM_MEMBER,
        Keyword => CompletionItemKind::KEYWORD,
        Snippet => CompletionItemKind::SNIPPET,
        Color => CompletionItemKind::COLOR,
        File => CompletionItemKind::FILE,
        Reference => CompletionItemKind::REFERENCE,
        Folder => CompletionItemKind::FOLDER,
        Constant => CompletionItemKind::CONSTANT,
        Struct => CompletionItemKind::STRUCT,
        Event => CompletionItemKind::EVENT,
        Operator => CompletionItemKind::OPERATOR,
        TypeParameter => CompletionItemKind::TYPE_PARAMETER,
        Text => CompletionItemKind::TEXT,
        Catalog | Document => CompletionItemKind::CLASS,
    })
}

fn parse_completion_data(item: &CompletionItem) -> (Option<String>, Option<String>) {
    let data = match item.data.as_ref() {
        Some(value) => value,
        None => return (None, None),
    };

    let kind = data
        .get("kind")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let owner_type = data
        .get("owner_type")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());

    (kind, owner_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use bsl_backend::system::IntellisenseIndexStore;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::TypeRepository;
    use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource};
    use bsl_shared::domain::types::{ParameterInfo, RawTypeData, RawDataSource};
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
            ..Default::default()
        };
        repository_impl
            .load_types(vec![raw_type])
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
        repository_impl.set_signature_index(index);

        let repository =
            repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let index = Arc::new(IntellisenseIndexStore::new("test", "test"));
        let deps = Arc::new(bsl_analysis_v2::SemanticDeps {
            signature_index: repository.get_signature_index_clone(),
            resolver: Some(resolver),
            repository,
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
        let file_content = analysis.file_text(file_id).ok().flatten().expect("file_text");
        let file_path = analysis.file_path(file_id).ok().flatten().expect("file_path");
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

        let (file_content, file_path, ir_program) =
            build_v2_ir(&content, &uri, deps.clone());
        let v2 = handle_completion_v2(
            file_content.clone(),
            file_path.clone(),
            ir_program.clone(),
            None,
            deps.clone(),
            position,
            &uri,
            &index_snapshot,
            true,
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
            deps,
            position,
            &uri,
            &index_snapshot,
            true,
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

        let (file_content, file_path, ir_program) =
            build_v2_ir(&content, &uri, deps.clone());
        let response = handle_completion_v2(
            file_content,
            file_path,
            ir_program,
            None,
            deps.clone(),
            position,
            &uri,
            &index_snapshot,
            true,
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

        let resolved_true =
            handle_completion_resolve(item.clone(), Some(deps.clone()), true).await;
        let resolved_false =
            handle_completion_resolve(item.clone(), Some(deps), false).await;

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
}
