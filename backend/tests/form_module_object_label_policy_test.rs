//! Regression tests for strict form-data user-facing labels of FormModule.Объект.

mod support;

use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverOutputFormat};
use bsl_shared::formatting::DetailLevel;

fn hover_for_detail_level(level: DetailLevel) -> String {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let file_path = "Documents/Док1/Forms/Форма1/Ext/Form/Module.bsl";
    let code = concat!(
        "Процедура Тест()\n",
        "    x = Объект;\n",
        "КонецПроцедуры\n",
    );

    let hover_config = HoverFormatConfig {
        detail_level: level,
        output_format: HoverOutputFormat::Markdown,
        ..Default::default()
    };

    support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        file_path,
        code,
        1,
        9,
        Some(hover_config),
    )
    .expect("hover should exist")
}

#[test]
fn form_module_object_label_policy_is_form_data_for_compact_and_full() {
    let compact = hover_for_detail_level(DetailLevel::Compact);
    assert!(
        compact.contains("ДанныеФормыСтруктура"),
        "compact hover should use form-data label, got:\n{}",
        compact
    );
    assert!(
        !compact.contains("ДокументОбъект."),
        "compact hover must not leak owner-facet label, got:\n{}",
        compact
    );

    let full = hover_for_detail_level(DetailLevel::Full);
    assert!(
        full.contains("ДанныеФормыСтруктура"),
        "full hover should use form-data label, got:\n{}",
        full
    );
    assert!(
        !full.contains("ДокументОбъект."),
        "full hover must not leak owner-facet label, got:\n{}",
        full
    );
}

#[test]
fn form_module_object_label_policy_is_form_data_for_detailed() {
    let detailed = hover_for_detail_level(DetailLevel::Detailed);
    assert!(
        detailed.contains("ДанныеФормыСтруктура"),
        "detailed hover should use form-data label, got:\n{}",
        detailed
    );
    assert!(
        !detailed.contains("ДокументОбъект."),
        "detailed hover must not leak owner-facet label, got:\n{}",
        detailed
    );
    assert!(
        !detailed.contains("Фасет:"),
        "detailed hover for form-data object must not show active facet, got:\n{}",
        detailed
    );
    assert!(
        !detailed.contains("Доступные фасеты:"),
        "detailed hover for form-data object must not show available facets, got:\n{}",
        detailed
    );
}
