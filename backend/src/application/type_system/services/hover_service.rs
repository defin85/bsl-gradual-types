//! Hover Service - hover and type-at-position operations
//!
//! Functions for LSP hover requests and getting type information at cursor position.

use tracing::{debug, info, warn};

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};

use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};

use super::super::extractors::symbol_extractor::extract_word_at_position;
use super::super::formatters::format_semantic_node_info;

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

    let node_at_position = ir_program.find_node_at_position(line, column);
    let word_under_cursor = extract_word_at_position(file_content, line, column);
    let type_at_cursor = analysis.type_at_position(file_id, line, column).ok().flatten();

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
            if access_kind.is_property() {
                if word_under_cursor
                    .as_ref()
                    .is_some_and(|word| word.eq_ignore_ascii_case(member_name))
                {
                    let owner_span = object_node
                        .and_then(|idx| ir_program.nodes.get(idx).map(|n| n.span))
                        .unwrap_or(node.span);
                    let mut owner_resolution =
                        type_at_span_start(analysis, file_id, owner_span).unwrap_or_else(|| {
                            // Если inference не дал результат, пробуем доменный резолвер по имени.
                            TypeResolution::unknown()
                        });
                    if owner_resolution.is_unknown() || owner_resolution.is_dynamic() {
                        if let Some(object_ident) = object_name.as_deref() {
                            let resolved = resolver.resolve_variable_with_context(
                                object_ident,
                                &ir_program.symbols,
                                node.scope_id,
                            );
                            if !resolved.is_unknown() && !resolved.is_dynamic() {
                                owner_resolution = resolved;
                            }
                        }
                    }

                    let (prop_type, is_readonly) = metadata_lookup
                        .get_properties(&owner_resolution)
                        .into_iter()
                        .find(|p| p.name.eq_ignore_ascii_case(member_name))
                        .map(|p| (p.prop_type, Some(p.is_readonly)))
                        .unwrap_or_else(|| (String::new(), None));

                    let property_resolution = if !prop_type.trim().is_empty() {
                        resolver.resolve_expression_sync(&prop_type)
                    } else {
                        // Фоллбек: иногда owner (например, `Объект` в модуле формы) не выводится
                        // через v2 type_at_position, но резолвится через доменный TypeResolver.
                        if let Some(object_ident) = object_name.as_deref() {
                            let expr = format!("{}.{}", object_ident, member_name);
                            let resolved = resolver.resolve_expression_sync(&expr);
                            if !resolved.is_unknown() {
                                return Some(formatter.format_property(
                                    Some(object_ident),
                                    &owner_resolution,
                                    member_name,
                                    &resolved,
                                    is_readonly,
                                ));
                            }
                        }
                        type_at_cursor.clone().unwrap_or_else(TypeResolution::unknown)
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
    }

    // IR-level hover для управляющих конструкций (если/пока/для), чтобы сохранять поведение
    // существующих тестов и UI (и избежать legacy flow-sensitive логики).
    if let Some(word) = word_under_cursor.as_deref() {
        if let Some(node) = control_node_at_position(ir_program, line, column) {
            if control_hover_requested(node, word) {
                return Some(format_semantic_node_info(node, file_content, metadata_lookup));
            }
        }
    }

    // Hover на вызове функции/метода должен показывать FunctionCall, а не "Переменная".
    if let (Some(node), Some(word)) = (node_at_position, word_under_cursor.as_deref()) {
        if let SemanticNodeKind::FunctionCall { function_name, .. } = &node.kind {
            if word.eq_ignore_ascii_case(function_name) {
                return Some(format_semantic_node_info(node, file_content, metadata_lookup));
            }
        }
    }

    if let (Some(word), Some(resolution)) = (&word_under_cursor, &type_at_cursor) {
        info!(
            "Hover v2 type_at_position({}, {}): {}",
            line,
            column,
            resolution.type_name()
        );
        return Some(formatter.format_variable(word, resolution));
    }

    // Milestone 2.11 Task B1: Logs when symbol not found
    debug!(
        "Hover v2: no type at position {}:{}",
        line, column
    );

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
        .type_at_position(file_id, span.start_line, span.start_column)
        .ok()
        .flatten()
}

fn control_node_at_position(
    ir_program: &SemanticProgram,
    line: u32,
    column: u32,
) -> Option<&SemanticNode> {
    ir_program
        .nodes
        .iter()
        .filter(|node| match node.kind {
            SemanticNodeKind::IfStatement { .. }
            | SemanticNodeKind::WhileLoop { .. }
            | SemanticNodeKind::ForLoop { .. }
            | SemanticNodeKind::ForEachLoop { .. } => true,
            _ => false,
        })
        .filter(|node| span_contains_position(node.span, line, column))
        .min_by_key(|node| {
            (
                node.span.end_line.saturating_sub(node.span.start_line),
                node.span.end_column.saturating_sub(node.span.start_column),
            )
        })
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

fn span_contains_position(span: Span, line: u32, column: u32) -> bool {
    if line < span.start_line || line > span.end_line {
        return false;
    }

    if span.start_line == span.end_line {
        return column >= span.start_column && column <= span.end_column;
    }

    if line == span.start_line {
        return column >= span.start_column;
    }

    if line == span.end_line {
        return column <= span.end_column;
    }

    true
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
