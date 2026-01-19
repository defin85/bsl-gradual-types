//! Hover Service - hover and type-at-position operations
//!
//! Functions for LSP hover requests and getting type information at cursor position.

use tracing::{debug, info, warn};

use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::ir::{ScopeId, SemanticNodeKind, SemanticProgram};

use crate::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};

use super::super::extractors::symbol_extractor::extract_word_at_position;
use super::super::formatters::hover_formatters::format_semantic_node_info;

/// Hover по уже готовому `SemanticProgram` (без legacy парсинга/IR build).
///
/// Используется в IntelliSense v2 (salsa) hot path.
pub fn get_hover_info_with_semantic_program(
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

    // Специальный кейс: hover на имени свойства (obj.Property) должен показывать тип свойства,
    // а не тип переменной-объекта слева от точки.
    if let Some(node) = node_at_position {
        if let SemanticNodeKind::MemberAccess {
            object_name,
            object_type,
            member_name,
            access_kind,
            result_type,
            ..
        } = &node.kind
        {
            if access_kind.is_property() {
                if let Some(word_under_cursor) =
                    extract_word_at_position(file_content, line, column)
                {
                    if word_under_cursor.eq_ignore_ascii_case(member_name) {
                        // 1) Тип объекта-владельца (flow-sensitive, если объект - переменная)
                        let owner_resolution = if let Some(obj_name) = object_name.as_deref() {
                            if let Some(flow_type) = find_variable_type_at_position(
                                ir_program,
                                obj_name,
                                node.scope_id,
                                line,
                            ) {
                                flow_type
                            } else {
                                resolver.resolve_variable_with_context(
                                    obj_name,
                                    &ir_program.symbols,
                                    node.scope_id,
                                )
                            }
                        } else {
                            object_type.clone()
                        };

                        // 2) Тип свойства из метаданных (если есть), иначе fallback на result_type узла
                        let (prop_type, is_readonly) = metadata_lookup
                            .get_properties(&owner_resolution)
                            .into_iter()
                            .find(|p| p.name.eq_ignore_ascii_case(member_name))
                            .map(|p| (p.prop_type, Some(p.is_readonly)))
                            .unwrap_or_else(|| (String::new(), None));

                        let property_resolution = if !prop_type.trim().is_empty() {
                            resolver.resolve_expression_sync(&prop_type)
                        } else {
                            resolver.resolve_expression_sync(&result_type.type_name())
                        };
                        let formatter = if let Some(config) = hover_config.clone() {
                            HoverFormatter::new(config, metadata_lookup.clone())
                        } else {
                            hover_formatter.clone()
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
    }

    // Direction 2: Use find_variable_with_scope() for Generic inference
    if let Some((var_name, _type_hint, scope_id)) =
        ir_program.find_variable_with_scope(line, column)
    {
        info!(
            "find_variable_with_scope({}, {}) found variable: '{}' in scope {:?}",
            line, column, var_name, scope_id
        );

        // FLOW-SENSITIVE: Find type at specific position (not final!)
        let resolution = if let Some(flow_type) =
            find_variable_type_at_position(ir_program, &var_name, scope_id, line)
        {
            info!(
                "Flow-sensitive type for '{}' at line {}: {}",
                var_name,
                line,
                flow_type.type_name()
            );
            flow_type
        } else {
            // Fallback: use SymbolTable (final type)
            resolver.resolve_variable_with_context(&var_name, &ir_program.symbols, scope_id)
        };

        // MILESTONE 3.6 Phase 1: Use passed config or default
        let formatter = if let Some(config) = hover_config.clone() {
            HoverFormatter::new(config, metadata_lookup.clone())
        } else {
            hover_formatter.clone()
        };

        // Format hover via TypeResolution (instead of old TypeHint enum)
        return Some(formatter.format_variable(&var_name, &resolution));
    }

    // Milestone 2.11 Task B1: Logs when variable not found
    debug!(
        "find_variable_with_scope({}, {}) did not find variable",
        line, column
    );

    // Fallback 1: Try find_node_at_position for other nodes (functions, loops, etc.)
    if let Some(node) = node_at_position {
        info!(
            "find_node_at_position({}, {}) found node (not variable): span={:?}",
            line, column, node.span
        );
        debug!("Found node: {:?} at span {:?}", node.kind, node.span);
        if let SemanticNodeKind::FunctionCall {
            function_name,
            object_type,
            ..
        } = &node.kind
        {
            if let Some(signature) =
                metadata_lookup.find_method_signature_for_call(object_type.as_ref(), function_name)
            {
                let formatter = if let Some(config) = hover_config.clone() {
                    HoverFormatter::new(config, metadata_lookup.clone())
                } else {
                    hover_formatter.clone()
                };
                let label = if object_type.is_some() {
                    "Метод"
                } else {
                    "Функция"
                };
                return Some(formatter.format_function_signature(label, &signature));
            }
        }
        return Some(format_semantic_node_info(
            node,
            file_content,
            metadata_lookup,
        ));
    }

    // Milestone 2.11 Task B1: Warning when node not found
    warn!("No node found at position {}:{} in IR", line, column);

    // Fallback 2: old logic by variable name (without AST, since IR cache is used now)
    if let Some(symbol_info) =
        extract_enhanced_symbol_info(resolver, file_content, line, column, None)
    {
        debug!("Fallback: using extract_enhanced_symbol_info");
        return Some(symbol_info);
    }

    warn!("Fallback also failed, returning generic BSL symbol message");
    Some(format!("BSL symbol at position {}:{}", line, column))
}

// === Helper functions ===

/// Flow-sensitive search for variable type at specified position
///
/// Searches for the last assignment to the variable BEFORE the specified line,
/// to show the actual type at cursor position.
pub(crate) fn find_variable_type_at_position(
    ir_program: &SemanticProgram,
    var_name: &str,
    target_scope: ScopeId,
    line: u32,
) -> Option<TypeResolution> {
    let mut assignments: Vec<(u32, TypeResolution)> = Vec::new();

    for node in &ir_program.nodes {
        // Check scope visibility (current or parent)
        if !is_scope_visible(ir_program, node.scope_id, target_scope) {
            continue;
        }

        match &node.kind {
            // Variable assignment
            SemanticNodeKind::Assignment {
                variable,
                value_type,
                ..
            } if variable.eq_ignore_ascii_case(var_name) && node.span.start_line <= line => {
                assignments.push((node.span.start_line, value_type.clone()));
            }
            // Declaration with initialization
            SemanticNodeKind::VariableDeclaration {
                name,
                initial_value_type: Some(value_type),
                ..
            } if name.eq_ignore_ascii_case(var_name) && node.span.start_line <= line => {
                assignments.push((node.span.start_line, value_type.clone()));
            }
            _ => {}
        }
    }

    // Sort by line and take last assignment
    assignments.sort_by_key(|(line, _)| *line);
    assignments.last().map(|(_, res)| res.clone())
}

/// Check scope visibility from another scope
fn is_scope_visible(
    ir_program: &SemanticProgram,
    source_scope: ScopeId,
    target_scope: ScopeId,
) -> bool {
    if source_scope == target_scope {
        return true;
    }

    // Check parent chain
    let mut current = Some(target_scope);
    while let Some(scope_id) = current {
        if scope_id == source_scope {
            return true;
        }
        current = ir_program
            .symbols
            .scopes
            .get(&scope_id)
            .and_then(|s| s.parent);
    }
    false
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
