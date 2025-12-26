//! Text document handlers for LSP
//!
//! Handles did_open, did_change, did_save, did_close notifications.

use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::{debug, error, info, warn};

use bsl_backend::application::TypeSystemService;
use bsl_backend::system::parser_coordinator::TextEdit;

use crate::config::BslSettings;
use crate::converters::position::utf16_to_byte_offset;
use crate::converters::{semantic_error_to_diagnostic, syntax_errors_to_diagnostics};

/// Handle textDocument/didOpen notification
pub async fn handle_did_open(
    uri: &Url,
    text: &str,
    _version: i32,
    type_service: Option<Arc<TypeSystemService>>,
    settings: &BslSettings,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let file_path_opt = uri
        .to_file_path()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // IR cache preheating is done in server.rs

    // PHASE 1: Syntax validation
    if let Some(ref service) = type_service {
        let parse_result = if let Some(ref path) = file_path_opt {
            service.parse_and_validate_for_file(text, path)
        } else {
            service.parse_and_validate(text)
        };
        match parse_result {
            Ok(errors) => {
                if !errors.is_empty() {
                    info!("Found {} syntax errors in {}", errors.len(), uri);
                    diagnostics.extend(syntax_errors_to_diagnostics(&errors));
                } else {
                    info!("No syntax errors in {}", uri);
                }
            }
            Err(e) => {
                error!("Failed to parse document {}: {}", uri, e);
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Parse error: {}", e),
                    source: Some("bsl-syntax".to_string()),
                    ..Default::default()
                });
            }
        }
    } else {
        warn!("TypeSystemService not yet initialized, skipping syntax validation");
    }

    // PHASE 2: Semantic validation
    if let Some(ref service) = type_service {
        let metrics = service.get_metrics_summary();
        let total_types = metrics
            .get("total_types")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if total_types == 0 {
            info!(
                "Skipping semantic validation for {}: platform types not yet loaded",
                uri
            );
        } else {
            let detail_level =
                bsl_shared::formatting::DetailLevel::parse(&settings.diagnostics.detail_level);

            let semantic_result = match file_path_opt {
                Some(ref path) => service
                    .validate_semantics_for_file(text, path, Some(detail_level))
                    .await,
                None => service.validate_semantics(text, Some(detail_level)).await,
            };
            match semantic_result {
                Ok(semantic_errors) => {
                    if !semantic_errors.is_empty() {
                        info!(
                            "Found {} semantic errors in {} (detail_level: {})",
                            semantic_errors.len(),
                            uri,
                            settings.diagnostics.detail_level
                        );
                        for error in semantic_errors {
                            diagnostics.push(semantic_error_to_diagnostic(&error));
                        }
                    }
                }
                Err(e) => {
                    warn!("Semantic validation failed for {}: {}", uri, e);
                }
            }
        }
    }

    diagnostics
}

/// Handle textDocument/didChange notification
pub async fn handle_did_change(
    uri: &Url,
    updated_text: &str,
    changes: &[TextDocumentContentChangeEvent],
    type_service: Option<Arc<TypeSystemService>>,
    config_root: Option<PathBuf>,
    settings: &BslSettings,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let file_path_opt = uri
        .to_file_path()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // Invalidate IR cache
    let uri_str = uri.to_string();
    if let Some(ref service) = type_service {
        service
            .invalidate_file_cache(&uri_str, updated_text, config_root.as_deref())
            .await;
        debug!("File changed: {}, cache invalidated", uri_str);
    }

    // Incremental parsing
    let text_edits: Vec<TextEdit> = changes
        .iter()
        .filter_map(|change| {
            change.range.map(|range| TextEdit {
                start_line: range.start.line,
                start_column: range.start.character,
                old_end_line: range.end.line,
                old_end_column: range.end.character,
                new_end_line: range.start.line + change.text.matches('\n').count() as u32,
                new_end_column: if change.text.contains('\n') {
                    change.text.lines().last().unwrap_or("").len() as u32
                } else {
                    range.start.character + change.text.len() as u32
                },
                new_text: change.text.clone(),
            })
        })
        .collect();

    let file_path_buf = uri
        .to_file_path()
        .unwrap_or_else(|_| std::path::PathBuf::from(uri.path()));

    if let Some(ref service) = type_service {
        if let Err(e) = service
            .parse_incremental(file_path_buf, updated_text.to_string(), text_edits)
            .await
        {
            error!("Incremental parsing failed: {}", e);
        } else {
            info!("Incremental parsing succeeded for: {}", uri.path());
        }
    }

    // Syntax validation
    if let Some(ref service) = type_service {
        let parse_result = if let Some(ref path) = file_path_opt {
            service.parse_and_validate_for_file(updated_text, path)
        } else {
            service.parse_and_validate(updated_text)
        };
        match parse_result {
            Ok(errors) => {
                if !errors.is_empty() {
                    info!("Found {} syntax errors in {}", errors.len(), uri);
                    diagnostics.extend(syntax_errors_to_diagnostics(&errors));
                } else {
                    info!("No syntax errors in {}", uri);
                }
            }
            Err(e) => {
                error!("Failed to parse document {}: {}", uri, e);
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Parse error: {}", e),
                    source: Some("bsl-syntax".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    // Semantic validation
    if let Some(ref service) = type_service {
        let metrics = service.get_metrics_summary();
        let total_types = metrics
            .get("total_types")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if total_types == 0 {
            info!(
                "Skipping semantic validation for {}: platform types not yet loaded",
                uri
            );
        } else {
            let detail_level =
                bsl_shared::formatting::DetailLevel::parse(&settings.diagnostics.detail_level);

            let semantic_result = match file_path_opt {
                Some(ref path) => service
                    .validate_semantics_for_file(updated_text, path, Some(detail_level))
                    .await,
                None => service
                    .validate_semantics(updated_text, Some(detail_level))
                    .await,
            };
            match semantic_result {
                Ok(semantic_errors) => {
                    if !semantic_errors.is_empty() {
                        info!("Found {} semantic errors in {}", semantic_errors.len(), uri);
                        for error in semantic_errors {
                            diagnostics.push(semantic_error_to_diagnostic(&error));
                        }
                    }
                }
                Err(e) => {
                    warn!("Semantic validation failed for {}: {}", uri, e);
                }
            }
        }
    }

    diagnostics
}

/// Apply text edit to source string
pub fn apply_text_edit(source: &str, range: Range, new_text: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;

    // Convert UTF-16 offsets to UTF-8 byte offsets
    let start_char = if let Some(start_line_text) = lines.get(start_line) {
        utf16_to_byte_offset(start_line_text, range.start.character)
    } else {
        0
    };

    let end_char = if let Some(end_line_text) = lines.get(end_line) {
        utf16_to_byte_offset(end_line_text, range.end.character)
    } else {
        0
    };

    let mut result = String::new();

    // Lines before change
    for line in lines.iter().take(start_line) {
        result.push_str(line);
        result.push('\n');
    }

    // Start of changed line
    if let Some(start_line_text) = lines.get(start_line) {
        result.push_str(&start_line_text[..start_char.min(start_line_text.len())]);
    }

    // New text
    result.push_str(new_text);

    // End of changed line
    if let Some(end_line_text) = lines.get(end_line) {
        result.push_str(&end_line_text[end_char.min(end_line_text.len())..]);
    }

    // Lines after change
    for line in lines.iter().skip(end_line + 1) {
        result.push('\n');
        result.push_str(line);
    }

    result
}
