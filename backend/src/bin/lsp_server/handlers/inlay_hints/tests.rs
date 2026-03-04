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
    let file_content = analysis
        .file_text(file_id)
        .ok()
        .flatten()
        .expect("file_text");
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
    let b = handle_inlay_hints_v2(
        &analysis,
        file_id,
        file_content,
        ir_program,
        range,
        &settings,
    );

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
    assert_eq!(
        norm(a.clone()),
        norm(b),
        "inlay hints must be deterministic"
    );

    let has_number = a.iter().any(|hint| match &hint.label {
        InlayHintLabel::String(text) => text.contains(": Число"),
        _ => false,
    });
    assert!(has_number, "expected a ': Число' hint");
}
