use super::*;

pub(super) fn is_valid_identifier_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_alphabetic()) {
        return false;
    }
    chars.all(|c| c == '_' || c.is_alphanumeric())
}

pub(super) fn resolve_target_at_position(
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

pub(super) fn collect_target_ranges(
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
pub(super) struct LocalDecl {
    name: String,
    decl_range: Range,
}

pub(super) fn is_ambiguous_local_name(local_decls: &[LocalDecl], name: &str) -> bool {
    local_decls.iter().filter(|d| d.name == name).count() > 1
}

pub(super) fn collect_local_var_decls(
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

pub(super) fn find_enclosing_routine(
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

pub(super) fn find_local_var_occurrence_range(
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

pub(super) fn find_identifier_at_position_in_statements(
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

pub(super) fn find_identifier_at_position_in_statement(
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

pub(super) fn find_identifier_at_position_in_expression(
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

pub(super) fn find_identifier_range_at_position_in_statements(
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

pub(super) fn find_identifier_range_at_position_in_statement(
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

pub(super) fn find_identifier_range_at_position_in_expression(
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

pub(super) fn collect_identifier_ranges_in_statements(
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

pub(super) fn collect_identifier_ranges_in_statement(
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

pub(super) fn collect_identifier_ranges_in_expression(
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
