use std::cmp::Ordering;

use bsl_line_index::LineIndex;
use bsl_syntax::ast::{ParseResult, Statement};
use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, Location, Position, Range, SymbolInformation,
    SymbolKind, Url,
};

#[derive(Debug, thiserror::Error)]
pub enum SymbolsError {
    #[error("tree-sitter language error: {0}")]
    Language(String),
    #[error("tree-sitter parse returned None")]
    ParseFailed,
}

pub fn build_document_symbols(
    source: &str,
    parse_result: &ParseResult,
) -> Result<DocumentSymbolResponse, SymbolsError> {
    let regions = collect_regions(source)?;
    let line_index = LineIndex::new(source);
    let routines = collect_routines(parse_result, source, &line_index);

    let mut top_level = Vec::<DocumentSymbol>::new();

    if regions.is_empty() {
        top_level.extend(routines.into_iter().map(|r| r.to_document_symbol()));
        return Ok(DocumentSymbolResponse::Nested(top_level));
    }

    let mut roots = regions;
    let mut top_level_routines = Vec::<RoutineSymbol>::new();
    for routine in routines {
        let inserted = insert_routine_into_regions(&mut roots, &routine);
        if !inserted {
            top_level_routines.push(routine);
        }
    }

    for region in roots {
        top_level.push(region.to_document_symbol());
    }
    for routine in top_level_routines {
        top_level.push(routine.to_document_symbol());
    }

    top_level.sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
    Ok(DocumentSymbolResponse::Nested(top_level))
}

pub fn build_workspace_symbols(
    query: &str,
    uri: &Url,
    source: &str,
    parse_result: &ParseResult,
) -> Vec<SymbolInformation> {
    let query_lower = query.trim().to_lowercase();
    let line_index = LineIndex::new(source);
    collect_routines(parse_result, source, &line_index)
        .into_iter()
        .filter(|routine| {
            query_lower.is_empty() || routine.name.to_lowercase().contains(&query_lower)
        })
        .map(|routine| {
            #[allow(deprecated)]
            let info = SymbolInformation {
                name: routine.name,
                kind: routine.kind,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: routine.range,
                },
                container_name: None,
            };
            info
        })
        .collect()
}

#[derive(Debug, Clone)]
struct RoutineSymbol {
    name: String,
    detail: Option<String>,
    kind: SymbolKind,
    range: Range,
    selection_range: Range,
    start_line: u32,
}

impl RoutineSymbol {
    #[allow(deprecated)]
    fn to_document_symbol(&self) -> DocumentSymbol {
        DocumentSymbol {
            name: self.name.clone(),
            detail: self.detail.clone(),
            kind: self.kind,
            tags: None,
            deprecated: None,
            range: self.range,
            selection_range: self.selection_range,
            children: None,
        }
    }
}

fn collect_routines(
    parse_result: &ParseResult,
    source: &str,
    line_index: &LineIndex,
) -> Vec<RoutineSymbol> {
    let mut out = Vec::new();
    for stmt in &parse_result.program.statements {
        match stmt {
            Statement::FunctionDecl {
                name,
                is_export,
                span,
                ..
            } => {
                out.push(routine_from_span(
                    name,
                    SymbolKind::FUNCTION,
                    *is_export,
                    *span,
                    source,
                    line_index,
                ));
            }
            Statement::ProcedureDecl {
                name,
                is_export,
                span,
                ..
            } => {
                out.push(routine_from_span(
                    name,
                    SymbolKind::METHOD,
                    *is_export,
                    *span,
                    source,
                    line_index,
                ));
            }
            _ => {}
        }
    }
    out.sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
    out
}

fn routine_from_span(
    name: &str,
    kind: SymbolKind,
    is_export: bool,
    span: bsl_shared::ir::Span,
    source: &str,
    line_index: &LineIndex,
) -> RoutineSymbol {
    let (start_line, start_character) =
        line_index.byte_offset_to_utf16_position(source, span.start as usize);
    let (end_line, end_character) =
        line_index.byte_offset_to_utf16_position(source, span.end as usize);
    let start = Position {
        line: start_line,
        character: start_character,
    };
    let end = Position {
        line: end_line,
        character: end_character,
    };
    let range = Range { start, end };
    let selection_range =
        selection_range_for_name(source, line_index, start_line, name).unwrap_or(range);
    RoutineSymbol {
        name: name.to_string(),
        detail: if is_export {
            Some("export".to_string())
        } else {
            None
        },
        kind,
        range,
        selection_range,
        start_line,
    }
}

fn selection_range_for_name(
    source: &str,
    line_index: &LineIndex,
    line: u32,
    name: &str,
) -> Option<Range> {
    let row = line as usize;
    let text = line_index.line_text(source, row);
    let start_byte = text.find(name)?;
    let end_byte = start_byte + name.len();

    let start_character = line_index.byte_column_to_utf16(source, row, start_byte);
    let end_character = line_index.byte_column_to_utf16(source, row, end_byte);

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

#[derive(Debug, Clone)]
struct RegionSymbol {
    name: String,
    range: Range,
    selection_range: Range,
    children_regions: Vec<RegionSymbol>,
    children_routines: Vec<RoutineSymbol>,
}

impl RegionSymbol {
    fn contains_line(&self, line: u32) -> bool {
        line >= self.range.start.line && line <= self.range.end.line
    }

    #[allow(deprecated)]
    fn to_document_symbol(&self) -> DocumentSymbol {
        let mut children: Vec<DocumentSymbol> = Vec::new();
        for region in &self.children_regions {
            children.push(region.to_document_symbol());
        }
        for routine in &self.children_routines {
            children.push(routine.to_document_symbol());
        }
        children.sort_by(|a, b| cmp_pos(a.range.start, b.range.start));

        DocumentSymbol {
            name: self.name.clone(),
            detail: None,
            kind: SymbolKind::NAMESPACE,
            tags: None,
            deprecated: None,
            range: self.range,
            selection_range: self.selection_range,
            children: Some(children),
        }
    }
}

fn insert_routine_into_regions(regions: &mut [RegionSymbol], routine: &RoutineSymbol) -> bool {
    for region in regions {
        if !region.contains_line(routine.start_line) {
            continue;
        }
        if insert_routine_into_regions(&mut region.children_regions, routine) {
            return true;
        }
        region.children_routines.push(routine.clone());
        region
            .children_routines
            .sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
        return true;
    }
    false
}

fn cmp_pos(a: Position, b: Position) -> Ordering {
    match a.line.cmp(&b.line) {
        Ordering::Equal => a.character.cmp(&b.character),
        other => other,
    }
}

#[derive(Debug, Clone)]
enum RegionEvent {
    Start { name: String, start: Range },
    End { end: Range },
}

fn collect_regions(source: &str) -> Result<Vec<RegionSymbol>, SymbolsError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_bsl::LANGUAGE.into())
        .map_err(|e| SymbolsError::Language(format!("{:?}", e)))?;
    let tree = parser
        .parse(source, None)
        .ok_or(SymbolsError::ParseFailed)?;

    let line_index = LineIndex::new(source);
    let mut events = Vec::<(Position, RegionEvent)>::new();

    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "PREPROC_REGION_KEYWORD" => {
                let name =
                    extract_region_name(&node, source).unwrap_or_else(|| "region".to_string());
                let start = range_from_node(&line_index, source, &node);
                events.push((start.start, RegionEvent::Start { name, start }));
            }
            "PREPROC_ENDREGION_KEYWORD" => {
                let end = range_from_node(&line_index, source, &node);
                events.push((end.start, RegionEvent::End { end }));
            }
            _ => {}
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    events.sort_by(|(a, _), (b, _)| cmp_pos(*a, *b));

    let mut roots: Vec<RegionSymbol> = Vec::new();
    let mut open: Vec<RegionSymbol> = Vec::new();

    for (_pos, ev) in events {
        match ev {
            RegionEvent::Start { name, start } => {
                open.push(RegionSymbol {
                    name,
                    range: start,
                    selection_range: start,
                    children_regions: Vec::new(),
                    children_routines: Vec::new(),
                });
            }
            RegionEvent::End { end } => {
                let Some(mut region) = open.pop() else {
                    continue;
                };
                region.range.end = end.end;
                if let Some(parent) = open.last_mut() {
                    parent.children_regions.push(region);
                    parent
                        .children_regions
                        .sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
                } else {
                    roots.push(region);
                    roots.sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
                }
            }
        }
    }

    // Close any unclosed regions at EOF.
    let eof = tree.root_node().end_position();
    let eof_character = line_index.byte_column_to_utf16(source, eof.row, eof.column);
    let eof_range = Range {
        start: Position {
            line: eof.row as u32,
            character: eof_character,
        },
        end: Position {
            line: eof.row as u32,
            character: eof_character,
        },
    };

    while let Some(mut region) = open.pop() {
        region.range.end = eof_range.end;
        if let Some(parent) = open.last_mut() {
            parent.children_regions.push(region);
            parent
                .children_regions
                .sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
        } else {
            roots.push(region);
            roots.sort_by(|a, b| cmp_pos(a.range.start, b.range.start));
        }
    }

    Ok(roots)
}

fn range_from_node(line_index: &LineIndex, source: &str, node: &tree_sitter::Node) -> Range {
    let start = node.start_position();
    let start_utf16 = line_index.byte_column_to_utf16(source, start.row, start.column);
    let end = node.end_position();
    let end_utf16 = line_index.byte_column_to_utf16(source, end.row, end.column);

    Range {
        start: Position {
            line: start.row as u32,
            character: start_utf16,
        },
        end: Position {
            line: end.row as u32,
            character: end_utf16,
        },
    }
}

fn extract_region_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    if let Some(parent) = node.parent() {
        let mut cursor = parent.walk();
        let mut passed_self = false;
        for child in parent.children(&mut cursor) {
            if !passed_self {
                if child.start_byte() == node.start_byte() && child.end_byte() == node.end_byte() {
                    passed_self = true;
                }
                continue;
            }
            if child.kind() == "identifier" {
                let text = child.utf8_text(source.as_bytes()).ok()?;
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
                break;
            }
        }
    }

    let mut sibling = node.next_sibling();
    while let Some(next) = sibling {
        if next.kind() == "identifier" {
            let text = next.utf8_text(source.as_bytes()).ok()?;
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
            break;
        }
        sibling = next.next_sibling();
    }
    None
}
