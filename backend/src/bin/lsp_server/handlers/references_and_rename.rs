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

#[path = "references_and_rename/helpers_calls.rs"]
mod helpers_calls;
#[path = "references_and_rename/helpers_identifiers.rs"]
mod helpers_identifiers;

use self::helpers_calls::*;
use self::helpers_identifiers::*;

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
