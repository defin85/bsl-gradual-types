//! Current Context handler for LSP
//!
//! MILESTONE 2.20.3: Handles bsl.getCurrentContext command.

use bsl_line_index::LineIndex;
use bsl_syntax::ast::{ParseResult, Statement};

/// Response for getCurrentContext command
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentContextResponse {
    pub function_name: Option<String>,
    pub function_kind: String, // "function", "procedure", "none"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,
}

impl CurrentContextResponse {
    pub fn empty() -> Self {
        Self {
            function_name: None,
            function_kind: "none".to_string(),
            params: None,
            return_type: None,
        }
    }
}

pub fn find_containing_function_in_parse_result(
    parse_result: &ParseResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<(String, String, Vec<String>, Option<String>)> {
    find_in_statements(
        &parse_result.program.statements,
        source,
        line_index,
        line,
        character,
        None,
    )
}

type RoutineContext = (String, String, Vec<String>, Option<String>);

fn find_in_statements(
    statements: &[Statement],
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
    current_routine: Option<&RoutineContext>,
) -> Option<RoutineContext> {
    for statement in statements {
        if let Some(result) = find_in_statement(
            statement,
            source,
            line_index,
            line,
            character,
            current_routine,
        ) {
            return Some(result);
        }
    }
    None
}

fn find_in_statement(
    statement: &Statement,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
    current_routine: Option<&RoutineContext>,
) -> Option<RoutineContext> {
    if !statement_contains_position(statement, source, line_index, line, character) {
        return None;
    }

    match statement {
        Statement::FunctionDecl {
            name, params, body, ..
        } => {
            let current = (name.clone(), "function".to_string(), params.clone(), None);
            find_in_statements(body, source, line_index, line, character, Some(&current))
                .or(Some(current))
        }
        Statement::ProcedureDecl {
            name, params, body, ..
        } => {
            let current = (name.clone(), "procedure".to_string(), params.clone(), None);
            find_in_statements(body, source, line_index, line, character, Some(&current))
                .or(Some(current))
        }
        Statement::If {
            then_body,
            else_body,
            ..
        } => find_in_statements(
            then_body,
            source,
            line_index,
            line,
            character,
            current_routine,
        )
        .or_else(|| {
            else_body.as_ref().and_then(|else_body| {
                find_in_statements(
                    else_body,
                    source,
                    line_index,
                    line,
                    character,
                    current_routine,
                )
            })
        })
        .or_else(|| current_routine.cloned()),
        Statement::For { body, .. }
        | Statement::ForEach { body, .. }
        | Statement::While { body, .. } => {
            find_in_statements(body, source, line_index, line, character, current_routine)
                .or_else(|| current_routine.cloned())
        }
        Statement::Try {
            try_body,
            except_body,
            ..
        } => find_in_statements(
            try_body,
            source,
            line_index,
            line,
            character,
            current_routine,
        )
        .or_else(|| {
            find_in_statements(
                except_body,
                source,
                line_index,
                line,
                character,
                current_routine,
            )
        })
        .or_else(|| current_routine.cloned()),
        _ => current_routine.cloned(),
    }
}

fn statement_contains_position(
    statement: &Statement,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> bool {
    let span = statement_span(statement);
    let (start_line, start_character) =
        line_index.byte_offset_to_utf16_position(source, span.start as usize);
    let (end_line, end_character) =
        line_index.byte_offset_to_utf16_position(source, span.end as usize);

    line >= start_line
        && line <= end_line
        && (line > start_line || character >= start_character)
        && (line < end_line || character <= end_character)
}

fn statement_span(statement: &Statement) -> bsl_syntax::ast::Span {
    match statement {
        Statement::Assignment { span, .. }
        | Statement::VarDeclaration { span, .. }
        | Statement::FunctionDecl { span, .. }
        | Statement::ProcedureDecl { span, .. }
        | Statement::If { span, .. }
        | Statement::For { span, .. }
        | Statement::ForEach { span, .. }
        | Statement::While { span, .. }
        | Statement::Return { span, .. }
        | Statement::Try { span, .. }
        | Statement::Call { span, .. }
        | Statement::Break { span, .. }
        | Statement::Continue { span, .. }
        | Statement::Goto { span, .. }
        | Statement::Label { span, .. }
        | Statement::Execute { span, .. }
        | Statement::RaiseError { span, .. }
        | Statement::AddHandler { span, .. }
        | Statement::RemoveHandler { span, .. }
        | Statement::Await { span, .. } => *span,
    }
}
