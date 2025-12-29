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
                    .map(|candidate| to_lsp_completion(candidate.item, candidate.owner_type))
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
) -> CompletionItem {
    if item.detail.is_some() || item.documentation.is_some() {
        return item;
    }

    let service = match type_service {
        Some(service) => service,
        None => return item,
    };

    let (kind, owner_type) = parse_completion_data(&item);
    let resolved = match (kind.as_deref(), owner_type.as_deref()) {
        (Some("method"), Some(owner)) => service.resolve_method_completion(owner, &item.label),
        (Some("type"), _) => service.resolve_type_completion(&item.label),
        _ => service.resolve_type_completion(&item.label),
    };

    if let Some((detail, documentation)) = resolved {
        item.detail = detail;
        if let Some(doc) = documentation {
            item.documentation = Some(Documentation::String(doc));
        }
    }

    item
}

fn to_lsp_completion(item: bsl_shared::domain::CompletionItem, owner_type: Option<String>) -> CompletionItem {
    let kind = map_completion_kind(item.kind);
    let insert_text_format = item
        .insert_text
        .as_deref()
        .filter(|text| text.contains("${"))
        .map(|_| InsertTextFormat::SNIPPET);

    let data = Some(json!({
        "kind": completion_kind_tag(item.kind),
        "owner_type": owner_type,
    }));

    CompletionItem {
        label: item.label,
        kind,
        detail: None,
        documentation: None,
        insert_text: item.insert_text,
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
