//! Контрактные проверки strict form-data policy для `FormModule.Объект`
//! на user-facing каналах: diagnostics, hover, completion, type-at-position.

mod support;

#[path = "../src/bin/lsp_server/converters/position.rs"]
pub mod position;

#[path = "../src/bin/lsp_server/handlers/completion.rs"]
mod completion_handler;

use std::path::PathBuf;
use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_backend::system::DepsBundleV2;
use bsl_shared::domain::types::{
    FacetKind, MetadataKind, RawDataSource, RawPropertyData, RawTypeData,
};
use bsl_shared::formatting::DetailLevel;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, Position, Url,
};

const FILE_PATH: &str = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
const OBJECT_MODULE_FILE_PATH: &str = "Documents/ЗаказНаряды/Ext/ObjectModule.bsl";
const RECORDSET_MODULE_FILE_PATH: &str =
    "InformationRegisters/ТестовыйРегистрСведений/Ext/RecordSetModule.bsl";
const CHARTS_OF_ACCOUNTS_MANAGER_FILE_PATH: &str =
    "ChartsOfAccounts/Хозрасчетный/Ext/ManagerModule.bsl";
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
        settings_id: SettingsId::from_hash("form-module-object-unified-contract"),
        diagnostics_detail_level,
    });
    host
}

fn setup_host(deps_bundle: &DepsBundleV2) -> AnalysisHostV2 {
    setup_host_with_detail_level(deps_bundle, DetailLevel::Full)
}

fn deps_bundle_v2_with_conf_fixture() -> Arc<DepsBundleV2> {
    let backend_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = backend_root.parent().expect("workspace root");
    let syntax_helper = workspace_root.join("examples").join("syntax_helper");
    let config_root = workspace_root
        .join("examples")
        .join("conf")
        .join("conf_test");

    assert!(
        syntax_helper.exists(),
        "syntax helper path does not exist: {}",
        syntax_helper.display()
    );
    assert!(
        config_root.exists(),
        "conf fixture path does not exist: {}",
        config_root.display()
    );

    support::deps_bundle_v2_for_paths(
        Some(syntax_helper.as_path()),
        Some(config_root.as_path()),
        Some("8.3.25"),
    )
}

fn inject_predefined_chart_of_accounts_type(deps_bundle: &DepsBundleV2) {
    deps_bundle
        .semantic_deps
        .repository
        .upsert_types(vec![RawTypeData {
            name: "ПланыСчетов.Хозрасчетный".to_string(),
            english_name: "ChartOfAccounts.Хозрасчетный".to_string(),
            source: RawDataSource::Configuration,
            kind: Some(MetadataKind::ChartOfAccounts),
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            properties: vec![RawPropertyData {
                name: "ГотоваяПродукция".to_string(),
                prop_type: "__predefined_manager__:ПланСчетовСсылка.Хозрасчетный".to_string(),
                is_readonly: true,
            }],
            ..Default::default()
        }])
        .expect("Failed to inject chart of accounts predefined fixture type");
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
fn diagnostics_hover_and_type_at_position_follow_unified_form_contract() {
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
async fn completion_and_resolve_follow_unified_form_contract() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri =
        Url::parse("file:///form_module_object_unified_contract_completion.bsl").expect("uri");
    let content = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "    Объект.\n",
        "КонецПроцедуры\n",
    );

    let mut host = setup_host(deps_bundle.as_ref());
    let (file_content, resolved_file_path, ir_program, parse_result) =
        apply_file(&mut host, V2FileId(1), FILE_PATH, content);
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
    assert!(
        !items
            .iter()
            .any(|item| item.label == "ПолучитьСсылкуНового"),
        "FormModule.Объект completion must not expose applied object-facet method ПолучитьСсылкуНового"
    );

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
async fn completion_form_module_object_fails_closed_without_shared_owner_hint() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri = Url::parse("file:///form_module_object_completion_no_hint.bsl").expect("uri");
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
    let labels = completion_items(response.response)
        .into_iter()
        .map(|item| item.label)
        .collect::<Vec<_>>();
    assert!(
        !labels.iter().any(|label| label == "Ссылка"),
        "completion without shared hint must not expose form-data members, labels={:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ПометкаУдаления"),
        "completion without shared hint must not expose form-data members, labels={:?}",
        labels
    );
    assert!(
        !labels.iter().any(|label| label == "ПолучитьСсылкуНового"),
        "completion without shared hint must not leak applied object-facet members, labels={:?}",
        labels
    );
}

#[test]
fn owner_member_fallback_is_applied_only_outside_form_module() {
    let form_deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let applied_deps_bundle = deps_bundle_v2_with_conf_fixture();

    let has_undeclared = |diagnostics: &[bsl_shared::domain::types::TypeDiagnostic],
                          identifier: &str| {
        diagnostics.iter().any(|diag| {
            diag.message.contains("Необъявленная переменная") && diag.message.contains(identifier)
        })
    };
    let messages = |diagnostics: &[bsl_shared::domain::types::TypeDiagnostic]| {
        diagnostics
            .iter()
            .map(|diag| diag.message.clone())
            .collect::<Vec<_>>()
    };

    let form_code = concat!(
        "Процедура Тест()\n",
        "    Проверка = ЗначениеЗаполнено(ДополнительныеСвойства);\n",
        "КонецПроцедуры\n",
    );
    let form_diagnostics =
        support::semantic_diagnostics_for_code(form_deps_bundle.as_ref(), FILE_PATH, form_code);
    assert!(
        has_undeclared(&form_diagnostics, "ДополнительныеСвойства"),
        "FormModule must stay strict for bare owner members, diagnostics={:?}",
        messages(&form_diagnostics)
    );

    let object_code = concat!(
        "Процедура Тест()\n",
        "    Проверка = НомерЗаказ;\n",
        "КонецПроцедуры\n",
    );
    let object_diagnostics = support::semantic_diagnostics_for_code(
        applied_deps_bundle.as_ref(),
        OBJECT_MODULE_FILE_PATH,
        object_code,
    );
    assert!(
        !has_undeclared(&object_diagnostics, "НомерЗаказ"),
        "ObjectModule must resolve bare owner members via fallback, diagnostics={:?}",
        messages(&object_diagnostics)
    );

    let recordset_code = concat!(
        "Процедура Тест()\n",
        "    Проверка1 = ТестовыйРесурс;\n",
        "    Проверка2 = ТестовыйРеквизит;\n",
        "    Проверка3 = ТестовоеИзмерение;\n",
        "КонецПроцедуры\n",
    );
    let recordset_diagnostics = support::semantic_diagnostics_for_code(
        applied_deps_bundle.as_ref(),
        RECORDSET_MODULE_FILE_PATH,
        recordset_code,
    );
    assert!(
        !has_undeclared(&recordset_diagnostics, "ТестовыйРесурс"),
        "RecordSetModule must resolve bare owner member ТестовыйРесурс, diagnostics={:?}",
        messages(&recordset_diagnostics)
    );
    assert!(
        !has_undeclared(&recordset_diagnostics, "ТестовыйРеквизит"),
        "RecordSetModule must resolve bare owner member ТестовыйРеквизит, diagnostics={:?}",
        messages(&recordset_diagnostics)
    );
    assert!(
        !has_undeclared(&recordset_diagnostics, "ТестовоеИзмерение"),
        "RecordSetModule must resolve bare owner member ТестовоеИзмерение, diagnostics={:?}",
        messages(&recordset_diagnostics)
    );
}

#[test]
fn recordset_module_resolves_system_members_and_manager_path_call() {
    let deps_bundle = deps_bundle_v2_with_conf_fixture();
    let owner_type = "РегистрСведенийМенеджер.ТестовыйРегистрСведений";
    let manager_method = "ВладелецБезопасногоХранилища";

    deps_bundle
        .semantic_deps
        .repository
        .add_config_method_signature(
            owner_type,
            bsl_shared::domain::signature_index::MethodSignature::new(
                manager_method.to_string(),
                Some(owner_type.to_string()),
                vec![],
                Some("Булево".to_string()),
                None,
                None,
                bsl_shared::domain::signature_index::SignatureSource::Configuration,
                None,
                bsl_shared::domain::signature_index::ContextRequirements::ServerOnly,
            ),
        );

    let code = concat!(
        "Процедура Тест()\n",
        "    Проверка1 = ОбменДанными.Загрузка;\n",
        "    Проверка2 = ДополнительныеСвойства.Свойство(\"Ключ\");\n",
        "    Проверка3 = РегистрыСведений.ТестовыйРегистрСведений.ВладелецБезопасногоХранилища();\n",
        "КонецПроцедуры\n",
    );
    let diagnostics = support::semantic_diagnostics_for_code(
        deps_bundle.as_ref(),
        RECORDSET_MODULE_FILE_PATH,
        code,
    );
    let messages = diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect::<Vec<_>>();

    let has_undeclared = |identifier: &str| {
        diagnostics.iter().any(|diag| {
            diag.message.contains("Необъявленная переменная") && diag.message.contains(identifier)
        })
    };
    assert!(
        !has_undeclared("ОбменДанными"),
        "RecordSetModule must resolve bare owner member ОбменДанными, diagnostics={:?}",
        messages
    );
    assert!(
        !has_undeclared("ДополнительныеСвойства"),
        "RecordSetModule must resolve bare owner member ДополнительныеСвойства, diagnostics={:?}",
        messages
    );

    let has_manager_path_call_error = diagnostics.iter().any(|diag| {
        diag.message.contains(manager_method)
            && (diag
                .message
                .contains("Неопределенная процедура или функция")
                || diag.message.contains("не существует")
                || diag.message.contains("не найден"))
    });
    assert!(
        !has_manager_path_call_error,
        "RecordSetModule manager path-call should resolve exported method, diagnostics={:?}",
        messages
    );
}

#[tokio::test]
async fn manager_predefined_member_resolves_and_is_visible_in_hover_completion() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    inject_predefined_chart_of_accounts_type(deps_bundle.as_ref());

    let diagnostics_code = concat!(
        "Процедура Тест()\n",
        "    Счет = ПланыСчетов.Хозрасчетный.ГотоваяПродукция;\n",
        "КонецПроцедуры\n",
    );
    let diagnostics = support::semantic_diagnostics_for_code(
        deps_bundle.as_ref(),
        CHARTS_OF_ACCOUNTS_MANAGER_FILE_PATH,
        diagnostics_code,
    );
    let diagnostic_messages = diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect::<Vec<_>>();
    let has_predefined_resolution_error = diagnostics.iter().any(|diag| {
        diag.message.contains("ГотоваяПродукция")
            && (diag.message.contains("Необъявленная переменная")
                || diag
                    .message
                    .contains("Неопределенная процедура или функция")
                || diag.message.contains("не существует")
                || diag.message.contains("не найден"))
    });
    assert!(
        !has_predefined_resolution_error,
        "Predefined manager member path must resolve without diagnostics, diagnostics={:?}",
        diagnostic_messages
    );

    let hover = support::hover_for_code(
        deps_bundle.as_ref(),
        CHARTS_OF_ACCOUNTS_MANAGER_FILE_PATH,
        diagnostics_code,
        1,
        utf16_len("    Счет = ПланыСчетов.Хозрасчетный.ГотоваяПродук"),
    )
    .expect("hover text for predefined manager member");
    assert!(
        hover.contains("ГотоваяПродукция"),
        "hover must mention predefined member name, hover={}",
        hover
    );
    assert!(
        hover.contains("ПланСчетовСсылка.Хозрасчетный"),
        "hover must expose decoded predefined member type, hover={}",
        hover
    );
    assert!(
        !hover.contains("__predefined_manager__:"),
        "hover must not leak internal predefined marker, hover={}",
        hover
    );

    let index_snapshot = deps_bundle.index_snapshot.clone();
    let uri = Url::parse("file:///manager_predefined_completion.bsl").expect("uri");
    let completion_content = concat!(
        "Процедура Тест()\n",
        "    ПланыСчетов.Хозрасчетный.\n",
        "КонецПроцедуры\n",
    );
    let mut host = setup_host(deps_bundle.as_ref());
    let (file_content, resolved_file_path, ir_program, parse_result) = apply_file(
        &mut host,
        V2FileId(1),
        CHARTS_OF_ACCOUNTS_MANAGER_FILE_PATH,
        completion_content,
    );

    let response = completion_handler::handle_completion_v2(
        file_content,
        resolved_file_path,
        ir_program,
        Some(parse_result),
        None,
        deps_bundle.semantic_deps.clone(),
        Position {
            line: 1,
            character: utf16_len("    ПланыСчетов.Хозрасчетный."),
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
    let labels = items
        .iter()
        .map(|item| item.label.clone())
        .collect::<Vec<_>>();
    assert!(
        labels.iter().any(|label| label == "ГотоваяПродукция"),
        "completion must include predefined manager member, labels={:?}",
        labels
    );
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
