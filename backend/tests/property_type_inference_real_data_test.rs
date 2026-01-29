//! Интеграционный тест: типизация свойства из Syntax Helper (без хардкода)

mod support;

use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_shared::domain::types::Certainty;
use bsl_shared::formatting::DetailLevel;

#[tokio::test]
async fn test_value_table_columns_property_type_is_resolved() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let code = r#"
Процедура Тест()
    ТаблЗнч = Новый ТаблицаЗначений;
    КолонкиТаблЗнч = ТаблЗнч.Колонки;
КонецПроцедуры
"#;

    let file_id = V2FileId(1);
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("tests"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(code),
        version: 0,
        path: Arc::from("test.bsl"),
    });

    let analysis = host.analysis();
    let columns_offset = code.rfind("Колонки").expect("expected Колонки in code");
    let resolved = analysis
        .type_at_byte_offset(file_id, columns_offset as u32)
        .ok()
        .flatten()
        .expect("type_at_byte_offset should exist");

    assert_eq!(
        resolved.type_name(),
        "КоллекцияКолонокТаблицыЗначений",
        "ТаблицаЗначений.Колонки должен иметь тип КоллекцияКолонокТаблицыЗначений"
    );
    assert_eq!(
        resolved.certainty,
        Certainty::Known,
        "Тип свойства должен быть Known (из документации платформы)"
    );
}
