//! Inlay hints handler for LSP (v2)
//!
//! Implements textDocument/inlayHint for BSL documents.

use std::sync::Arc;

use tower_lsp::lsp_types::*;

use bsl_line_index::LineIndex;
use bsl_shared::domain::types::Certainty;
use bsl_shared::domain::types::{ResolutionResult, TypeResolution};
use bsl_shared::ir::{SemanticNodeKind, Span};

use crate::config::TypeHintsSettings;

const MAX_HINTS: usize = 200;

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

fn find_identifier_end_byte(source: &str, span: Span, identifier: &str) -> Option<u32> {
    let start = span.start as usize;
    let end = span.end as usize;
    if start >= source.len() || end > source.len() || start >= end {
        return None;
    }

    let slice = &source[start..end];
    let rel = slice.find(identifier)?;
    let abs = start + rel + identifier.len();
    u32::try_from(abs).ok()
}

pub fn handle_inlay_hints_v2(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: Arc<str>,
    ir_program: Arc<bsl_shared::ir::SemanticProgram>,
    range: Range,
    settings: &TypeHintsSettings,
) -> Vec<InlayHint> {
    if !settings.enabled {
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

    let mut hints = Vec::new();
    for node in &ir_program.nodes {
        if !settings.show_variable_types {
            break;
        }

        let SemanticNodeKind::Assignment { variable, .. } = &node.kind else {
            continue;
        };
        if !spans_intersect(node.span, query_span) {
            continue;
        }

        let Some(var_end) = find_identifier_end_byte(file_content.as_ref(), node.span, variable)
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
        let Some(type_label) = format_type_label(&resolution, settings) else {
            continue;
        };

        let (line, character) =
            index.byte_offset_to_utf16_position(file_content.as_ref(), var_end as usize);

        hints.push(InlayHint {
            position: Position::new(line, character),
            label: InlayHintLabel::String(format!(": {}", type_label)),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: None,
            data: None,
        });
    }

    hints.sort_by(|a, b| {
        (
            a.position.line,
            a.position.character,
            inlay_hint_label_key(&a.label),
        )
            .cmp(&(
                b.position.line,
                b.position.character,
                inlay_hint_label_key(&b.label),
            ))
    });

    if hints.len() > MAX_HINTS {
        hints.truncate(MAX_HINTS);
    }

    hints
}

fn inlay_hint_label_key(label: &InlayHintLabel) -> String {
    match label {
        InlayHintLabel::String(text) => text.clone(),
        InlayHintLabel::LabelParts(parts) => parts
            .iter()
            .map(|part| part.value.clone())
            .collect::<Vec<_>>()
            .join(""),
    }
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
        let file_content = analysis.file_text(file_id).ok().flatten().expect("file_text");
        let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
        (analysis, file_id, file_content, ir_program)
    }

    #[test]
    fn inlay_hints_are_deterministic_and_non_empty_when_enabled() {
        let content = "Процедура Тест()\nПерем X;\nX = 1;\nКонецПроцедуры\n";
        let (analysis, file_id, file_content, ir_program) = build_v2_ir(content);

        let settings = TypeHintsSettings {
            enabled: true,
            show_variable_types: true,
            show_return_types: false,
            show_union_details: true,
            min_certainty: 0.7,
        };

        let range = Range::new(Position::new(0, 0), Position::new(10, 0));
        let a = handle_inlay_hints_v2(
            &analysis,
            file_id,
            file_content.clone(),
            ir_program.clone(),
            range,
            &settings,
        );
        let b = handle_inlay_hints_v2(&analysis, file_id, file_content, ir_program, range, &settings);

        assert!(!a.is_empty(), "expected at least one inlay hint");
        let norm = |hints: Vec<InlayHint>| {
            hints
                .into_iter()
                .map(|hint| {
                    let label = match hint.label {
                        InlayHintLabel::String(text) => text,
                        InlayHintLabel::LabelParts(parts) => parts
                            .into_iter()
                            .map(|p| p.value)
                            .collect::<Vec<_>>()
                            .join(""),
                    };
                    (hint.position.line, hint.position.character, label)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(norm(a.clone()), norm(b), "inlay hints must be deterministic");

        let has_number = a.iter().any(|hint| match &hint.label {
            InlayHintLabel::String(text) => text.contains(": Число"),
            _ => false,
        });
        assert!(has_number, "expected a ': Число' hint");
    }
}
