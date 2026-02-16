//! Контрактные проверки strict form-data policy для `FormModule.Объект`
//! на user-facing каналах: diagnostics, hover, completion, type-at-position.

mod support;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_backend::system::DepsBundleV2;
use bsl_shared::formatting::DetailLevel;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, Position, Url,
};

const FILE_PATH: &str = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
const FORM_DATA_LABEL: &str = "ДанныеФормыСтруктура";
const LEGACY_ALIAS: &str = "ДанныеФормыОбъект";
const INTERNAL_DESCRIPTOR_MARKER: &str = "contextual:";

fn utf16_len(text: &str) -> u32 {
    text.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32
}

fn setup_host_with_detail_level(
    deps_bundle: &DepsBundleV2,
    diagnostics_detail_level: DetailLevel,
) -> AnalysisHostV2 {
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("form-module-object-dual-layer-contract"),
        diagnostics_detail_level,
    });
    host
}

fn setup_host(deps_bundle: &DepsBundleV2) -> AnalysisHostV2 {
    setup_host_with_detail_level(deps_bundle, DetailLevel::Full)
}

fn apply_file(
    host: &mut AnalysisHostV2,
    file_id: V2FileId,
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
        version: 1,
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

fn completion_items(response: CompletionResponse) -> Vec<CompletionItem> {
    match response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(items) => items,
    }
}

fn assert_no_internal_or_legacy_names(value: &str, channel: &str) {
    assert!(
        !value.contains(LEGACY_ALIAS),
        "legacy alias leaked to {}: {}",
        channel,
        value
    );
    assert!(
        !value.contains(INTERNAL_DESCRIPTOR_MARKER),
        "internal descriptor marker leaked to {}: {}",
        channel,
        value
    );
}

#[test]
fn diagnostics_hover_and_type_at_position_follow_dual_layer_contract() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let code = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "    y = Объект.НесуществующееСвойство;\n",
        "КонецПроцедуры\n",
    );

    let diagnostics = support::semantic_diagnostics_for_code(deps_bundle.as_ref(), FILE_PATH, code);
    let non_existent_diag = diagnostics
        .iter()
        .find(|diag| diag.message.contains("НесуществующееСвойство"))
        .unwrap_or_else(|| {
            panic!("expected NonExistentProperty diagnostic, got: {diagnostics:#?}")
        });
    assert!(
        non_existent_diag.message.contains(FORM_DATA_LABEL),
        "diagnostic should use form-data label, got: {}",
        non_existent_diag.message
    );
    assert!(
        !non_existent_diag.message.contains("ДокументОбъект."),
        "diagnostic should not contain owner-facet label, got: {}",
        non_existent_diag.message
    );
    for diag in &diagnostics {
        assert_no_internal_or_legacy_names(&diag.message, "diagnostics");
    }

    let mut detailed_host =
        setup_host_with_detail_level(deps_bundle.as_ref(), DetailLevel::Detailed);
    detailed_host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code.to_string()),
        version: 1,
        path: Arc::from(FILE_PATH.to_string()),
    });
    let detailed_analysis = detailed_host.analysis();
    let detailed_diagnostics = detailed_analysis
        .semantic_diagnostics(V2FileId(1))
        .ok()
        .flatten()
        .as_deref()
        .cloned()
        .unwrap_or_default();
    let detailed_non_existent_diag = detailed_diagnostics
        .iter()
        .find(|diag| diag.message.contains("НесуществующееСвойство"))
        .unwrap_or_else(|| {
            panic!(
                "expected detailed NonExistentProperty diagnostic, got: {detailed_diagnostics:#?}"
            )
        });
    assert!(
        detailed_non_existent_diag.message.contains(FORM_DATA_LABEL),
        "detailed diagnostic should keep form-data label, got: {}",
        detailed_non_existent_diag.message
    );
    assert!(
        !detailed_non_existent_diag
            .message
            .contains("ДокументОбъект."),
        "detailed diagnostic should not contain owner-facet label, got: {}",
        detailed_non_existent_diag.message
    );
    for diag in &detailed_diagnostics {
        assert_no_internal_or_legacy_names(&diag.message, "diagnostics(detailed)");
    }

    let hover_full = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        FILE_PATH,
        code,
        1,
        utf16_len("    x = Объе"),
        Some(HoverFormatConfig {
            detail_level: DetailLevel::Full,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        }),
    )
    .expect("hover full text");
    assert!(
        hover_full.contains(FORM_DATA_LABEL),
        "full hover should include form-data label, got:\n{}",
        hover_full
    );
    assert!(
        !hover_full.contains("ДокументОбъект."),
        "full hover must not include owner-facet label, got:\n{}",
        hover_full
    );
    assert_no_internal_or_legacy_names(&hover_full, "hover(full)");

    let hover_detailed = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        FILE_PATH,
        code,
        1,
        utf16_len("    x = Объе"),
        Some(HoverFormatConfig {
            detail_level: DetailLevel::Detailed,
            output_format: HoverOutputFormat::Markdown,
            ..Default::default()
        }),
    )
    .expect("hover detailed text");
    assert!(
        hover_detailed.contains(FORM_DATA_LABEL),
        "detailed hover should keep form-data label, got:\n{}",
        hover_detailed
    );
    assert!(
        !hover_detailed.contains("ДокументОбъект."),
        "detailed hover should not include owner-facet label, got:\n{}",
        hover_detailed
    );
    assert_no_internal_or_legacy_names(&hover_detailed, "hover(detailed)");

    let mut host = setup_host(deps_bundle.as_ref());
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code.to_string()),
        version: 1,
        path: Arc::from(FILE_PATH.to_string()),
    });
    let analysis = host.analysis();
    let object_offset = code.find("Объект").expect("Объект offset") as u32;
    let object_type = analysis
        .type_at_byte_offset(V2FileId(1), object_offset)
        .expect("type_at_byte_offset query")
        .map(|ty| bsl_shared::formatting::user_facing_resolution_type_name(&ty))
        .expect("type at Объект");
    assert_eq!(
        object_type, FORM_DATA_LABEL,
        "type-at-position should use form-data label"
    );
    assert_no_internal_or_legacy_names(&object_type, "type-at-position");
}

#[tokio::test]
async fn completion_and_resolve_follow_dual_layer_contract() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri =
        Url::parse("file:///form_module_object_dual_layer_contract_completion.bsl").expect("uri");
    let content = concat!("Процедура Тест()\n", "    Объект.\n", "КонецПроцедуры\n",);

    let mut host = setup_host(deps_bundle.as_ref());
    let (file_content, resolved_file_path, ir_program, parse_result) =
        apply_file(&mut host, V2FileId(1), FILE_PATH, content);

    let response = completion_handler::handle_completion_v2(
        file_content,
        resolved_file_path,
        ir_program,
        Some(parse_result),
        None,
        deps_bundle.semantic_deps.clone(),
        Position {
            line: 1,
            character: utf16_len("    Объект."),
        },
        &uri,
        index_snapshot.as_ref(),
        false,
        false,
    )
    .await
    .expect("completion response");

    assert!(!response.had_error, "completion returned error");
    let items = completion_items(response.response);
    assert!(!items.is_empty(), "expected completion items");

    for item in items.iter().take(40) {
        assert_no_internal_or_legacy_names(&item.label, "completion label");

        if let Some(data) = &item.data {
            assert_no_internal_or_legacy_names(&data.to_string(), "completion item data");
        }
    }

    for candidate in items.into_iter().take(20) {
        let resolved = completion_handler::handle_completion_resolve(
            candidate,
            Some(deps_bundle.semantic_deps.clone()),
            false,
        )
        .await;

        if let Some(detail) = resolved.detail.as_deref() {
            assert_no_internal_or_legacy_names(detail, "completion detail");
        }

        if let Some(Documentation::String(doc)) = resolved.documentation {
            assert_no_internal_or_legacy_names(&doc, "completion docs");
        }
    }
}

#[tokio::test]
async fn completion_selected_member_does_not_trigger_false_nonexistent_property() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri =
        Url::parse("file:///form_module_object_completion_diagnostics_guard.bsl").expect("uri");
    let completion_content = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "    Объект.\n",
        "КонецПроцедуры\n",
    );

    let mut host = setup_host(deps_bundle.as_ref());
    let (file_content, resolved_file_path, ir_program, parse_result) =
        apply_file(&mut host, V2FileId(1), FILE_PATH, completion_content);
    let object_offset = completion_content
        .find("x = Объект")
        .expect("Объект offset in completion guard")
        + "x = ".len();
    let member_access_owner_type_hint = host
        .analysis()
        .type_at_byte_offset(V2FileId(1), object_offset as u32)
        .expect("type_at_byte_offset query");
    assert!(
        member_access_owner_type_hint.is_some(),
        "expected type hint for FormModule.Объект member completion guard"
    );

    let response = completion_handler::handle_completion_v2(
        file_content,
        resolved_file_path,
        ir_program,
        Some(parse_result),
        member_access_owner_type_hint,
        deps_bundle.semantic_deps.clone(),
        Position {
            line: 2,
            character: utf16_len("    Объект."),
        },
        &uri,
        index_snapshot.as_ref(),
        false,
        false,
    )
    .await
    .expect("completion response");
    assert!(!response.had_error, "completion returned error");

    let completion_items = completion_items(response.response);
    let labels = completion_items
        .iter()
        .map(|item| item.label.clone())
        .collect::<Vec<_>>();
    let selected_member = completion_items
        .into_iter()
        .find(|item| {
            matches!(
                item.kind,
                Some(CompletionItemKind::PROPERTY) | Some(CompletionItemKind::FIELD)
            )
        })
        .map(|item| item.label)
        .expect("completion must include at least one property/field for FormModule.Объект");

    let selected_code = format!(
        "Процедура Тест()\n    Проверка = Объект.{};\nКонецПроцедуры\n",
        selected_member
    );
    let diagnostics =
        support::semantic_diagnostics_for_code(deps_bundle.as_ref(), FILE_PATH, &selected_code);

    let has_false_nonexistent_property = diagnostics.iter().any(|diag| {
        let msg = diag.message.as_str();
        msg.contains(&selected_member)
            && (msg.contains("Свойство") || msg.contains("Метод"))
            && (msg.contains("не существует") || msg.contains("не найден"))
    });
    assert!(
        !has_false_nonexistent_property,
        "completion-selected member '{}' produced false unknown-member diagnostic: {:?}, completion_labels={:?}",
        selected_member,
        diagnostics,
        labels
    );
}
