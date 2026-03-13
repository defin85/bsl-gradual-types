//! Hover Service - hover and type-at-position operations
//!
//! Functions for LSP hover requests and getting type information at cursor position.

use tracing::{debug, info};

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
use crate::system::LineIndex;

use super::super::extractors::symbol_extractor::{extract_word_at_position, is_identifier_char};
use super::super::formatters::format_semantic_node_info;
use super::super::formatters::hover_formatters::format_expected_type_hover;

/// Hover по уже готовому `SemanticProgram` (без legacy парсинга/IR build).
///
/// Используется в IntelliSense v2 (salsa) hot path.
#[allow(clippy::too_many_arguments)]
pub fn get_hover_info_with_semantic_program(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    column: u32,
    include_flow_sensitive: bool,
    metadata_lookup: &TypeMetadataLookup,
    hover_formatter: &HoverFormatter,
    hover_config: Option<HoverFormatConfig>,
    resolver: &TypeResolver,
    ir_program: std::sync::Arc<SemanticProgram>,
) -> Option<String> {
    if !hover_exact_type_index_available_at_position(
        analysis,
        file_id,
        file_content,
        line,
        column,
        ir_program.as_ref(),
    ) {
        return None;
    }
    compute_hover_info_from_ir(
        ir_program.as_ref(),
        analysis,
        file_id,
        resolver,
        metadata_lookup,
        hover_formatter,
        file_content,
        line,
        column,
        include_flow_sensitive,
        hover_config,
    )
}

pub fn hover_exact_type_index_available_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    column: u32,
    ir_program: &SemanticProgram,
) -> bool {
    let _ = (file_content, line, column, ir_program);
    analysis
        .current_type_index_serve_only_ready(file_id)
        .ok()
        .unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn compute_hover_info_from_ir(
    ir_program: &SemanticProgram,
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    resolver: &TypeResolver,
    metadata_lookup: &TypeMetadataLookup,
    hover_formatter: &HoverFormatter,
    file_content: &str,
    line: u32,
    column: u32,
    _include_flow_sensitive: bool,
    hover_config: Option<HoverFormatConfig>,
) -> Option<String> {
    // Milestone 2.11 Task B1: DEBUG logs for node search
    debug!("Looking for node at position {}:{}", line, column);

    let byte_offset = analysis
        .utf16_position_to_byte_offset(file_id, line, column)
        .ok()
        .flatten()
        .map(|offset| offset.min(u32::MAX as usize) as u32);

    let node_at_position =
        byte_offset.and_then(|offset| ir_program.find_node_at_byte_offset(offset));
    let word_under_cursor = extract_word_at_position(file_content, line, column);
    let mut type_at_cursor = byte_offset
        .and_then(|offset| exact_semantic_type_at_offset(analysis, file_id, ir_program, offset));
    if type_at_cursor.is_none() {
        if let Some(offset) = byte_offset {
            if let Some(identifier_span) =
                identifier_span_at_byte_offset(file_content, offset as usize)
            {
                type_at_cursor =
                    exact_semantic_type_at_span(analysis, file_id, ir_program, identifier_span);
            }
        }
    }
    if type_at_cursor.is_none() {
        if let Some(node) = node_at_position {
            type_at_cursor = exact_semantic_type_at_span(analysis, file_id, ir_program, node.span);
        }
    }
    let indexed_expression_hover = byte_offset.and_then(|offset| {
        let span = indexed_expression_span_at_byte_offset(file_content, offset as usize)?;
        let resolution = exact_semantic_type_at_span(analysis, file_id, ir_program, span)?;
        if resolution.is_unknown() || resolution.is_dynamic() {
            return None;
        }
        let label = span_text(file_content, span)?.trim();
        if label.is_empty() {
            return None;
        }
        Some((label.to_string(), resolution))
    });
    if let Some((_, resolution)) = &indexed_expression_hover {
        type_at_cursor = Some(resolution.clone());
    }

    let formatter = if let Some(config) = hover_config.clone() {
        HoverFormatter::new(config, metadata_lookup.clone())
    } else {
        hover_formatter.clone()
    };

    // Специальный кейс: hover на имени свойства (obj.Property) должен показывать тип свойства,
    // а не тип переменной-объекта слева от точки.
    if let Some(node) = node_at_position {
        if let SemanticNodeKind::MemberAccess {
            object_node,
            object_name,
            member_name,
            access_kind,
            ..
        } = &node.kind
        {
            if access_kind.is_property()
                && word_under_cursor
                    .as_ref()
                    .is_some_and(|word| word.eq_ignore_ascii_case(member_name))
            {
                let owner_span = object_node
                    .and_then(|idx| ir_program.nodes.get(idx).map(|n| n.span))
                    .unwrap_or(node.span);
                let owner_resolution =
                    exact_semantic_type_at_span(analysis, file_id, ir_program, owner_span)
                        .unwrap_or_else(TypeResolution::unknown);

                let (prop_type, is_readonly) = metadata_lookup
                    .get_properties(&owner_resolution)
                    .into_iter()
                    .find(|p| p.name.eq_ignore_ascii_case(member_name))
                    .map(|p| (p.prop_type, Some(p.is_readonly)))
                    .unwrap_or_else(|| (String::new(), None));

                let property_resolution = if !prop_type.trim().is_empty() {
                    resolver.resolve_expression_sync(&prop_type)
                } else {
                    type_at_cursor
                        .clone()
                        .unwrap_or_else(TypeResolution::unknown)
                };

                return Some(formatter.format_property(
                    object_name.as_deref(),
                    &owner_resolution,
                    member_name,
                    &property_resolution,
                    is_readonly,
                ));
            }
        }
    }

    // IR-level hover для управляющих конструкций (если/пока/для), чтобы сохранять поведение
    // существующих тестов и UI (и избежать legacy flow-sensitive логики).
    if let Some(word) = word_under_cursor.as_deref() {
        if let Some(offset) = byte_offset {
            if let Some(node) = control_node_at_position(ir_program, offset) {
                if control_hover_requested(node, word) {
                    return format_control_flow_hover(
                        file_content,
                        line,
                        column,
                        analysis,
                        file_id,
                        ir_program,
                        node,
                    )
                    .or_else(|| {
                        Some(format_semantic_node_info(
                            node,
                            file_content,
                            metadata_lookup,
                        ))
                    });
                }
            }
        }
    }

    // Hover на вызове функции/метода должен показывать FunctionCall, а не "Переменная".
    if let (Some(node), Some(word)) = (node_at_position, word_under_cursor.as_deref()) {
        if let SemanticNodeKind::FunctionCall { function_name, .. } = &node.kind {
            if word.eq_ignore_ascii_case(function_name) {
                return Some(format_semantic_node_info(
                    node,
                    file_content,
                    metadata_lookup,
                ));
            }
        }
    }

    if let Some((label, resolution)) = indexed_expression_hover {
        info!(
            "Hover v2 type_at_byte_offset({}): {}",
            byte_offset.unwrap_or_default(),
            resolution.type_name()
        );
        return Some(formatter.format_variable(&label, &resolution));
    }

    if let (Some(word), Some(resolution)) = (&word_under_cursor, &type_at_cursor) {
        info!(
            "Hover v2 type_at_byte_offset({}): {}",
            byte_offset.unwrap_or_default(),
            resolution.type_name()
        );
        return Some(formatter.format_variable(word, resolution));
    }

    if let Some(resolution) = &type_at_cursor {
        if let Some(label) = node_at_position
            .and_then(|node| span_text(file_content, node.span))
            .map(str::trim)
            .filter(|label| !label.is_empty())
        {
            info!(
                "Hover v2 type_at_byte_offset({}): {}",
                byte_offset.unwrap_or_default(),
                resolution.type_name()
            );
            return Some(formatter.format_variable(label, resolution));
        }
    }

    // Milestone 2.11 Task B1: Logs when symbol not found
    debug!("Hover v2: no type at position {}:{}", line, column);
    None
}

fn exact_semantic_type_at_span(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    program: &SemanticProgram,
    span: Span,
) -> Option<TypeResolution> {
    analysis
        .type_for_span_serve_only(file_id, span)
        .ok()
        .flatten()
        .or_else(|| exact_semantic_type_at_offset(analysis, file_id, program, span.start))
        .or_else(|| {
            span.end
                .checked_sub(1)
                .and_then(|probe| exact_semantic_type_at_offset(analysis, file_id, program, probe))
        })
}

fn exact_semantic_type_at_offset(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    program: &SemanticProgram,
    probe: u32,
) -> Option<TypeResolution> {
    analysis
        .type_at_byte_offset_serve_only(file_id, probe)
        .ok()
        .flatten()
        .or_else(|| {
            program.find_node_at_byte_offset(probe).and_then(|node| {
                analysis
                    .type_for_span_serve_only(file_id, node.span)
                    .ok()
                    .flatten()
            })
        })
}

fn identifier_span_at_byte_offset(file_content: &str, byte_offset: usize) -> Option<Span> {
    let chars = file_content.char_indices().collect::<Vec<_>>();
    if chars.is_empty() {
        return None;
    }

    let mut char_idx = chars.partition_point(|(idx, _)| *idx <= byte_offset);
    if char_idx == chars.len() {
        char_idx = chars.len().saturating_sub(1);
    }

    if !is_identifier_char(chars[char_idx].1) {
        let can_use_prev = chars[char_idx].0 == byte_offset
            && char_idx > 0
            && is_identifier_char(chars[char_idx - 1].1);
        if can_use_prev {
            char_idx -= 1;
        } else {
            return None;
        }
    }

    let mut start_idx = char_idx;
    while start_idx > 0 && is_identifier_char(chars[start_idx - 1].1) {
        start_idx -= 1;
    }

    let mut end_idx = char_idx + 1;
    while end_idx < chars.len() && is_identifier_char(chars[end_idx].1) {
        end_idx += 1;
    }

    let start = chars[start_idx].0;
    let end = if end_idx < chars.len() {
        chars[end_idx].0
    } else {
        file_content.len()
    };

    Some(Span::new(start as u32, end as u32))
}

fn indexed_expression_span_at_byte_offset(file_content: &str, byte_offset: usize) -> Option<Span> {
    let identifier_span = identifier_span_at_byte_offset(file_content, byte_offset)?;
    let mut cursor = identifier_span.end as usize;
    while let Some(ch) = file_content.get(cursor..)?.chars().next() {
        if !ch.is_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    if file_content.get(cursor..)?.chars().next()? != '[' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut chars = file_content.get(cursor..)?.char_indices().peekable();
    while let Some((relative_idx, ch)) = chars.next() {
        let absolute_idx = cursor + relative_idx;
        if in_string {
            if ch == '"' {
                if chars.peek().is_some_and(|(_, next)| *next == '"') {
                    let _ = chars.next();
                    continue;
                }
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(Span::new(
                        identifier_span.start,
                        (absolute_idx + ch.len_utf8()) as u32,
                    ));
                }
            }
            _ => {}
        }
    }

    None
}

fn span_text(file_content: &str, span: Span) -> Option<&str> {
    let start = span.start as usize;
    let end = span.end as usize;
    file_content.get(start..end)
}

fn control_node_at_position(
    ir_program: &SemanticProgram,
    byte_offset: u32,
) -> Option<&SemanticNode> {
    ir_program
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                SemanticNodeKind::IfStatement { .. }
                    | SemanticNodeKind::WhileLoop { .. }
                    | SemanticNodeKind::ForLoop { .. }
                    | SemanticNodeKind::ForEachLoop { .. }
            )
        })
        .filter(|node| node.span.contains(byte_offset))
        .min_by_key(|node| node.span.len())
}

fn control_hover_requested(node: &SemanticNode, word: &str) -> bool {
    match &node.kind {
        SemanticNodeKind::IfStatement { .. } => word.eq_ignore_ascii_case("Если"),
        SemanticNodeKind::WhileLoop { .. } => word.eq_ignore_ascii_case("Пока"),
        SemanticNodeKind::ForLoop { variable, .. } => {
            word.eq_ignore_ascii_case("Для") || word.eq_ignore_ascii_case(variable)
        }
        SemanticNodeKind::ForEachLoop { variable, .. } => {
            word.eq_ignore_ascii_case("Для")
                || word.eq_ignore_ascii_case("Каждого")
                || word.eq_ignore_ascii_case(variable)
        }
        _ => false,
    }
}

#[allow(dead_code)]
fn span_contains_position(span: Span, byte_offset: u32) -> bool {
    span.contains(byte_offset)
}

fn format_control_flow_hover(
    file_content: &str,
    line: u32,
    column: u32,
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    ir_program: &SemanticProgram,
    node: &SemanticNode,
) -> Option<String> {
    let _ = column;

    let index = LineIndex::new(file_content);
    let line_text = index.line_text(file_content, line as usize);
    let line_start = index.utf16_position_to_byte_offset(file_content, line, 0);
    let line_start: u32 = line_start.try_into().ok()?;

    fn first_non_ws_byte_index(s: &str) -> Option<usize> {
        s.char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(idx, _)| idx)
    }

    fn slice_range_after_keyword<'a>(
        line: &'a str,
        keyword_lower: &str,
        terminator_lower: Option<&str>,
    ) -> Option<(usize, &'a str)> {
        let lower = line.to_lowercase();
        let start = lower.find(keyword_lower)?;
        let after = start + keyword_lower.len();
        let end = if let Some(term) = terminator_lower {
            lower[after..].find(term).map(|idx| after + idx)?
        } else {
            line.len()
        };
        Some((after, &line[after..end]))
    }

    let (title, expected_type, probe_in_line) = match &node.kind {
        SemanticNodeKind::IfStatement { .. } => {
            let (after, cond) = slice_range_after_keyword(line_text, "если", Some("тогда"))?;
            let rel = cond
                .find(|ch: char| ['<', '>', '='].contains(&ch))
                .or_else(|| first_non_ws_byte_index(cond))?;
            (
                "**Условие:** `Если ... Тогда`".to_string(),
                "Булево",
                after + rel,
            )
        }
        SemanticNodeKind::WhileLoop { .. } => {
            let (after, cond) = slice_range_after_keyword(line_text, "пока", Some("цикл"))?;
            let rel = first_non_ws_byte_index(cond)?;
            (
                "**Цикл:** `Пока ... Цикл`".to_string(),
                "Булево",
                after + rel,
            )
        }
        SemanticNodeKind::ForLoop { variable, .. } => {
            let lower = line_text.to_lowercase();
            let eq_idx = line_text.find('=')?;
            let po_idx = lower[eq_idx + 1..].find("по").map(|idx| eq_idx + 1 + idx)?;
            let range_start = &line_text[eq_idx + 1..po_idx];
            let rel = first_non_ws_byte_index(range_start)?;
            (
                format!("**Цикл:** `Для {} = ... По ... Цикл`", variable),
                "Число",
                (eq_idx + 1) + rel,
            )
        }
        SemanticNodeKind::ForEachLoop { variable, .. } => {
            let (after, expr) = slice_range_after_keyword(line_text, "из", Some("цикл"))?;
            let rel = first_non_ws_byte_index(expr)?;
            (
                format!("**Цикл:** `Для Каждого {} Из ... Цикл`", variable),
                "Коллекция",
                after + rel,
            )
        }
        _ => return None,
    };

    let probe_abs = line_start.checked_add(probe_in_line.try_into().ok()?)?;
    let actual_type = exact_semantic_type_at_offset(analysis, file_id, ir_program, probe_abs)
        .unwrap_or_else(TypeResolution::unknown);

    let mut out = String::new();
    out.push_str(&title);
    out.push_str("\n\n");
    out.push_str(&format_expected_type_hover(expected_type, &actual_type));
    out.push_str(&format!(
        "\n\n📍 Span: {}..{}",
        node.span.start, node.span.end
    ));
    Some(out)
}
