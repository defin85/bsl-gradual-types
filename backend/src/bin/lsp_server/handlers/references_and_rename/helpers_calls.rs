use super::*;

pub(super) fn collect_routine_call_ranges_in_program(
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

pub(super) fn collect_routine_call_ranges_in_statements(
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

pub(super) fn collect_routine_call_ranges_in_statement(
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

pub(super) fn collect_routine_call_ranges_in_expression(
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

pub(super) fn find_called_identifier_at_position(
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

pub(super) fn find_called_identifier_at_position_in_statements(
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

pub(super) fn find_called_identifier_at_position_in_statement(
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

pub(super) fn find_called_identifier_at_position_in_expression(
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
