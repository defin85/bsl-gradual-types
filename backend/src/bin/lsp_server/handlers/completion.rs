//! Completion handler for LSP
//!
//! Handles textDocument/completion requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::{error, info};

use bsl_backend::application::TypeSystemService;

/// Handle textDocument/completion request
pub async fn handle_completion(
    file_content: &str,
    position: Position,
    type_service: Option<Arc<TypeSystemService>>,
) -> Option<CompletionResponse> {
    info!(
        "Completion requested at {}:{}",
        position.line, position.character
    );

    if let Some(service) = type_service {
        match service
            .get_completion(file_content, position.line, position.character)
            .await
        {
            Ok(completions) => {
                let lsp_completions: Vec<CompletionItem> = completions
                    .into_iter()
                    .map(|item| CompletionItem {
                        label: item.label,
                        detail: item.detail,
                        insert_text: item.insert_text,
                        kind: Some(CompletionItemKind::KEYWORD),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        ..Default::default()
                    })
                    .collect();
                info!("Returning {} completions", lsp_completions.len());
                Some(CompletionResponse::Array(lsp_completions))
            }
            Err(e) => {
                error!("Failed to get completions: {}", e);
                Some(CompletionResponse::Array(vec![]))
            }
        }
    } else {
        Some(CompletionResponse::Array(vec![]))
    }
}
