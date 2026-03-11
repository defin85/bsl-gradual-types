use std::sync::Arc;

use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature};
use bsl_shared::ir::{SemanticNode, SemanticNodeKind, SemanticProgram};

use crate::system::LineIndex;
use crate::system::SystemCoordinator;

#[derive(Debug, Clone)]
pub struct SignatureHelpData {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: u32,
}

#[derive(Debug, Clone)]
pub struct SignatureHelpQuery {
    pub function_name: String,
    pub is_constructor: bool,
    pub call_start_line: u32,
    pub call_start_character: u32,
    pub receiver_end_character: Option<u32>,
    pub receiver_text: Option<String>,
}

pub fn get_signature_help_v2(
    file_content: &str,
    line: u32,
    character: u32,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
) -> Option<SignatureHelpData> {
    get_signature_help_v2_with_analysis(
        file_content,
        line,
        character,
        None,
        None,
        ir_program,
        deps,
        None,
    )
}

pub fn get_signature_help_v2_with_analysis(
    file_content: &str,
    line: u32,
    character: u32,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    ir_program: Arc<SemanticProgram>,
    deps: Arc<bsl_analysis_v2::SemanticDeps>,
    coordinator: Option<&SystemCoordinator>,
) -> Option<SignatureHelpData> {
    let call_context = signature_help_query(file_content, line, character)?;
    if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        if !exact_type_index_ready(analysis, file_id, coordinator) {
            return None;
        }
    }
    let _ = (analysis, file_id, deps, coordinator);
    let signature_info =
        signature_target_from_semantic_facts(file_content, &call_context, ir_program.as_ref())?;

    let active_param = calculate_active_parameter(file_content, &call_context, line, character);
    let (label, parameters) = match signature_info {
        SignatureTarget::Method(signature) => {
            build_signature_labels(&signature.name, &signature.params)
        }
        SignatureTarget::Constructor(signature) => {
            let name = format!(
                "\u{041D}\u{043E}\u{0432}\u{044B}\u{0439} {}",
                signature.type_name
            );
            build_signature_labels(&name, &signature.params)
        }
    };

    Some(SignatureHelpData {
        label,
        parameters,
        active_parameter: active_param,
    })
}

pub fn signature_help_exact_type_index_available_at_position(
    file_content: &str,
    line: u32,
    character: u32,
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
) -> bool {
    if signature_help_query(file_content, line, character).is_none() {
        return true;
    }
    exact_type_index_ready(analysis, file_id, None)
}

pub fn signature_help_query(
    content: &str,
    line: u32,
    character: u32,
) -> Option<SignatureHelpQuery> {
    let index = LineIndex::new(content);
    let lines: Vec<&str> = content.lines().collect();
    let max_line = lines.len().saturating_sub(1);
    let search_until_line = (line as usize).min(max_line);

    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut in_string = false;
    let mut in_block_comment = false;

    for line_idx in 0..=search_until_line {
        let line_text = lines.get(line_idx)?;

        let end_char_idx = if line_idx == line as usize {
            let end_byte = index.utf16_column_to_byte(content, line_idx, character);
            line_text[..end_byte].chars().count()
        } else {
            line_text.chars().count()
        };

        let chars: Vec<char> = line_text.chars().collect();
        let mut char_idx = 0;

        while char_idx < end_char_idx {
            let ch = chars.get(char_idx).copied()?;
            let next = chars.get(char_idx + 1).copied();

            if in_string {
                if ch == '"' {
                    if next == Some('"') {
                        char_idx += 2;
                        continue;
                    }
                    in_string = false;
                }
                char_idx += 1;
                continue;
            }

            if in_block_comment {
                if ch == '*' && next == Some('/') {
                    in_block_comment = false;
                    char_idx += 2;
                    continue;
                }
                char_idx += 1;
                continue;
            }

            if ch == '/' && next == Some('/') {
                break;
            }

            if ch == '/' && next == Some('*') {
                in_block_comment = true;
                char_idx += 2;
                continue;
            }

            if ch == '"' {
                in_string = true;
                char_idx += 1;
                continue;
            }

            match ch {
                '(' => stack.push((line_idx, char_idx)),
                ')' => {
                    stack.pop();
                }
                _ => {}
            }

            char_idx += 1;
        }
    }

    let (line_idx, char_idx) = stack.pop()?;
    let line_text = lines.get(line_idx)?;
    let before_paren: String = line_text.chars().take(char_idx).collect();
    let (function_name, receiver_end_char_idx, receiver_text, is_constructor) =
        extract_function_name(&before_paren)?;

    let byte_column = char_index_to_byte_column(line_text, char_idx);
    let call_start_character = index.byte_column_to_utf16(content, line_idx, byte_column);

    let receiver_end_character = receiver_end_char_idx.map(|char_idx| {
        let byte_column = char_index_to_byte_column(line_text, char_idx);
        index.byte_column_to_utf16(content, line_idx, byte_column)
    });

    Some(SignatureHelpQuery {
        function_name,
        is_constructor,
        call_start_line: line_idx as u32,
        call_start_character,
        receiver_end_character,
        receiver_text,
    })
}

fn calculate_active_parameter(
    content: &str,
    context: &SignatureHelpQuery,
    line: u32,
    character: u32,
) -> u32 {
    let index = LineIndex::new(content);
    let lines: Vec<&str> = content.lines().collect();
    let mut param_index = 0;
    let mut paren_depth = 0;
    let mut in_string = false;
    let mut in_block_comment = false;

    for line_idx in context.call_start_line..=line {
        let line_text = match lines.get(line_idx as usize) {
            Some(value) => value,
            None => break,
        };

        let start_char_idx = if line_idx == context.call_start_line {
            let start_byte = index.utf16_column_to_byte(
                content,
                line_idx as usize,
                context.call_start_character.saturating_add(1),
            );
            line_text[..start_byte].chars().count()
        } else {
            0
        };

        let end_char_idx = if line_idx == line {
            let end_byte = index.utf16_column_to_byte(content, line_idx as usize, character);
            line_text[..end_byte].chars().count()
        } else {
            line_text.chars().count()
        };

        let chars: Vec<char> = line_text.chars().collect();
        let mut char_idx = start_char_idx;
        while char_idx < end_char_idx {
            let ch = match chars.get(char_idx) {
                Some(ch) => *ch,
                None => break,
            };
            let next = chars.get(char_idx + 1).copied();

            if in_string {
                if ch == '"' {
                    if next == Some('"') {
                        char_idx += 2;
                        continue;
                    }
                    in_string = false;
                }
                char_idx += 1;
                continue;
            }

            if in_block_comment {
                if ch == '*' && next == Some('/') {
                    in_block_comment = false;
                    char_idx += 2;
                    continue;
                }
                char_idx += 1;
                continue;
            }

            if ch == '/' && next == Some('/') {
                break;
            }

            if ch == '/' && next == Some('*') {
                in_block_comment = true;
                char_idx += 2;
                continue;
            }

            if ch == '"' {
                in_string = true;
                char_idx += 1;
                continue;
            }

            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    }
                }
                ',' if paren_depth == 0 => {
                    param_index += 1;
                }
                _ => {}
            }

            char_idx += 1;
        }
    }

    param_index
}

fn char_index_to_byte_column(line: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }

    match line.char_indices().nth(char_idx) {
        Some((byte_idx, _)) => byte_idx,
        None => line.len(),
    }
}

fn extract_function_name(text: &str) -> Option<(String, Option<usize>, Option<String>, bool)> {
    let trimmed = text.trim_end();

    if let Some(constructor_name) = extract_constructor_name(trimmed) {
        return Some((constructor_name, None, None, true));
    }

    if let Some(dot_byte_pos) = trimmed.rfind('.') {
        let after_dot = trimmed[dot_byte_pos + 1..].trim_start();

        let method_name = after_dot
            .chars()
            .take_while(|c| is_identifier_char(*c))
            .collect::<String>();

        if !method_name.is_empty() {
            let receiver = trimmed[..dot_byte_pos].trim_end();
            let receiver_end_char_idx = if receiver.is_empty() {
                None
            } else {
                Some(receiver.chars().count().saturating_sub(1))
            };
            let receiver_compact: String =
                receiver.chars().filter(|c| !c.is_whitespace()).collect();
            let receiver_text = if is_simple_receiver(&receiver_compact) {
                Some(receiver_compact)
            } else {
                None
            };
            return Some((method_name, receiver_end_char_idx, receiver_text, false));
        }
    }

    let function_name = trimmed
        .chars()
        .rev()
        .take_while(|c| is_identifier_char(*c))
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();

    if !function_name.is_empty() {
        if is_control_keyword(&function_name) {
            return None;
        }
        Some((function_name, None, None, false))
    } else {
        None
    }
}

fn extract_constructor_name(text: &str) -> Option<String> {
    let mut iter = text.split_whitespace();
    let keyword = iter.next()?;
    if keyword.to_lowercase() != "\u{043D}\u{043E}\u{0432}\u{044B}\u{0439}" {
        return None;
    }
    let remainder: String = iter.collect::<Vec<_>>().join(" ");
    if remainder.is_empty() {
        return None;
    }
    let normalized: String = remainder.chars().filter(|c| !c.is_whitespace()).collect();
    if is_simple_receiver(&normalized) {
        Some(normalized)
    } else {
        None
    }
}

fn is_control_keyword(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "\u{0435}\u{0441}\u{043B}\u{0438}"
            | "\u{0438}\u{043D}\u{0430}\u{0447}\u{0435}\u{0435}\u{0441}\u{043B}\u{0438}"
            | "\u{043F}\u{043E}\u{043A}\u{0430}"
            | "\u{0434}\u{043B}\u{044F}"
            | "\u{043A}\u{0430}\u{0436}\u{0434}\u{043E}\u{0433}\u{043E}"
            | "\u{043F}\u{043E}\u{043F}\u{044B}\u{0442}\u{043A}\u{0430}"
            | "\u{0438}\u{0441}\u{043A}\u{043B}\u{044E}\u{0447}\u{0435}\u{043D}\u{0438}\u{0435}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{0435}\u{0441}\u{043B}\u{0438}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{0446}\u{0438}\u{043A}\u{043B}\u{0430}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{043F}\u{043E}\u{043F}\u{044B}\u{0442}\u{043A}\u{0438}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{043F}\u{0440}\u{043E}\u{0446}\u{0435}\u{0434}\u{0443}\u{0440}\u{044B}"
            | "\u{043A}\u{043E}\u{043D}\u{0435}\u{0446}\u{0444}\u{0443}\u{043D}\u{043A}\u{0446}\u{0438}\u{0438}"
            | "\u{0432}\u{043E}\u{0437}\u{0432}\u{0440}\u{0430}\u{0442}"
            | "\u{0432}\u{044B}\u{0431}\u{043E}\u{0440}"
            | "\u{043A}\u{043E}\u{0433}\u{0434}\u{0430}"
            | "\u{0438}\u{043D}\u{0430}\u{0447}\u{0435}"
    )
}

fn is_simple_receiver(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    text.chars().all(|c| c == '.' || is_identifier_char(c))
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || ('\u{0410}'..='\u{044F}').contains(&c)
        || c == '\u{0401}'
        || c == '\u{0451}'
}

fn signature_target_from_semantic_facts(
    file_content: &str,
    call_context: &SignatureHelpQuery,
    ir_program: &SemanticProgram,
) -> Option<SignatureTarget> {
    let line_index = LineIndex::new(file_content);
    let call_start_offset = line_index
        .utf16_position_to_byte_offset(
            file_content,
            call_context.call_start_line,
            call_context.call_start_character,
        )
        .min(u32::MAX as usize) as u32;
    if let Some(target) = signature_target_from_fact_span(ir_program, call_start_offset) {
        return Some(target);
    }

    let direct = [
        call_start_offset.saturating_sub(1),
        call_start_offset,
        call_start_offset.saturating_add(1),
    ]
    .into_iter()
    .find_map(|offset| {
        ir_program
            .find_node_at_byte_offset(offset)
            .and_then(|node| signature_target_for_node(ir_program, node))
    });
    if direct.is_some() {
        return direct;
    }

    ir_program
        .nodes
        .iter()
        .filter(|node| node.span.contains(call_start_offset))
        .min_by_key(|node| node.span.len())
        .and_then(|node| signature_target_for_node(ir_program, node))
}

fn signature_target_from_fact_span(
    ir_program: &SemanticProgram,
    call_start_offset: u32,
) -> Option<SignatureTarget> {
    for offset in [
        call_start_offset.saturating_sub(1),
        call_start_offset,
        call_start_offset.saturating_add(1),
    ] {
        let constructor = ir_program
            .semantic_facts
            .constructor_targets_by_span
            .iter()
            .filter(|(span, target)| span.contains(offset) && target.signature.is_some())
            .min_by_key(|(span, _)| span.len())
            .and_then(|(_, target)| target.signature.clone())
            .map(SignatureTarget::Constructor);
        if constructor.is_some() {
            return constructor;
        }

        let call_method = ir_program
            .semantic_facts
            .call_method_targets_by_span
            .iter()
            .filter(|(span, target)| span.contains(offset) && target.signature.is_some())
            .min_by_key(|(span, _)| span.len())
            .and_then(|(_, target)| target.signature.clone())
            .map(SignatureTarget::Method);
        if call_method.is_some() {
            return call_method;
        }

        let member_method = ir_program
            .semantic_facts
            .member_method_targets_by_span
            .iter()
            .filter(|(span, target)| span.contains(offset) && target.signature.is_some())
            .min_by_key(|(span, _)| span.len())
            .and_then(|(_, target)| target.signature.clone())
            .map(SignatureTarget::Method);
        if member_method.is_some() {
            return member_method;
        }
    }

    None
}

fn signature_target_for_node(
    ir_program: &SemanticProgram,
    node: &SemanticNode,
) -> Option<SignatureTarget> {
    match &node.kind {
        SemanticNodeKind::FunctionCall { .. } => ir_program
            .semantic_facts
            .call_method_targets_by_span
            .get(&node.span)
            .and_then(|target| target.signature.clone())
            .map(SignatureTarget::Method),
        SemanticNodeKind::MemberAccess { .. } => ir_program
            .semantic_facts
            .member_method_targets_by_span
            .get(&node.span)
            .and_then(|target| target.signature.clone())
            .map(SignatureTarget::Method),
        SemanticNodeKind::NewExpression { .. } => ir_program
            .semantic_facts
            .constructor_targets_by_span
            .get(&node.span)
            .and_then(|target| target.signature.clone())
            .map(SignatureTarget::Constructor),
        _ => None,
    }
}

fn exact_type_index_ready(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    coordinator: Option<&SystemCoordinator>,
) -> bool {
    let ready = analysis
        .current_type_index_serve_only_ready(file_id)
        .ok()
        .unwrap_or(false);
    if let Some(coordinator) = coordinator {
        coordinator.record_intellisense_v2_type_index_reason(if ready {
            bsl_analysis_v2::TypeIndexServeReasonCode::TypeIndexExactHit.as_str()
        } else {
            bsl_analysis_v2::TypeIndexServeReasonCode::TypeIndexFallbackUnavailable.as_str()
        });
    }
    ready
}

enum SignatureTarget {
    Method(MethodSignature),
    Constructor(ConstructorSignature),
}

fn build_signature_labels(
    name: &str,
    params: &[bsl_shared::domain::types::ParameterInfo],
) -> (String, Vec<String>) {
    let params_str = params
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
    let label = format!("{}({})", name, params_str);

    let parameters = params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.type_name.as_deref().unwrap_or("Any")))
        .collect();

    (label, parameters)
}
