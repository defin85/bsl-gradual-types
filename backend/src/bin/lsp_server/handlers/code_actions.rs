//! Code actions handler for LSP (v2)
//!
//! Implements textDocument/codeAction for BSL documents.

use std::sync::Arc;

use tower_lsp::lsp_types::*;

use bsl_line_index::LineIndex;
use bsl_shared::domain::types::Certainty;
use bsl_shared::domain::types::{ResolutionResult, TypeResolution};
use bsl_shared::ir::{SemanticNodeKind, Span};

use crate::config::{CodeActionsSettings, TypeHintsSettings};

fn certainty_score(certainty: Certainty) -> f64 {
    match certainty {
        Certainty::Known => 1.0,
        Certainty::Inferred => 0.8,
        Certainty::InferredWeak => 0.5,
        Certainty::Unknown => 0.0,
    }
}

fn format_type_label(resolution: &TypeResolution, settings: &TypeHintsSettings) -> Option<String> {
    if resolution.is_unknown() {
        return None;
    }

    if certainty_score(resolution.certainty) < settings.min_certainty {
        return None;
    }

    match &resolution.result {
        ResolutionResult::Union(variants) if !settings.show_union_details => {
            if variants.is_empty() {
                return None;
            }
            let mut names: Vec<String> = variants
                .iter()
                .map(|variant| TypeResolution::known(variant.type_.clone()).type_name())
                .collect();
            names.sort();
            names.first().cloned()
        }
        _ => Some(resolution.type_name()),
    }
}

fn spans_intersect(a: Span, b: Span) -> bool {
    a.start <= b.end && b.start <= a.end
}

fn is_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn find_identifier_end_byte_in_range(
    source: &str,
    start: usize,
    end: usize,
    identifier: &str,
    pick_last: bool,
) -> Option<u32> {
    if start >= source.len() || end > source.len() || start >= end {
        return None;
    }

    let slice = &source[start..end];
    let mut from = 0usize;
    let mut found: Option<usize> = None;
    while let Some(rel) = slice[from..].find(identifier) {
        let match_start = from + rel;
        let match_end = match_start + identifier.len();

        if slice.is_char_boundary(match_start) && slice.is_char_boundary(match_end) {
            let prev_ok = if match_start == 0 {
                true
            } else {
                slice[..match_start]
                    .chars()
                    .last()
                    .is_some_and(|ch| !is_identifier_char(ch))
            };
            let next_ok = if match_end == slice.len() {
                true
            } else {
                slice[match_end..]
                    .chars()
                    .next()
                    .is_some_and(|ch| !is_identifier_char(ch))
            };

            if prev_ok && next_ok {
                found = Some(start + match_end);
                if !pick_last {
                    break;
                }
            }
        }

        from = match_start.saturating_add(1);
        if from >= slice.len() {
            break;
        }
    }

    found.and_then(|abs| u32::try_from(abs).ok())
}

fn find_assignment_lhs_identifier_end_byte(
    source: &str,
    span: Span,
    identifier: &str,
) -> Option<u32> {
    let start = span.start as usize;
    let end = span.end as usize;
    if start >= source.len() || end > source.len() || start >= end {
        return None;
    }

    let slice = &source[start..end];
    let lhs_end = slice.find('=').map(|idx| start + idx).unwrap_or(end);
    find_identifier_end_byte_in_range(source, start, lhs_end, identifier, true)
}

fn find_identifier_end_byte_in_span(source: &str, span: Span, identifier: &str) -> Option<u32> {
    let start = span.start as usize;
    let end = span.end as usize;
    find_identifier_end_byte_in_range(source, start, end, identifier, false)
}

fn line_indent(line: &str) -> &str {
    let trimmed = line.trim_start_matches(|ch: char| ch == ' ' || ch == '\t');
    &line[..line.len().saturating_sub(trimmed.len())]
}

fn pick_tmp_name(source: &str) -> String {
    if !source.contains("tmp") {
        return "tmp".to_string();
    }
    for i in 2..1000 {
        let candidate = format!("tmp{}", i);
        if !source.contains(&candidate) {
            return candidate;
        }
    }
    "tmp9999".to_string()
}

pub fn handle_code_actions_v2(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: Arc<str>,
    ir_program: Arc<bsl_shared::ir::SemanticProgram>,
    uri: &Url,
    range: Range,
    code_actions: &CodeActionsSettings,
    type_hints: &TypeHintsSettings,
) -> Vec<CodeActionOrCommand> {
    if !code_actions.enabled {
        return Vec::new();
    }

    let Some(range_start) = analysis
        .utf16_position_to_byte_offset(file_id, range.start.line, range.start.character)
        .ok()
        .flatten()
    else {
        return Vec::new();
    };
    let Some(range_end) = analysis
        .utf16_position_to_byte_offset(file_id, range.end.line, range.end.character)
        .ok()
        .flatten()
    else {
        return Vec::new();
    };

    let Ok(range_start) = u32::try_from(range_start) else {
        return Vec::new();
    };
    let Ok(range_end) = u32::try_from(range_end) else {
        return Vec::new();
    };

    let query_span = Span::new(range_start, range_end);
    let index = LineIndex::new(file_content.as_ref());

    let mut actions: Vec<CodeAction> = Vec::new();

    // Quick fix: add explicit type annotation to `Перем <var>;` if we can infer a type.
    //
    // MVP rule: triggered when the requested range intersects an assignment to that variable
    // and a corresponding VariableDeclaration exists in the same scope without `type_hint`.
    for node in &ir_program.nodes {
        let SemanticNodeKind::Assignment { variable, .. } = &node.kind else {
            continue;
        };
        if !spans_intersect(node.span, query_span) {
            continue;
        }

        let Some(var_end) =
            find_assignment_lhs_identifier_end_byte(file_content.as_ref(), node.span, variable)
        else {
            continue;
        };
        let query_offset = var_end.saturating_sub(1);
        let Some(resolution) = analysis
            .type_at_byte_offset(file_id, query_offset)
            .ok()
            .flatten()
        else {
            continue;
        };
        let Some(type_label) = format_type_label(&resolution, type_hints) else {
            continue;
        };

        let decl_node = ir_program.nodes.iter().find(|decl| {
            decl.scope_id == node.scope_id
                && matches!(
                    &decl.kind,
                    SemanticNodeKind::VariableDeclaration {
                        name,
                        type_hint: None,
                        ..
                    } if name == variable
                )
        });
        let Some(decl_node) = decl_node else {
            continue;
        };

        let Some(decl_var_end) =
            find_identifier_end_byte_in_span(file_content.as_ref(), decl_node.span, variable)
        else {
            continue;
        };

        let (line, character) =
            index.byte_offset_to_utf16_position(file_content.as_ref(), decl_var_end as usize);
        let insert_pos = Position::new(line, character);

        let edit = WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(
                uri.clone(),
                vec![TextEdit {
                    range: Range::new(insert_pos, insert_pos),
                    new_text: format!(": {}", type_label),
                }],
            )])),
            document_changes: None,
            change_annotations: None,
        };

        actions.push(CodeAction {
            title: format!("Add type annotation for '{}'", variable),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(edit),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        });
    }

    // Refactor: extract selected expression into a temporary variable.
    // MVP constraints:
    // - selection must be non-empty, single-line
    // - applies within the same document only
    if query_span.start < query_span.end {
        let selected = file_content
            .get(query_span.start as usize..query_span.end as usize)
            .unwrap_or("")
            .to_string();
        if !selected.trim().is_empty() && !selected.contains('\n') && !selected.contains('\r') {
            let var_name = pick_tmp_name(file_content.as_ref());
            let line_text = index.line_text(file_content.as_ref(), range.start.line as usize);
            let indent = line_indent(line_text);

            let insert_pos = Position::new(range.start.line, 0);
            let replace_edit = TextEdit {
                range,
                new_text: var_name.clone(),
            };
            let insert_edit = TextEdit {
                range: Range::new(insert_pos, insert_pos),
                new_text: format!("{indent}{var_name} = {};\n", selected.trim()),
            };

            let edit = WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(
                    uri.clone(),
                    vec![insert_edit, replace_edit],
                )])),
                document_changes: None,
                change_annotations: None,
            };

            actions.push(CodeAction {
                title: format!("Extract to variable '{var_name}'"),
                kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                diagnostics: None,
                edit: Some(edit),
                command: None,
                is_preferred: None,
                disabled: None,
                data: None,
            });
        }
    }

    actions.sort_by(|a, b| a.title.cmp(&b.title));
    actions
        .into_iter()
        .map(CodeActionOrCommand::CodeAction)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_v2_ir(
        content: &str,
    ) -> (
        bsl_analysis_v2::AnalysisV2,
        bsl_analysis_v2::FileId,
        Arc<str>,
        Arc<bsl_shared::ir::SemanticProgram>,
        Url,
    ) {
        let mut host = bsl_analysis_v2::AnalysisHostV2::default();
        let file_id = bsl_analysis_v2::FileId(1);
        host.apply_change(bsl_analysis_v2::Change::SetFile {
            file_id,
            text: Arc::from(content.to_string()),
            version: 0,
            path: Arc::from("test.bsl"),
        });
        let analysis = host.analysis();
        let file_content = analysis
            .file_text(file_id)
            .ok()
            .flatten()
            .expect("file_text");
        let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
        let uri = Url::parse("file:///test.bsl").expect("uri");
        (analysis, file_id, file_content, ir_program, uri)
    }

    #[test]
    fn code_actions_include_quickfix_when_declaration_exists() {
        let content = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
        let (analysis, file_id, file_content, ir_program, uri) = build_v2_ir(content);

        let settings = CodeActionsSettings { enabled: true };
        let type_hints = TypeHintsSettings {
            enabled: true,
            show_variable_types: true,
            show_return_types: false,
            show_union_details: true,
            min_certainty: 0.7,
        };

        let range = Range::new(Position::new(2, 0), Position::new(2, 5));
        let actions = handle_code_actions_v2(
            &analysis,
            file_id,
            file_content,
            ir_program,
            &uri,
            range,
            &settings,
            &type_hints,
        );

        let has_quickfix = actions.iter().any(|action| match action {
            CodeActionOrCommand::CodeAction(action) => {
                action.kind == Some(CodeActionKind::QUICKFIX)
            }
            _ => false,
        });
        assert!(has_quickfix, "expected a quick fix code action");
    }

    #[test]
    fn code_actions_include_extract_refactor_when_selection_is_non_empty() {
        let content = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
        let (analysis, file_id, file_content, ir_program, uri) = build_v2_ir(content);

        let settings = CodeActionsSettings { enabled: true };
        let type_hints = TypeHintsSettings::default();

        let range = Range::new(Position::new(2, 4), Position::new(2, 5)); // "1"
        let actions = handle_code_actions_v2(
            &analysis,
            file_id,
            file_content,
            ir_program,
            &uri,
            range,
            &settings,
            &type_hints,
        );

        let has_refactor = actions.iter().any(|action| match action {
            CodeActionOrCommand::CodeAction(action) => {
                action.kind == Some(CodeActionKind::REFACTOR_EXTRACT)
            }
            _ => false,
        });
        assert!(has_refactor, "expected a refactor.extract code action");
    }
}
