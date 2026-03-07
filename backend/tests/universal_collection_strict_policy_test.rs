mod support;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_shared::formatting::{user_facing_resolution_type_name, DetailLevel};

const FILE_ID: V2FileId = V2FileId(1);
const FILE_PATH: &str = "Documents/Док1/Ext/ObjectModule.bsl";

fn has_unknown_member_diagnostic(message: &str, member_name: &str) -> bool {
    let lower_message = message.to_lowercase();
    lower_message.contains(&member_name.to_lowercase())
        && (lower_message.contains("не существует") || lower_message.contains("не найден"))
}

fn host_for_code(code: &str) -> AnalysisHostV2 {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("universal-collection-strict-policy-tests"),
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

fn type_name_at(code: &str, needle: &str) -> String {
    let host = host_for_code(code);
    let analysis = host.analysis();
    let offset = code
        .find(needle)
        .map(|idx| idx + needle.len() - 1)
        .unwrap_or_else(|| panic!("needle '{needle}' not found")) as u32;
    let resolution = analysis
        .type_at_byte_offset(FILE_ID, offset)
        .expect("type_at_byte_offset query")
        .expect("type_at_byte_offset result");

    user_facing_resolution_type_name(&resolution)
}

#[test]
fn dynamic_map_key_uses_safe_policy_without_unknown_key_diagnostic() {
    let code = concat!(
        "Процедура Тест()\n",
        "    map = Новый Соответствие;\n",
        "    map.Вставить(\"Идентификатор\", 10);\n",
        "    Ключ = \"Идентификатор\";\n",
        "    probe = map[Ключ];\n",
        "КонецПроцедуры\n",
    );

    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let diagnostics = support::semantic_diagnostics_for_code(deps_bundle.as_ref(), FILE_PATH, code);
    assert!(
        !diagnostics.iter().any(|diag| {
            let lower = diag.message.to_lowercase();
            lower.contains("ключ") && lower.contains("не найден")
        }),
        "dynamic key must not emit unknown-key hard-fail diagnostics: {diagnostics:?}"
    );

    assert_eq!(type_name_at(code, "map[Ключ]"), "Число");
}

#[test]
fn typed_structure_unknown_field_emits_non_existent_property_diagnostic() {
    let code = concat!(
        "Процедура Тест()\n",
        "    S = Новый Структура;\n",
        "    S.Вставить(\"Идентификатор\", \"A-01\");\n",
        "    probe = S.Идентифкатор;\n",
        "КонецПроцедуры\n",
    );

    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let diagnostics = support::semantic_diagnostics_for_code(deps_bundle.as_ref(), FILE_PATH, code);
    assert!(
        diagnostics
            .iter()
            .any(|diag| has_unknown_member_diagnostic(&diag.message, "Идентифкатор")),
        "typed structure typo must emit NonExistentProperty diagnostic: {diagnostics:?}"
    );
}

#[test]
fn typed_value_table_row_unknown_column_emits_non_existent_property_diagnostic() {
    let code = concat!(
        "Процедура Тест()\n",
        "    ТЗ = Новый ТаблицаЗначений;\n",
        "    ТЗ.Колонки.Добавить(\"Идентификатор\", Новый ОписаниеТипов(\"Строка\"));\n",
        "    Стр = ТЗ.Добавить();\n",
        "    probe = Стр.Идентифкатор;\n",
        "КонецПроцедуры\n",
    );

    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let diagnostics = support::semantic_diagnostics_for_code(deps_bundle.as_ref(), FILE_PATH, code);
    assert!(
        diagnostics
            .iter()
            .any(|diag| has_unknown_member_diagnostic(&diag.message, "Идентифкатор")),
        "typed row typo must emit NonExistentProperty diagnostic: {diagnostics:?}"
    );
}
