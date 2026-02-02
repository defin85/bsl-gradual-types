//! Hover Service - hover and type-at-position operations
//!
//! Functions for LSP hover requests and getting type information at cursor position.

use tracing::{debug, info, warn};

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::ir::{
    CfgNodeAtByteOffsetBias, SemanticNode, SemanticNodeKind, SemanticProgram, Span,
};

use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
use crate::system::LineIndex;

use super::super::extractors::symbol_extractor::extract_word_at_position;
use super::super::formatters::format_semantic_node_info;
use super::super::formatters::hover_formatters::format_expected_type_hover;
use super::flow_sensitive::narrow_type_for_variable_at;

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
    metadata_lookup: &TypeMetadataLookup,
    hover_formatter: &HoverFormatter,
    hover_config: Option<HoverFormatConfig>,
    resolver: &TypeResolver,
    ir_program: std::sync::Arc<SemanticProgram>,
) -> Option<String> {
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
        hover_config,
    )
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
    let type_at_cursor =
        byte_offset.and_then(|offset| analysis.type_at_byte_offset(file_id, offset).ok().flatten());

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
                let owner_resolution = type_at_span_start(analysis, file_id, owner_span)
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
                        analysis,
                        file_id,
                        file_content,
                        line,
                        column,
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

    if let (Some(word), Some(offset)) = (word_under_cursor.as_deref(), byte_offset) {
        let base = type_at_cursor
            .clone()
            .unwrap_or_else(TypeResolution::unknown);
        if let Some(narrowed) = narrow_type_for_variable_at(
            ir_program,
            offset,
            word,
            base,
            CfgNodeAtByteOffsetBias::Exact,
        ) {
            info!(
                "Hover v2 flow-sensitive narrowed_at({}): {}",
                offset,
                narrowed.type_name()
            );
            return Some(formatter.format_variable(word, &narrowed));
        }
    }

    if let (Some(word), Some(resolution)) = (&word_under_cursor, &type_at_cursor) {
        info!(
            "Hover v2 type_at_byte_offset({}): {}",
            byte_offset.unwrap_or_default(),
            resolution.type_name()
        );
        return Some(formatter.format_variable(word, resolution));
    }

    // Milestone 2.11 Task B1: Logs when symbol not found
    debug!("Hover v2: no type at position {}:{}", line, column);

    // Fallback: old logic by variable name (without AST, since IR cache is used now)
    if let Some(symbol_info) =
        extract_enhanced_symbol_info(resolver, file_content, line, column, None)
    {
        debug!("Fallback: using extract_enhanced_symbol_info");
        return Some(symbol_info);
    }

    warn!("Fallback also failed, returning generic BSL symbol message");
    Some(format!("BSL symbol at position {}:{}", line, column))
}

fn type_at_span_start(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    span: Span,
) -> Option<TypeResolution> {
    analysis
        .type_at_byte_offset(file_id, span.start)
        .ok()
        .flatten()
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
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    line: u32,
    column: u32,
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
    let actual_type = analysis
        .type_at_byte_offset(file_id, probe_abs)
        .ok()
        .flatten()
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

/// Extract symbol information at specified position from AST
fn extract_enhanced_symbol_info(
    resolver: &TypeResolver,
    file_content: &str,
    line: u32,
    column: u32,
    parse_result: Option<&crate::parsing::Program>,
) -> Option<String> {
    use super::super::extractors::type_extractor::expression_to_type_name;
    use crate::parsing::bsl::ast::{Expression, Statement};

    // Step 1: Extract word under cursor
    let word_under_cursor = extract_word_at_position(file_content, line, column)?;

    // Step 2: If AST exists, look for word info in it
    if let Some(parse_result) = parse_result {
        for statement in &parse_result.statements {
            match statement {
                Statement::VarDeclaration { name, .. } if name == &word_under_cursor => {
                    return Some(format!("**Переменная:** `{}`\n\n*Тип:* Неопределено (требуется flow-sensitive анализ)", name));
                }
                #[allow(clippy::collapsible_match)]
                Statement::Assignment { target, value, .. } => {
                    if let Expression::Identifier { name: var_name, .. } = target {
                        if var_name == &word_under_cursor {
                            // Application Layer: Map AST -> type name
                            if let Some(type_name) = expression_to_type_name(value) {
                                // Domain Layer: Resolve via TypeResolver
                                let resolution = resolver.resolve_expression_sync(&type_name);
                                let type_info = format_type_for_hover(&type_name, &resolution);
                                return Some(format!(
                                    "**Присваивание:** `{} = ...`\n\n{}",
                                    var_name, type_info
                                ));
                            } else {
                                return Some(format!("**Присваивание:** `{} = ...`\n\n*Тип:* Требуется расширенный анализ", var_name));
                            }
                        }
                    }
                }
                Statement::FunctionDecl {
                    name,
                    params,
                    compiler_directive,
                    is_export: _,
                    ..
                } if name == &word_under_cursor => {
                    let directive_str = match compiler_directive {
                        Some(d) => format!("\n\n*Директива:* {:?}", d),
                        None => String::new(),
                    };
                    return Some(format!(
                        "**Функция:** `{}({})`\n\n*Параметры:* {}{}",
                        name,
                        params.join(", "),
                        params.len(),
                        directive_str
                    ));
                }
                Statement::ProcedureDecl {
                    name,
                    params,
                    compiler_directive,
                    is_export: _,
                    ..
                } if name == &word_under_cursor => {
                    let directive_str = match compiler_directive {
                        Some(d) => format!("\n\n*Директива:* {:?}", d),
                        None => String::new(),
                    };
                    return Some(format!(
                        "**Процедура:** `{}({})`\n\n*Параметры:* {}{}",
                        name,
                        params.join(", "),
                        params.len(),
                        directive_str
                    ));
                }
                _ => {}
            }
        }
    }

    // Step 3: Fallback - try to resolve type for identifier via TypeResolver
    resolve_type_for_identifier(resolver, &word_under_cursor)
}

/// Try to resolve type for identifier via TypeResolver
fn resolve_type_for_identifier(resolver: &TypeResolver, identifier: &str) -> Option<String> {
    use bsl_shared::domain::types::Certainty;

    // Domain Layer: Resolve via TypeResolver
    let resolution = resolver.resolve_expression_sync(identifier);

    // Check if type was found
    if !matches!(resolution.certainty, Certainty::Unknown) {
        let type_info = format_type_for_hover(identifier, &resolution);
        return Some(format!(
            "**Тип платформы:** `{}`\n\n{}",
            identifier, type_info
        ));
    }

    // If not found - local variable or unknown type
    Some(format!(
        "**Идентификатор:** `{}`\n\n*Информация:* Локальная переменная или неизвестный тип\n\n*Подсказка:* Для точного определения типа требуется flow-sensitive анализ",
        identifier
    ))
}

/// Format TypeResolution for hover tooltip with full type description
fn format_type_for_hover(type_name: &str, resolution: &TypeResolution) -> String {
    use super::super::formatters::type_formatters::format_resolution_result;

    let type_str = format_resolution_result(&resolution.result);
    format!(
        "**Тип:** `{}`\n\n*Категория:* {:?}\n*Certainty:* {:?}\n*Структура:* {}",
        type_name, resolution.source, resolution.certainty, type_str
    )
}
