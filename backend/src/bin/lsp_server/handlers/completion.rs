//! Completion handler for LSP
//!
//! Handles textDocument/completion requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use serde_json::json;
use tracing::{error, info};

use bsl_backend::application::TypeSystemService;
use bsl_backend::application::CompletionStats;

pub struct CompletionResponseWithStats {
    pub response: CompletionResponse,
    pub stats: Option<CompletionStats>,
}

/// Handle textDocument/completion request
pub async fn handle_completion(
    file_content: &str,
    position: Position,
    file_uri: &Url,
    type_service: Option<Arc<TypeSystemService>>,
    snippet_support: bool,
) -> Option<CompletionResponseWithStats> {
    info!(
        "Completion requested at {}:{}",
        position.line, position.character
    );

    if let Some(service) = type_service {
        match service
            .get_completion(
                file_content,
                position.line,
                position.character,
                Some(file_uri.as_str()),
            )
            .await
        {
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
                info!("Returning {} completions", lsp_completions.len());
                Some(CompletionResponseWithStats {
                    response: CompletionResponse::List(CompletionList {
                        is_incomplete: result.is_incomplete,
                        items: lsp_completions,
                    }),
                    stats: Some(result.stats),
                })
            }
            Err(e) => {
                error!("Failed to get completions: {}", e);
                Some(CompletionResponseWithStats {
                    response: CompletionResponse::List(CompletionList {
                        is_incomplete: false,
                        items: vec![],
                    }),
                    stats: None,
                })
            }
        }
    } else {
        Some(CompletionResponseWithStats {
            response: CompletionResponse::List(CompletionList {
                is_incomplete: false,
                items: vec![],
            }),
            stats: None,
        })
    }
}

pub async fn handle_completion_resolve(
    mut item: CompletionItem,
    type_service: Option<Arc<TypeSystemService>>,
    snippet_support: bool,
) -> CompletionItem {
    if item.detail.is_some() && item.documentation.is_some() && !snippet_support {
        return item;
    }

    let service = match type_service {
        Some(service) => service,
        None => return item,
    };

    let (kind, owner_type) = parse_completion_data(&item);
    let resolved = match (kind.as_deref(), owner_type.as_deref()) {
        (Some("method"), Some(owner)) => service
            .resolve_method_completion(owner, &item.label, snippet_support)
            .map(|details| (details.detail, details.documentation, details.insert_text)),
        (Some("function"), _) => service
            .resolve_function_completion(&item.label, snippet_support)
            .map(|details| (details.detail, details.documentation, details.insert_text)),
        (Some("type"), _) => service
            .resolve_type_completion(&item.label)
            .map(|(detail, documentation)| (detail, documentation, None)),
        _ => service
            .resolve_type_completion(&item.label)
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

    use bsl_backend::system::{AnalysisCache, IntellisenseIndexStore, IrCache, ParserCoordinator};
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature, SignatureIndex, SignatureSource};
    use bsl_shared::domain::types::{ParameterInfo, RawTypeData, RawDataSource};
    use bsl_shared::engine::AnalysisEngine;
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

    fn create_test_service() -> Arc<TypeSystemService> {
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

        let repository = repository_impl.clone() as Arc<dyn bsl_shared::domain::repository::TypeRepository>;
        let resolver = Arc::new(TypeResolver::new(repository.clone()));
        let analysis_engine = Arc::new(AnalysisEngine::new(resolver.clone(), repository.clone()));
        let cache = Arc::new(AnalysisCache::new(16));
        let ir_cache = Arc::new(IrCache::new(16));
        let intellisense_index = Arc::new(IntellisenseIndexStore::new("test", "test"));
        let parser = Arc::new(ParserCoordinator::new_with_resolver(repository, resolver));

        Arc::new(TypeSystemService::new(
            analysis_engine,
            cache,
            parser,
            ir_cache,
            intellisense_index,
        ))
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

    #[tokio::test]
    async fn m5_completion_resolve_snippets_snapshot() {
        let content = read_fixture("m5_snippets_resolve.bsl");
        let position = find_position(&content, "Массив.");
        let uri = Url::parse("file:///m5_snippets_resolve.bsl").expect("url");
        let service = create_test_service();

        let response = handle_completion(
            &content,
            position,
            &uri,
            Some(service.clone()),
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

        let resolved_true = handle_completion_resolve(item.clone(), Some(service.clone()), true).await;
        let resolved_false = handle_completion_resolve(item.clone(), Some(service), false).await;

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
