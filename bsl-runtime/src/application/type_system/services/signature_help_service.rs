use std::sync::Arc;

use bsl_shared::domain::repository::TypeRepository;
use bsl_shared::domain::signature_index::{ConstructorSignature, MethodSignature};
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

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
        let probe_offset = LineIndex::new(file_content)
            .utf16_position_to_byte_offset(file_content, line, character)
            .min(u32::MAX as usize) as u32;
        if !exact_type_index_available(analysis, file_id, probe_offset, coordinator) {
            return None;
        }
    }
    let receiver_type_hint = signature_receiver_type(
        file_content,
        &call_context,
        ir_program.as_ref(),
        analysis,
        file_id,
        coordinator,
    );
    if call_context.receiver_text.is_some()
        && !call_context.is_constructor
        && receiver_type_hint.is_none()
    {
        return None;
    }

    let signature_info = get_signature_for_function_with_repository(
        &call_context.function_name,
        receiver_type_hint.as_ref(),
        call_context.is_constructor,
        &deps.repository,
    )?;

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

    let probe_offset = LineIndex::new(file_content)
        .utf16_position_to_byte_offset(file_content, line, character)
        .min(u32::MAX as usize) as u32;
    exact_type_index_available(analysis, file_id, probe_offset, None)
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

fn get_signature_for_function_with_repository(
    function_name: &str,
    receiver_type_hint: Option<&TypeResolution>,
    is_constructor: bool,
    repository: &Arc<dyn TypeRepository>,
) -> Option<SignatureTarget> {
    if is_constructor {
        return repository
            .find_constructor(function_name)
            .map(SignatureTarget::Constructor);
    }

    let owner_type = receiver_type_hint.and_then(signature_owner_type_name);
    repository
        .find_method_signature(owner_type.as_deref(), function_name)
        .map(SignatureTarget::Method)
}

fn signature_receiver_type_from_ir(
    file_content: &str,
    call_context: &SignatureHelpQuery,
    ir_program: &SemanticProgram,
) -> Option<TypeResolution> {
    let line_index = LineIndex::new(file_content);
    let call_start_offset = line_index
        .utf16_position_to_byte_offset(
            file_content,
            call_context.call_start_line,
            call_context.call_start_character,
        )
        .min(u32::MAX as usize) as u32;
    if let Some(node) = [
        call_start_offset.saturating_sub(1),
        call_start_offset,
        call_start_offset.saturating_add(1),
    ]
    .into_iter()
    .find_map(|offset| ir_program.find_node_at_byte_offset(offset))
    {
        if let Some(receiver) = semantic_receiver_type(ir_program, node) {
            return Some(receiver);
        }
    }

    let receiver_end_character = call_context.receiver_end_character?;
    let byte_offset = line_index
        .utf16_position_to_byte_offset(
            file_content,
            call_context.call_start_line,
            receiver_end_character,
        )
        .min(u32::MAX as usize) as u32;
    ir_program.semantic_facts.type_at_byte_offset(byte_offset)
}

fn signature_receiver_type(
    file_content: &str,
    call_context: &SignatureHelpQuery,
    ir_program: &SemanticProgram,
    analysis: Option<&bsl_analysis_v2::AnalysisV2>,
    file_id: Option<bsl_analysis_v2::FileId>,
    coordinator: Option<&SystemCoordinator>,
) -> Option<TypeResolution> {
    if let (Some(analysis), Some(file_id)) = (analysis, file_id) {
        let line_index = LineIndex::new(file_content);
        let call_start_offset = line_index
            .utf16_position_to_byte_offset(
                file_content,
                call_context.call_start_line,
                call_context.call_start_character,
            )
            .min(u32::MAX as usize) as u32;
        if let Some(node) = [
            call_start_offset.saturating_sub(1),
            call_start_offset,
            call_start_offset.saturating_add(1),
        ]
        .into_iter()
        .find_map(|offset| ir_program.find_node_at_byte_offset(offset))
        {
            if let Some(receiver) = semantic_receiver_type_with_exact_index(
                analysis,
                file_id,
                ir_program,
                node,
                coordinator,
            ) {
                return Some(receiver);
            }
        }

        let receiver_end_character = call_context.receiver_end_character?;
        let byte_offset = line_index
            .utf16_position_to_byte_offset(
                file_content,
                call_context.call_start_line,
                receiver_end_character,
            )
            .min(u32::MAX as usize) as u32;
        return serve_only_type_at_offset(analysis, file_id, byte_offset, coordinator);
    }

    signature_receiver_type_from_ir(file_content, call_context, ir_program)
}

fn semantic_type_at_offset(program: &SemanticProgram, offset: u32) -> Option<TypeResolution> {
    program
        .semantic_facts
        .type_at_byte_offset(offset)
        .or_else(|| {
            program
                .find_node_at_byte_offset(offset)
                .and_then(|node| program.semantic_facts.type_resolution_for_span(node.span))
        })
}

fn receiver_span_for_node(program: &SemanticProgram, node: &SemanticNode) -> Option<Span> {
    match &node.kind {
        SemanticNodeKind::MemberAccess {
            object_node,
            object_span,
            ..
        }
        | SemanticNodeKind::FunctionCall {
            object_node,
            object_span,
            ..
        } => object_span.or_else(|| {
            object_node.and_then(|idx| program.nodes.get(idx).map(|candidate| candidate.span))
        }),
        _ => None,
    }
}

fn semantic_receiver_type(
    program: &SemanticProgram,
    node: &SemanticNode,
) -> Option<TypeResolution> {
    let span_fallback = |span: Span| {
        semantic_type_at_offset(program, span.start)
            .or_else(|| {
                span.end
                    .checked_sub(1)
                    .and_then(|offset| semantic_type_at_offset(program, offset))
            })
            .or_else(|| program.semantic_facts.type_resolution_for_span(span))
    };

    match &node.kind {
        SemanticNodeKind::MemberAccess { .. } => program
            .semantic_facts
            .member_access_object_type_by_span
            .get(&node.span)
            .cloned()
            .or_else(|| receiver_span_for_node(program, node).and_then(span_fallback)),
        SemanticNodeKind::FunctionCall { .. } => program
            .semantic_facts
            .call_receiver_type_by_span
            .get(&node.span)
            .cloned()
            .or_else(|| receiver_span_for_node(program, node).and_then(span_fallback)),
        _ => None,
    }
}

fn semantic_receiver_type_with_exact_index(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    program: &SemanticProgram,
    node: &SemanticNode,
    coordinator: Option<&SystemCoordinator>,
) -> Option<TypeResolution> {
    receiver_span_for_node(program, node)
        .and_then(|span| serve_only_type_in_span(analysis, file_id, span, coordinator))
}

fn serve_only_type_in_span(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    span: Span,
    coordinator: Option<&SystemCoordinator>,
) -> Option<TypeResolution> {
    serve_only_type_at_offset(analysis, file_id, span.start, coordinator).or_else(|| {
        span.end
            .checked_sub(1)
            .and_then(|offset| serve_only_type_at_offset(analysis, file_id, offset, coordinator))
    })
}

fn serve_only_type_at_offset(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    byte_offset: u32,
    coordinator: Option<&SystemCoordinator>,
) -> Option<TypeResolution> {
    let profiled = serve_only_profiled(analysis, file_id, byte_offset, coordinator)?;
    profiled.resolution
}

fn exact_type_index_available(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    byte_offset: u32,
    coordinator: Option<&SystemCoordinator>,
) -> bool {
    serve_only_profiled(analysis, file_id, byte_offset, coordinator).is_some_and(|profiled| {
        profiled.serve_reason_code == bsl_analysis_v2::TypeIndexServeReasonCode::TypeIndexExactHit
    })
}

fn serve_only_profiled(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    byte_offset: u32,
    coordinator: Option<&SystemCoordinator>,
) -> Option<bsl_analysis_v2::TypeAtByteOffsetProfiledResult> {
    let profiled = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, byte_offset)
        .ok()?;
    if let Some(coordinator) = coordinator {
        coordinator.record_intellisense_v2_type_index_reason(profiled.serve_reason_code.as_str());
    }
    Some(profiled)
}

fn signature_owner_type_name(resolution: &TypeResolution) -> Option<String> {
    if resolution.is_unknown() || resolution.is_dynamic() {
        return None;
    }

    let type_name = resolution.type_name();
    let without_generic = type_name.split('<').next().unwrap_or(&type_name);
    let without_union = without_generic.split('|').next().unwrap_or(without_generic);
    let normalized = without_union.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
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
