//! Signature Help handler for LSP
//!
//! Handles textDocument/signatureHelp requests.

use std::sync::Arc;
use tower_lsp::lsp_types::*;
use tracing::debug;

use bsl_shared::domain::signature_index::MethodSignature;
use bsl_shared::engine::AnalysisEngine;

use crate::converters::position::{char_to_utf16_index, utf16_to_char_index};

/// Context of a function call
#[derive(Debug)]
pub struct CallContext {
    pub function_name: String,
    pub receiver_type: Option<String>,
    pub call_start: Position,
}

/// Handle textDocument/signatureHelp request
pub async fn handle_signature_help(
    file_content: &str,
    position: Position,
    analysis_engine: Option<Arc<AnalysisEngine>>,
) -> Option<SignatureHelp> {
    debug!(
        "SignatureHelp requested at {}:{}",
        position.line, position.character
    );

    // Find call context
    let call_context = find_call_context(file_content, position)?;

    debug!(
        "Found call context: function={}, receiver={:?}",
        call_context.function_name, call_context.receiver_type
    );

    // Get signature from repository
    let signature_info = get_signature_for_function(
        &call_context.function_name,
        call_context.receiver_type.as_deref(),
        analysis_engine,
    )?;

    // Calculate active parameter
    let active_param = calculate_active_parameter(file_content, &call_context, position);

    // Build response
    Some(build_signature_help_response(signature_info, active_param))
}

/// Find function call context
pub fn find_call_context(content: &str, position: Position) -> Option<CallContext> {
    let lines: Vec<&str> = content.lines().collect();

    // Search for opening parenthesis, moving backwards from cursor
    let mut paren_depth = 0;
    let mut call_start: Option<(usize, usize)> = None; // (line_idx, char_idx)

    let max_line = if lines.is_empty() {
        return None;
    } else {
        lines.len() - 1
    };
    let search_until_line = position.line.min(max_line as u32) as usize;

    for line_idx in (0..=search_until_line).rev() {
        let line = lines.get(line_idx)?;

        let end_char_idx = if line_idx == position.line as usize {
            utf16_to_char_index(line, position.character as usize)?
        } else {
            line.chars().count()
        };

        let chars: Vec<char> = line.chars().collect();

        for char_idx in (0..end_char_idx).rev() {
            let ch = chars.get(char_idx)?;

            match ch {
                ')' => paren_depth += 1,
                '(' => {
                    if paren_depth == 0 {
                        call_start = Some((line_idx, char_idx));
                        break;
                    }
                    paren_depth -= 1;
                }
                _ => {}
            }
        }

        if call_start.is_some() {
            break;
        }
    }

    let (line_idx, char_idx) = call_start?;

    // Extract function name before parenthesis
    let line = lines.get(line_idx)?;
    let before_paren: String = line.chars().take(char_idx).collect();

    let (function_name, receiver_type) = extract_function_name(&before_paren)?;

    let utf16_char = char_to_utf16_index(line, char_idx);

    Some(CallContext {
        function_name,
        receiver_type,
        call_start: Position {
            line: line_idx as u32,
            character: utf16_char as u32,
        },
    })
}

/// Extract function name from text before parenthesis
fn extract_function_name(text: &str) -> Option<(String, Option<String>)> {
    let trimmed = text.trim_end();

    // First search for dot (for object methods)
    if let Some(dot_byte_pos) = trimmed.rfind('.') {
        let after_dot = &trimmed[dot_byte_pos + 1..];

        let method_name = after_dot
            .chars()
            .take_while(|c| {
                c.is_alphanumeric()
                    || *c == '_'
                    || (*c >= '\u{0410}' && *c <= '\u{044F}')
                    || *c == '\u{0401}'
                    || *c == '\u{0451}'
            })
            .collect::<String>();

        if !method_name.is_empty() {
            // TODO: determine receiver type through type inference
            return Some((method_name, None));
        }
    }

    // Global function: extract last valid identifier
    let function_name = trimmed
        .chars()
        .rev()
        .take_while(|c| {
            c.is_alphanumeric()
                || *c == '_'
                || (*c >= '\u{0410}' && *c <= '\u{044F}')
                || *c == '\u{0401}'
                || *c == '\u{0451}'
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    if !function_name.is_empty() {
        Some((function_name, None))
    } else {
        None
    }
}

/// Calculate active parameter index
pub fn calculate_active_parameter(content: &str, context: &CallContext, position: Position) -> u32 {
    let lines: Vec<&str> = content.lines().collect();
    let mut param_index = 0;
    let mut paren_depth = 0;
    let mut in_string = false;

    for line_idx in context.call_start.line..=position.line {
        let line = match lines.get(line_idx as usize) {
            Some(l) => l,
            None => break,
        };
        let chars: Vec<char> = line.chars().collect();

        let start_char_idx = if line_idx == context.call_start.line {
            utf16_to_char_index(line, (context.call_start.character + 1) as usize).unwrap_or(0)
        } else {
            0
        };

        let end_char_idx = if line_idx == position.line {
            utf16_to_char_index(line, position.character as usize).unwrap_or(chars.len())
        } else {
            chars.len()
        };

        for char_idx in start_char_idx..end_char_idx {
            if let Some(&ch) = chars.get(char_idx) {
                match ch {
                    '"' => in_string = !in_string,
                    '(' if !in_string => paren_depth += 1,
                    ')' if !in_string => paren_depth -= 1,
                    ',' if !in_string && paren_depth == 0 => {
                        param_index += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    param_index
}

/// Get function signature from TypeRepository
fn get_signature_for_function(
    function_name: &str,
    receiver_type: Option<&str>,
    analysis_engine: Option<Arc<AnalysisEngine>>,
) -> Option<MethodSignature> {
    let engine = analysis_engine?;
    let repo = engine.get_repository();
    repo.find_method_signature(receiver_type, function_name)
}

/// Build LSP SignatureHelp response
fn build_signature_help_response(signature: MethodSignature, active_param: u32) -> SignatureHelp {
    let params_str = signature
        .params
        .iter()
        .map(|p| {
            let type_str = p.type_name.as_deref().unwrap_or("Any");
            if p.is_optional {
                format!("[{}: {}]", p.name, type_str)
            } else {
                format!("{}: {}", p.name, type_str)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    let label = format!("{}({})", signature.name, params_str);

    let parameters = signature
        .params
        .iter()
        .map(|p| {
            let param_label = format!(
                "{}: {}",
                p.name,
                p.type_name.as_deref().unwrap_or("Any")
            );

            ParameterInformation {
                label: ParameterLabel::Simple(param_label),
                documentation: None,
            }
        })
        .collect();

    SignatureHelp {
        signatures: vec![SignatureInformation {
            label,
            documentation: None,
            parameters: Some(parameters),
            active_parameter: Some(active_param),
        }],
        active_signature: Some(0),
        active_parameter: Some(active_param),
    }
}
