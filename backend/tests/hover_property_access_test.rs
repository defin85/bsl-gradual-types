use std::sync::Arc;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::get_hover_info_with_semantic_program;
use bsl_backend::helpers::hover_formatter::HoverFormatConfig;
use bsl_backend::helpers::hover_formatter::HoverFormatter;
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;

mod support;

#[tokio::test]
async fn test_hover_on_property_name_shows_property_type() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();

    let code = "Процедура Тест()\n\
    ТаблЗнч = Новый ТаблицаЗначений;\n\
    КолонкиТаблЗнач = ТаблЗнч.Колонки;\n\
КонецПроцедуры";

    // line/column: 0-based, column UTF-16.
    // В строке 'КолонкиТаблЗнач = ТаблЗнч.Колонки;' имя свойства начинается с колонки 30.
    let hover = support::hover_for_code_with_config(
        deps_bundle.as_ref(),
        "inline.bsl",
        code,
        2,
        30,
        Some(HoverFormatConfig::default()),
    )
    .expect("hover should exist");

    assert!(
        hover.contains("**Свойство:**"),
        "hover должен быть для свойства, а не для переменной объекта: {}",
        hover
    );
    assert!(
        hover.contains("КоллекцияКолонокТаблицыЗначений"),
        "должен показываться тип свойства ТаблицаЗначений.Колонки: {}",
        hover
    );
}

#[tokio::test]
async fn test_hover_on_property_name_works_with_empty_request_time_repository() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let code = "Процедура Тест()\n\
    ТаблЗнч = Новый ТаблицаЗначений;\n\
    КолонкиТаблЗнач = ТаблЗнч.Колонки;\n\
КонецПроцедуры";

    let file_id = V2FileId(1);
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps: deps_bundle.semantic_deps.clone(),
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("hover-empty-request-time-repository"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(code.to_string()),
        version: 0,
        path: Arc::from("inline.bsl"),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, Some(0), 0)
        .expect("precompute exact type index");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

    let empty_repository: Arc<dyn TypeRepository> = Arc::new(InMemoryTypeRepository::new());
    let metadata_lookup = TypeMetadataLookup::new(empty_repository.clone());
    let hover_formatter = HoverFormatter::new(HoverFormatConfig::default(), metadata_lookup.clone());
    let resolver = TypeResolver::new(empty_repository);

    let hover = get_hover_info_with_semantic_program(
        &analysis,
        file_id,
        code,
        2,
        30,
        false,
        &metadata_lookup,
        &hover_formatter,
        Some(HoverFormatConfig::default()),
        &resolver,
        ir_program,
    )
    .expect("property hover from semantic facts");

    assert!(
        hover.contains("**Свойство:**"),
        "hover должен остаться property-hover без request-time repository: {}",
        hover
    );
    assert!(
        hover.contains("КоллекцияКолонокТаблицыЗначений"),
        "тип свойства должен приходить из exact semantic path, got: {}",
        hover
    );
}
