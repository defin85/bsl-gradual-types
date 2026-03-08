//! Интеграционные проверки, что legacy alias `ДанныеФормыОбъект.*`
//! не попадает в user-facing выдачу.

mod support;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::system::DepsBundleV2;
use bsl_shared::formatting::DetailLevel;
use tower_lsp::lsp_types::{CompletionItem, CompletionResponse, Documentation, Position, Url};

const LEGACY_ALIAS: &str = "ДанныеФормыОбъект";

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
        settings_id: SettingsId::from_hash("legacy-form-object-alias-outputs"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host
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

fn shared_owner_hint_at_marker(
    host: &AnalysisHostV2,
    file_id: V2FileId,
    content: &str,
    marker: &str,
) -> Option<bsl_shared::domain::types::TypeResolution> {
    let object_offset = content.find(marker).expect("marker offset") + "x = ".len();
    host.analysis()
        .type_at_byte_offset(file_id, object_offset as u32)
        .expect("type_at_byte_offset query")
}

fn completion_items(response: CompletionResponse) -> Vec<CompletionItem> {
    match response {
        CompletionResponse::List(list) => list.items,
        CompletionResponse::Array(items) => items,
    }
}

#[test]
fn diagnostics_hover_and_type_at_position_do_not_expose_legacy_form_alias() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let code = concat!(
        "Процедура Тест()\n",
        "    x = Объект.Ссылка;\n",
        "КонецПроцедуры\n",
    );

    let diagnostics = support::semantic_diagnostics_for_code(deps_bundle.as_ref(), file_path, code);
    assert!(
        diagnostics
            .iter()
            .all(|diag| !diag.message.contains(LEGACY_ALIAS)),
        "legacy alias leaked to diagnostics: {:?}",
        diagnostics
            .iter()
            .map(|diag| diag.message.clone())
            .collect::<Vec<_>>()
    );

    let hover = support::hover_for_code(
        deps_bundle.as_ref(),
        file_path,
        code,
        1,
        utf16_len("    x = Объе"),
    )
    .expect("hover text");
    assert!(
        !hover.contains(LEGACY_ALIAS),
        "legacy alias leaked to hover: {}",
        hover
    );

    let mut host = setup_host(deps_bundle.as_ref());
    host.apply_change(ChangeV2::SetFile {
        file_id: V2FileId(1),
        text: Arc::from(code.to_string()),
        version: 1,
        path: Arc::from(file_path.to_string()),
    });
    let analysis = host.analysis();
    let object_offset = code.find("Объект").expect("Объект offset") as u32;
    let object_type = analysis
        .type_at_byte_offset(V2FileId(1), object_offset)
        .expect("type_at_byte_offset query")
        .map(|ty| ty.type_name())
        .expect("type at Объект");
    assert!(
        !object_type.contains(LEGACY_ALIAS),
        "legacy alias leaked to type-at-position: {}",
        object_type
    );
}

#[tokio::test]
async fn completion_and_resolve_do_not_expose_legacy_form_alias() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri = Url::parse("file:///legacy_form_alias_completion.bsl").expect("uri");
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let content = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "    Объект.\n",
        "КонецПроцедуры\n",
    );

    let mut host = setup_host(deps_bundle.as_ref());
    let (file_content, resolved_file_path, ir_program, parse_result) =
        apply_file(&mut host, V2FileId(1), file_path, content);
    let member_access_owner_type_hint =
        shared_owner_hint_at_marker(&host, V2FileId(1), content, "x = Объект");
    assert!(
        member_access_owner_type_hint.is_some(),
        "expected shared owner hint for FormModule.Объект completion"
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
    let items = completion_items(response.response);
    assert!(!items.is_empty(), "expected completion items");

    for item in items.iter().take(40) {
        assert!(
            !item.label.contains(LEGACY_ALIAS),
            "legacy alias leaked to completion label: {:?}",
            item.label
        );

        if let Some(data) = &item.data {
            assert!(
                !data.to_string().contains(LEGACY_ALIAS),
                "legacy alias leaked to completion item data: {}",
                data
            );
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
            assert!(
                !detail.contains(LEGACY_ALIAS),
                "legacy alias leaked to completion detail: {}",
                detail
            );
        }

        if let Some(Documentation::String(doc)) = resolved.documentation {
            assert!(
                !doc.contains(LEGACY_ALIAS),
                "legacy alias leaked to completion docs: {}",
                doc
            );
        }
    }
}

#[tokio::test]
async fn completion_catalog_form_module_object_includes_intrinsic_properties() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri = Url::parse("file:///catalog_form_intrinsic_completion.bsl").expect("uri");
    let file_path = "Catalogs/Спр1/Forms/ФормаЭлемента/Ext/Form/Module.bsl";
    let content = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "    Объект.\n",
        "КонецПроцедуры\n",
    );

    let mut host = setup_host(deps_bundle.as_ref());
    let (file_content, resolved_file_path, ir_program, parse_result) =
        apply_file(&mut host, V2FileId(1), file_path, content);
    let object_offset = content.find("x = Объект").expect("Объект offset") + "x = ".len();
    let object_offset = object_offset as u32;
    let member_access_owner_type_hint = host
        .analysis()
        .type_at_byte_offset(V2FileId(1), object_offset)
        .expect("type_at_byte_offset query");
    assert!(
        member_access_owner_type_hint.is_some(),
        "expected type hint for implicit Объект in catalog form module"
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
    let labels = completion_items(response.response)
        .into_iter()
        .map(|item| item.label)
        .collect::<Vec<_>>();
    assert!(
        labels.iter().any(|label| label == "Ссылка"),
        "completion should include intrinsic property Ссылка, labels={:?}",
        labels
    );
    assert!(
        labels.iter().any(|label| label == "ПометкаУдаления"),
        "completion should include intrinsic property ПометкаУдаления, labels={:?}",
        labels
    );
}
