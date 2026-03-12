//! Интеграционный тест: completion по метаданным fixture конфигурации (M5).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use bsl_analysis_v2::{AnalysisHostV2, Change as ChangeV2, FileId as V2FileId, SettingsId};
use bsl_backend::application::get_completion_with_semantic_program_snapshot_v2;
use bsl_backend::system::DepsBundleV2;
use bsl_shared::domain::types::FacetKind;
use bsl_shared::domain::types::MetadataKind;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::TypeResolution;
use bsl_shared::formatting::DetailLevel;

mod support;

fn workspace_root() -> PathBuf {
    let backend_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    backend_root.parent().expect("workspace root").to_path_buf()
}

static FIXTURE_DEPS: LazyLock<Arc<DepsBundleV2>> = LazyLock::new(|| {
    let root = workspace_root();
    let syntax_helper = root.join("examples").join("syntax_helper");
    let config_root = root.join("examples").join("conf").join("conf_test");
    support::deps_bundle_v2_for_paths(
        Some(syntax_helper.as_path()),
        Some(config_root.as_path()),
        Some("8.3.25"),
    )
});

fn utf16_column(line: &str) -> u32 {
    line.chars().map(|ch| ch.len_utf16()).sum::<usize>() as u32
}

#[tokio::test]
async fn metadata_completion_supports_documents_facets_and_tabular_sections() {
    let deps_bundle = FIXTURE_DEPS.clone();
    let repo = deps_bundle.semantic_deps.repository.clone();
    assert!(
        repo.get_stats().configuration_types > 0,
        "fixture config should be loaded"
    );

    let resolver = deps_bundle
        .semantic_deps
        .resolver
        .clone()
        .expect("resolver");
    let metadata_lookup = TypeMetadataLookup::new(repo.clone());

    let content = concat!(
        "Процедура Тест()\n",
        "    Заказ\n",
        "    Документы.\n",
        "    Документы.ЗаказНаряды.\n",
        "    Документы.ЗаказНаряды.СоздатьДокумент().\n",
        "    Документы.ЗаказНаряды.СоздатьДокумент().ПолучитьСсылкуНового().\n",
        "    Документы.ЗаказНаряды.СоздатьДокумент().Работы.\n",
        "    Документы.ЗаказНаряды.СоздатьДокумент().Работы.Добавить().\n",
        "КонецПроцедуры\n",
    );

    let deps = deps_bundle.semantic_deps.clone();
    let mut host = AnalysisHostV2::default();
    host.apply_change(ChangeV2::SetDepsSnapshot {
        deps_id: deps_bundle.deps_id.clone(),
        deps,
    });
    host.apply_change(ChangeV2::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("m5-metadata-completion"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    let file_id = V2FileId(1);
    host.apply_change(ChangeV2::SetFile {
        file_id,
        text: Arc::from(content.to_string()),
        version: 0,
        path: Arc::from("m5_metadata_completion_fixture.bsl"),
    });

    let analysis = host.analysis();
    analysis
        .precompute_type_index_for_file(file_id, None, 0)
        .expect("type index precompute");
    let ir_program = analysis.ir(file_id).ok().flatten().expect("ir");

    let file_uri = Some("file:///m5_metadata_completion_fixture.bsl");
    let index_snapshot = deps_bundle.index_snapshot.as_ref();
    assert!(
        index_snapshot
            .metadata_index
            .contains_key(&MetadataKind::Document),
        "index snapshot should contain document metadata, kinds: {:?}",
        index_snapshot.metadata_index.keys().collect::<Vec<_>>()
    );
    let mut repository_backed_snapshot = index_snapshot.clone();
    repository_backed_snapshot.metadata_index = Arc::new(HashMap::new());

    // 1) non-member metadata completion не зависит от metadata_index
    let line = 1u32;
    let line_text = "    Заказ";
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program.clone(),
        None,
        false,
    )
    .await
    .expect("completion ok");
    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    assert!(
        labels.contains(&"ЗаказНаряды"),
        "non-member metadata completion should include ЗаказНаряды without metadata_index, labels: {:?}",
        labels
    );

    // 2) Документы. -> имена документов + kind/detail
    let line = 2u32;
    let line_text = "    Документы.";
    assert!(
        analysis
            .current_type_index_serve_only_ready(file_id)
            .expect("serve-only readiness"),
        "Документы. fixture must have exact serve-only artifact"
    );
    let documents_probe = content
        .find("Документы")
        .map(|idx| idx + "Документы".len() - 1)
        .expect("Документы probe") as u32;
    let documents_profiled = analysis
        .type_at_byte_offset_serve_only_profiled(file_id, documents_probe)
        .expect("Документы serve-only lookup");
    assert_eq!(
        documents_profiled
            .resolution
            .as_ref()
            .map(TypeResolution::type_name),
        Some("Документы".to_string()),
        "Документы. fixture must materialize exact owner type before completion"
    );
    let owner_hint = support::completion_owner_hint_for_position(
        &analysis,
        file_id,
        content,
        line,
        utf16_column(line_text),
    );
    assert_eq!(
        owner_hint.as_ref().map(TypeResolution::type_name),
        Some("Документы".to_string()),
        "Документы. should surface canonical owner hint before completion"
    );
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program.clone(),
        owner_hint,
        false,
    )
    .await
    .expect("completion ok");

    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    let item = result
        .items
        .iter()
        .find(|c| c.item.label == "ЗаказНаряды")
        .map(|c| &c.item)
        .unwrap_or_else(|| {
            panic!(
                "Документы. should include ЗаказНаряды, labels: {:?}",
                labels
            )
        });
    assert!(
        matches!(item.kind, bsl_shared::domain::CompletionKind::Document),
        "Документы. items should be DOCUMENT kind"
    );
    assert!(
        item.detail
            .as_deref()
            .unwrap_or_default()
            .contains("Документ"),
        "Документы. items should have detail with kind name"
    );

    // 3) Документы.ЗаказНаряды. -> методы менеджера
    let line = 3u32;
    let line_text = "    Документы.ЗаказНаряды.";
    let owner_hint = support::completion_owner_hint_for_position(
        &analysis,
        file_id,
        content,
        line,
        utf16_column(line_text),
    );
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program.clone(),
        owner_hint,
        false,
    )
    .await
    .expect("completion ok");
    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    assert!(
        labels.contains(&"СоздатьДокумент"),
        "Документы.ЗаказНаряды. should include СоздатьДокумент, labels: {:?}",
        labels
    );

    // 4) ...СоздатьДокумент(). -> свойства объекта (в т.ч. фасет Ссылка и ТЧ)
    let line = 4u32;
    let line_text = "    Документы.ЗаказНаряды.СоздатьДокумент().";
    let owner_hint = support::completion_owner_hint_for_position(
        &analysis,
        file_id,
        content,
        line,
        utf16_column(line_text),
    );
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program.clone(),
        owner_hint,
        false,
    )
    .await
    .expect("completion ok");
    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    assert!(
        labels.contains(&"ПолучитьСсылкуНового"),
        "ДокументОбъект.* should include ПолучитьСсылкуНового, labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"Работы"),
        "ДокументОбъект.* should include tabular section 'Работы', labels: {:?}",
        labels
    );

    // 5) ...СоздатьДокумент().ПолучитьСсылкуНового(). -> методы ссылки
    let line = 5u32;
    let line_text = "    Документы.ЗаказНаряды.СоздатьДокумент().ПолучитьСсылкуНового().";
    let owner_hint = support::completion_owner_hint_for_position(
        &analysis,
        file_id,
        content,
        line,
        utf16_column(line_text),
    );
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program.clone(),
        owner_hint,
        false,
    )
    .await
    .expect("completion ok");
    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    assert!(
        labels.contains(&"ПолучитьОбъект"),
        "ДокументСсылка.* should include ПолучитьОбъект, labels: {:?}",
        labels
    );

    // 6) ...СоздатьДокумент().Работы. -> методы табличной части (коллекция)
    let line = 6u32;
    let line_text = "    Документы.ЗаказНаряды.СоздатьДокумент().Работы.";
    let owner_hint = support::completion_owner_hint_for_position(
        &analysis,
        file_id,
        content,
        line,
        utf16_column(line_text),
    );
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program.clone(),
        owner_hint,
        false,
    )
    .await
    .expect("completion ok");
    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    assert!(
        labels.contains(&"Добавить"),
        "ТабличнаяЧасть.* should include Добавить, labels: {:?}",
        labels
    );

    // 7) ...Работы.Добавить(). -> свойства строки табличной части
    let line = 7u32;
    let line_text = "    Документы.ЗаказНаряды.СоздатьДокумент().Работы.Добавить().";
    let owner_hint = support::completion_owner_hint_for_position(
        &analysis,
        file_id,
        content,
        line,
        utf16_column(line_text),
    );
    let result = get_completion_with_semantic_program_snapshot_v2(
        content,
        line,
        utf16_column(line_text),
        file_uri,
        &repository_backed_snapshot,
        &metadata_lookup,
        "m5_metadata_completion_fixture.bsl",
        resolver.as_ref(),
        ir_program,
        owner_hint,
        false,
    )
    .await
    .expect("completion ok");
    let labels: Vec<&str> = result.items.iter().map(|c| c.item.label.as_str()).collect();
    assert!(
        labels.contains(&"ВидРаботы"),
        "СтрокаРаботы.* should include ВидРаботы, labels: {:?}",
        labels
    );
    assert!(
        labels.contains(&"LineNumber"),
        "СтрокаРаботы.* should include LineNumber, labels: {:?}",
        labels
    );

    // Доп. sanity: active facet реально переключился на Collection для табличной части
    let ts_resolution = resolver.resolve_expression_sync("Документы.ЗаказНаряды.Работы");
    assert_eq!(ts_resolution.active_facet, Some(FacetKind::Collection));
}
