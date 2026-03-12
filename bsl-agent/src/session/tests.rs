use super::*;
use crate::jobs::JobManager;
use crate::server::types::{
    BslDefinitionParams, BslDiagnosticsParams, BslMembersParams, BslReferencesParams,
    BslSymbolSearchParams, BslTypeAtPositionParams, CanonicalDocumentRef, ContextExpandParams,
    ContextFocus, ContextInclude, ContextPackParams, DocumentRef, FileRef, Position,
    WorkspaceDocumentsSetFile, WorkspaceOpenParams, WorkspaceScope, WorkspaceScopeTagged,
};
use crate::types::JobStateDto;
use std::sync::Arc;

const UNIFIED_STAGE_COUNTER_KEYS: &[&str] = &[
    "intellisense_v2_snapshot_diagnostics_total",
    "intellisense_v2_semantic_diagnostics_query_total",
    "intellisense_v2_snapshot_other_total",
    "intellisense_v2_ir_query_other_total",
    "intellisense_v2_parse_result_query_total",
    "intellisense_v2_singleflight_key_unavailable_total",
    "intellisense_v2_observability_contract_violation_total",
    "intellisense_v2_projection_missing_total",
    "intellisense_v2_runtime_saturation_sample_total",
];

const UNIFIED_STAGE_HISTOGRAM_KEYS: &[&str] = &[
    "intellisense_v2_snapshot_diagnostics_ms",
    "intellisense_v2_semantic_diagnostics_query_ms",
    "intellisense_v2_snapshot_other_ms",
    "intellisense_v2_ir_query_other_ms",
    "intellisense_v2_parse_result_query_ms",
];

const UNIFIED_STAGE_GAUGE_KEYS: &[&str] = &[
    "intellisense_v2_runtime_saturation_waiters_interactive",
    "intellisense_v2_runtime_saturation_waiters_background",
    "intellisense_v2_runtime_saturation_permits_interactive",
    "intellisense_v2_runtime_saturation_permits_background",
    "intellisense_v2_runtime_saturation_permits_shared",
    "intellisense_v2_runtime_saturation_queue_depth_total",
];

fn counter_value(metrics: &serde_json::Value, key: &str) -> u64 {
    metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .and_then(|counters| counters.get(key))
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
}

fn type_index_reason_total(metrics: &serde_json::Value) -> u64 {
    metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .map(|counters| {
            counters
                .iter()
                .filter(|(key, _)| {
                    key.starts_with("intellisense_v2_type_index_reason_total_reason_")
                })
                .map(|(_, value)| value.as_u64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
}

fn assert_unified_intellisense_v2_stage_contract(metrics: &serde_json::Value) {
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");
    let gauges = metrics
        .get("gauges")
        .and_then(|value| value.as_object())
        .expect("metrics.gauges object");
    let histograms = metrics
        .get("histograms")
        .and_then(|value| value.as_object())
        .expect("metrics.histograms object");

    for key in UNIFIED_STAGE_COUNTER_KEYS {
        assert!(
            counters.contains_key(*key),
            "missing counter key {key}, got keys={:?}",
            counters.keys().collect::<Vec<_>>()
        );
    }

    for key in UNIFIED_STAGE_HISTOGRAM_KEYS {
        assert!(
            histograms.contains_key(*key),
            "missing histogram key {key}, got keys={:?}",
            histograms.keys().collect::<Vec<_>>()
        );
    }

    for key in UNIFIED_STAGE_GAUGE_KEYS {
        assert!(
            gauges.contains_key(*key),
            "missing gauge key {key}, got keys={:?}",
            gauges.keys().collect::<Vec<_>>()
        );
    }

    assert!(
        counters
            .keys()
            .any(|key| key.starts_with("intellisense_v2_drilldown_stage_total_origin_agent_")),
        "missing agent drilldown stage_total counters"
    );
    assert!(
        histograms
            .keys()
            .any(|key| key.starts_with("intellisense_v2_drilldown_stage_latency_ms_origin_agent_")),
        "missing agent drilldown stage_latency histograms"
    );
}

#[test]
fn observability_cancellation_counters_follow_unified_stage_contract() {
    let coordinator = bsl_runtime::system::SystemCoordinator::new();
    let analysis = bsl_analysis_v2::AnalysisHostV2::default().snapshot();
    let context = ExecutionContext {
        origin: ObservabilityOrigin::Agent,
        operation: SemanticOperation::Members,
        completion_mode: None,
        completion_large_churn_active: false,
        file_id: FileId(1),
        min_file_version: None,
        expected_deps_id: None,
        flow_sensitive: false,
        settings: ExecutionSettings {
            settings_id: SettingsId::from_hash("tests"),
            diagnostics_detail_level: DetailLevel::Full,
        },
        cancellation: CancellationPolicy::BestEffort,
    };

    let _ = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::IrQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    );
    let _ = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::SyntaxDiagnosticsQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    );
    let _ = IntellisenseV2Facade::run_optional_query(
        &context,
        ObservabilityStage::SemanticDiagnosticsQuery,
        &analysis,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    );
    let _ = IntellisenseV2Facade::run_parse_result_query(
        &context,
        &analysis,
        true,
        Some(&coordinator),
        |_analysis| Err::<Option<()>, ()>(()),
    );

    let metrics = coordinator.observability_metrics();
    let counters = metrics
        .get("counters")
        .and_then(|value| value.as_object())
        .expect("metrics.counters object");

    for key in [
        "intellisense_v2_ir_query_cancelled_total_other",
        "intellisense_v2_query_cancelled_total_syntax",
        "intellisense_v2_query_cancelled_total_semantic",
        "intellisense_v2_query_cancelled_total_other",
    ] {
        assert!(
            counters
                .get(key)
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                > 0,
            "expected positive cancelled counter for key {key}, counters={counters:?}"
        );
    }
}

#[test]
fn optional_query_outcome_classification_is_shared() {
    let success = Ok::<Option<()>, ()>(Some(()));
    let empty = Ok::<Option<()>, ()>(None);
    let cancelled = Err::<Option<()>, ()>(());

    assert_eq!(
        bsl_runtime::application::classify_optional_query(&success),
        bsl_runtime::application::SemanticOutcome::Success
    );
    assert_eq!(
        bsl_runtime::application::classify_optional_query(&empty),
        bsl_runtime::application::SemanticOutcome::Empty
    );
    assert_eq!(
        bsl_runtime::application::classify_optional_query(&cancelled),
        bsl_runtime::application::SemanticOutcome::Cancelled
    );
}

#[test]
fn collect_type_at_position_preserves_available_facets_for_object_module_binding() {
    use bsl_analysis_v2::{DepsSnapshotId, SemanticDeps};
    use bsl_runtime::system::{IndexSnapshot, IndexSnapshotId, SystemCoordinator};
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::SignatureIndex;
    use bsl_shared::domain::types::{FacetKind, MetadataKind, RawDataSource, RawTypeData};

    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.Док1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            ..Default::default()
        }])
        .expect("load types");

    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let response = collect_type_at_position(TypeAtPositionRequest {
        analysis_revision: 1,
        flow_sensitive_enabled: false,
        deps_id: DepsSnapshotId::from_hash("type-at-position-facet-preservation"),
        deps,
        index_snapshot: Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(
            "type-at-position-facet-preservation",
        ))),
        coordinator: Arc::new(SystemCoordinator::new()),
        text: "Процедура Тест()\n    x = ЭтотОбъект;\nКонецПроцедуры\n".to_string(),
        version: 0,
        abs_path: "Documents/Док1/Ext/ObjectModule.bsl".to_string(),
        position: Position {
            line: 1,
            character: 10,
        },
    })
    .expect("type_at_position");

    let type_info = response.type_info.expect("type info");
    assert!(
        response.warnings.is_empty(),
        "warnings: {:?}",
        response.warnings
    );
    assert!(
        type_info.name.contains("Док1"),
        "type name: {}",
        type_info.name
    );
    assert_eq!(type_info.active_facet.as_deref(), Some("Object"));
    assert_eq!(
        type_info.available_facets,
        vec![
            "Manager".to_string(),
            "Object".to_string(),
            "Reference".to_string(),
        ]
    );
}

#[test]
fn collect_type_at_position_preserves_available_facets_for_recordset_module_binding() {
    use bsl_analysis_v2::{DepsSnapshotId, SemanticDeps};
    use bsl_runtime::system::{IndexSnapshot, IndexSnapshotId, SystemCoordinator};
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::SignatureIndex;
    use bsl_shared::domain::types::{FacetKind, MetadataKind, RawDataSource, RawTypeData};

    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "РегистрыСведений.Регистр1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object],
            kind: Some(MetadataKind::InformationRegister),
            ..Default::default()
        }])
        .expect("load types");

    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let response = collect_type_at_position(TypeAtPositionRequest {
        analysis_revision: 1,
        flow_sensitive_enabled: false,
        deps_id: DepsSnapshotId::from_hash("type-at-position-recordset-facet-preservation"),
        deps,
        index_snapshot: Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(
            "type-at-position-recordset-facet-preservation",
        ))),
        coordinator: Arc::new(SystemCoordinator::new()),
        text: "Процедура Тест()\n    x = ЭтотОбъект;\nКонецПроцедуры\n".to_string(),
        version: 0,
        abs_path: "InformationRegisters/Регистр1/Ext/RecordSetModule.bsl".to_string(),
        position: Position {
            line: 1,
            character: 10,
        },
    })
    .expect("type_at_position");

    let type_info = response.type_info.expect("type info");
    assert!(
        response.warnings.is_empty(),
        "warnings: {:?}",
        response.warnings
    );
    assert!(
        type_info.name.contains("Регистр1"),
        "type name: {}",
        type_info.name
    );
    assert_eq!(type_info.active_facet.as_deref(), Some("Object"));
    assert_eq!(
        type_info.available_facets,
        vec!["Manager".to_string(), "Object".to_string()]
    );
}

#[test]
fn semantic_helpers_fail_closed_without_precomputed_type_index() {
    use bsl_analysis_v2::{DepsSnapshotId, SemanticDeps};
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::SignatureIndex;
    use bsl_shared::domain::types::{FacetKind, MetadataKind, RawDataSource, RawTypeData};

    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.Док1".to_string(),
            source: RawDataSource::Configuration,
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            ..Default::default()
        }])
        .expect("load types");

    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });

    let content = concat!(
        "Процедура Тест()\n",
        "    x = ЭтотОбъект;\n",
        "    ЭтотОбъект.\n",
        "КонецПроцедуры\n"
    );
    let mut host = bsl_analysis_v2::AnalysisHostV2::default();
    host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
        deps_id: DepsSnapshotId::from_hash("mcp-helper-fail-closed"),
        deps,
    });
    host.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
        settings_id: SettingsId::from_hash("mcp-helper-fail-closed"),
        diagnostics_detail_level: DetailLevel::Full,
    });
    host.apply_change(bsl_analysis_v2::Change::SetFile {
        file_id: bsl_analysis_v2::FileId(1),
        text: Arc::from(content.to_string()),
        version: 1,
        path: Arc::from("Documents/Док1/Ext/ObjectModule.bsl"),
    });

    let analysis = host.analysis();
    assert!(
        type_at_utf16_position(&analysis, bsl_analysis_v2::FileId(1), 1, 10, false, None).is_none(),
        "type-at-position helper must fail closed without exact type_index artifact"
    );

    let member_column = "    ЭтотОбъект"
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>() as u32;
    assert!(
        member_access_owner_type_hint_at_position(
            &analysis,
            bsl_analysis_v2::FileId(1),
            content,
            2,
            member_column,
            false,
            None,
        )
        .is_none(),
        "member-access helper must fail closed without exact type_index artifact"
    );
}

#[test]
fn collect_members_uses_exact_owner_hint_on_default_path() {
    use bsl_analysis_v2::{DepsSnapshotId, SemanticDeps};
    use bsl_runtime::system::{IndexSnapshot, IndexSnapshotId, SystemCoordinator};
    use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use bsl_shared::domain::resolver::TypeResolver;
    use bsl_shared::domain::signature_index::SignatureIndex;
    use bsl_shared::domain::types::{
        FacetKind, MetadataKind, RawDataSource, RawMethodData, RawPropertyData, RawTypeData,
    };

    const EXACT_HIT_REASON_KEY: &str =
        "intellisense_v2_type_index_reason_total_reason_type_index_exact_hit";

    let repository_impl = Arc::new(InMemoryTypeRepository::new());
    repository_impl
        .load_types(vec![RawTypeData {
            name: "Документы.Док1".to_string(),
            source: RawDataSource::Configuration,
            methods: vec![RawMethodData {
                name: "ПометитьУдаление".to_string(),
                return_type: "Неопределено".to_string(),
                ..Default::default()
            }],
            properties: vec![RawPropertyData {
                name: "Ссылка".to_string(),
                prop_type: "ДокументСсылка.Док1".to_string(),
                ..Default::default()
            }],
            facets: vec![FacetKind::Manager, FacetKind::Object, FacetKind::Reference],
            kind: Some(MetadataKind::Document),
            ..Default::default()
        }])
        .expect("load types");

    let repository = repository_impl.clone() as Arc<dyn TypeRepository>;
    let resolver = Arc::new(TypeResolver::new(repository.clone()));
    let deps = Arc::new(SemanticDeps {
        repository,
        signature_index: SignatureIndex::new(),
        resolver: Some(resolver),
        platform_signatures_loaded: true,
    });
    let coordinator = Arc::new(SystemCoordinator::new());
    let runtime = tokio::runtime::Runtime::new().expect("runtime");

    let response = collect_members(MembersRequest {
        analysis_revision: 1,
        flow_sensitive_enabled: false,
        deps_id: DepsSnapshotId::from_hash("mcp-members-fail-closed-no-owner-hint"),
        deps,
        index_snapshot: Arc::new(IndexSnapshot::empty(IndexSnapshotId::from_hash(
            "mcp-members-fail-closed-no-owner-hint",
        ))),
        coordinator: coordinator.clone(),
        text: concat!(
            "Процедура Тест()\n",
            "    x = ЭтотОбъект;\n",
            "    ЭтотОбъект.\n",
            "КонецПроцедуры\n"
        )
        .to_string(),
        version: 1,
        abs_path: "Documents/Док1/Ext/ObjectModule.bsl".to_string(),
        position: Position {
            line: 2,
            character: "    ЭтотОбъект."
                .chars()
                .map(|ch| ch.len_utf16())
                .sum::<usize>() as u32,
        },
        limit: 50,
        completion_runtime: runtime.handle().clone(),
    })
    .expect("collect_members");

    assert!(
        response
            .members
            .iter()
            .any(|member| member.name == "ПометитьУдаление")
            && response
                .members
                .iter()
                .any(|member| member.name == "Ссылка"),
        "MCP members must preserve expected object-facet members on default path: {:?}",
        response.members
    );
    assert_eq!(response.analysis_revision, 1);
    assert!(
        counter_value(&coordinator.observability_metrics(), EXACT_HIT_REASON_KEY) > 0,
        "MCP members default path must emit shared exact type-index reason via owner-hint lookup, metrics={}",
        coordinator.observability_metrics()
    );
}

#[tokio::test]
async fn definition_without_target_does_not_emit_bounded_public_reason_metric() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();
    let file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id,
            path: "Documents/Док1/Ext/ObjectModule.bsl".to_string(),
        }),
        text: Some(
            concat!(
                "Процедура Тест()\n",
                "    ЭтотОбъект.Несуществующий();\n",
                "КонецПроцедуры\n"
            )
            .to_string(),
        ),
        version: Some(1),
    };

    let metric_key =
        "intellisense_v2_fail_closed_reason_total_origin_agent_operation_definition_reason_missing_semantic_index";
    let baseline_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability baseline");
    let baseline_total = counter_value(&baseline_metrics.metrics, metric_key);

    let response = manager
        .bsl_definition(BslDefinitionParams {
            session_id: session_id.clone(),
            symbol_id: None,
            file: Some(file),
            position: Some(Position {
                line: 1,
                character: 16,
            }),
        })
        .await
        .expect("definition");
    assert!(
        response.location.is_none(),
        "definition without target must stay empty"
    );

    let after_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability after type_at_position");
    let after_total = counter_value(&after_metrics.metrics, metric_key);
    assert!(
        after_total == baseline_total,
        "agent definition without target must not emit bounded public fail-closed reason metric: before={baseline_total}, after={after_total}, metrics={}",
        after_metrics.metrics
    );
}

#[tokio::test]
async fn definition_resolves_object_module_member_definition_via_shared_exact_type_index() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let config_root = temp.path();
    let module_rel_path = "Documents/Док1/Ext/ObjectModule.bsl";
    let module_path = config_root.join(module_rel_path);
    let module_code = concat!(
        "Процедура МойМетод() Экспорт\n",
        "КонецПроцедуры\n",
        "\n",
        "Процедура Тест()\n",
        "    ЭтотОбъект.МойМетод();\n",
        "КонецПроцедуры\n"
    );

    std::fs::write(
        config_root.join("Configuration.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Configuration uuid="00000000-0000-0000-0000-000000000000">
    <Properties>
      <Name>TestConfig</Name>
      <CompatibilityMode>Version8_3_25</CompatibilityMode>
    </Properties>
    <ChildObjects>
      <Document>Док1</Document>
    </ChildObjects>
  </Configuration>
</MetaDataObject>
"#,
    )
    .expect("write Configuration.xml");
    std::fs::create_dir_all(module_path.parent().expect("module parent"))
        .expect("mkdir object module");
    std::fs::write(
        config_root.join("Documents/Док1.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<MetaDataObject xmlns="http://v8.1c.ru/8.3/MDClasses">
  <Document uuid="00000000-0000-0000-0000-000000000001">
    <Properties>
      <Name>Док1</Name>
    </Properties>
  </Document>
</MetaDataObject>
"#,
    )
    .expect("write document xml");
    std::fs::write(&module_path, module_code).expect("write object module");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![config_root.to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: Some("8.3.25".to_string()),
                configuration_path: Some(config_root.to_string_lossy().to_string()),
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let call_line = module_code.lines().nth(4).expect("call line");
    let method_byte = call_line.find("МойМетод").expect("method byte");
    let method_column = call_line[..method_byte]
        .chars()
        .map(|ch| ch.len_utf16())
        .sum::<usize>()
        .min(u32::MAX as usize) as u32;

    let before_metrics = manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("observability before definition");
    let before_total = type_index_reason_total(&before_metrics.metrics);

    let definition = manager
        .bsl_definition(BslDefinitionParams {
            session_id: open.session_id.clone(),
            symbol_id: None,
            file: Some(FileRef {
                doc: DocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            }),
            position: Some(Position {
                line: 4,
                character: method_column,
            }),
        })
        .await
        .expect("definition");

    let location = definition.location.expect("definition location");
    assert_eq!(location.file.path, module_rel_path);
    assert_eq!(location.range.start.line, 0);

    let after_metrics = manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("observability after definition");
    let after_total = type_index_reason_total(&after_metrics.metrics);
    assert!(
        after_total > before_total,
        "definition must emit shared type-index serve reasons on the default MCP path: before={before_total}, after={after_total}, metrics={}",
        after_metrics.metrics
    );
}

async fn wait_startup(job_manager: &JobManager, open: &WorkspaceOpenResponse) {
    let job_id = open
        .startup_job_id
        .as_deref()
        .expect("startup_job_id missing");
    loop {
        let status = job_manager.wait(job_id, 60_000).await.expect("job_wait");
        match status.state {
            JobStateDto::Succeeded => break,
            JobStateDto::Queued | JobStateDto::Running => continue,
            other => panic!("startup job ended unexpectedly: {}", other.as_str()),
        }
    }
}

#[tokio::test]
async fn observability_metrics_rejects_not_ready_session_deterministically() {
    let manager = SessionManager::new();
    let session_uuid = uuid::Uuid::new_v4();
    let session_id = session_uuid.to_string();
    let temp = tempfile::TempDir::new().expect("tempdir");

    {
        let mut sessions = manager.sessions.write().await;
        sessions.insert(
            session_uuid,
            WorkspaceSession {
                roots: vec![RootEntry {
                    root_id: "root".to_string(),
                    path: temp.path().to_path_buf(),
                }],
                documents: DocumentStore::default(),
                analysis_revision: 0,
                settings: WorkspaceSettings {
                    platform_docs_archive: None,
                    platform_version: None,
                    configuration_path: None,
                    mode: None,
                    env_overrides: HashMap::new(),
                    dev_env_overrides: HashMap::new(),
                    allow_dev_overrides: false,
                },
                startup: None,
                startup_job_id: None,
                startup_phase: "startup/starting".to_string(),
                startup_progress: 0,
                startup_error: None,
                created_at: crate::state::now_unix_secs(),
                id_map: IdMap::default(),
                pack_store: PackStore::default(),
            },
        );
    }

    let err = manager
        .observability_metrics_get(&session_id)
        .await
        .expect_err("workspace_get_observability_metrics must reject not-ready session");
    assert_eq!(err.code.0, rmcp::model::ErrorCode::INVALID_PARAMS.0);
    assert!(
        err.message.contains("workspace not ready"),
        "unexpected error message: {}",
        err.message
    );
}

#[tokio::test]
async fn observability_metrics_exposes_unified_stage_contract_for_ready_session() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(
        &module_path,
        "Процедура Тест()\n    ЛокМассив = Новый Массив;\n    ЛокМассив.\nКонецПроцедуры\n",
    )
    .expect("write module");
    let manager = Arc::new(SessionManager::new());
    let job_manager = Arc::new(JobManager::new());

    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(&job_manager, &open).await;

    let _diagnostics = manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: open.session_id.clone(),
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Project),
            limit: 200,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: false,
        })
        .await
        .expect("bsl_diagnostics");

    let _members = manager
        .bsl_members(BslMembersParams {
            session_id: open.session_id.clone(),
            file: FileRef {
                doc: DocumentRef::Path(module_path.to_string_lossy().to_string()),
                text: None,
                version: None,
            },
            position: Position {
                line: 2,
                character: 13,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("bsl_members");

    let session_uuid = Uuid::parse_str(&open.session_id).expect("session uuid");
    let coordinator = {
        let sessions = manager.sessions.read().await;
        sessions
            .get(&session_uuid)
            .and_then(|session| session.startup.as_ref())
            .expect("ready startup")
            .coordinator
            .clone()
    };

    let metrics = manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("workspace_get_observability_metrics");
    assert_eq!(metrics.metrics, coordinator.observability_metrics());
    assert_unified_intellisense_v2_stage_contract(&metrics.metrics);
}

#[tokio::test]
async fn bsl_members_does_not_execute_parse_result_query_on_semantic_path() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();
    let overlay_file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id,
            path: "src/CommonModules/Foo/Module.bsl".to_string(),
        }),
        text: Some(
            "Procedure Test()\n    arr = Новый Массив;\n    arr.Добавить(1);\n    arr.\nEndProcedure\n"
                .to_string(),
        ),
        version: Some(1),
    };
    manager
        .documents_set(
            &session_id,
            &[WorkspaceDocumentsSetFile::File(overlay_file.clone())],
            true,
        )
        .await
        .expect("documents_set");

    let baseline_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability before members");
    let baseline_parse_result_queries = counter_value(
        &baseline_metrics.metrics,
        "intellisense_v2_parse_result_query_total",
    );

    let members = manager
        .bsl_members(BslMembersParams {
            session_id: session_id.clone(),
            file: FileRef {
                doc: overlay_file.doc.clone(),
                text: None,
                version: None,
            },
            position: Position {
                line: 3,
                character: 6,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("bsl_members");
    assert!(
        !members.truncated,
        "members query must not truncate response"
    );

    let after_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability after members");
    let after_parse_result_queries = counter_value(
        &after_metrics.metrics,
        "intellisense_v2_parse_result_query_total",
    );
    assert_eq!(
        after_parse_result_queries, baseline_parse_result_queries,
        "semantic MCP members path must not execute parse_result query; before={baseline_parse_result_queries}, after={after_parse_result_queries}, metrics={}",
        after_metrics.metrics
    );
}

#[tokio::test]
async fn documents_set_and_clear_bump_revision_only_on_change() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    assert_eq!(open.analysis_revision, 0);

    let root_id = open.roots[0].root_id.clone();
    let file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id: root_id.clone(),
            path: "src/CommonModules/Foo/Module.bsl".to_string(),
        }),
        text: Some("x".to_string()),
        version: Some(1),
    };

    let set = manager
        .documents_set(
            &open.session_id,
            &[WorkspaceDocumentsSetFile::File(file.clone())],
            true,
        )
        .await
        .expect("set");
    assert_eq!(set.analysis_revision, 1);

    let set_again = manager
        .documents_set(
            &open.session_id,
            &[WorkspaceDocumentsSetFile::File(file)],
            true,
        )
        .await
        .expect("set again");
    assert_eq!(set_again.analysis_revision, 1);

    let clear = manager
        .documents_clear(
            &open.session_id,
            &[DocumentRef::Canonical(CanonicalDocumentRef {
                root_id,
                path: "src/CommonModules/Foo/Module.bsl".to_string(),
            })],
            true,
        )
        .await
        .expect("clear");
    assert_eq!(clear.analysis_revision, 2);
}

#[tokio::test]
async fn context_pack_and_expand_are_deterministic_and_budgeted() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();

    let overlay_file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id: root_id.clone(),
            path: "src/CommonModules/Foo/Module.bsl".to_string(),
        }),
        text: Some("Procedure Test()\n    A = 1;\nEndProcedure\n".to_string()),
        version: Some(1),
    };
    manager
        .documents_set(
            &session_id,
            &[WorkspaceDocumentsSetFile::File(overlay_file.clone())],
            true,
        )
        .await
        .expect("documents_set");

    let focus_file = FileRef {
        doc: overlay_file.doc.clone(),
        text: None,
        version: None,
    };

    let params = ContextPackParams {
        session_id: session_id.clone(),
        goal: Some("Test pack".to_string()),
        focus: Some(ContextFocus::Position {
            file: focus_file,
            position: Position {
                line: 1,
                character: 4,
            },
        }),
        scope: Some(WorkspaceScope::Simple("hot".to_string())),
        budget_chars: Some(900),
        budget_tokens: None,
        include: ContextInclude::default(),
    };

    let pack1 = manager.context_pack(params.clone()).await.expect("pack1");
    let pack2 = manager.context_pack(params).await.expect("pack2");

    assert_eq!(pack1.analysis_revision, pack2.analysis_revision);
    assert_eq!(pack1.pack_id, pack2.pack_id);
    assert_eq!(pack1.text, pack2.text);
    assert_eq!(pack1.items.len(), pack2.items.len());
    for (left, right) in pack1.items.iter().zip(pack2.items.iter()) {
        assert_eq!(left.kind, right.kind);
        assert_eq!(left.summary, right.summary);
        assert_eq!(left.file, right.file);
        assert_eq!(left.range, right.range);
        assert_eq!(left.item_id, right.item_id);
    }

    assert!(pack1.text.chars().count() <= 900);

    let mut snapshot_value = serde_json::to_value(&pack1).expect("json");
    if let Some(pack_id) = snapshot_value.get_mut("pack_id") {
        *pack_id = serde_json::Value::String("<pack_id>".to_string());
    }
    if let Some(items) = snapshot_value
        .get_mut("items")
        .and_then(|v| v.as_array_mut())
    {
        for item in items {
            if let Some(item_id) = item.get_mut("item_id") {
                *item_id = serde_json::Value::String("<item_id>".to_string());
            }
            if let Some(file) = item.get_mut("file").and_then(|v| v.as_object_mut()) {
                if let Some(root) = file.get_mut("root_id") {
                    *root = serde_json::Value::String("<root_id>".to_string());
                }
            }
        }
    }

    insta::assert_json_snapshot!("context_pack_position", snapshot_value);

    let item_id = pack1.items[0].item_id.clone();
    let expand_params = ContextExpandParams {
        session_id,
        pack_id: pack1.pack_id,
        item_id,
        budget_chars: Some(400),
        budget_tokens: None,
    };
    let expand1 = manager
        .context_expand(expand_params.clone())
        .await
        .expect("expand1");
    let expand2 = manager
        .context_expand(expand_params)
        .await
        .expect("expand2");

    assert_eq!(expand1.analysis_revision, expand2.analysis_revision);
    assert_eq!(expand1.text, expand2.text);
    assert!(expand1.text.chars().count() <= 400);

    insta::assert_json_snapshot!(
        "context_expand_snippet",
        serde_json::to_value(expand1).expect("json")
    );
}

#[tokio::test]
async fn flow_sensitive_flags_are_explicit_in_mcp_responses() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();

    let overlay_file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id: root_id.clone(),
            path: "src/CommonModules/Foo/Module.bsl".to_string(),
        }),
        text: Some(
            "Procedure Test()\n    x = Null;\n    x.Добавить(1);\n    x.\nEndProcedure\n"
                .to_string(),
        ),
        version: Some(1),
    };
    manager
        .documents_set(
            &session_id,
            &[WorkspaceDocumentsSetFile::File(overlay_file.clone())],
            true,
        )
        .await
        .expect("documents_set");

    // Diagnostics: null-safety only when enabled.
    let diags_base = manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: session_id.clone(),
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Hot),
            limit: 500,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: false,
        })
        .await
        .expect("diagnostics base");
    assert!(!diags_base.flow_sensitive_enabled);

    let diags_flow = manager
        .bsl_diagnostics(BslDiagnosticsParams {
            session_id: session_id.clone(),
            scope: WorkspaceScope::Tagged(WorkspaceScopeTagged::Hot),
            limit: 500,
            include_impact: false,
            include_coverage: false,
            include_flow_sensitive: true,
        })
        .await
        .expect("diagnostics flow");
    assert!(diags_flow.flow_sensitive_enabled);

    // Type-at-position: flag is explicit even when narrowing might not apply.
    let file = FileRef {
        doc: overlay_file.doc.clone(),
        text: None,
        version: None,
    };
    let type_base = manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: session_id.clone(),
            file: file.clone(),
            position: Position {
                line: 3,
                character: 6,
            },
            include_flow_sensitive: false,
        })
        .await
        .expect("type_at_position base");
    assert!(!type_base.flow_sensitive_enabled);

    let type_flow = manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: session_id.clone(),
            file: file.clone(),
            position: Position {
                line: 3,
                character: 6,
            },
            include_flow_sensitive: true,
        })
        .await
        .expect("type_at_position flow");
    assert!(type_flow.flow_sensitive_enabled);

    // Members: flag is explicit.
    let members_base = manager
        .bsl_members(BslMembersParams {
            session_id: session_id.clone(),
            file: file.clone(),
            position: Position {
                line: 3,
                character: 6,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("members base");
    assert!(!members_base.flow_sensitive_enabled);

    let members_flow = manager
        .bsl_members(BslMembersParams {
            session_id,
            file,
            position: Position {
                line: 3,
                character: 6,
            },
            limit: 50,
            include_flow_sensitive: true,
        })
        .await
        .expect("members flow");
    assert!(members_flow.flow_sensitive_enabled);
}

#[tokio::test]
async fn type_at_position_and_members_emit_interactive_runtime_exec_metrics() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();
    let overlay_file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id,
            path: "src/CommonModules/Foo/Module.bsl".to_string(),
        }),
        text: Some(
            "Procedure Test()\n    arr = Новый Массив;\n    arr.Добавить(1);\n    arr.\nEndProcedure\n"
                .to_string(),
        ),
        version: Some(1),
    };
    manager
        .documents_set(
            &session_id,
            &[WorkspaceDocumentsSetFile::File(overlay_file.clone())],
            true,
        )
        .await
        .expect("documents_set");

    let file = FileRef {
        doc: overlay_file.doc.clone(),
        text: None,
        version: None,
    };
    let metric_key = "intellisense_v2_runtime_exec_interactive_total";

    let baseline_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability baseline");
    let baseline_exec_total = counter_value(&baseline_metrics.metrics, metric_key);

    let type_response = manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: session_id.clone(),
            file: file.clone(),
            position: Position {
                line: 3,
                character: 6,
            },
            include_flow_sensitive: false,
        })
        .await
        .expect("type_at_position");
    assert!(
        type_response.warnings.is_empty(),
        "type_at_position should not emit warnings: {:?}",
        type_response.warnings
    );

    let after_type_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability after type_at_position");
    let after_type_exec_total = counter_value(&after_type_metrics.metrics, metric_key);
    assert!(
        after_type_exec_total > baseline_exec_total,
        "type_at_position should increment {metric_key}: before={baseline_exec_total}, after={after_type_exec_total}, metrics={}",
        after_type_metrics.metrics
    );

    let members_response = manager
        .bsl_members(BslMembersParams {
            session_id,
            file,
            position: Position {
                line: 3,
                character: 6,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("members");
    assert!(!members_response.truncated);

    let after_members_metrics = manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("observability after members");
    let after_members_exec_total = counter_value(&after_members_metrics.metrics, metric_key);
    assert!(
        after_members_exec_total > after_type_exec_total,
        "members should increment {metric_key}: before={after_type_exec_total}, after={after_members_exec_total}, metrics={}",
        after_members_metrics.metrics
    );
}

#[tokio::test]
async fn type_at_position_members_and_definition_emit_shared_type_index_reason_metrics() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();
    let overlay_file = FileRef {
        doc: DocumentRef::Canonical(CanonicalDocumentRef {
            root_id,
            path: "src/CommonModules/Foo/Module.bsl".to_string(),
        }),
        text: Some(
            "Procedure Foo()\nEndProcedure\nProcedure Test()\n    arr = Новый Массив;\n    Foo();\n    arr.\nEndProcedure\n"
                .to_string(),
        ),
        version: Some(1),
    };
    manager
        .documents_set(
            &session_id,
            &[WorkspaceDocumentsSetFile::File(overlay_file.clone())],
            true,
        )
        .await
        .expect("documents_set");

    let file = FileRef {
        doc: overlay_file.doc.clone(),
        text: None,
        version: None,
    };

    let baseline_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability baseline");
    let baseline_total = type_index_reason_total(&baseline_metrics.metrics);

    let type_response = manager
        .bsl_type_at_position(BslTypeAtPositionParams {
            session_id: session_id.clone(),
            file: file.clone(),
            position: Position {
                line: 5,
                character: 6,
            },
            include_flow_sensitive: false,
        })
        .await
        .expect("type_at_position");
    assert!(
        type_response.warnings.is_empty(),
        "type_at_position warnings: {:?}",
        type_response.warnings
    );

    let after_type_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability after type_at_position");
    let after_type_total = type_index_reason_total(&after_type_metrics.metrics);
    assert!(
        after_type_total > baseline_total,
        "type_at_position must emit type-index reasons: before={baseline_total}, after={after_type_total}, metrics={}",
        after_type_metrics.metrics
    );

    let members = manager
        .bsl_members(BslMembersParams {
            session_id: session_id.clone(),
            file: file.clone(),
            position: Position {
                line: 5,
                character: 8,
            },
            limit: 50,
            include_flow_sensitive: false,
        })
        .await
        .expect("members");
    assert!(!members.truncated, "members query must stay complete");

    let after_members_metrics = manager
        .observability_metrics_get(&session_id)
        .await
        .expect("observability after members");
    let after_members_total = type_index_reason_total(&after_members_metrics.metrics);
    assert_eq!(
        after_members_total, after_type_total,
        "members must reuse current semantic state without extra type-index reasons: before={after_type_total}, after={after_members_total}, metrics={}",
        after_members_metrics.metrics
    );

    let definition = manager
        .bsl_definition(BslDefinitionParams {
            session_id,
            symbol_id: None,
            file: Some(file),
            position: Some(Position {
                line: 4,
                character: 4,
            }),
        })
        .await
        .expect("definition");
    assert!(
        definition.location.is_some(),
        "definition should resolve Foo()"
    );

    let after_definition_metrics = manager
        .observability_metrics_get(&open.session_id)
        .await
        .expect("observability after definition");
    let after_definition_total = type_index_reason_total(&after_definition_metrics.metrics);
    assert!(
        after_definition_total > after_members_total,
        "definition must emit at least one shared type-index serve reason on the default MCP path: before={after_members_total}, after={after_definition_total}, metrics={}",
        after_definition_metrics.metrics
    );
}

#[tokio::test]
async fn symbol_search_and_references_work_via_bounded_blocking_workers() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let module_path = temp.path().join("Module.bsl");
    std::fs::write(
            &module_path,
            "Процедура ТестBoundedBlocking()\nКонецПроцедуры\nПроцедура Вызов()\n    ТестBoundedBlocking();\nКонецПроцедуры\n",
        )
        .expect("write module");

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let search = manager
        .bsl_symbol_search(BslSymbolSearchParams {
            session_id: open.session_id.clone(),
            query: "ТестBoundedBlocking".to_string(),
            limit: 20,
        })
        .await
        .expect("symbol_search");
    assert!(
        !search.symbols.is_empty(),
        "expected non-empty symbol search results"
    );
    let symbol_id = search.symbols[0].symbol_id.clone();

    let refs = manager
        .bsl_references(BslReferencesParams {
            session_id: open.session_id.clone(),
            symbol_id,
            limit: 50,
            include_snippets: false,
        })
        .await
        .expect("references");
    assert!(
        refs.count > 0,
        "expected at least one reference in the source module"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn batch_symbol_search_does_not_starve_members_query() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let src_dir = temp.path().join("src/CommonModules/Foo");
    std::fs::create_dir_all(&src_dir).expect("mkdir src");
    let module_path = src_dir.join("Module.bsl");
    std::fs::write(
            &module_path,
            "Процедура ТестStarvation()\nКонецПроцедуры\nПроцедура Вызов()\n    ТестStarvation();\nКонецПроцедуры\n",
        )
        .expect("write module");
    for idx in 0..32 {
        std::fs::write(
                temp.path().join(format!("Batch{idx}.bsl")),
                format!(
                    "Процедура ТестStarvation{idx}()\nКонецПроцедуры\nПроцедура Вызов{idx}()\n    ТестStarvation{}();\nКонецПроцедуры\n",
                    idx
                ),
            )
            .expect("write batch module");
    }

    let job_manager = Arc::new(JobManager::new());
    let manager = Arc::new(SessionManager::new());
    let open = manager
        .open(
            WorkspaceOpenParams {
                roots: vec![temp.path().to_string_lossy().to_string()],
                platform_docs_archive: None,
                platform_version: None,
                configuration_path: None,
                mode: None,
            },
            Arc::clone(&job_manager),
        )
        .await
        .expect("open");
    wait_startup(job_manager.as_ref(), &open).await;

    let session_id = open.session_id.clone();
    let root_id = open.roots[0].root_id.clone();
    let overlay_file = FileRef {
            doc: DocumentRef::Canonical(CanonicalDocumentRef {
                root_id,
                path: "src/CommonModules/Foo/Module.bsl".to_string(),
            }),
            text: Some(
                "Процедура ТестStarvation()\nКонецПроцедуры\nПроцедура Вызов()\n    ТестStarvation();\nКонецПроцедуры\n"
                    .to_string(),
            ),
            version: Some(1),
        };
    manager
        .documents_set(
            &session_id,
            &[WorkspaceDocumentsSetFile::File(overlay_file.clone())],
            true,
        )
        .await
        .expect("documents_set");

    let mut batch_handles = Vec::new();
    for _ in 0..6 {
        let manager_clone = manager.clone();
        let session_clone = session_id.clone();
        batch_handles.push(tokio::spawn(async move {
            manager_clone
                .bsl_symbol_search(BslSymbolSearchParams {
                    session_id: session_clone,
                    query: "ТестStarvation".to_string(),
                    limit: 200,
                })
                .await
        }));
    }

    let members = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        manager.bsl_members(BslMembersParams {
            session_id,
            file: overlay_file,
            position: Position {
                line: 3,
                character: 10,
            },
            limit: 50,
            include_flow_sensitive: false,
        }),
    )
    .await
    .expect("members query should finish under batch symbol-search load")
    .expect("members");
    assert!(
        !members.truncated,
        "members query should complete without truncation while batch workers are running"
    );

    for handle in batch_handles {
        let search = handle
            .await
            .expect("batch symbol_search join")
            .expect("batch symbol_search");
        assert!(
            !search.symbols.is_empty(),
            "batch symbol_search must still return results"
        );
    }
}

#[test]
fn infer_platform_version_from_config_dump_uses_max_compatibility_mode() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    std::fs::write(
        temp.path().join("Configuration.xml"),
        r#"<Configuration uuid="00000000-0000-0000-0000-000000000000">
  <Properties>
    <Name>Base</Name>
    <CompatibilityMode>Version8_3_24</CompatibilityMode>
  </Properties>
</Configuration>
"#,
    )
    .expect("write Configuration.xml");

    let ext_dir = temp.path().join("Ext");
    std::fs::create_dir_all(&ext_dir).expect("mkdir ext");
    std::fs::write(
        ext_dir.join("Configuration.xml"),
        r#"<Configuration uuid="00000000-0000-0000-0000-000000000001">
  <Properties>
    <Name>Ext</Name>
    <ObjectBelonging>Adopted</ObjectBelonging>
    <ConfigurationExtensionCompatibilityMode>Version8_3_25</ConfigurationExtensionCompatibilityMode>
  </Properties>
</Configuration>
"#,
    )
    .expect("write ext Configuration.xml");

    let inferred =
        infer_platform_version_from_config_dump(temp.path()).expect("infer platform_version");
    assert_eq!(inferred, "8.3.25");
}

#[test]
fn infer_platform_version_from_config_dump_failure_mentions_platform_version() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    let err = infer_platform_version_from_config_dump(temp.path()).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("platform_version"));
    assert!(message.contains("provide platform_version"));
}

#[test]
fn infer_platform_version_from_config_dump_missing_compatibility_mode_mentions_platform_version() {
    let temp = tempfile::TempDir::new().expect("tempdir");

    std::fs::write(
        temp.path().join("Configuration.xml"),
        r#"<Configuration uuid="00000000-0000-0000-0000-000000000000">
  <Properties>
    <Name>Base</Name>
  </Properties>
</Configuration>
"#,
    )
    .expect("write Configuration.xml");

    let err = infer_platform_version_from_config_dump(temp.path()).unwrap_err();
    let message = err.to_string();
    assert!(message.contains("platform_version"));
    assert!(message.contains("provide platform_version"));
}

#[test]
fn document_ref_absolute_path_resolves_root_by_longest_prefix() {
    let root = tempfile::TempDir::new().expect("root");
    let ext = tempfile::TempDir::new_in(root.path()).expect("ext");

    let (roots, _dtos) = restore_roots(&[
        root.path().to_string_lossy().to_string(),
        ext.path().to_string_lossy().to_string(),
    ])
    .expect("restore_roots");

    let abs = ext
        .path()
        .join("src/CommonModules/Foo/Module.bsl")
        .to_string_lossy()
        .to_string();
    let key =
        document_key_from_ref(&roots, &DocumentRef::Path(abs)).expect("document_key_from_ref");

    assert_eq!(key.root_id, roots[1].root_id);
    assert_eq!(key.path, "src/CommonModules/Foo/Module.bsl");
}
