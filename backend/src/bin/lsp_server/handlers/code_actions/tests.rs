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
        CodeActionOrCommand::CodeAction(action) => action.kind == Some(CodeActionKind::QUICKFIX),
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
