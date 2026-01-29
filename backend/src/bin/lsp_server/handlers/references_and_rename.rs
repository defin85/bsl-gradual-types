use std::collections::{HashMap, HashSet};

use bsl_line_index::LineIndex;
use bsl_syntax::ast::{Expression, ParseResult, Statement};
use tower_lsp::lsp_types::{
    Location, Position, PrepareRenameResponse, Range, RenameParams, TextDocumentPositionParams,
    TextEdit, Url, WorkspaceEdit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutineKind {
    Function,
    Procedure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SymbolTarget {
    LocalVar {
        name: String,
        routine_span: bsl_shared::ir::Span,
        decl_range: Range,
    },
    Routine {
        name: String,
        kind: RoutineKind,
        decl_range: Range,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum RenameError {
    #[error("invalid new name")]
    InvalidNewName,
    #[error("rename not supported for this symbol")]
    Unsupported,
}

pub fn handle_references(
    source: &str,
    parse_result: &ParseResult,
    uri: &Url,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let line_index = LineIndex::new(source);
    let target = resolve_target_at_position(source, &line_index, parse_result, position)?;

    let ranges = collect_target_ranges(
        source,
        &line_index,
        parse_result,
        &target,
        include_declaration,
    );
    Some(
        ranges
            .into_iter()
            .map(|range| Location {
                uri: uri.clone(),
                range,
            })
            .collect(),
    )
}

pub fn handle_prepare_rename(
    source: &str,
    parse_result: &ParseResult,
    position_params: TextDocumentPositionParams,
) -> Option<PrepareRenameResponse> {
    let line_index = LineIndex::new(source);
    let position = position_params.position;
    let target = resolve_target_at_position(source, &line_index, parse_result, position)?;

    let (range, placeholder) = match target {
        SymbolTarget::LocalVar { ref name, .. } => {
            let range =
                find_local_var_occurrence_range(source, &line_index, parse_result, position, name)
                    .or_else(|| {
                        let SymbolTarget::LocalVar { decl_range, .. } = target else {
                            return None;
                        };
                        Some(decl_range)
                    })?;
            (range, name.clone())
        }
        SymbolTarget::Routine {
            ref name,
            decl_range,
            ..
        } => (decl_range, name.clone()),
    };

    Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder })
}

pub fn handle_rename(
    source: &str,
    parse_result: &ParseResult,
    params: RenameParams,
) -> Result<WorkspaceEdit, RenameError> {
    if params.new_name.trim().is_empty() || params.new_name.chars().any(|c| c.is_whitespace()) {
        return Err(RenameError::InvalidNewName);
    }
    if !is_valid_identifier_name(&params.new_name) {
        return Err(RenameError::InvalidNewName);
    }

    let line_index = LineIndex::new(source);
    let position = params.text_document_position.position;
    let target = resolve_target_at_position(source, &line_index, parse_result, position)
        .ok_or(RenameError::Unsupported)?;

    let target_name = match &target {
        SymbolTarget::LocalVar { name, .. } => name.as_str(),
        SymbolTarget::Routine { name, .. } => name.as_str(),
    };
    if target_name == params.new_name {
        return Ok(WorkspaceEdit::default());
    }

    let ranges = collect_target_ranges(source, &line_index, parse_result, &target, true);
    let edits: Vec<TextEdit> = ranges
        .into_iter()
        .map(|range| TextEdit {
            range,
            new_text: params.new_name.clone(),
        })
        .collect();

    let mut changes = HashMap::<Url, Vec<TextEdit>>::new();
    changes.insert(params.text_document_position.text_document.uri, edits);

    Ok(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

fn is_valid_identifier_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_alphabetic()) {
        return false;
    }
    chars.all(|c| c == '_' || c.is_alphanumeric())
}

fn resolve_target_at_position(
    source: &str,
    line_index: &LineIndex,
    parse_result: &ParseResult,
    position: Position,
) -> Option<SymbolTarget> {
    // 1) Local variable (VarDeclaration inside enclosing routine)
    if let Some((routine_kind, routine_name, routine_span, routine_body)) =
        find_enclosing_routine(source, line_index, parse_result, position)
    {
        let local_decls = collect_local_var_decls(source, line_index, &routine_body);

        // Cursor on declaration name.
        for decl in &local_decls {
            if range_contains_position(decl.decl_range, position) {
                if is_ambiguous_local_name(&local_decls, &decl.name) {
                    return None;
                }
                return Some(SymbolTarget::LocalVar {
                    name: decl.name.clone(),
                    routine_span,
                    decl_range: decl.decl_range,
                });
            }
        }

        // Cursor on usage identifier inside routine body.
        if let Some(name) =
            find_identifier_at_position_in_statements(source, line_index, &routine_body, position)
        {
            if is_ambiguous_local_name(&local_decls, &name) {
                return None;
            }
            if let Some(decl) = local_decls.iter().find(|d| d.name == name) {
                return Some(SymbolTarget::LocalVar {
                    name,
                    routine_span,
                    decl_range: decl.decl_range,
                });
            }
        }

        // Cursor on routine name itself => routine target.
        let routine_decl_range = selection_range_for_name_in_span_line(
            source,
            line_index,
            &routine_span,
            &routine_name,
        )?;
        if range_contains_position(routine_decl_range, position) {
            return Some(SymbolTarget::Routine {
                name: routine_name,
                kind: routine_kind,
                decl_range: routine_decl_range,
            });
        }
    }

    // 2) Routine by declaration at top-level.
    for stmt in &parse_result.program.statements {
        let (kind, name, span) = match stmt {
            Statement::FunctionDecl { name, span, .. } => {
                (RoutineKind::Function, name.clone(), *span)
            }
            Statement::ProcedureDecl { name, span, .. } => {
                (RoutineKind::Procedure, name.clone(), *span)
            }
            _ => continue,
        };
        let decl_range = selection_range_for_name_in_span_line(source, line_index, &span, &name)?;
        if range_contains_position(decl_range, position) {
            return Some(SymbolTarget::Routine {
                name,
                kind,
                decl_range,
            });
        }
    }

    // 3) Routine by call-site (direct call Identifier(...)) if declaration exists in the document.
    if let Some(name) =
        find_called_identifier_at_position(source, line_index, parse_result, position)
    {
        for stmt in &parse_result.program.statements {
            match stmt {
                Statement::FunctionDecl {
                    name: decl_name,
                    span,
                    ..
                }
                | Statement::ProcedureDecl {
                    name: decl_name,
                    span,
                    ..
                } => {
                    if decl_name != &name {
                        continue;
                    }
                    let decl_range =
                        selection_range_for_name_in_span_line(source, line_index, span, decl_name)?;
                    let kind = match stmt {
                        Statement::FunctionDecl { .. } => RoutineKind::Function,
                        Statement::ProcedureDecl { .. } => RoutineKind::Procedure,
                        _ => unreachable!(),
                    };
                    return Some(SymbolTarget::Routine {
                        name,
                        kind,
                        decl_range,
                    });
                }
                _ => {}
            }
        }
    }

    None
}

fn collect_target_ranges(
    source: &str,
    line_index: &LineIndex,
    parse_result: &ParseResult,
    target: &SymbolTarget,
    include_declaration: bool,
) -> Vec<Range> {
    let mut ranges = Vec::<Range>::new();
    match target {
        SymbolTarget::LocalVar {
            name,
            routine_span: _,
            decl_range,
        } => {
            let Some((_kind, _routine_name, _span, body)) =
                find_enclosing_routine(source, line_index, parse_result, decl_range.start)
            else {
                return Vec::new();
            };

            if include_declaration {
                ranges.push(*decl_range);
            }
            collect_identifier_ranges_in_statements(source, line_index, &body, name, &mut ranges);
        }
        SymbolTarget::Routine {
            name, decl_range, ..
        } => {
            if include_declaration {
                ranges.push(*decl_range);
            }
            collect_routine_call_ranges_in_program(
                source,
                line_index,
                parse_result,
                name,
                &mut ranges,
            );
        }
    }

    let mut seen = HashSet::<(u32, u32, u32, u32)>::new();
    ranges.retain(|range| {
        seen.insert((
            range.start.line,
            range.start.character,
            range.end.line,
            range.end.character,
        ))
    });
    ranges.sort_by(|a, b| cmp_pos(a.start, b.start));
    ranges
}

#[derive(Debug, Clone)]
struct LocalDecl {
    name: String,
    decl_range: Range,
}

fn is_ambiguous_local_name(local_decls: &[LocalDecl], name: &str) -> bool {
    local_decls.iter().filter(|d| d.name == name).count() > 1
}

fn collect_local_var_decls(
    source: &str,
    line_index: &LineIndex,
    body: &[Statement],
) -> Vec<LocalDecl> {
    let mut out = Vec::new();
    for stmt in body {
        match stmt {
            Statement::VarDeclaration { name, span, .. } => {
                let Some(decl_range) =
                    selection_range_for_name_in_span_line(source, line_index, span, name)
                else {
                    continue;
                };
                out.push(LocalDecl {
                    name: name.clone(),
                    decl_range,
                });
            }
            Statement::If {
                then_body,
                else_body,
                ..
            } => {
                out.extend(collect_local_var_decls(source, line_index, then_body));
                if let Some(else_body) = else_body {
                    out.extend(collect_local_var_decls(source, line_index, else_body));
                }
            }
            Statement::For { body, .. }
            | Statement::ForEach { body, .. }
            | Statement::While { body, .. } => {
                out.extend(collect_local_var_decls(source, line_index, body));
            }
            Statement::Try {
                try_body,
                except_body,
                ..
            } => {
                out.extend(collect_local_var_decls(source, line_index, try_body));
                out.extend(collect_local_var_decls(source, line_index, except_body));
            }
            Statement::FunctionDecl { .. } | Statement::ProcedureDecl { .. } => {}
            _ => {}
        }
    }
    out
}

fn find_enclosing_routine(
    source: &str,
    line_index: &LineIndex,
    parse_result: &ParseResult,
    position: Position,
) -> Option<(RoutineKind, String, bsl_shared::ir::Span, Vec<Statement>)> {
    for stmt in &parse_result.program.statements {
        match stmt {
            Statement::FunctionDecl {
                name, body, span, ..
            } => {
                if span_contains_position(source, line_index, *span, position) {
                    return Some((RoutineKind::Function, name.clone(), *span, body.clone()));
                }
            }
            Statement::ProcedureDecl {
                name, body, span, ..
            } => {
                if span_contains_position(source, line_index, *span, position) {
                    return Some((RoutineKind::Procedure, name.clone(), *span, body.clone()));
                }
            }
            _ => {}
        }
    }
    None
}

fn find_local_var_occurrence_range(
    source: &str,
    line_index: &LineIndex,
    parse_result: &ParseResult,
    position: Position,
    name: &str,
) -> Option<Range> {
    let (_kind, _routine_name, _span, body) =
        find_enclosing_routine(source, line_index, parse_result, position)?;
    find_identifier_range_at_position_in_statements(source, line_index, &body, position, name)
}

fn find_identifier_at_position_in_statements(
    source: &str,
    line_index: &LineIndex,
    stmts: &[Statement],
    position: Position,
) -> Option<String> {
    for stmt in stmts {
        if let Some(name) =
            find_identifier_at_position_in_statement(source, line_index, stmt, position)
        {
            return Some(name);
        }
    }
    None
}

fn find_identifier_at_position_in_statement(
    source: &str,
    line_index: &LineIndex,
    stmt: &Statement,
    position: Position,
) -> Option<String> {
    match stmt {
        Statement::Assignment { target, value, .. } => find_identifier_at_position_in_expression(
            source, line_index, target, position,
        )
        .or_else(|| find_identifier_at_position_in_expression(source, line_index, value, position)),
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => find_identifier_at_position_in_expression(source, line_index, condition, position)
            .or_else(|| {
                find_identifier_at_position_in_statements(source, line_index, then_body, position)
                    .or_else(|| {
                        else_body.as_ref().and_then(|b| {
                            find_identifier_at_position_in_statements(
                                source, line_index, b, position,
                            )
                        })
                    })
            }),
        Statement::For {
            start, end, body, ..
        } => find_identifier_at_position_in_expression(source, line_index, start, position)
            .or_else(|| {
                find_identifier_at_position_in_expression(source, line_index, end, position)
            })
            .or_else(|| {
                find_identifier_at_position_in_statements(source, line_index, body, position)
            }),
        Statement::ForEach {
            collection, body, ..
        } => find_identifier_at_position_in_expression(source, line_index, collection, position)
            .or_else(|| {
                find_identifier_at_position_in_statements(source, line_index, body, position)
            }),
        Statement::While {
            condition, body, ..
        } => find_identifier_at_position_in_expression(source, line_index, condition, position)
            .or_else(|| {
                find_identifier_at_position_in_statements(source, line_index, body, position)
            }),
        Statement::Return { value, .. } => value.as_ref().and_then(|e| {
            find_identifier_at_position_in_expression(source, line_index, e, position)
        }),
        Statement::Try {
            try_body,
            except_body,
            ..
        } => find_identifier_at_position_in_statements(source, line_index, try_body, position)
            .or_else(|| {
                find_identifier_at_position_in_statements(source, line_index, except_body, position)
            }),
        Statement::Call { expression, .. } => {
            find_identifier_at_position_in_expression(source, line_index, expression, position)
        }
        Statement::Execute { code, .. } => {
            find_identifier_at_position_in_expression(source, line_index, code, position)
        }
        Statement::RaiseError { message, .. } => message.as_ref().and_then(|e| {
            find_identifier_at_position_in_expression(source, line_index, e, position)
        }),
        Statement::AddHandler { event, handler, .. } => {
            find_identifier_at_position_in_expression(source, line_index, event, position).or_else(
                || find_identifier_at_position_in_expression(source, line_index, handler, position),
            )
        }
        Statement::RemoveHandler { event, handler, .. } => {
            find_identifier_at_position_in_expression(source, line_index, event, position).or_else(
                || find_identifier_at_position_in_expression(source, line_index, handler, position),
            )
        }
        Statement::Await { expression, .. } => {
            find_identifier_at_position_in_expression(source, line_index, expression, position)
        }
        Statement::FunctionDecl { .. }
        | Statement::ProcedureDecl { .. }
        | Statement::VarDeclaration { .. }
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::Goto { .. }
        | Statement::Label { .. } => None,
    }
}

fn find_identifier_at_position_in_expression(
    source: &str,
    line_index: &LineIndex,
    expr: &Expression,
    position: Position,
) -> Option<String> {
    match expr {
        Expression::Identifier { name, span } => {
            if span_contains_position(source, line_index, *span, position) {
                Some(name.clone())
            } else {
                None
            }
        }
        Expression::Call { function, args, .. } => {
            find_identifier_at_position_in_expression(source, line_index, function, position)
                .or_else(|| {
                    args.iter().find_map(|a| {
                        find_identifier_at_position_in_expression(source, line_index, a, position)
                    })
                })
        }
        Expression::Binary { left, right, .. } => find_identifier_at_position_in_expression(
            source, line_index, left, position,
        )
        .or_else(|| find_identifier_at_position_in_expression(source, line_index, right, position)),
        Expression::Unary { operand, .. } => {
            find_identifier_at_position_in_expression(source, line_index, operand, position)
        }
        Expression::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => find_identifier_at_position_in_expression(source, line_index, condition, position)
            .or_else(|| {
                find_identifier_at_position_in_expression(source, line_index, then_expr, position)
            })
            .or_else(|| {
                find_identifier_at_position_in_expression(source, line_index, else_expr, position)
            }),
        Expression::New { args, .. } => args.iter().find_map(|a| {
            find_identifier_at_position_in_expression(source, line_index, a, position)
        }),
        Expression::PropertyAccess { object, .. } => {
            find_identifier_at_position_in_expression(source, line_index, object, position)
        }
        Expression::IndexAccess { object, index, .. } => find_identifier_at_position_in_expression(
            source, line_index, object, position,
        )
        .or_else(|| find_identifier_at_position_in_expression(source, line_index, index, position)),
        Expression::Await { expression, .. } => {
            find_identifier_at_position_in_expression(source, line_index, expression, position)
        }
        Expression::String { .. }
        | Expression::Number { .. }
        | Expression::Boolean { .. }
        | Expression::Date { .. } => None,
    }
}

fn find_identifier_range_at_position_in_statements(
    source: &str,
    line_index: &LineIndex,
    stmts: &[Statement],
    position: Position,
    name: &str,
) -> Option<Range> {
    for stmt in stmts {
        if let Some(range) =
            find_identifier_range_at_position_in_statement(source, line_index, stmt, position, name)
        {
            return Some(range);
        }
    }
    None
}

fn find_identifier_range_at_position_in_statement(
    source: &str,
    line_index: &LineIndex,
    stmt: &Statement,
    position: Position,
    name: &str,
) -> Option<Range> {
    match stmt {
        Statement::Assignment { target, value, .. } => {
            find_identifier_range_at_position_in_expression(
                source, line_index, target, position, name,
            )
            .or_else(|| {
                find_identifier_range_at_position_in_expression(
                    source, line_index, value, position, name,
                )
            })
        }
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => find_identifier_range_at_position_in_expression(
            source, line_index, condition, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_statements(
                source, line_index, then_body, position, name,
            )
            .or_else(|| {
                else_body.as_ref().and_then(|b| {
                    find_identifier_range_at_position_in_statements(
                        source, line_index, b, position, name,
                    )
                })
            })
        }),
        Statement::For {
            start, end, body, ..
        } => find_identifier_range_at_position_in_expression(
            source, line_index, start, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_expression(source, line_index, end, position, name)
        })
        .or_else(|| {
            find_identifier_range_at_position_in_statements(
                source, line_index, body, position, name,
            )
        }),
        Statement::ForEach {
            collection, body, ..
        } => find_identifier_range_at_position_in_expression(
            source, line_index, collection, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_statements(
                source, line_index, body, position, name,
            )
        }),
        Statement::While {
            condition, body, ..
        } => find_identifier_range_at_position_in_expression(
            source, line_index, condition, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_statements(
                source, line_index, body, position, name,
            )
        }),
        Statement::Return { value, .. } => value.as_ref().and_then(|e| {
            find_identifier_range_at_position_in_expression(source, line_index, e, position, name)
        }),
        Statement::Try {
            try_body,
            except_body,
            ..
        } => find_identifier_range_at_position_in_statements(
            source, line_index, try_body, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_statements(
                source,
                line_index,
                except_body,
                position,
                name,
            )
        }),
        Statement::Call { expression, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, expression, position, name,
        ),
        Statement::Execute { code, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, code, position, name,
        ),
        Statement::RaiseError { message, .. } => message.as_ref().and_then(|e| {
            find_identifier_range_at_position_in_expression(source, line_index, e, position, name)
        }),
        Statement::AddHandler { event, handler, .. } => {
            find_identifier_range_at_position_in_expression(
                source, line_index, event, position, name,
            )
            .or_else(|| {
                find_identifier_range_at_position_in_expression(
                    source, line_index, handler, position, name,
                )
            })
        }
        Statement::RemoveHandler { event, handler, .. } => {
            find_identifier_range_at_position_in_expression(
                source, line_index, event, position, name,
            )
            .or_else(|| {
                find_identifier_range_at_position_in_expression(
                    source, line_index, handler, position, name,
                )
            })
        }
        Statement::Await { expression, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, expression, position, name,
        ),
        Statement::FunctionDecl { .. }
        | Statement::ProcedureDecl { .. }
        | Statement::VarDeclaration { .. }
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::Goto { .. }
        | Statement::Label { .. } => None,
    }
}

fn find_identifier_range_at_position_in_expression(
    source: &str,
    line_index: &LineIndex,
    expr: &Expression,
    position: Position,
    name: &str,
) -> Option<Range> {
    match expr {
        Expression::Identifier { name: found, span } => {
            if found == name && span_contains_position(source, line_index, *span, position) {
                Some(range_from_span(source, line_index, *span))
            } else {
                None
            }
        }
        Expression::Call { function, args, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, function, position, name,
        )
        .or_else(|| {
            args.iter().find_map(|a| {
                find_identifier_range_at_position_in_expression(
                    source, line_index, a, position, name,
                )
            })
        }),
        Expression::Binary { left, right, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, left, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_expression(
                source, line_index, right, position, name,
            )
        }),
        Expression::Unary { operand, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, operand, position, name,
        ),
        Expression::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => find_identifier_range_at_position_in_expression(
            source, line_index, condition, position, name,
        )
        .or_else(|| {
            find_identifier_range_at_position_in_expression(
                source, line_index, then_expr, position, name,
            )
        })
        .or_else(|| {
            find_identifier_range_at_position_in_expression(
                source, line_index, else_expr, position, name,
            )
        }),
        Expression::New { args, .. } => args.iter().find_map(|a| {
            find_identifier_range_at_position_in_expression(source, line_index, a, position, name)
        }),
        Expression::PropertyAccess { object, .. } => {
            find_identifier_range_at_position_in_expression(
                source, line_index, object, position, name,
            )
        }
        Expression::IndexAccess { object, index, .. } => {
            find_identifier_range_at_position_in_expression(
                source, line_index, object, position, name,
            )
            .or_else(|| {
                find_identifier_range_at_position_in_expression(
                    source, line_index, index, position, name,
                )
            })
        }
        Expression::Await { expression, .. } => find_identifier_range_at_position_in_expression(
            source, line_index, expression, position, name,
        ),
        Expression::String { .. }
        | Expression::Number { .. }
        | Expression::Boolean { .. }
        | Expression::Date { .. } => None,
    }
}

fn collect_identifier_ranges_in_statements(
    source: &str,
    line_index: &LineIndex,
    stmts: &[Statement],
    name: &str,
    out: &mut Vec<Range>,
) {
    for stmt in stmts {
        collect_identifier_ranges_in_statement(source, line_index, stmt, name, out);
    }
}

fn collect_identifier_ranges_in_statement(
    source: &str,
    line_index: &LineIndex,
    stmt: &Statement,
    name: &str,
    out: &mut Vec<Range>,
) {
    match stmt {
        Statement::Assignment { target, value, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, target, name, out);
            collect_identifier_ranges_in_expression(source, line_index, value, name, out);
        }
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_identifier_ranges_in_expression(source, line_index, condition, name, out);
            collect_identifier_ranges_in_statements(source, line_index, then_body, name, out);
            if let Some(else_body) = else_body {
                collect_identifier_ranges_in_statements(source, line_index, else_body, name, out);
            }
        }
        Statement::For {
            start, end, body, ..
        } => {
            collect_identifier_ranges_in_expression(source, line_index, start, name, out);
            collect_identifier_ranges_in_expression(source, line_index, end, name, out);
            collect_identifier_ranges_in_statements(source, line_index, body, name, out);
        }
        Statement::ForEach {
            collection, body, ..
        } => {
            collect_identifier_ranges_in_expression(source, line_index, collection, name, out);
            collect_identifier_ranges_in_statements(source, line_index, body, name, out);
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_identifier_ranges_in_expression(source, line_index, condition, name, out);
            collect_identifier_ranges_in_statements(source, line_index, body, name, out);
        }
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                collect_identifier_ranges_in_expression(source, line_index, value, name, out);
            }
        }
        Statement::Try {
            try_body,
            except_body,
            ..
        } => {
            collect_identifier_ranges_in_statements(source, line_index, try_body, name, out);
            collect_identifier_ranges_in_statements(source, line_index, except_body, name, out);
        }
        Statement::Call { expression, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, expression, name, out)
        }
        Statement::Execute { code, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, code, name, out)
        }
        Statement::RaiseError { message, .. } => {
            if let Some(message) = message {
                collect_identifier_ranges_in_expression(source, line_index, message, name, out);
            }
        }
        Statement::AddHandler { event, handler, .. }
        | Statement::RemoveHandler { event, handler, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, event, name, out);
            collect_identifier_ranges_in_expression(source, line_index, handler, name, out);
        }
        Statement::Await { expression, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, expression, name, out)
        }
        Statement::FunctionDecl { .. }
        | Statement::ProcedureDecl { .. }
        | Statement::VarDeclaration { .. }
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::Goto { .. }
        | Statement::Label { .. } => {}
    }
}

fn collect_identifier_ranges_in_expression(
    source: &str,
    line_index: &LineIndex,
    expr: &Expression,
    name: &str,
    out: &mut Vec<Range>,
) {
    match expr {
        Expression::Identifier { name: found, span } => {
            if found == name {
                out.push(range_from_span(source, line_index, *span));
            }
        }
        Expression::Call { function, args, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, function, name, out);
            for arg in args {
                collect_identifier_ranges_in_expression(source, line_index, arg, name, out);
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, left, name, out);
            collect_identifier_ranges_in_expression(source, line_index, right, name, out);
        }
        Expression::Unary { operand, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, operand, name, out)
        }
        Expression::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_identifier_ranges_in_expression(source, line_index, condition, name, out);
            collect_identifier_ranges_in_expression(source, line_index, then_expr, name, out);
            collect_identifier_ranges_in_expression(source, line_index, else_expr, name, out);
        }
        Expression::New { args, .. } => {
            for arg in args {
                collect_identifier_ranges_in_expression(source, line_index, arg, name, out);
            }
        }
        Expression::PropertyAccess { object, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, object, name, out)
        }
        Expression::IndexAccess { object, index, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, object, name, out);
            collect_identifier_ranges_in_expression(source, line_index, index, name, out);
        }
        Expression::Await { expression, .. } => {
            collect_identifier_ranges_in_expression(source, line_index, expression, name, out)
        }
        Expression::String { .. }
        | Expression::Number { .. }
        | Expression::Boolean { .. }
        | Expression::Date { .. } => {}
    }
}

fn collect_routine_call_ranges_in_program(
    source: &str,
    line_index: &LineIndex,
    parse_result: &ParseResult,
    routine_name: &str,
    out: &mut Vec<Range>,
) {
    for stmt in &parse_result.program.statements {
        match stmt {
            Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => {
                collect_routine_call_ranges_in_statements(
                    source,
                    line_index,
                    body,
                    routine_name,
                    out,
                );
            }
            _ => {}
        }
    }
}

fn collect_routine_call_ranges_in_statements(
    source: &str,
    line_index: &LineIndex,
    stmts: &[Statement],
    routine_name: &str,
    out: &mut Vec<Range>,
) {
    for stmt in stmts {
        collect_routine_call_ranges_in_statement(source, line_index, stmt, routine_name, out);
    }
}

fn collect_routine_call_ranges_in_statement(
    source: &str,
    line_index: &LineIndex,
    stmt: &Statement,
    routine_name: &str,
    out: &mut Vec<Range>,
) {
    match stmt {
        Statement::Assignment { target, value, .. } => {
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                target,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_expression(source, line_index, value, routine_name, out);
        }
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                condition,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_statements(
                source,
                line_index,
                then_body,
                routine_name,
                out,
            );
            if let Some(else_body) = else_body {
                collect_routine_call_ranges_in_statements(
                    source,
                    line_index,
                    else_body,
                    routine_name,
                    out,
                );
            }
        }
        Statement::For {
            start, end, body, ..
        } => {
            collect_routine_call_ranges_in_expression(source, line_index, start, routine_name, out);
            collect_routine_call_ranges_in_expression(source, line_index, end, routine_name, out);
            collect_routine_call_ranges_in_statements(source, line_index, body, routine_name, out);
        }
        Statement::ForEach {
            collection, body, ..
        } => {
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                collection,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_statements(source, line_index, body, routine_name, out);
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                condition,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_statements(source, line_index, body, routine_name, out);
        }
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                collect_routine_call_ranges_in_expression(
                    source,
                    line_index,
                    value,
                    routine_name,
                    out,
                );
            }
        }
        Statement::Try {
            try_body,
            except_body,
            ..
        } => {
            collect_routine_call_ranges_in_statements(
                source,
                line_index,
                try_body,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_statements(
                source,
                line_index,
                except_body,
                routine_name,
                out,
            );
        }
        Statement::Call { expression, .. } => collect_routine_call_ranges_in_expression(
            source,
            line_index,
            expression,
            routine_name,
            out,
        ),
        Statement::Execute { code, .. } => {
            collect_routine_call_ranges_in_expression(source, line_index, code, routine_name, out)
        }
        Statement::RaiseError { message, .. } => {
            if let Some(message) = message {
                collect_routine_call_ranges_in_expression(
                    source,
                    line_index,
                    message,
                    routine_name,
                    out,
                );
            }
        }
        Statement::AddHandler { event, handler, .. }
        | Statement::RemoveHandler { event, handler, .. } => {
            collect_routine_call_ranges_in_expression(source, line_index, event, routine_name, out);
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                handler,
                routine_name,
                out,
            );
        }
        Statement::Await { expression, .. } => collect_routine_call_ranges_in_expression(
            source,
            line_index,
            expression,
            routine_name,
            out,
        ),
        Statement::FunctionDecl { .. }
        | Statement::ProcedureDecl { .. }
        | Statement::VarDeclaration { .. }
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::Goto { .. }
        | Statement::Label { .. } => {}
    }
}

fn collect_routine_call_ranges_in_expression(
    source: &str,
    line_index: &LineIndex,
    expr: &Expression,
    routine_name: &str,
    out: &mut Vec<Range>,
) {
    match expr {
        Expression::Call { function, args, .. } => {
            if let Expression::Identifier { name, span } = function.as_ref() {
                if name == routine_name {
                    out.push(range_from_span(source, line_index, *span));
                }
            }
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                function,
                routine_name,
                out,
            );
            for arg in args {
                collect_routine_call_ranges_in_expression(
                    source,
                    line_index,
                    arg,
                    routine_name,
                    out,
                );
            }
        }
        Expression::Binary { left, right, .. } => {
            collect_routine_call_ranges_in_expression(source, line_index, left, routine_name, out);
            collect_routine_call_ranges_in_expression(source, line_index, right, routine_name, out);
        }
        Expression::Unary { operand, .. } => collect_routine_call_ranges_in_expression(
            source,
            line_index,
            operand,
            routine_name,
            out,
        ),
        Expression::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                condition,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                then_expr,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                else_expr,
                routine_name,
                out,
            );
        }
        Expression::New { args, .. } => {
            for arg in args {
                collect_routine_call_ranges_in_expression(
                    source,
                    line_index,
                    arg,
                    routine_name,
                    out,
                );
            }
        }
        Expression::PropertyAccess { object, .. } => {
            collect_routine_call_ranges_in_expression(source, line_index, object, routine_name, out)
        }
        Expression::IndexAccess { object, index, .. } => {
            collect_routine_call_ranges_in_expression(
                source,
                line_index,
                object,
                routine_name,
                out,
            );
            collect_routine_call_ranges_in_expression(source, line_index, index, routine_name, out);
        }
        Expression::Await { expression, .. } => collect_routine_call_ranges_in_expression(
            source,
            line_index,
            expression,
            routine_name,
            out,
        ),
        Expression::Identifier { .. }
        | Expression::String { .. }
        | Expression::Number { .. }
        | Expression::Boolean { .. }
        | Expression::Date { .. } => {}
    }
}

fn find_called_identifier_at_position(
    source: &str,
    line_index: &LineIndex,
    parse_result: &ParseResult,
    position: Position,
) -> Option<String> {
    for stmt in &parse_result.program.statements {
        match stmt {
            Statement::FunctionDecl { body, .. } | Statement::ProcedureDecl { body, .. } => {
                if let Some(name) = find_called_identifier_at_position_in_statements(
                    source, line_index, body, position,
                ) {
                    return Some(name);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_called_identifier_at_position_in_statements(
    source: &str,
    line_index: &LineIndex,
    stmts: &[Statement],
    position: Position,
) -> Option<String> {
    for stmt in stmts {
        if let Some(name) =
            find_called_identifier_at_position_in_statement(source, line_index, stmt, position)
        {
            return Some(name);
        }
    }
    None
}

fn find_called_identifier_at_position_in_statement(
    source: &str,
    line_index: &LineIndex,
    stmt: &Statement,
    position: Position,
) -> Option<String> {
    match stmt {
        Statement::Assignment { target, value, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, target, position)
                .or_else(|| {
                    find_called_identifier_at_position_in_expression(
                        source, line_index, value, position,
                    )
                })
        }
        Statement::If {
            condition,
            then_body,
            else_body,
            ..
        } => find_called_identifier_at_position_in_expression(
            source, line_index, condition, position,
        )
        .or_else(|| {
            find_called_identifier_at_position_in_statements(
                source, line_index, then_body, position,
            )
            .or_else(|| {
                else_body.as_ref().and_then(|b| {
                    find_called_identifier_at_position_in_statements(
                        source, line_index, b, position,
                    )
                })
            })
        }),
        Statement::For {
            start, end, body, ..
        } => find_called_identifier_at_position_in_expression(source, line_index, start, position)
            .or_else(|| {
                find_called_identifier_at_position_in_expression(source, line_index, end, position)
            })
            .or_else(|| {
                find_called_identifier_at_position_in_statements(source, line_index, body, position)
            }),
        Statement::ForEach {
            collection, body, ..
        } => find_called_identifier_at_position_in_expression(
            source, line_index, collection, position,
        )
        .or_else(|| {
            find_called_identifier_at_position_in_statements(source, line_index, body, position)
        }),
        Statement::While {
            condition, body, ..
        } => find_called_identifier_at_position_in_expression(
            source, line_index, condition, position,
        )
        .or_else(|| {
            find_called_identifier_at_position_in_statements(source, line_index, body, position)
        }),
        Statement::Return { value, .. } => value.as_ref().and_then(|e| {
            find_called_identifier_at_position_in_expression(source, line_index, e, position)
        }),
        Statement::Try {
            try_body,
            except_body,
            ..
        } => {
            find_called_identifier_at_position_in_statements(source, line_index, try_body, position)
                .or_else(|| {
                    find_called_identifier_at_position_in_statements(
                        source,
                        line_index,
                        except_body,
                        position,
                    )
                })
        }
        Statement::Call { expression, .. } => find_called_identifier_at_position_in_expression(
            source, line_index, expression, position,
        ),
        Statement::Execute { code, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, code, position)
        }
        Statement::RaiseError { message, .. } => message.as_ref().and_then(|e| {
            find_called_identifier_at_position_in_expression(source, line_index, e, position)
        }),
        Statement::AddHandler { event, handler, .. }
        | Statement::RemoveHandler { event, handler, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, event, position)
                .or_else(|| {
                    find_called_identifier_at_position_in_expression(
                        source, line_index, handler, position,
                    )
                })
        }
        Statement::Await { expression, .. } => find_called_identifier_at_position_in_expression(
            source, line_index, expression, position,
        ),
        Statement::FunctionDecl { .. }
        | Statement::ProcedureDecl { .. }
        | Statement::VarDeclaration { .. }
        | Statement::Break { .. }
        | Statement::Continue { .. }
        | Statement::Goto { .. }
        | Statement::Label { .. } => None,
    }
}

fn find_called_identifier_at_position_in_expression(
    source: &str,
    line_index: &LineIndex,
    expr: &Expression,
    position: Position,
) -> Option<String> {
    match expr {
        Expression::Call { function, args, .. } => {
            if let Expression::Identifier { name, span } = function.as_ref() {
                if span_contains_position(source, line_index, *span, position) {
                    return Some(name.clone());
                }
            }
            find_called_identifier_at_position_in_expression(source, line_index, function, position)
                .or_else(|| {
                    args.iter().find_map(|a| {
                        find_called_identifier_at_position_in_expression(
                            source, line_index, a, position,
                        )
                    })
                })
        }
        Expression::Binary { left, right, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, left, position)
                .or_else(|| {
                    find_called_identifier_at_position_in_expression(
                        source, line_index, right, position,
                    )
                })
        }
        Expression::Unary { operand, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, operand, position)
        }
        Expression::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => find_called_identifier_at_position_in_expression(
            source, line_index, condition, position,
        )
        .or_else(|| {
            find_called_identifier_at_position_in_expression(
                source, line_index, then_expr, position,
            )
        })
        .or_else(|| {
            find_called_identifier_at_position_in_expression(
                source, line_index, else_expr, position,
            )
        }),
        Expression::New { args, .. } => args.iter().find_map(|a| {
            find_called_identifier_at_position_in_expression(source, line_index, a, position)
        }),
        Expression::PropertyAccess { object, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, object, position)
        }
        Expression::IndexAccess { object, index, .. } => {
            find_called_identifier_at_position_in_expression(source, line_index, object, position)
                .or_else(|| {
                    find_called_identifier_at_position_in_expression(
                        source, line_index, index, position,
                    )
                })
        }
        Expression::Await { expression, .. } => find_called_identifier_at_position_in_expression(
            source, line_index, expression, position,
        ),
        Expression::Identifier { .. }
        | Expression::String { .. }
        | Expression::Number { .. }
        | Expression::Boolean { .. }
        | Expression::Date { .. } => None,
    }
}

fn selection_range_for_name_in_span_line(
    source: &str,
    line_index: &LineIndex,
    span: &bsl_shared::ir::Span,
    name: &str,
) -> Option<Range> {
    let (row, start_byte_column) = line_index.byte_offset_to_point(source, span.start as usize);
    let line = line_index.line_text(source, row);

    let candidate = &line[start_byte_column..];
    let rel = candidate.find(name)?;
    let name_start_byte = start_byte_column + rel;
    let name_end_byte = name_start_byte + name.len();

    let start_character = line_index.byte_column_to_utf16(source, row, name_start_byte);
    let end_character = line_index.byte_column_to_utf16(source, row, name_end_byte);
    let line = row as u32;

    Some(Range {
        start: Position {
            line,
            character: start_character,
        },
        end: Position {
            line,
            character: end_character,
        },
    })
}

fn range_from_span(source: &str, line_index: &LineIndex, span: bsl_shared::ir::Span) -> Range {
    let (start_line, start_character) =
        line_index.byte_offset_to_utf16_position(source, span.start as usize);
    let (end_line, end_character) =
        line_index.byte_offset_to_utf16_position(source, span.end as usize);
    Range {
        start: Position {
            line: start_line,
            character: start_character,
        },
        end: Position {
            line: end_line,
            character: end_character,
        },
    }
}

fn span_contains_position(
    source: &str,
    line_index: &LineIndex,
    span: bsl_shared::ir::Span,
    position: Position,
) -> bool {
    let offset =
        line_index.utf16_position_to_byte_offset(source, position.line, position.character) as u32;
    if span.contains(offset) {
        return true;
    }
    offset
        .checked_sub(1)
        .is_some_and(|probe| span.contains(probe))
}

fn range_contains_position(range: Range, position: Position) -> bool {
    range_bounds_contains_position(range.start, range.end, position)
}

fn range_bounds_contains_position(start: Position, end: Position, position: Position) -> bool {
    if start == end {
        return position == start;
    }
    cmp_pos(position, start) != std::cmp::Ordering::Less
        && cmp_pos(position, end) == std::cmp::Ordering::Less
}

fn cmp_pos(a: Position, b: Position) -> std::cmp::Ordering {
    match a.line.cmp(&b.line) {
        std::cmp::Ordering::Equal => a.character.cmp(&b.character),
        other => other,
    }
}
