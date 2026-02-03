//! Тесты для simplified architecture компонентов

use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::get_completion_with_semantic_program_snapshot;
use bsl_backend::application::get_hover_info_with_semantic_program;
use bsl_backend::helpers::hover_formatter::{HoverFormatConfig, HoverFormatter};
use bsl_backend::system::{
    build_deps_bundle_v2, DepsBundleV2, ParserCoordinator, SystemCoordinator,
};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::SemanticProgram;

fn deps_bundle_v2_with_syntax_helper() -> Arc<DepsBundleV2> {
    static BUNDLE: LazyLock<Arc<DepsBundleV2>> = LazyLock::new(|| {
        let coordinator = SystemCoordinator::new();
        coordinator
            .start_with_paths_blocking(Some(Path::new("examples/syntax_helper")), None, None, None)
            .expect("startup");

        Arc::new(
            build_deps_bundle_v2(
                &coordinator,
                Some(Path::new("examples/syntax_helper")),
                None,
            )
            .expect("deps bundle v2"),
        )
    });

    BUNDLE.clone()
}

fn analysis_and_ir_program_for_code(
    deps_bundle: &DepsBundleV2,
    file_path: &str,
    code: &str,
) -> (bsl_analysis_v2::AnalysisV2, Arc<SemanticProgram>) {
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
        file_id: V2FileId(1),
        text: Arc::from(code),
        version: 0,
        path: Arc::from(file_path.to_string()),
    });

    let analysis = host.analysis();
    let ir_program = analysis
        .ir(V2FileId(1))
        .expect("ir query cancelled")
        .expect("ir unavailable");

    (analysis, ir_program)
}

#[tokio::test]
async fn test_system_coordinator_creation() {
    let coordinator = SystemCoordinator::new();

    let health = coordinator.health_status();
    assert_eq!(health.status, "healthy");
    assert_eq!(health.components.len(), 3);
}

#[tokio::test]
async fn test_system_coordinator_startup() {
    let coordinator = SystemCoordinator::new();

    coordinator.start().await.expect("Coordinator should start");

    assert!(
        coordinator.domain_bundle().is_some(),
        "DomainBundle должен быть инициализирован после старта"
    );
}

#[test]
fn test_parser_coordinator() {
    let parser = ParserCoordinator::with_fallback();

    let content = "Функция Тест() Возврат 42; КонецФункции";
    let result = parser.parse(content);
    assert!(
        result.is_ok(),
        "Парсер должен успешно разбирать базовый код"
    );
}

#[test]
fn test_basic_observability() {
    use bsl_backend::system::basic_observability::BasicObservability;

    let observability = BasicObservability::default();

    let health = observability.health_check();
    assert_eq!(health.status, "healthy");
    assert_eq!(health.components.len(), 3);

    observability.log_analysis("test.bsl", Duration::from_millis(100));

    let metrics = observability.get_metrics();
    assert_eq!(metrics.get_counter("analyses_total"), 1);
    assert_eq!(metrics.get_gauge("analysis_duration_ms"), 100.0);

    let exported = metrics.export_metrics();
    assert!(exported.is_object());
}

#[tokio::test]
async fn test_v2_completion_and_hover_smoke() {
    let deps_bundle = deps_bundle_v2_with_syntax_helper();

    let code = r#"
Процедура Тест()
    Массив = Новый Массив();
    Массив.
КонецПроцедуры
"#;

    let (analysis, ir_program) =
        analysis_and_ir_program_for_code(deps_bundle.as_ref(), "inline.bsl", code);

    let deps = deps_bundle.semantic_deps.clone();
    let resolver = deps
        .resolver
        .clone()
        .unwrap_or_else(|| Arc::new(TypeResolver::new(deps.repository.clone())));

    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());
    let hover_formatter =
        HoverFormatter::new(HoverFormatConfig::default(), metadata_lookup.clone());

    let hover = get_hover_info_with_semantic_program(
        &analysis,
        V2FileId(1),
        code,
        3,
        5,
        &metadata_lookup,
        &hover_formatter,
        None,
        resolver.as_ref(),
        ir_program.clone(),
    );
    assert!(hover.is_some(), "Hover должен вернуть строку");

    let completion = get_completion_with_semantic_program_snapshot(
        code,
        3,
        10,
        None,
        deps_bundle.index_snapshot.as_ref(),
        &metadata_lookup,
        "inline.bsl",
        resolver.as_ref(),
        ir_program,
        None,
    )
    .await
    .expect("completion");

    assert!(
        !completion.items.is_empty(),
        "Completion не должен быть пустым"
    );
}
