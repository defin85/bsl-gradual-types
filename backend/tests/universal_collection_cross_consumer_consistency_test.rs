mod support;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::system::DepsBundleV2;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::formatting::{user_facing_resolution_type_name, DetailLevel};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse, Position, Url};

const FILE_ID: V2FileId = V2FileId(1);
const FILE_PATH: &str = "Documents/Док1/Ext/ObjectModule.bsl";

fn byte_offset_of(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("needle '{needle}' not found in fixture"))
}

fn byte_offset_to_utf16_position(text: &str, byte_offset: usize) -> Position {
    let prefix = &text[..byte_offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = prefix.rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let character = prefix[line_start..]
        .chars()
        .map(|ch| ch.len_utf16() as u32)
        .sum::<u32>();
    Position { line, character }
}

fn setup_host(deps_bundle: &DepsBundleV2, code: &str) -> AnalysisHostV2 {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("universal-collection-consistency-tests"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id: FILE_ID,
        text: Arc::from(code.to_string()),
        version: 1,
        path: Arc::from(FILE_PATH.to_string()),
    });
    host
}

fn member_access_owner_hint_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_content: &str,
    position: Position,
) -> Option<TypeResolution> {
    let line_text = file_content.lines().nth(position.line as usize)?;
    let cursor_byte = bsl_analysis_v2::utf16_to_byte_offset(line_text, position.character);
    let line_prefix = line_text.get(..cursor_byte)?;
    let dot_idx = line_prefix.rfind('.')?;
    let receiver = line_prefix.get(..dot_idx)?.trim_end();
    let (probe_byte, _) = receiver
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_whitespace())?;
    let probe_utf16 = bsl_analysis_v2::byte_offset_to_utf16(line_text, probe_byte);
    let probe_offset = analysis
        .utf16_position_to_byte_offset(FILE_ID, position.line, probe_utf16)
        .ok()
        .flatten()?;

    analysis
        .type_at_byte_offset(FILE_ID, probe_offset.min(u32::MAX as usize) as u32)
        .ok()
        .flatten()
}

fn completion_items(response: CompletionResponse) -> Vec<CompletionItem> {
    match response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(items) => items,
    }
}

fn has_unknown_member_diagnostic(message: &str, member_name: &str) -> bool {
    let lower_message = message.to_lowercase();
    lower_message.contains(&member_name.to_lowercase())
        && (lower_message.contains("не существует") || lower_message.contains("не найден"))
}

fn is_simple_member_identifier(label: &str) -> bool {
    !label.is_empty() && label.chars().all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn pick_member_for_diagnostics_probe(items: &[CompletionItem]) -> Option<(String, bool)> {
    for item in items {
        if !is_simple_member_identifier(&item.label) {
            continue;
        }
        match item.kind {
            Some(CompletionItemKind::PROPERTY) | Some(CompletionItemKind::FIELD) => {
                return Some((item.label.clone(), false));
            }
            Some(CompletionItemKind::METHOD) | Some(CompletionItemKind::FUNCTION) => {
                return Some((item.label.clone(), true));
            }
            _ => {}
        }
    }

    items
        .iter()
        .find(|item| is_simple_member_identifier(&item.label))
        .map(|item| (item.label.clone(), false))
}

#[allow(clippy::too_many_arguments)]
async fn assert_cross_consumer_consistency(code_template: &str, completion_prefix: &str) {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri = Url::parse("file:///universal_collection_cross_consumer_consistency.bsl")
        .expect("test uri");

    let host = setup_host(deps_bundle.as_ref(), code_template);
    let analysis = host.analysis();
    let file_content = analysis
        .file_text(FILE_ID)
        .ok()
        .flatten()
        .expect("file_text");
    let resolved_file_path = analysis
        .file_path(FILE_ID)
        .ok()
        .flatten()
        .expect("file_path");
    let ir_program = analysis.ir(FILE_ID).ok().flatten().expect("ir");
    let parse_result = analysis
        .parse_result(FILE_ID)
        .ok()
        .flatten()
        .expect("parse_result");

    let completion_offset =
        byte_offset_of(code_template, completion_prefix) + completion_prefix.len();
    let completion_position = byte_offset_to_utf16_position(code_template, completion_offset);
    let owner_hint =
        member_access_owner_hint_at_position(&analysis, file_content.as_ref(), completion_position);
    assert!(
        owner_hint.is_some(),
        "expected non-empty owner hint for completion at {:?}",
        completion_position
    );
    let completion = completion_handler::handle_completion_v2(
        file_content.clone(),
        resolved_file_path.clone(),
        ir_program,
        Some(parse_result),
        owner_hint,
        deps_bundle.semantic_deps.clone(),
        completion_position,
        &uri,
        index_snapshot.as_ref(),
        false,
        false,
    )
    .await
    .expect("completion response");
    assert!(!completion.had_error, "completion must not fail");

    let items = completion_items(completion.response);
    assert!(
        !items.is_empty(),
        "completion must return at least one candidate at {:?}",
        completion_position
    );
    let labels = items
        .iter()
        .map(|item| item.label.clone())
        .collect::<Vec<_>>();
    let (selected_member, is_method_call) = pick_member_for_diagnostics_probe(&items)
        .unwrap_or_else(|| {
            panic!("no completion member suitable for diagnostics probe: {labels:?}")
        });
    let selected_access = if is_method_call {
        format!("{}()", selected_member)
    } else {
        selected_member.clone()
    };
    let code = code_template.replacen("__MEMBER__", &selected_access, 1);
    assert!(
        code != code_template,
        "fixture template must contain '__MEMBER__' placeholder"
    );

    let host = setup_host(deps_bundle.as_ref(), &code);
    let analysis = host.analysis();
    let receiver_probe_offset = completion_offset;
    let type_resolution = analysis
        .type_at_byte_offset(FILE_ID, receiver_probe_offset as u32)
        .expect("type_at_byte_offset query")
        .expect("type_at_byte_offset result");
    let type_name = user_facing_resolution_type_name(&type_resolution);
    assert!(
        !type_name.is_empty(),
        "type-at-position must return non-empty type name at byte_offset={receiver_probe_offset}"
    );

    let hover_position = byte_offset_to_utf16_position(&code, receiver_probe_offset);
    let hover_text = support::hover_for_code(
        deps_bundle.as_ref(),
        FILE_PATH,
        &code,
        hover_position.line,
        hover_position.character,
    )
    .expect("hover text");
    assert!(
        !hover_text.trim().is_empty(),
        "hover must return non-empty payload at {hover_position:?}"
    );

    let diagnostics = analysis
        .semantic_diagnostics(FILE_ID)
        .ok()
        .flatten()
        .as_deref()
        .cloned()
        .unwrap_or_default();
    let false_unknown_member = diagnostics
        .iter()
        .any(|diag| has_unknown_member_diagnostic(&diag.message, &selected_member));
    assert!(
        !false_unknown_member,
        "diagnostics drift: false unknown-member diagnostic for '{}', diagnostics={diagnostics:?}, completion_labels={labels:?}",
        selected_member
    );
}

#[tokio::test]
async fn map_index_access_cross_consumer_consistency() {
    let code = concat!(
        "Процедура Тест()\n",
        "    map = Новый Соответствие;\n",
        "    map.Вставить(\"k\", Новый ТаблицаЗначений);\n",
        "    probe = map[\"k\"].__MEMBER__;\n",
        "КонецПроцедуры\n",
    );

    assert_cross_consumer_consistency(code, "    probe = map[\"k\"].").await;
}

#[tokio::test]
async fn structure_field_cross_consumer_consistency() {
    let code = concat!(
        "Процедура Тест()\n",
        "    S = Новый Структура;\n",
        "    S.Вставить(\"Идентификатор\", \"A-01\");\n",
        "    probe = S.__MEMBER__;\n",
        "КонецПроцедуры\n",
    );

    assert_cross_consumer_consistency(code, "    probe = S.").await;
}

#[tokio::test]
async fn value_table_row_column_cross_consumer_consistency() {
    let code = concat!(
        "Процедура Тест()\n",
        "    ТЗ = Новый ТаблицаЗначений;\n",
        "    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n",
        "    Стр = ТЗ.Добавить();\n",
        "    probe = Стр.__MEMBER__;\n",
        "КонецПроцедуры\n",
    );

    assert_cross_consumer_consistency(code, "    probe = Стр.").await;
}
