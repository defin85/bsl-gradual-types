//! LSP completion incremental edits tests (VS Code-like scenarios) (M8).

mod support;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::system::DepsBundleV2;
use bsl_shared::formatting::DetailLevel;
use tower_lsp::lsp_types::{CompletionResponse, Position, Url};

fn utf16_len(text: &str) -> u32 {
    text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32
}

fn setup_host(deps_bundle: &DepsBundleV2) -> AnalysisHostV2 {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("m8-lsp-incremental"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host
}

fn apply_file(
    host: &mut AnalysisHostV2,
    file_id: V2FileId,
    version: i32,
    file_path: &str,
    content: &str,
) -> (
    Arc<str>,
    Arc<str>,
    Arc<bsl_shared::ir::SemanticProgram>,
    Arc<bsl_syntax::ast::ParseResult>,
) {
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    let file_content = analysis
        .file_text(file_id)
        .ok()
        .flatten()
        .expect("file_text");
    let file_path = analysis
        .file_path(file_id)
        .ok()
        .flatten()
        .expect("file_path");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");
    let parse_result = analysis
        .parse_result(file_id)
        .ok()
        .flatten()
        .expect("parse_result");

    (file_content, file_path, ir_program, parse_result)
}

fn completion_labels(response: CompletionResponse) -> Vec<String> {
    let items = match response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(items) => items,
    };
    items.into_iter().map(|item| item.label).collect()
}

fn format_expr_with_cursor_guard(typed: &str) -> String {
    if typed.ends_with('.') {
        format!("{typed} X")
    } else {
        format!("{typed}X")
    }
}

#[allow(clippy::too_many_arguments)]
async fn apply_and_complete_at_member_dot(
    host: &mut AnalysisHostV2,
    deps_bundle: &Arc<DepsBundleV2>,
    index_snapshot: &Arc<bsl_backend::system::IndexSnapshot>,
    uri: &Url,
    file_path: &str,
    file_id: V2FileId,
    version: i32,
    typed: &str,
) -> completion_handler::CompletionResponseWithStats {
    let expr = format_expr_with_cursor_guard(typed);
    let content = format!(
        "Процедура M8()\n    ТаблЗнач = Новый ТаблицаЗначений;\n    __tmp = {expr};\nКонецПроцедуры\n"
    );

    let (file_content, file_path, ir_program, parse_result) =
        apply_file(host, file_id, version, file_path, &content);

    let prefix_len = utf16_len("    __tmp = ");
    let position = Position {
        line: 2,
        character: prefix_len + utf16_len(typed),
    };

    completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        Some(parse_result),
        None,
        deps_bundle.semantic_deps.clone(),
        position,
        uri,
        index_snapshot.as_ref(),
        false,
        false,
    )
    .await
    .expect("completion response")
}

#[tokio::test]
async fn m8_lsp_incremental_typing_triggers_completion_on_dot() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();

    let uri = Url::parse("file:///m8_lsp_incremental.bsl").expect("uri");
    let file_path = "m8_lsp_incremental.bsl";
    let file_id = V2FileId(1);

    let mut host = setup_host(deps_bundle.as_ref());

    let target = "ТаблЗнач.Колонки.";
    let mut typed = String::new();
    let mut version: i32 = 0;

    for ch in target.chars() {
        typed.push(ch);
        let expr = format_expr_with_cursor_guard(&typed);
        let content = format!(
            "Процедура M8()\n    ТаблЗнач = Новый ТаблицаЗначений;\n    __tmp = {expr};\nКонецПроцедуры\n"
        );

        version = version.saturating_add(1);
        let (file_content, file_path, ir_program, parse_result) =
            apply_file(&mut host, file_id, version, file_path, &content);

        if !typed.ends_with('.') {
            continue;
        }

        let expected = match typed.as_str() {
            "ТаблЗнач." => Some("Колонки"),
            "ТаблЗнач.Колонки." => Some("Добавить"),
            _ => None,
        };
        let Some(expected) = expected else {
            continue;
        };

        let prefix_len = utf16_len("    __tmp = ");
        let position = Position {
            line: 2,
            character: prefix_len + utf16_len(&typed),
        };

        let response = completion_handler::handle_completion_v2(
            file_content,
            file_path,
            ir_program,
            Some(parse_result),
            None,
            deps_bundle.semantic_deps.clone(),
            position,
            &uri,
            index_snapshot.as_ref(),
            false,
            false,
        )
        .await
        .expect("completion response");

        assert!(!response.had_error);
        let labels = completion_labels(response.response);
        assert!(
            labels.iter().any(|label| label == expected),
            "typed='{}' should include '{}', labels={:?}",
            typed,
            expected,
            labels
        );
    }
}

#[tokio::test]
async fn m8_lsp_edits_around_dot_do_not_break_completion() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();

    let uri = Url::parse("file:///m8_lsp_incremental_dot_edits.bsl").expect("uri");
    let file_path = "m8_lsp_incremental_dot_edits.bsl";
    let file_id = V2FileId(1);

    let mut host = setup_host(deps_bundle.as_ref());

    let mut version: i32 = 0;

    let full = "ТаблЗнач.Колонки.";
    version = version.saturating_add(1);
    let response = apply_and_complete_at_member_dot(
        &mut host,
        &deps_bundle,
        &index_snapshot,
        &uri,
        file_path,
        file_id,
        version,
        full,
    )
    .await;
    let labels = completion_labels(response.response);
    assert!(labels.iter().any(|label| label == "Добавить"));

    let broken = "ТаблЗначКолонки.";
    version = version.saturating_add(1);
    let response = apply_and_complete_at_member_dot(
        &mut host,
        &deps_bundle,
        &index_snapshot,
        &uri,
        file_path,
        file_id,
        version,
        broken,
    )
    .await;
    assert!(!response.had_error);
    let labels = completion_labels(response.response);
    assert!(
        !labels.is_empty(),
        "completion should not be empty on broken code"
    );

    version = version.saturating_add(1);
    let response = apply_and_complete_at_member_dot(
        &mut host,
        &deps_bundle,
        &index_snapshot,
        &uri,
        file_path,
        file_id,
        version,
        full,
    )
    .await;
    let labels = completion_labels(response.response);
    assert!(labels.iter().any(|label| label == "Добавить"));
}

#[tokio::test]
async fn m8_lsp_completion_inside_string_and_comment_does_not_suggest_member_access() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();

    let uri = Url::parse("file:///m8_lsp_incremental_strings.bsl").expect("uri");
    let file_path = "m8_lsp_incremental_strings.bsl";
    let file_id = V2FileId(1);

    let mut host = setup_host(deps_bundle.as_ref());

    let content = concat!(
        "Процедура M8()\n",
        "    ТаблЗнач = Новый ТаблицаЗначений;\n",
        "    __tmp = \"ТаблЗнач. Колонки\";\n",
        "    // ТаблЗнач. Колонки\n",
        "КонецПроцедуры\n",
    );

    let (file_content, file_path, ir_program, parse_result) =
        apply_file(&mut host, file_id, 1, file_path, content);

    let position_in_string = Position {
        line: 2,
        character: utf16_len("    __tmp = \"ТаблЗнач."),
    };
    let response = completion_handler::handle_completion_v2(
        file_content.clone(),
        file_path.clone(),
        ir_program.clone(),
        Some(parse_result.clone()),
        None,
        deps_bundle.semantic_deps.clone(),
        position_in_string,
        &uri,
        index_snapshot.as_ref(),
        false,
        false,
    )
    .await
    .expect("completion response");
    let labels = completion_labels(response.response);
    assert!(!labels.iter().any(|label| label == "Колонки"));
    assert!(!labels.iter().any(|label| label == "Добавить"));

    let position_in_comment = Position {
        line: 3,
        character: utf16_len("    // ТаблЗнач."),
    };
    let response = completion_handler::handle_completion_v2(
        file_content,
        file_path,
        ir_program,
        Some(parse_result),
        None,
        deps_bundle.semantic_deps.clone(),
        position_in_comment,
        &uri,
        index_snapshot.as_ref(),
        false,
        false,
    )
    .await
    .expect("completion response");
    let labels = completion_labels(response.response);
    assert!(!labels.iter().any(|label| label == "Колонки"));
    assert!(!labels.iter().any(|label| label == "Добавить"));
}
