const REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS: u64 = 1_000;
const REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS: u64 = 1_000;

fn p56_duration_ms_u64(duration: std::time::Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn p56_percentile_ms(values: &[u64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = ((sorted.len().saturating_sub(1)) as f64 * percentile).ceil() as usize;
    sorted[rank.min(sorted.len() - 1)] as f64
}

fn p56_cycle_u64(cycle: &serde_json::Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| cycle.get(*key).and_then(|value| value.as_u64()))
}

fn p56_cycle_str<'a>(cycle: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| cycle.get(*key).and_then(|value| value.as_str()))
}

fn p56_ready_install_wait_u64(cycle: &serde_json::Value, key: &str) -> Option<u64> {
    cycle
        .pointer("/background_parse_task_state_after_timeout/ready_install_exact_type_index_wait")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_u64())
}

fn p56_ready_install_wait_str<'a>(cycle: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    cycle
        .pointer("/background_parse_task_state_after_timeout/ready_install_exact_type_index_wait")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
}

fn p56_pure_did_change_ready_install_wait_u64(
    cycle: &serde_json::Value,
    key: &str,
) -> Option<u64> {
    cycle
        .pointer("/pure_did_change_ready_install_exact_type_index_wait")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_u64())
}

fn p56_pure_did_change_ready_install_wait_str<'a>(
    cycle: &'a serde_json::Value,
    key: &str,
) -> Option<&'a str> {
    cycle
        .pointer("/pure_did_change_ready_install_exact_type_index_wait")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
}

fn p56_ready_install_terminal_str<'a>(
    cycle: &'a serde_json::Value,
    key: &str,
) -> Option<&'a str> {
    cycle
        .get("ready_install_exact_type_index_wait_terminal")
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
}

fn p56_cycle_has_contract_approved_ready_install_resolution(cycle: &serde_json::Value) -> bool {
    match p56_ready_install_terminal_str(cycle, "terminal") {
        Some("canonical_ready_snapshot_materialized") => true,
        Some("classified_blocker") => {
            p56_ready_install_terminal_str(cycle, "snapshot_failure_reason")
                == Some("exact_type_index_deadline_before_ready_install")
        }
        _ => false,
    }
}

fn p56_ready_install_wait_snapshot_json(
    snapshot: &crate::server::ReadyInstallExactTypeIndexWaitSnapshotV2,
) -> serde_json::Value {
    serde_json::json!({
        "active": snapshot.active,
        "elapsed_ms": snapshot.elapsed_ms,
        "ceiling_ms": snapshot.ceiling_ms,
        "outcome": snapshot.outcome,
        "waiter_action": snapshot.waiter_action,
        "matching_task_state": snapshot.matching_task_state,
        "task_phase": snapshot.task_phase,
        "task_requested_version": snapshot.task_requested_version,
        "task_active_requested_version": snapshot.task_active_requested_version,
        "observed_file_version": snapshot.observed_file_version,
        "exact_ready": snapshot.exact_ready,
        "ready_snapshot_version": snapshot.ready_snapshot_version,
        "parse_snapshot_incremental": snapshot.parse_snapshot_incremental,
        "parse_snapshot_changed_ranges_count": snapshot.parse_snapshot_changed_ranges_count,
        "parse_snapshot_serve_only_blocked": snapshot.parse_snapshot_serve_only_blocked,
        "blocker_class": snapshot.blocker_class,
    })
}

async fn p56_pure_did_change_ready_install_wait_snapshot_json(
    server: &crate::server::BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    requested_version: i32,
    text_hash: [u8; 32],
) -> Option<serde_json::Value> {
    let tasks = server.background_parse_snapshot_apply_tasks_v2.lock().await;
    tasks.get(&file_id).and_then(|task| {
        let target = task
            .target
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if target.requested_version != requested_version
            || target.text_hash != text_hash
            || target.save_cycle_sequence.is_some()
            || !matches!(
                target.source,
                crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange
            )
        {
            return None;
        }
        Some(p56_ready_install_wait_snapshot_json(
            &task.control.ready_install_exact_type_index_wait_snapshot(),
        ))
    })
}

fn p56_background_parse_snapshot_source_label(
    source: crate::server::BackgroundParseSnapshotApplyTaskSourceV2,
) -> &'static str {
    match source {
        crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidOpen => "did_open",
        crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidChange => "did_change",
        crate::server::BackgroundParseSnapshotApplyTaskSourceV2::DidSave => "did_save",
    }
}

fn p56_ready_parse_snapshot_worker_termination_counts(
    counters: &serde_json::Map<String, serde_json::Value>,
    source: &str,
) -> (serde_json::Map<String, serde_json::Value>, u64) {
    let mut reason_counts = serde_json::Map::new();
    let mut total = 0;
    for reason in [
        "aborted",
        "superseded",
        "retargeted_before_parse",
        "retargeted_during_parse",
        "retargeted_before_materialization",
        "retargeted_before_exact_ready_install",
        "superseded_before_exact_ready_install",
        "latest_version_mismatch",
        "latest_version_mismatch_before_exact_ready_install",
        "exact_type_index_deadline_before_ready_install",
        "build_snapshot_aborted",
        "other",
    ] {
        let key = format!(
            "intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_{source}_reason_{reason}"
        );
        let count = read_u64_metric(counters.get(&key));
        total += count;
        reason_counts.insert(reason.to_string(), serde_json::json!(count));
    }
    (reason_counts, total)
}

fn p56_program_lowering_reuse_outcome(cycle: &serde_json::Value) -> Option<&str> {
    p56_cycle_str(
        cycle,
        &[
            "program_lowering_reuse_outcome",
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome",
        ],
    )
}

fn p56_program_lowering_reused_lowering_units(cycle: &serde_json::Value) -> Option<u64> {
    p56_cycle_u64(
        cycle,
        &[
            "program_lowering_reused_lowering_units",
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units",
        ],
    )
}

fn p56_program_lowering_rebuilt_lowering_units(cycle: &serde_json::Value) -> Option<u64> {
    p56_cycle_u64(
        cycle,
        &[
            "program_lowering_rebuilt_lowering_units",
            "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units",
        ],
    )
}

fn p56_cycle_has_slow_first_publish(cycle: &serde_json::Value) -> bool {
    p56_cycle_u64(cycle, &["first_publish_elapsed_ms"])
        .is_some_and(|value| value > REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS)
        || p56_cycle_u64(cycle, &["first_publish_syntax_query_ms"])
            .is_some_and(|value| value > REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS)
}

fn p56_cycle_has_late_detached_program_lowering_full_rebuild(cycle: &serde_json::Value) -> bool {
    p56_cycle_str(cycle, &["followup_semantic_path"]) == Some("detached_ready_artifacts")
        && p56_cycle_str(cycle, &["followup_ready_snapshot_timeout_phase"]) == Some("parse_exec")
        && p56_cycle_str(cycle, &["followup_ready_snapshot_timeout_leaf"])
            == Some("program_lowering")
        && (p56_cycle_str(cycle, &["followup_ready_snapshot_wait_probe"]) == Some("timeout")
            || p56_cycle_str(cycle, &["followup_ready_snapshot_bounded_wait_winner"])
                == Some("timeout")
            || p56_cycle_str(cycle, &["followup_ready_snapshot_relief_valve_outcome"])
                == Some("engaged_timed_out"))
        && p56_program_lowering_reuse_outcome(cycle) == Some("full_rebuild")
        && p56_program_lowering_reused_lowering_units(cycle) == Some(0)
        && p56_program_lowering_rebuilt_lowering_units(cycle).is_some_and(|units| units > 0)
        && matches!(
            p56_cycle_str(
                cycle,
                &[
                    "followup_did_save_exact_producer_final_lifecycle_state",
                    "followup_did_save_exact_producer_lifecycle_state",
                ],
            ),
            Some("detached_diagnostics_ready_published" | "fully_materialized")
        )
}

#[test]
fn p56_refactor54_gate_predicates_reject_incident_contours() {
    let slow_first_publish = serde_json::json!({
        "followup_semantic_path": "detached_ready_artifacts",
        "followup_ready_snapshot_wait_probe": "not_ready",
        "followup_ready_snapshot_bounded_wait_winner": "detached_ready_artifacts",
        "first_publish_elapsed_ms": 3397,
        "first_publish_syntax_query_ms": 3397,
        "program_lowering_reuse_outcome": "routine_body_reuse",
        "program_lowering_reused_lowering_units": 2079,
        "program_lowering_rebuilt_lowering_units": 9,
        "followup_did_save_exact_producer_final_lifecycle_state": "detached_diagnostics_ready_published",
    });
    assert!(p56_cycle_has_slow_first_publish(&slow_first_publish));
    assert!(!p56_cycle_has_late_detached_program_lowering_full_rebuild(
        &slow_first_publish
    ));

    let late_full_rebuild = serde_json::json!({
        "followup_semantic_path": "detached_ready_artifacts",
        "followup_ready_snapshot_wait_probe": "timeout",
        "followup_ready_snapshot_bounded_wait_winner": "timeout",
        "followup_ready_snapshot_relief_valve_outcome": "engaged_timed_out",
        "followup_ready_snapshot_timeout_phase": "parse_exec",
        "followup_ready_snapshot_timeout_leaf": "program_lowering",
        "first_publish_elapsed_ms": 55,
        "first_publish_syntax_query_ms": 55,
        "program_lowering_reuse_outcome": "full_rebuild",
        "program_lowering_reused_lowering_units": 0,
        "program_lowering_rebuilt_lowering_units": 2088,
        "followup_did_save_exact_producer_final_lifecycle_state": "detached_diagnostics_ready_published",
    });
    assert!(!p56_cycle_has_slow_first_publish(&late_full_rebuild));
    assert!(p56_cycle_has_late_detached_program_lowering_full_rebuild(
        &late_full_rebuild
    ));

    let raw_timeline_keys_late_full_rebuild = serde_json::json!({
        "followup_semantic_path": "detached_ready_artifacts",
        "followup_ready_snapshot_wait_probe": "timeout",
        "followup_ready_snapshot_relief_valve_outcome": "engaged_timed_out",
        "followup_ready_snapshot_timeout_phase": "parse_exec",
        "followup_ready_snapshot_timeout_leaf": "program_lowering",
        "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome": "full_rebuild",
        "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units": 0,
        "followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units": 2088,
        "followup_did_save_exact_producer_lifecycle_state": "fully_materialized",
    });
    assert!(p56_cycle_has_late_detached_program_lowering_full_rebuild(
        &raw_timeline_keys_late_full_rebuild
    ));

    let healthy_current_contour = serde_json::json!({
        "followup_semantic_path": "detached_ready_artifacts",
        "followup_ready_snapshot_wait_probe": "not_ready",
        "followup_ready_snapshot_bounded_wait_winner": "detached_ready_artifacts",
        "first_publish_elapsed_ms": 208,
        "first_publish_syntax_query_ms": 83,
        "program_lowering_reuse_outcome": "routine_body_reuse",
        "program_lowering_reused_lowering_units": 2079,
        "program_lowering_rebuilt_lowering_units": 9,
        "followup_did_save_exact_producer_final_lifecycle_state": "detached_diagnostics_ready_published",
    });
    assert!(!p56_cycle_has_slow_first_publish(&healthy_current_contour));
    assert!(!p56_cycle_has_late_detached_program_lowering_full_rebuild(
        &healthy_current_contour
    ));
}

#[test]
fn p56_real_conf_big_diagnostics_representative_save_followup_bundle_live() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("p56 tokio runtime");
    runtime.block_on(async {
        init_test_tracing();
        struct EnvVarGuard {
            key: &'static str,
            previous: Option<String>,
            reload_runtime_config: bool,
        }

        impl EnvVarGuard {
            fn set(key: &'static str, value: &str) -> Self {
                Self::set_with_reload(key, value, false)
            }

            fn set_with_reload(
                key: &'static str,
                value: &str,
                reload_runtime_config: bool,
            ) -> Self {
                let previous = std::env::var(key).ok();
                std::env::set_var(key, value);
                if reload_runtime_config {
                    bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
                }
                Self {
                    key,
                    previous,
                    reload_runtime_config,
                }
            }
        }

        impl Drop for EnvVarGuard {
            fn drop(&mut self) {
                if let Some(previous) = &self.previous {
                    std::env::set_var(self.key, previous);
                } else {
                    std::env::remove_var(self.key);
                }
                if self.reload_runtime_config {
                    bsl_runtime::system::global_runtime_config().reload_env_bootstrap_from_env();
                }
            }
        }

        fn utf16_range_for_substring(source: &str, needle: &str) -> Range {
            let start_byte = source
                .find(needle)
                .unwrap_or_else(|| panic!("needle not found: {needle}"));
            let end_byte = start_byte + needle.len();
            let start = &source[..start_byte];
            let end = &source[..end_byte];
            let start_line = start.lines().count().saturating_sub(1) as u32;
            let start_character = start
                .lines()
                .last()
                .unwrap_or("")
                .chars()
                .map(|ch| ch.len_utf16())
                .sum::<usize>() as u32;
            let end_line = end.lines().count().saturating_sub(1) as u32;
            let end_character = end
                .lines()
                .last()
                .unwrap_or("")
                .chars()
                .map(|ch| ch.len_utf16())
                .sum::<usize>() as u32;
            Range {
                start: Position::new(start_line, start_character),
                end: Position::new(end_line, end_character),
            }
        }

        fn trace_u64(trace: &serde_json::Value, field: &str) -> u64 {
            trace
                .get(field)
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        }

        fn trace_cycle_selection_key(trace: &serde_json::Value) -> (u64, u64, u64) {
            (
                trace_u64(trace, "save_cycle_sequence"),
                trace_u64(trace, "diagnostics_generation"),
                trace_u64(trace, "started_at_ms"),
            )
        }

        const PROFILE_NAME: &str =
            "p56_real_conf_big_diagnostics_representative_save_followup_bundle_live";
        const TIMELINE_OBSERVE_BUDGET_MS: u64 = 90_000;
        const SAVE_CYCLE_COUNT: usize = 4;
        const READY_SNAPSHOT_MATERIALIZATION_TIMEOUT_SECS: u64 = 180;
        const BASELINE_CAPTURED_AT: &str = "2026-04-18T18:52:50Z";
        const BASELINE_DETACHED_READY_ARTIFACTS_COUNT: u64 = 4;
        const BASELINE_READY_ARTIFACTS_COUNT: u64 = 0;
        const BASELINE_SHADOW_STATE_COUNT: u64 = 0;
        const BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MIN_MS: u64 = 5_052;
        const BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS: u64 = 5_219;
        const BASELINE_READY_SNAPSHOT_PARSE_EXEC_MIN_MS: u64 = 3_222;
        const BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS: u64 = 3_327;
        const BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS: f64 = 3_226.0;
        const BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS: f64 = 3_329.0;
        const V1_STATEMENT: &str = "СтруктураВозврата = Новый Структура;";

        let _env_lock = lock_test_env().await;
        let _apply_delay_guard = EnvVarGuard::set("BSL_TEST_RUNTIME_APPLY_SET_FILE_DELAY_MS", "0");
        let _did_change_blocking_parse_delay_guard =
            EnvVarGuard::set("BSL_TEST_DID_CHANGE_BLOCKING_PARSE_DELAY_MS", "0");
        let _debounce_guard =
            EnvVarGuard::set_with_reload("BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS", "1200", true);

        let allow_fixture_skip = std::env::var_os("BSL_TEST_ALLOW_MISSING_CONF_BIG").is_some();
        let change_id = std::env::var("CHANGE_ID").unwrap_or_else(|_| {
            "refactor-49-save-followup-same-version-ready-snapshot-rebuild-bounding".to_string()
        });

        let Some(conf_big_root) = conf_big_root_for_tests() else {
            if allow_fixture_skip {
                eprintln!(
                    "skipping {PROFILE_NAME}: examples/conf_big fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set"
                );
                return;
            }
            panic!(
                "examples/conf_big fixture is missing; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly"
            );
        };

        let module_path = conf_big_large_module_path_for_tests(&conf_big_root);
        if !module_path.exists() {
            if allow_fixture_skip {
                eprintln!(
                    "skipping {PROFILE_NAME}: module fixture is missing and BSL_TEST_ALLOW_MISSING_CONF_BIG is set: {}",
                    module_path.display()
                );
                return;
            }
            panic!(
                "conf_big module fixture is missing: {}; set BSL_TEST_ALLOW_MISSING_CONF_BIG=1 to skip explicitly",
                module_path.display()
            );
        }

        let module_text = std::fs::read_to_string(&module_path)
            .expect("read conf_big module text for representative bundle");
        let workspace_setup = ScaleAwareWorkspaceSetup {
            platform_docs_archive: syntax_helper_path_for_tests(),
            configuration_path: conf_big_root.clone(),
            platform_version: "8.3.25".to_string(),
        };
        let coordinator = Arc::new(SystemCoordinator::new());
        let (mut harness, server) = spawn_live_lsp_transport_harness(coordinator.clone()).await;
        initialize_live_lsp_transport(&mut harness).await;
        prime_server_with_workspace_setup(
            &server,
            &workspace_setup,
            "p56_real_conf_big_representative_bundle_setup",
        )
        .await;

        let uri = Url::from_file_path(&module_path).expect("real conf_big module uri for p56");
        harness
            .send_notification(
                "textDocument/didOpen",
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "bsl".to_string(),
                        version: 1,
                        text: module_text.clone(),
                    },
                },
            )
            .await;

        let file_id = server.get_or_create_file_id_v2(&uri).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .get(&file_id)
                    .copied()
                    == Some(1)
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("didOpen must register version 1 for p56");
        tokio::time::timeout(Duration::from_secs(180), async {
            loop {
                let ready = server
                    .latest_ready_parse_snapshots_v2
                    .read()
                    .await
                    .get(&file_id)
                    .cloned();
                if ready
                    .as_ref()
                    .is_some_and(|state| state.parse_snapshot.file_version == 1)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("didOpen must materialize same-version ready parse snapshot for p56");

        let mut current_text = module_text.clone();
        let mut current_version = 1_i32;
        let mut current_statement = V1_STATEMENT.to_string();
        let mut cycles = Vec::with_capacity(SAVE_CYCLE_COUNT);

        for cycle_index in 0..SAVE_CYCLE_COUNT {
            let cycle_number = cycle_index + 1;
            let stage1_version = current_version
                .checked_add(1)
                .expect("p56 stage1 version overflow");
            let stage2_version = current_version
                .checked_add(2)
                .expect("p56 stage2 version overflow");
            let stage1_statement = format!(
                "СтруктураВозврата = НеобъявленнаяПеременнаяP56Cycle{cycle_number}Stage1;"
            );
            let stage2_statement = format!(
                "СтруктураВозврата = НеобъявленнаяПеременнаяP56Cycle{cycle_number}Stage2;"
            );

            let stage1_range = utf16_range_for_substring(&current_text, &current_statement);
            let stage1_text = current_text.replacen(&current_statement, &stage1_statement, 1);
            assert_ne!(
                stage1_text, current_text,
                "p56 cycle {cycle_number} must update the current statement on stage1"
            );
            live_transport_ranged_did_change(
                &mut harness,
                &uri,
                stage1_version,
                vec![TextDocumentContentChangeEvent {
                    range: Some(stage1_range),
                    range_length: None,
                    text: stage1_statement.clone(),
                }],
            )
            .await;
            current_text = stage1_text;
            let stage1_text_hash = *blake3::hash(current_text.as_bytes()).as_bytes();

            let mut pure_did_change_ready_install_exact_type_index_wait = serde_json::Value::Null;
            let pure_did_change_materialization_started = Instant::now();
            tokio::time::timeout(
                Duration::from_secs(READY_SNAPSHOT_MATERIALIZATION_TIMEOUT_SECS),
                async {
                loop {
                    if let Some(snapshot) =
                        p56_pure_did_change_ready_install_wait_snapshot_json(
                            &server,
                            file_id,
                            stage1_version,
                            stage1_text_hash,
                        )
                        .await
                    {
                        pure_did_change_ready_install_exact_type_index_wait = snapshot;
                    }
                    let ready = server
                        .latest_ready_parse_snapshots_v2
                        .read()
                        .await
                        .get(&file_id)
                        .cloned();
                    if ready
                        .as_ref()
                        .is_some_and(|state| state.parse_snapshot.file_version == stage1_version)
                    {
                        break;
                    }
                    let failure = server
                        .latest_snapshot_failures_v2
                        .read()
                        .await
                        .get(&file_id)
                        .filter(|state| state.requested_version == stage1_version)
                        .map(|state| state.reason.as_ref().to_string());
                    if let Some(reason) = failure {
                        panic!(
                            "p56 cycle {cycle_number} stage1 pure didChange classified before materialization: reason={reason}, ready_install_wait={pure_did_change_ready_install_exact_type_index_wait:?}"
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            },
            )
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "p56 cycle {cycle_number} must materialize same-version ready snapshot for stage1"
                )
            });
            if pure_did_change_ready_install_exact_type_index_wait.is_null() {
                pure_did_change_ready_install_exact_type_index_wait =
                    p56_pure_did_change_ready_install_wait_snapshot_json(
                        &server,
                        file_id,
                        stage1_version,
                        stage1_text_hash,
                    )
                    .await
                    .unwrap_or(serde_json::Value::Null);
            }
            let pure_did_change_materialization_elapsed_ms =
                p56_duration_ms_u64(pure_did_change_materialization_started.elapsed());

            let stage2_range = utf16_range_for_substring(&current_text, &stage1_statement);
            let stage2_text = current_text.replacen(&stage1_statement, &stage2_statement, 1);
            assert_ne!(
                stage2_text, current_text,
                "p56 cycle {cycle_number} must update the current statement on stage2"
            );
            live_transport_ranged_did_change(
                &mut harness,
                &uri,
                stage2_version,
                vec![TextDocumentContentChangeEvent {
                    range: Some(stage2_range),
                    range_length: None,
                    text: stage2_statement.clone(),
                }],
            )
            .await;
            current_text = stage2_text;
            let stage2_text_hash = *blake3::hash(current_text.as_bytes()).as_bytes();

            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let received_version = server
                        .latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied();
                    let shadow_version = server
                        .latest_document_shadow_state_v2
                        .read()
                        .await
                        .get(&file_id)
                        .map(|state| state.version);
                    if received_version == Some(stage2_version) && shadow_version == Some(stage2_version)
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "p56 cycle {cycle_number} must advance latest received version and shadow state to stage2"
                )
            });

            live_transport_save_document(&mut harness, &uri).await;

            let timeline_deadline = Instant::now() + Duration::from_millis(TIMELINE_OBSERVE_BUDGET_MS);
            let timeline = loop {
                let timeline =
                    live_transport_get_diagnostics_save_timeline(
                        &mut harness,
                        56_100_900 + cycle_index as i64,
                        16,
                    )
                    .await;
                let traces = timeline
                    .get("traces")
                    .and_then(|value| value.as_array())
                    .expect("diagnostics save timeline traces for p56");
                let matching_trace = traces
                    .iter()
                    .filter(|trace| {
                        trace.get("uri").and_then(|value| value.as_str()) == Some(uri.as_str())
                            && trace
                                .get("requested_version")
                                .and_then(|value| value.as_i64())
                                == Some(stage2_version as i64)
                    })
                    .max_by_key(|trace| trace_cycle_selection_key(trace))
                    .cloned();
                let Some(trace) = matching_trace else {
                    if Instant::now() >= timeline_deadline {
                        panic!(
                            "p56 cycle {cycle_number} must expose a diagnostics save trace for requested_version={stage2_version}"
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                };
                let followup_publish = trace
                    .get("followup_publish")
                    .and_then(|value| value.as_object());
                let followup_semantic_path = trace
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str());
                if followup_publish.is_some() || followup_semantic_path == Some("shadow_state") {
                    break trace;
                }
                if Instant::now() >= timeline_deadline {
                    panic!(
                        "p56 cycle {cycle_number} must observe a bounded follow-up semantic-path decision, last_trace={trace:?}"
                    );
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };

            let followup_publish = timeline
                .get("followup_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str()) == Some("idle_heavy")
                });
            let first_publish = timeline
                .get("first_publish")
                .and_then(|value| value.as_object())
                .filter(|publish| {
                    publish.get("profile").and_then(|value| value.as_str())
                        == Some("save_fastlane")
                });
            let followup_semantic_path = followup_publish
                .and_then(|publish| publish.get("semantic_path").and_then(|value| value.as_str()))
                .or_else(|| {
                    timeline
                        .get("followup_semantic_path")
                        .and_then(|value| value.as_str())
                });

            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_task_state")
                    .and_then(|value| value.as_str()),
                Some("in_flight_same_version"),
                "p56 representative cycle must stay on same-version in-flight producer, trace={timeline:?}"
            );
            assert_eq!(
                timeline
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str()),
                Some("not_ready"),
                "p56 representative cycle must reach save follow-up before the exact producer materializes, trace={timeline:?}"
            );
            let followup_ready_snapshot_wait_probe = timeline
                .get("followup_ready_snapshot_wait_probe")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_bounded_wait_winner = timeline
                .get("followup_ready_snapshot_bounded_wait_winner")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_bounded_wait_elapsed_ms = timeline
                .get("followup_ready_snapshot_bounded_wait_elapsed_ms")
                .and_then(|value| value.as_u64());
            let followup_ready_snapshot_continuation_reason = timeline
                .get("followup_ready_snapshot_continuation_reason")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_relief_valve_outcome = timeline
                .get("followup_ready_snapshot_relief_valve_outcome")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_timeout_phase = timeline
                .get("followup_ready_snapshot_timeout_phase")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_timeout_leaf = timeline
                .get("followup_ready_snapshot_timeout_leaf")
                .and_then(|value| value.as_str());
            let followup_ready_snapshot_timeout_leaf_elapsed_ms = timeline
                .get("followup_ready_snapshot_timeout_leaf_elapsed_ms")
                .and_then(|value| value.as_u64());
            let followup_wait_reason = timeline
                .get("followup_wait_reason")
                .and_then(|value| value.as_str());
            let followup_did_save_exact_producer_lifecycle_state = timeline
                .get("followup_did_save_exact_producer_lifecycle_state")
                .and_then(|value| value.as_str());
            let followup_did_save_exact_producer_lifecycle_state_at_timeout = timeline
                .get("followup_did_save_exact_producer_lifecycle_state_at_timeout")
                .and_then(|value| value.as_str());
            let followup_did_save_exact_producer_final_lifecycle_state = timeline
                .get("followup_did_save_exact_producer_final_lifecycle_state")
                .and_then(|value| value.as_str());
            let followup_save_fastlane_gate_outcome = timeline
                .get("followup_save_fastlane_gate_outcome")
                .and_then(|value| value.as_str());
            let followup_save_fastlane_gate_wait_ms = timeline
                .get("followup_save_fastlane_gate_wait_ms")
                .and_then(|value| value.as_u64());
            let followup_admission_queue_wait_ms = timeline
                .get("followup_admission_queue_wait_ms")
                .and_then(|value| value.as_u64());
            let analysis_after_timeout = server.analysis_v2.snapshot().await;
            let exact_ready_after_timeout = analysis_after_timeout
                .current_type_index_serve_only_ready(file_id)
                .expect("current_type_index_serve_only_ready after p56 timeout");
            let completion_head_ready_after_timeout = analysis_after_timeout
                .current_completion_head_ready(file_id)
                .expect("current_completion_head_ready after p56 timeout");
            let type_index_parse_snapshot_meta_after_timeout = analysis_after_timeout
                .current_type_index_parse_snapshot_meta(file_id)
                .expect("current_type_index_parse_snapshot_meta after p56 timeout");
            let observed_version_after_timeout = analysis_after_timeout
                .file_version(file_id)
                .expect("file_version after p56 timeout");
            let ready_snapshot_state_after_timeout = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| {
                    (
                        state.parse_snapshot.file_version,
                        format!("{:?}", state.source),
                        state.syntax_errors_complete,
                    )
                });
            let type_index_task_state_after_timeout = {
                let tasks = server.type_index_precompute_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    (
                        task.supersession_key.requested_version,
                        task.work_class,
                        crate::server::core::deps_and_precompute::TypeIndexPrecomputePhaseV2::from_atomic(
                            task.phase.load(Ordering::Relaxed),
                        ),
                        task.active_requested_version.load(Ordering::Relaxed),
                        task.handle.is_finished(),
                    )
                })
            };
            let current_revision_head_precompute_task_state_after_timeout = {
                let tasks = server.current_revision_head_precompute_tasks_v2.lock().await;
                tasks.get(&file_id).map(|task| {
                    (
                        task.requested_version.load(Ordering::Relaxed),
                        task.handle.is_finished(),
                    )
                })
            };
            let background_parse_task_state_after_timeout = server
                .background_parse_snapshot_apply_tasks_v2
                .lock()
                .await
                .get(&file_id)
                .and_then(|task| {
                    let target = task
                        .target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    if target.requested_version != stage2_version
                        || target.text_hash != stage2_text_hash
                    {
                        return None;
                    }
                    let ready_install_exact_type_index_wait =
                        task.control.ready_install_exact_type_index_wait_snapshot();
                    Some((
                        crate::server::BackgroundParseSnapshotApplyTaskPhaseV2::from_raw(
                            task.control.phase.load(Ordering::SeqCst),
                        ),
                        task.control.promotion_requested.load(Ordering::SeqCst),
                        task.control.materialized.load(Ordering::SeqCst),
                        ready_install_exact_type_index_wait,
                        p56_background_parse_snapshot_source_label(target.source),
                        target.save_cycle_sequence,
                        target.epoch,
                    ))
                });
            let observability_metrics_after_timeout =
                live_transport_get_observability_metrics(&mut harness, 56_100_949 + cycle_index as i64)
                    .await;
            let observability_histograms_after_timeout = observability_metrics_after_timeout
                .get("histograms")
                .and_then(|value| value.as_object())
                .expect("observability metrics.histograms after p56 timeout");
            let observability_counters_after_timeout = observability_metrics_after_timeout
                .get("counters")
                .and_then(|value| value.as_object())
                .expect("observability metrics.counters after p56 timeout");
            let type_index_precompute_exec_histogram_after_timeout = histogram_metric_value_or_zero(
                observability_histograms_after_timeout,
                "intellisense_v2_runtime_type_index_precompute_exec_ms",
                None,
            );
            let type_index_precompute_ir_exec_histogram_after_timeout =
                histogram_metric_value_or_zero(
                    observability_histograms_after_timeout,
                    "intellisense_v2_runtime_type_index_precompute_ir_exec_ms",
                    None,
                );
            let type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout =
                histogram_metric_value_or_zero(
                    observability_histograms_after_timeout,
                    "intellisense_v2_runtime_type_index_precompute_semantic_facts_local_function_summaries_exec_ms",
                    None,
                );
            let ir_singleflight_wait_histogram_after_timeout = histogram_metric_value_or_zero(
                observability_histograms_after_timeout,
                "intellisense_v2_singleflight_wait_ms",
                None,
            );
            let ir_singleflight_counters_after_timeout = serde_json::json!({
                "leader_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_leader_query_kind_ir"
                )),
                "shared_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_shared_query_kind_ir"
                )),
                "key_unavailable_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_drilldown_singleflight_effectiveness_total_origin_lsp_outcome_key_unavailable_query_kind_ir"
                )),
            });
            let type_index_counters_after_timeout = serde_json::json!({
                "exact_stored_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_type_index_reason_total_reason_type_index_precompute_exact_stored"
                )),
                "superseded_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_type_index_reason_total_reason_type_index_precompute_superseded"
                )),
                "cancelled_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_type_index_reason_total_reason_type_index_precompute_cancelled"
                )),
                "exact_wait_ready_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_ready"
                )),
                "exact_wait_deadline_total": read_u64_metric(observability_counters_after_timeout.get(
                    "intellisense_v2_completion_exact_type_index_wait_outcome_total_reason_deadline"
                )),
            });
            let ready_artifacts_publish = match followup_semantic_path {
                Some("detached_ready_artifacts") => {
                    assert!(
                        matches!(
                            followup_did_save_exact_producer_lifecycle_state,
                            Some(
                                "admitted"
                                    | "detached_diagnostics_ready_published"
                                    | "fully_materialized"
                            )
                        ),
                        "p56 detached cycle must expose producer lifecycle for the detached-ready path, trace={timeline:?}"
                    );
                    assert_eq!(
                        followup_ready_snapshot_wait_probe,
                        Some("not_ready"),
                        "p56 detached cycle must keep the canonical bounded wait probe in not_ready while the detached winner is still-current, trace={timeline:?}"
                    );
                    assert_eq!(
                        followup_ready_snapshot_bounded_wait_winner,
                        Some("detached_ready_artifacts"),
                        "p56 detached cycle must attribute bounded wait completion to detached_ready_artifacts, trace={timeline:?}"
                    );
                    assert!(
                        followup_ready_snapshot_bounded_wait_elapsed_ms
                            .is_some(),
                        "p56 detached cycle must export bounded wait elapsed attribution even when the detached wakeup rounds down to 0ms, trace={timeline:?}"
                    );
                    assert!(
                        followup_ready_snapshot_timeout_leaf.is_none(),
                        "p56 detached bounded-wait winner must not fabricate a timeout leaf, trace={timeline:?}"
                    );
                    let publish = followup_publish.expect(
                        "p56 detached path must expose an idle_heavy follow-up publish object",
                    );
                    assert_eq!(
                        publish
                            .get("semantic_parse_source")
                            .and_then(|value| value.as_str()),
                        Some("snapshot")
                    );
                    assert_eq!(
                        publish
                            .get("semantic_ir_source")
                            .and_then(|value| value.as_str()),
                        Some("snapshot_build")
                    );
                    assert_eq!(
                        followup_ready_snapshot_continuation_reason,
                        None,
                        "p56 detached cycle must stay on the primary canonical wait path without synthetic continuation, trace={timeline:?}"
                    );
                    Some(publish)
                }
                Some("ready_artifacts") => panic!(
                    "p56 representative bundle must now surface the still-current late path through detached_ready_artifacts, trace={timeline:?}, observed_version_after_timeout={observed_version_after_timeout:?}, exact_ready_after_timeout={exact_ready_after_timeout}, completion_head_ready_after_timeout={completion_head_ready_after_timeout}, type_index_parse_snapshot_meta_after_timeout={type_index_parse_snapshot_meta_after_timeout:?}, ready_snapshot_state_after_timeout={ready_snapshot_state_after_timeout:?}, type_index_task_state_after_timeout={type_index_task_state_after_timeout:?}, current_revision_head_precompute_task_state_after_timeout={current_revision_head_precompute_task_state_after_timeout:?}, background_parse_task_state_after_timeout={background_parse_task_state_after_timeout:?}, type_index_precompute_exec_histogram_after_timeout={type_index_precompute_exec_histogram_after_timeout:?}, type_index_precompute_ir_exec_histogram_after_timeout={type_index_precompute_ir_exec_histogram_after_timeout:?}, type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout={type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout:?}, ir_singleflight_wait_histogram_after_timeout={ir_singleflight_wait_histogram_after_timeout:?}, ir_singleflight_counters_after_timeout={ir_singleflight_counters_after_timeout:?}, type_index_counters_after_timeout={type_index_counters_after_timeout:?}"
                ),
                Some("shadow_state") => panic!(
                    "p56 representative bundle must fail the inherited refactor-50 contour: still-current same-version save reached shadow_state before detached-ready publication, trace={timeline:?}, observed_version_after_timeout={observed_version_after_timeout:?}, exact_ready_after_timeout={exact_ready_after_timeout}, completion_head_ready_after_timeout={completion_head_ready_after_timeout}, type_index_parse_snapshot_meta_after_timeout={type_index_parse_snapshot_meta_after_timeout:?}, ready_snapshot_state_after_timeout={ready_snapshot_state_after_timeout:?}, type_index_task_state_after_timeout={type_index_task_state_after_timeout:?}, current_revision_head_precompute_task_state_after_timeout={current_revision_head_precompute_task_state_after_timeout:?}, background_parse_task_state_after_timeout={background_parse_task_state_after_timeout:?}, type_index_precompute_exec_histogram_after_timeout={type_index_precompute_exec_histogram_after_timeout:?}, type_index_precompute_ir_exec_histogram_after_timeout={type_index_precompute_ir_exec_histogram_after_timeout:?}, type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout={type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout:?}, ir_singleflight_wait_histogram_after_timeout={ir_singleflight_wait_histogram_after_timeout:?}, ir_singleflight_counters_after_timeout={ir_singleflight_counters_after_timeout:?}, type_index_counters_after_timeout={type_index_counters_after_timeout:?}"
                ),
                _ => panic!(
                    "p56 representative bundle must resolve each cycle to detached_ready_artifacts, trace={timeline:?}, observed_version_after_timeout={observed_version_after_timeout:?}, exact_ready_after_timeout={exact_ready_after_timeout}, completion_head_ready_after_timeout={completion_head_ready_after_timeout:?}, type_index_parse_snapshot_meta_after_timeout={type_index_parse_snapshot_meta_after_timeout:?}, ready_snapshot_state_after_timeout={ready_snapshot_state_after_timeout:?}, type_index_task_state_after_timeout={type_index_task_state_after_timeout:?}, current_revision_head_precompute_task_state_after_timeout={current_revision_head_precompute_task_state_after_timeout:?}, background_parse_task_state_after_timeout={background_parse_task_state_after_timeout:?}, type_index_precompute_exec_histogram_after_timeout={type_index_precompute_exec_histogram_after_timeout:?}, type_index_precompute_ir_exec_histogram_after_timeout={type_index_precompute_ir_exec_histogram_after_timeout:?}, type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout={type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout:?}, ir_singleflight_wait_histogram_after_timeout={ir_singleflight_wait_histogram_after_timeout:?}, ir_singleflight_counters_after_timeout={ir_singleflight_counters_after_timeout:?}, type_index_counters_after_timeout={type_index_counters_after_timeout:?}"
                ),
            };
            let followup_ready_snapshot_parse_exec_ms = timeline
                .get("followup_ready_snapshot_parse_exec_ms")
                .and_then(|value| value.as_u64());
            let first_publish_elapsed_ms = first_publish
                .and_then(|publish| publish.get("elapsed_ms").and_then(|value| value.as_u64()));
            let first_publish_syntax_query_ms = first_publish.and_then(|publish| {
                publish
                    .get("syntax_diagnostics_query_ms")
                    .and_then(|value| value.as_u64())
            });
            let first_publish_runtime_queue_wait_ms = first_publish.and_then(|publish| {
                publish
                    .get("runtime_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            });
            let first_publish_publish_wait_ms = first_publish.and_then(|publish| {
                publish
                    .get("publish_wait_ms")
                    .and_then(|value| value.as_u64())
            });
            let followup_runtime_queue_wait_ms = timeline
                .get("followup_runtime_queue_wait_ms")
                .and_then(|value| value.as_u64());
            let followup_apply_lag_ms = timeline
                .get("followup_apply_lag_ms")
                .and_then(|value| value.as_u64());
            let followup_wait_for_file_version_ms = timeline
                .get("followup_wait_for_file_version_ms")
                .and_then(|value| value.as_u64());
            let followup_snapshot_with_deps_ms = timeline
                .get("followup_snapshot_with_deps_ms")
                .and_then(|value| value.as_u64());
            let followup_publish_elapsed_ms = ready_artifacts_publish.and_then(|publish| {
                publish.get("elapsed_ms").and_then(|value| value.as_u64())
            });
            let followup_publish_runtime_queue_wait_ms = ready_artifacts_publish.and_then(|publish| {
                publish
                    .get("runtime_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            });
            let followup_publish_blocking_queue_wait_ms = ready_artifacts_publish.and_then(|publish| {
                publish
                    .get("blocking_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            });
            let followup_publish_wait_for_file_version_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("wait_for_file_version_ms")
                        .and_then(|value| value.as_u64())
                });
            let followup_publish_snapshot_with_deps_ms = ready_artifacts_publish.and_then(|publish| {
                publish
                    .get("snapshot_with_deps_ms")
                    .and_then(|value| value.as_u64())
            });
            let followup_publish_publish_wait_ms = ready_artifacts_publish.and_then(|publish| {
                publish
                    .get("publish_wait_ms")
                    .and_then(|value| value.as_u64())
            });
            let followup_publish_semantic_diagnostics_query_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("semantic_diagnostics_query_ms")
                        .and_then(|value| value.as_u64())
                });
            let semantic_query_dominates_parse_exec = followup_publish_semantic_diagnostics_query_ms
                .zip(followup_ready_snapshot_parse_exec_ms)
                .map(|(query_ms, parse_exec_ms)| query_ms > parse_exec_ms);
            let followup_publish_semantic_diagnostics_ir_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("semantic_diagnostics_ir_ms")
                        .and_then(|value| value.as_u64())
                });
            let followup_publish_semantic_diagnostics_collect_ms =
                ready_artifacts_publish.and_then(|publish| {
                    publish
                        .get("semantic_diagnostics_collect_ms")
                        .and_then(|value| value.as_u64())
                });
            let followup_publish_non_query_residual_ms = followup_publish_elapsed_ms
                .zip(followup_publish_semantic_diagnostics_query_ms)
                .map(|(publish_ms, query_ms)| publish_ms.saturating_sub(query_ms));
            let followup_ready_snapshot_program_lowering_ms = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms")
                .and_then(|value| value.as_u64());
            let program_lowering_reuse_outcome = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome")
                .and_then(|value| value.as_str());
            let program_lowering_reused_lowering_units = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units")
                .and_then(|value| value.as_u64());
            let program_lowering_rebuilt_lowering_units = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units")
                .and_then(|value| value.as_u64());
            let program_lowering_reused_window_count = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count")
                .and_then(|value| value.as_u64());
            let program_lowering_rebuilt_window_count = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count")
                .and_then(|value| value.as_u64());
            let program_lowering_reuse_plan_build_source = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source")
                .and_then(|value| value.as_str());
            let program_lowering_reuse_plan_take_if_unique_hit = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit")
                .and_then(|value| value.as_bool());
            let program_lowering_reuse_plan_borrowed_cache_hit = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit")
                .and_then(|value| value.as_bool());
            let program_lowering_reuse_plan_build_ms = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_ms")
                .and_then(|value| value.as_u64());
            let program_lowering_reused_progress_ms = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_progress_ms")
                .and_then(|value| value.as_u64());
            let program_lowering_rebuild_dispatch_ms = timeline
                .get("followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuild_dispatch_ms")
                .and_then(|value| value.as_u64());
            if ready_artifacts_publish.is_some() {
                assert!(
                    followup_publish_elapsed_ms.is_some_and(|value| value > 0),
                    "p56 cycle {cycle_number} must expose non-zero followup publish latency on the detached path, trace={timeline:?}"
                );
                assert!(
                    followup_publish_semantic_diagnostics_query_ms
                        .is_some_and(|value| value > 0),
                    "p56 cycle {cycle_number} must expose non-zero semantic_diagnostics_query_ms on the detached path, trace={timeline:?}"
                );
                assert_eq!(
                    semantic_query_dominates_parse_exec,
                    Some(true),
                    "p56 cycle {cycle_number} must prove that semantic_diagnostics_query now dominates ready-snapshot parse_exec, trace={timeline:?}, followup_ready_snapshot_program_lowering_ms={followup_ready_snapshot_program_lowering_ms:?}, program_lowering_reuse_outcome={program_lowering_reuse_outcome:?}, program_lowering_reused_lowering_units={program_lowering_reused_lowering_units:?}, program_lowering_rebuilt_lowering_units={program_lowering_rebuilt_lowering_units:?}, program_lowering_reused_window_count={program_lowering_reused_window_count:?}, program_lowering_rebuilt_window_count={program_lowering_rebuilt_window_count:?}, program_lowering_reuse_plan_build_source={program_lowering_reuse_plan_build_source:?}, program_lowering_reuse_plan_take_if_unique_hit={program_lowering_reuse_plan_take_if_unique_hit:?}, program_lowering_reuse_plan_borrowed_cache_hit={program_lowering_reuse_plan_borrowed_cache_hit:?}, program_lowering_reuse_plan_build_ms={program_lowering_reuse_plan_build_ms:?}, program_lowering_reused_progress_ms={program_lowering_reused_progress_ms:?}, program_lowering_rebuild_dispatch_ms={program_lowering_rebuild_dispatch_ms:?}"
                );
            }

            let mut cycle_summary = serde_json::json!({
                "cycle": cycle_number,
                "stage1_version": stage1_version,
                "requested_version": stage2_version,
                "save_cycle_sequence": timeline
                    .get("save_cycle_sequence")
                    .and_then(|value| value.as_u64()),
                "followup_semantic_path": followup_semantic_path,
                "followup_publish_semantic_path": ready_artifacts_publish
                    .and_then(|publish| publish.get("semantic_path").and_then(|value| value.as_str())),
                "followup_profile_phase_marks": timeline
                    .get("followup_profile_phase_marks")
                    .and_then(|value| value.as_array())
                    .cloned()
                    .unwrap_or_default(),
                "followup_ready_snapshot_task_state": timeline
                    .get("followup_ready_snapshot_task_state")
                    .and_then(|value| value.as_str()),
                "followup_ready_snapshot_zero_probe": timeline
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str()),
                "followup_ready_snapshot_wait_probe": followup_ready_snapshot_wait_probe,
                "followup_ready_snapshot_bounded_wait_winner": followup_ready_snapshot_bounded_wait_winner,
                "followup_did_save_exact_producer_lifecycle_state": followup_did_save_exact_producer_lifecycle_state,
                "followup_did_save_exact_producer_lifecycle_state_at_timeout": followup_did_save_exact_producer_lifecycle_state_at_timeout,
                "followup_did_save_exact_producer_final_lifecycle_state": followup_did_save_exact_producer_final_lifecycle_state,
                "followup_save_fastlane_gate_outcome": followup_save_fastlane_gate_outcome,
                "followup_save_fastlane_gate_wait_ms": followup_save_fastlane_gate_wait_ms,
                "followup_admission_queue_wait_ms": followup_admission_queue_wait_ms,
                "followup_ready_snapshot_bounded_wait_elapsed_ms": followup_ready_snapshot_bounded_wait_elapsed_ms,
                "followup_ready_snapshot_parse_exec_ms": followup_ready_snapshot_parse_exec_ms,
                "followup_ready_snapshot_program_lowering_ms": followup_ready_snapshot_program_lowering_ms,
                "first_publish_elapsed_ms": first_publish_elapsed_ms,
                "first_publish_syntax_query_ms": first_publish_syntax_query_ms,
                "first_publish_runtime_queue_wait_ms": first_publish_runtime_queue_wait_ms,
                "first_publish_publish_wait_ms": first_publish_publish_wait_ms,
                "followup_runtime_queue_wait_ms": followup_runtime_queue_wait_ms,
                "followup_apply_lag_ms": followup_apply_lag_ms,
                "followup_wait_for_file_version_ms": followup_wait_for_file_version_ms,
                "followup_snapshot_with_deps_ms": followup_snapshot_with_deps_ms,
                "followup_publish_elapsed_ms": followup_publish_elapsed_ms,
                "followup_publish_runtime_queue_wait_ms": followup_publish_runtime_queue_wait_ms,
                "followup_publish_blocking_queue_wait_ms": followup_publish_blocking_queue_wait_ms,
                "followup_publish_wait_for_file_version_ms": followup_publish_wait_for_file_version_ms,
                "followup_publish_snapshot_with_deps_ms": followup_publish_snapshot_with_deps_ms,
                "followup_publish_publish_wait_ms": followup_publish_publish_wait_ms,
                "followup_publish_semantic_diagnostics_query_ms": followup_publish_semantic_diagnostics_query_ms,
                "followup_publish_semantic_diagnostics_ir_ms": followup_publish_semantic_diagnostics_ir_ms,
                "followup_publish_semantic_diagnostics_collect_ms": followup_publish_semantic_diagnostics_collect_ms,
                "semantic_query_dominates_parse_exec": semantic_query_dominates_parse_exec,
                "followup_publish_non_query_residual_ms": followup_publish_non_query_residual_ms,
                "detached_diagnostics_ready_wait_elapsed_ms": followup_ready_snapshot_bounded_wait_elapsed_ms,
                "detached_diagnostics_ready_publish_elapsed_ms": followup_publish_elapsed_ms,
                "program_lowering_reuse_outcome": program_lowering_reuse_outcome,
                "program_lowering_reused_lowering_units": program_lowering_reused_lowering_units,
                "program_lowering_rebuilt_lowering_units": program_lowering_rebuilt_lowering_units,
                "program_lowering_reused_window_count": program_lowering_reused_window_count,
                "program_lowering_rebuilt_window_count": program_lowering_rebuilt_window_count,
                "program_lowering_reuse_plan_build_source": program_lowering_reuse_plan_build_source,
                "program_lowering_reuse_plan_take_if_unique_hit": program_lowering_reuse_plan_take_if_unique_hit,
                "program_lowering_reuse_plan_borrowed_cache_hit": program_lowering_reuse_plan_borrowed_cache_hit,
                "program_lowering_reuse_plan_build_ms": program_lowering_reuse_plan_build_ms,
                "program_lowering_reused_progress_ms": program_lowering_reused_progress_ms,
                "program_lowering_rebuild_dispatch_ms": program_lowering_rebuild_dispatch_ms,
                "followup_ready_snapshot_continuation_reason": followup_ready_snapshot_continuation_reason,
                "followup_ready_snapshot_relief_valve_outcome": followup_ready_snapshot_relief_valve_outcome,
                "followup_ready_snapshot_timeout_phase": followup_ready_snapshot_timeout_phase,
                "followup_ready_snapshot_timeout_leaf": followup_ready_snapshot_timeout_leaf,
                "followup_ready_snapshot_timeout_leaf_elapsed_ms": followup_ready_snapshot_timeout_leaf_elapsed_ms,
                "followup_wait_reason": followup_wait_reason,
                "observed_version_after_timeout": observed_version_after_timeout,
                "exact_ready_after_timeout": exact_ready_after_timeout,
                "completion_head_ready_after_timeout": completion_head_ready_after_timeout,
                "type_index_parse_snapshot_meta_after_timeout": type_index_parse_snapshot_meta_after_timeout.as_ref().map(
                    |(incremental, changed_ranges_count, serve_only_blocked)| serde_json::json!({
                        "incremental": incremental,
                        "changed_ranges_count": changed_ranges_count,
                        "serve_only_blocked": serve_only_blocked,
                    })
                ),
                "ready_snapshot_state_after_timeout": ready_snapshot_state_after_timeout.as_ref().map(
                    |(file_version, source, syntax_errors_complete)| serde_json::json!({
                        "file_version": file_version,
                        "source": source,
                        "syntax_errors_complete": syntax_errors_complete,
                    })
                ),
                "type_index_task_state_after_timeout": type_index_task_state_after_timeout.as_ref().map(
                    |(requested_version, work_class, phase, active_requested_version, finished)| serde_json::json!({
                        "requested_version": requested_version,
                        "work_class": format!("{work_class:?}"),
                        "phase": phase.as_str(),
                        "active_requested_version": active_requested_version,
                        "finished": finished,
                    })
                ),
                "current_revision_head_precompute_task_state_after_timeout": current_revision_head_precompute_task_state_after_timeout.as_ref().map(
                    |(requested_version, finished)| serde_json::json!({
                        "requested_version": requested_version,
                        "finished": finished,
                    })
                ),
                "background_parse_task_state_after_timeout": background_parse_task_state_after_timeout.as_ref().map(
                    |(phase, promotion_requested, materialized, ready_install_wait, effective_source, save_cycle_sequence, epoch)| serde_json::json!({
                        "phase": format!("{phase:?}"),
                        "promotion_requested": promotion_requested,
                        "materialized": materialized,
                        "original_source": "did_change",
                        "effective_source": effective_source,
                        "source_transition": if *effective_source == "did_save" {
                            "same_version_did_save_promotion"
                        } else {
                            "none"
                        },
                        "save_cycle_sequence": save_cycle_sequence,
                        "epoch": epoch,
                        "ready_install_exact_type_index_wait": p56_ready_install_wait_snapshot_json(ready_install_wait),
                    })
                ),
                "pure_did_change_ready_install_exact_type_index_wait": pure_did_change_ready_install_exact_type_index_wait,
                "successful_pure_did_change_materialization_elapsed_ms": pure_did_change_materialization_elapsed_ms,
                "type_index_precompute_exec_histogram_after_timeout": type_index_precompute_exec_histogram_after_timeout,
                "type_index_precompute_ir_exec_histogram_after_timeout": type_index_precompute_ir_exec_histogram_after_timeout,
                "type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout": type_index_precompute_semantic_facts_local_function_summaries_exec_histogram_after_timeout,
                "ir_singleflight_wait_histogram_after_timeout": ir_singleflight_wait_histogram_after_timeout,
                "ir_singleflight_counters_after_timeout": ir_singleflight_counters_after_timeout,
                "type_index_counters_after_timeout": type_index_counters_after_timeout,
                "final_statement": stage2_statement,
            });

            let ready_install_terminal = tokio::time::timeout(
                Duration::from_secs(READY_SNAPSHOT_MATERIALIZATION_TIMEOUT_SECS),
                async {
                loop {
                    let ready = server
                        .latest_ready_parse_snapshots_v2
                        .read()
                        .await
                        .get(&file_id)
                        .cloned();
                    if ready
                        .as_ref()
                        .is_some_and(|state| state.parse_snapshot.file_version == stage2_version)
                    {
                        break serde_json::json!({
                            "terminal": "canonical_ready_snapshot_materialized",
                            "ready_snapshot_version": stage2_version,
                        });
                    }
                    let failure = server
                        .latest_snapshot_failures_v2
                        .read()
                        .await
                        .get(&file_id)
                        .filter(|state| state.requested_version == stage2_version)
                        .map(|state| state.reason.as_ref().to_string());
                    if let Some(reason) = failure {
                        break serde_json::json!({
                            "terminal": "classified_blocker",
                            "snapshot_failure_reason": reason,
                        });
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            },
            )
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "p56 cycle {cycle_number} must eventually materialize or classify the saved exact ready snapshot"
                )
            });
            cycle_summary
                .as_object_mut()
                .expect("cycle summary object")
                .insert(
                    "ready_install_exact_type_index_wait_terminal".to_string(),
                    ready_install_terminal,
                );
            let canonical_ready_snapshot_state_after_terminal = server
                .latest_ready_parse_snapshots_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| {
                    serde_json::json!({
                        "file_version": state.parse_snapshot.file_version,
                        "source": p56_background_parse_snapshot_source_label(state.source),
                        "syntax_errors_complete": state.syntax_errors_complete,
                    })
                });
            let final_canonical_source_after_terminal =
                canonical_ready_snapshot_state_after_terminal
                    .as_ref()
                    .and_then(|state| state.get("source"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
            let final_canonical_version_after_terminal =
                canonical_ready_snapshot_state_after_terminal
                    .as_ref()
                    .and_then(|state| state.get("file_version"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
            let cycle_object = cycle_summary.as_object_mut().expect("cycle summary object");
            cycle_object.insert(
                "canonical_ready_snapshot_state_after_terminal".to_string(),
                canonical_ready_snapshot_state_after_terminal.unwrap_or(serde_json::Value::Null),
            );
            cycle_object.insert(
                "final_canonical_source_after_terminal".to_string(),
                final_canonical_source_after_terminal,
            );
            cycle_object.insert(
                "final_canonical_version_after_terminal".to_string(),
                final_canonical_version_after_terminal,
            );
            cycles.push(cycle_summary);

            current_version = stage2_version;
            current_statement = stage2_statement;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        let detached_ready_artifacts_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("detached_ready_artifacts")
            })
            .count() as u64;
        let ready_artifacts_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("ready_artifacts")
            })
            .count() as u64;
        let shadow_state_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("shadow_state")
            })
            .count() as u64;
        let waiting_shadow_state_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("shadow_state")
                    && cycle
                        .get("followup_ready_snapshot_timeout_phase")
                        .and_then(|value| value.as_str())
                        == Some("waiting")
                    && cycle
                        .get("followup_ready_snapshot_timeout_leaf")
                        .and_then(|value| value.as_str())
                        == Some("waiting")
            })
            .count() as u64;
        let program_lowering_full_rebuild_shadow_state_later_detached_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("shadow_state")
                    && cycle
                        .get("followup_ready_snapshot_timeout_phase")
                        .and_then(|value| value.as_str())
                        == Some("parse_exec")
                    && cycle
                        .get("followup_ready_snapshot_timeout_leaf")
                        .and_then(|value| value.as_str())
                        == Some("program_lowering")
                    && p56_program_lowering_reuse_outcome(cycle) == Some("full_rebuild")
                    && p56_program_lowering_reused_lowering_units(cycle) == Some(0)
                    && p56_program_lowering_rebuilt_lowering_units(cycle)
                        .is_some_and(|units| units > 0)
                    && matches!(
                        cycle
                            .get("followup_did_save_exact_producer_final_lifecycle_state")
                            .and_then(|value| value.as_str()),
                        Some("detached_diagnostics_ready_published" | "fully_materialized")
                    )
            })
            .count() as u64;
        let program_lowering_full_rebuild_detached_ready_late_count = cycles
            .iter()
            .filter(|cycle| p56_cycle_has_late_detached_program_lowering_full_rebuild(cycle))
            .count() as u64;
        let started_parser_base_shadow_without_terminal_reason_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("shadow_state")
                    && cycle
                        .get("followup_ready_snapshot_timeout_leaf")
                        .and_then(|value| value.as_str())
                        == Some("parser_base_recovery")
                    && matches!(
                        cycle
                            .get("followup_did_save_exact_producer_lifecycle_state_at_timeout")
                            .or_else(|| {
                                cycle.get("followup_did_save_exact_producer_lifecycle_state")
                            })
                            .and_then(|value| value.as_str()),
                        Some("started")
                    )
                    && cycle
                        .get("semantic_query_dominates_parse_exec")
                        .and_then(|value| value.as_bool())
                        == Some(true)
                    && !matches!(
                        cycle
                            .get("followup_did_save_exact_producer_final_lifecycle_state")
                            .and_then(|value| value.as_str()),
                        Some(
                            "detached_diagnostics_ready_published"
                                | "fully_materialized"
                                | "superseded"
                                | "cancelled"
                                | "failed"
                                | "continuity_lost"
                        )
                    )
            })
            .count() as u64;
        let bounded_wait_winner_detached_ready_artifacts_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_bounded_wait_winner")
                    .and_then(|value| value.as_str())
                    == Some("detached_ready_artifacts")
            })
            .count() as u64;
        let zero_probe_not_ready_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_zero_probe")
                    .and_then(|value| value.as_str())
                    == Some("not_ready")
            })
            .count() as u64;
        let wait_probe_not_ready_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_wait_probe")
                    .and_then(|value| value.as_str())
                    == Some("not_ready")
            })
            .count() as u64;
        let wait_probe_timeout_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_wait_probe")
                    .and_then(|value| value.as_str())
                    == Some("timeout")
            })
            .count() as u64;
        let semantic_query_dominates_parse_exec_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("semantic_query_dominates_parse_exec")
                    .and_then(|value| value.as_bool())
                    == Some(true)
            })
            .count() as u64;
        let slow_first_publish_elapsed_count = cycles
            .iter()
            .filter(|cycle| {
                p56_cycle_u64(cycle, &["first_publish_elapsed_ms"]).is_some_and(|value| {
                    value > REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS
                })
            })
            .count() as u64;
        let slow_first_publish_syntax_query_count = cycles
            .iter()
            .filter(|cycle| {
                p56_cycle_u64(cycle, &["first_publish_syntax_query_ms"]).is_some_and(|value| {
                    value > REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS
                })
            })
            .count() as u64;
        let slow_first_publish_count = cycles
            .iter()
            .filter(|cycle| p56_cycle_has_slow_first_publish(cycle))
            .count() as u64;
        let producer_lifecycle_detached_or_materialized_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("detached_ready_artifacts")
                    || matches!(
                        cycle
                            .get("followup_did_save_exact_producer_final_lifecycle_state")
                            .and_then(|value| value.as_str()),
                        Some("detached_diagnostics_ready_published" | "fully_materialized")
                    )
                    || matches!(
                    cycle
                        .get("followup_did_save_exact_producer_lifecycle_state")
                        .and_then(|value| value.as_str()),
                    Some("detached_diagnostics_ready_published" | "fully_materialized")
                )
            })
            .count() as u64;
        let continuation_reason_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_continuation_reason")
                    .and_then(|value| value.as_str())
                    .is_some()
            })
            .count() as u64;
        let timeout_leaf_ready_install_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_ready_snapshot_timeout_leaf")
                    .and_then(|value| value.as_str())
                    == Some("ready_install")
            })
            .count() as u64;
        let ready_install_exact_type_index_wait_ready_count = cycles
            .iter()
            .filter(|cycle| p56_ready_install_wait_str(cycle, "outcome") == Some("ready"))
            .count() as u64;
        let ready_install_exact_type_index_wait_deadline_count = cycles
            .iter()
            .filter(|cycle| {
                p56_ready_install_wait_str(cycle, "outcome") == Some("deadline")
                    || p56_ready_install_terminal_str(cycle, "snapshot_failure_reason")
                        == Some("exact_type_index_deadline_before_ready_install")
            })
            .count() as u64;
        let ready_install_exact_type_index_wait_classified_blocker_count = cycles
            .iter()
            .filter(|cycle| {
                p56_ready_install_terminal_str(cycle, "terminal") == Some("classified_blocker")
            })
            .count() as u64;
        let ready_install_exact_type_index_wait_materialized_count = cycles
            .iter()
            .filter(|cycle| {
                p56_ready_install_terminal_str(cycle, "terminal")
                    == Some("canonical_ready_snapshot_materialized")
            })
            .count() as u64;
        let ready_install_exact_type_index_wait_contract_approved_count = cycles
            .iter()
            .filter(|cycle| p56_cycle_has_contract_approved_ready_install_resolution(cycle))
            .count() as u64;
        let same_version_did_save_promotion_source_transition_count = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .pointer("/background_parse_task_state_after_timeout/source_transition")
                    .and_then(|value| value.as_str())
                    == Some("same_version_did_save_promotion")
            })
            .count() as u64;
        let promoted_save_cycle_sample_count = cycles
            .iter()
            .filter(|cycle| {
                let explicit_source_transition = cycle
                    .pointer("/background_parse_task_state_after_timeout/source_transition")
                    .and_then(|value| value.as_str())
                    == Some("same_version_did_save_promotion");
                let save_cycle_detached_or_materialized = cycle
                    .get("save_cycle_sequence")
                    .and_then(|value| value.as_u64())
                    .is_some()
                    && (cycle
                        .get("followup_semantic_path")
                        .and_then(|value| value.as_str())
                        == Some("detached_ready_artifacts")
                        || matches!(
                            cycle
                                .get("followup_did_save_exact_producer_lifecycle_state")
                                .and_then(|value| value.as_str()),
                            Some("detached_diagnostics_ready_published" | "fully_materialized")
                        ))
                    && cycle
                        .get("followup_ready_snapshot_task_state")
                        .and_then(|value| value.as_str())
                        == Some("in_flight_same_version");
                explicit_source_transition || save_cycle_detached_or_materialized
            })
            .count() as u64;
        let pure_did_change_ready_install_exact_type_index_wait_ready_count = cycles
            .iter()
            .filter(|cycle| {
                p56_pure_did_change_ready_install_wait_str(cycle, "outcome") == Some("ready")
            })
            .count() as u64;
        let pure_did_change_ready_install_exact_type_index_wait_deadline_count = cycles
            .iter()
            .filter(|cycle| {
                p56_pure_did_change_ready_install_wait_str(cycle, "outcome") == Some("deadline")
            })
            .count() as u64;
        let max_pure_did_change_ready_install_exact_type_index_wait_elapsed_ms = cycles
            .iter()
            .filter_map(|cycle| {
                p56_pure_did_change_ready_install_wait_u64(cycle, "elapsed_ms")
            })
            .max();
        let max_ready_install_exact_type_index_wait_elapsed_ms = cycles
            .iter()
            .filter_map(|cycle| p56_ready_install_wait_u64(cycle, "elapsed_ms"))
            .max();
        let max_followup_ready_snapshot_bounded_wait_elapsed_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_ready_snapshot_bounded_wait_elapsed_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_elapsed_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_elapsed_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_semantic_diagnostics_query_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_semantic_diagnostics_query_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_ready_snapshot_parse_exec_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_ready_snapshot_parse_exec_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_first_publish_elapsed_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("first_publish_elapsed_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_first_publish_syntax_query_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("first_publish_syntax_query_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_save_fastlane_gate_wait_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_save_fastlane_gate_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_admission_queue_wait_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_admission_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_runtime_queue_wait_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_runtime_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_runtime_queue_wait_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_runtime_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_blocking_queue_wait_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_blocking_queue_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_wait_for_file_version_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_wait_for_file_version_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_snapshot_with_deps_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_snapshot_with_deps_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();
        let max_followup_publish_publish_wait_ms = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("followup_publish_publish_wait_ms")
                    .and_then(|value| value.as_u64())
            })
            .max();

        let final_metrics = live_transport_get_observability_metrics(&mut harness, 56_100_950).await;
        let final_histograms = final_metrics
            .get("histograms")
            .and_then(|value| value.as_object())
            .expect("final metrics.histograms object");
        let final_counters = final_metrics
            .get("counters")
            .and_then(|value| value.as_object())
            .expect("final metrics.counters object");
        let did_change_materialization_histogram = final_histograms
            .get("intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_did_change")
            .and_then(|value| value.as_object())
            .expect("p56 did_change materialization histogram");
        let did_change_materialization_histogram_count = read_u64_metric(
            did_change_materialization_histogram.get("count"),
        );
        let did_save_materialization_count = read_u64_metric(final_counters.get(
            "intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_did_save",
        ));
        let (
            excluded_did_change_terminal_reasons,
            excluded_did_change_non_success_count,
        ) = p56_ready_parse_snapshot_worker_termination_counts(final_counters, "did_change");
        let did_change_materialization_p50_ms =
            read_numeric_metric(did_change_materialization_histogram.get("p50"));
        let did_change_materialization_p95_ms =
            read_numeric_metric(did_change_materialization_histogram.get("p95"));
        let successful_pure_did_change_materialization_samples = cycles
            .iter()
            .filter_map(|cycle| {
                cycle
                    .get("successful_pure_did_change_materialization_elapsed_ms")
                    .and_then(|value| value.as_u64())
            })
            .collect::<Vec<_>>();
        let successful_pure_did_change_materialization_sample_count =
            successful_pure_did_change_materialization_samples.len() as u64;
        let successful_pure_did_change_materialization_p50_ms =
            p56_percentile_ms(&successful_pure_did_change_materialization_samples, 0.50);
        let successful_pure_did_change_materialization_p95_ms =
            p56_percentile_ms(&successful_pure_did_change_materialization_samples, 0.95);

        assert_eq!(
            cycles.len(),
            SAVE_CYCLE_COUNT,
            "p56 must record every representative save cycle"
        );
        assert_eq!(
            detached_ready_artifacts_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 representative bundle must resolve every still-current same-version save cycle through detached_ready_artifacts, cycles={cycles:?}"
        );
        assert_eq!(
            shadow_state_count, 0,
            "p56 representative bundle must fail closed on waiting-phase shadow_state terminal publish before later exact readiness, cycles={cycles:?}"
        );
        assert_eq!(
            waiting_shadow_state_count, 0,
            "p56 representative bundle must fail the inherited refactor-50 waiting shadow_state contour, cycles={cycles:?}"
        );
        assert_eq!(
            wait_probe_timeout_count, 0,
            "p56 representative bundle must not time out the bounded wait before detached-ready publication on still-current save families, cycles={cycles:?}"
        );
        assert_eq!(
            slow_first_publish_count, 0,
            "p56 representative bundle must fail the 2026-04-24 contour: a later detached-ready follow-up cannot hide slow save_fastlane first publish, elapsed_ceiling={}ms, syntax_query_ceiling={}ms, elapsed_slow_count={slow_first_publish_elapsed_count}, syntax_query_slow_count={slow_first_publish_syntax_query_count}, cycles={cycles:?}",
            REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS,
            REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS
        );
        assert_eq!(
            producer_lifecycle_detached_or_materialized_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 representative bundle must expose detached-ready publication evidence for every cycle, cycles={cycles:?}"
        );
        assert_eq!(
            detached_ready_artifacts_count,
            bounded_wait_winner_detached_ready_artifacts_count,
            "p56 representative bundle must preserve detached_ready_artifacts attribution for every detached winner cycle, cycles={cycles:?}"
        );
        assert_eq!(
            shadow_state_count,
            waiting_shadow_state_count,
            "p56 representative bundle must only allow shadow_state when the still-current exact worker is separately attributed to the waiting bucket, cycles={cycles:?}"
        );
        assert_eq!(
            program_lowering_full_rebuild_shadow_state_later_detached_count,
            0,
            "p56 representative bundle must fail the 2026-04-24 contour: parse_exec/program_lowering full_rebuild shadow_state before later detached-ready/full materialization, cycles={cycles:?}"
        );
        assert_eq!(
            program_lowering_full_rebuild_detached_ready_late_count,
            0,
            "p56 representative bundle must fail the 2026-04-24 contour: terminal detached_ready_artifacts arrived only after bounded-wait/relief timeout on parse_exec/program_lowering full_rebuild, cycles={cycles:?}"
        );
        assert_eq!(
            started_parser_base_shadow_without_terminal_reason_count,
            0,
            "p56 representative bundle must fail refactor-52 started parser_base_recovery shadow_state residuals without final producer lifecycle evidence, cycles={cycles:?}"
        );
        assert_eq!(
            bounded_wait_winner_detached_ready_artifacts_count,
            detached_ready_artifacts_count,
            "p56 representative bundle must attribute bounded wait completion to detached_ready_artifacts on every still-current detached cycle, cycles={cycles:?}"
        );
        assert_eq!(
            zero_probe_not_ready_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 representative bundle must exercise the same-family in-flight producer before bounded wait succeeds, cycles={cycles:?}"
        );
        assert_eq!(
            wait_probe_not_ready_count + wait_probe_timeout_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 representative bundle must attribute every representative cycle to either detached ready completion or waiting-phase timeout, cycles={cycles:?}"
        );
        assert_eq!(
            wait_probe_timeout_count,
            waiting_shadow_state_count,
            "p56 representative bundle must keep timeout wait-probe attribution aligned with waiting-phase shadow_state cycles, cycles={cycles:?}"
        );
        assert_eq!(
            semantic_query_dominates_parse_exec_count,
            detached_ready_artifacts_count,
            "p56 representative bundle must prove that semantic_diagnostics_query dominates ready-snapshot parse_exec on every detached cycle, cycles={cycles:?}"
        );
        assert_eq!(
            continuation_reason_count,
            0,
            "p56 representative bundle must not need a follow-up continuation reason after refactor-46, cycles={cycles:?}"
        );
        assert_eq!(
            timeout_leaf_ready_install_count,
            0,
            "p56 representative bundle must not export a timeout leaf when detached_ready_artifacts wins the bounded wait, cycles={cycles:?}"
        );
        assert!(
            max_followup_ready_snapshot_bounded_wait_elapsed_ms
                .is_some(),
            "p56 representative bundle must export bounded wait elapsed samples, observed_max={max_followup_ready_snapshot_bounded_wait_elapsed_ms:?}, cycles={cycles:?}"
        );
        assert!(
            max_first_publish_elapsed_ms.is_some(),
            "p56 representative bundle must export save_fastlane first-publish elapsed samples, cycles={cycles:?}"
        );
        assert!(
            max_first_publish_syntax_query_ms.is_some(),
            "p56 representative bundle must export save_fastlane syntax_diagnostics_query samples, cycles={cycles:?}"
        );
        assert!(
            max_first_publish_elapsed_ms
                .map(|value| value <= REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS)
                .unwrap_or(false),
            "p56 representative bundle must keep save_fastlane first publish at or below {}ms so a later detached-ready follow-up cannot mask first-publish latency, observed_max={max_first_publish_elapsed_ms:?}, cycles={cycles:?}",
            REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS
        );
        assert!(
            max_first_publish_syntax_query_ms
                .map(|value| value <= REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS)
                .unwrap_or(false),
            "p56 representative bundle must keep save_fastlane syntax_diagnostics_query at or below {}ms, observed_max={max_first_publish_syntax_query_ms:?}, cycles={cycles:?}",
            REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS
        );
        assert!(
            max_followup_publish_elapsed_ms
                .map(|value| value <= BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS)
                .unwrap_or(true),
            "p56 representative bundle must stay at or below the {BASELINE_CAPTURED_AT} publish baseline ceiling of {}ms when detached publish samples exist, observed_max={max_followup_publish_elapsed_ms:?}, cycles={cycles:?}",
            BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS
        );
        assert!(
            max_followup_ready_snapshot_parse_exec_ms
                .map(|value| value <= BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS)
                .unwrap_or(true),
            "p56 representative bundle must stay at or below the {BASELINE_CAPTURED_AT} parse_exec baseline ceiling of {}ms when parse_exec samples exist, observed_max={max_followup_ready_snapshot_parse_exec_ms:?}, cycles={cycles:?}",
            BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS
        );
        assert!(
            did_change_materialization_histogram_count > 0,
            "p56 representative bundle must export did_change ready-snapshot materialization latency, final_histograms={final_histograms:?}"
        );
        assert_eq!(
            successful_pure_did_change_materialization_sample_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 pure didChange baseline must count successful stage1 didChange materializations explicitly; compatibility source histograms may include older source-labelled samples, final_counters={final_counters:?}, cycles={cycles:?}"
        );
        assert_eq!(
            pure_did_change_ready_install_exact_type_index_wait_deadline_count,
            0,
            "p56 pure didChange ready-install wait must not hit deadline on the baseline contour, cycles={cycles:?}"
        );
        assert_eq!(
            ready_install_exact_type_index_wait_contract_approved_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 canonical ready-install must either materialize or export a contract-approved exact type-index blocker for every cycle, cycles={cycles:?}"
        );
        assert_eq!(
            promoted_save_cycle_sample_count,
            SAVE_CYCLE_COUNT as u64,
            "p56 report must preserve didSave-promoted/save-cycle evidence for every representative save cycle, cycles={cycles:?}"
        );
        let did_change_materialization_within_baseline =
            successful_pure_did_change_materialization_p50_ms
                <= BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS
                && successful_pure_did_change_materialization_p95_ms
                    <= BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS;
        assert!(
            did_change_materialization_within_baseline,
            "p56 pure didChange materialization must stay within the captured baseline; later save-cycle blocker classification cannot mask this failure, p50={successful_pure_did_change_materialization_p50_ms}, p95={successful_pure_did_change_materialization_p95_ms}, baseline_p50={BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS}, baseline_p95={BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS}, compatibility_histogram_p50={did_change_materialization_p50_ms}, compatibility_histogram_p95={did_change_materialization_p95_ms}, excluded_did_change_non_success_count={excluded_did_change_non_success_count}, excluded_did_change_terminal_reasons={excluded_did_change_terminal_reasons:?}, cycles={cycles:?}"
        );
        let representative_cycle = cycles
            .iter()
            .filter(|cycle| {
                cycle
                    .get("followup_semantic_path")
                    .and_then(|value| value.as_str())
                    == Some("shadow_state")
                    && cycle
                        .get("followup_ready_snapshot_timeout_phase")
                        .and_then(|value| value.as_str())
                        == Some("waiting")
            })
            .max_by_key(|cycle| {
                cycle
                    .get("followup_ready_snapshot_timeout_leaf_elapsed_ms")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0)
            })
            .or_else(|| {
                cycles
                    .iter()
                    .filter(|cycle| {
                        cycle
                            .get("followup_semantic_path")
                            .and_then(|value| value.as_str())
                            == Some("detached_ready_artifacts")
                    })
                    .max_by_key(|cycle| {
                        cycle
                            .get("followup_publish_elapsed_ms")
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0)
                    })
            })
            .cloned()
            .expect("p56 must keep either a waiting-phase shadow_state slice or a detached_ready_artifacts representative cycle summary");

        let report = serde_json::json!({
            "profile": PROFILE_NAME,
            "change_id": change_id,
            "module_path": module_path.display().to_string(),
            "uri": uri.to_string(),
            "cycle_count": SAVE_CYCLE_COUNT,
            "baseline": {
                "captured_at": BASELINE_CAPTURED_AT,
                "save_fastlane_first_publish_elapsed_ms_ceiling": REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS,
                "save_fastlane_first_publish_syntax_query_ms_ceiling": REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS,
                "followup_publish_elapsed_ms": [BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MIN_MS, BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS],
                "followup_ready_snapshot_parse_exec_ms": [BASELINE_READY_SNAPSHOT_PARSE_EXEC_MIN_MS, BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS],
                "did_change_ready_snapshot_materialization_ms": {
                    "p50": BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
                    "p95": BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
                },
                "followup_semantic_path_detached_ready_artifacts": BASELINE_DETACHED_READY_ARTIFACTS_COUNT,
                "followup_semantic_path_ready_artifacts": BASELINE_READY_ARTIFACTS_COUNT,
                "followup_semantic_path_shadow_state": BASELINE_SHADOW_STATE_COUNT,
            },
            "summary": {
                "followup_semantic_path_detached_ready_artifacts": detached_ready_artifacts_count,
                "followup_semantic_path_ready_artifacts": ready_artifacts_count,
                "followup_semantic_path_shadow_state": shadow_state_count,
                "followup_semantic_path_shadow_state_waiting": waiting_shadow_state_count,
                "followup_ready_snapshot_bounded_wait_winner_detached_ready_artifacts": bounded_wait_winner_detached_ready_artifacts_count,
                "followup_ready_snapshot_zero_probe_not_ready": zero_probe_not_ready_count,
                "followup_ready_snapshot_wait_probe_not_ready": wait_probe_not_ready_count,
                "followup_ready_snapshot_wait_probe_timeout": wait_probe_timeout_count,
                "followup_did_save_exact_producer_lifecycle_detached_or_materialized": producer_lifecycle_detached_or_materialized_count,
                "followup_ready_snapshot_continuation_reason_count": continuation_reason_count,
                "followup_ready_snapshot_timeout_leaf_ready_install_count": timeout_leaf_ready_install_count,
                "followup_ready_snapshot_rebuild_dominated_shadow_state_count": program_lowering_full_rebuild_shadow_state_later_detached_count,
                "followup_ready_snapshot_program_lowering_full_rebuild_shadow_state_later_detached_count": program_lowering_full_rebuild_shadow_state_later_detached_count,
                "followup_ready_snapshot_program_lowering_full_rebuild_detached_ready_late_count": program_lowering_full_rebuild_detached_ready_late_count,
                "followup_ready_snapshot_started_parser_base_shadow_without_terminal_reason_count": started_parser_base_shadow_without_terminal_reason_count,
                "ready_install_exact_type_index_wait_ready_count": ready_install_exact_type_index_wait_ready_count,
                "ready_install_exact_type_index_wait_deadline_count": ready_install_exact_type_index_wait_deadline_count,
                "ready_install_exact_type_index_wait_classified_blocker_count": ready_install_exact_type_index_wait_classified_blocker_count,
                "ready_install_exact_type_index_wait_materialized_count": ready_install_exact_type_index_wait_materialized_count,
                "ready_install_exact_type_index_wait_contract_approved_count": ready_install_exact_type_index_wait_contract_approved_count,
                "pure_did_change_ready_install_exact_type_index_wait_ready_count": pure_did_change_ready_install_exact_type_index_wait_ready_count,
                "pure_did_change_ready_install_exact_type_index_wait_deadline_count": pure_did_change_ready_install_exact_type_index_wait_deadline_count,
                "same_version_did_save_promotion_source_transition_count": same_version_did_save_promotion_source_transition_count,
                "successful_pure_did_change_materialization_sample_count": successful_pure_did_change_materialization_sample_count,
                "excluded_did_change_non_success_count": excluded_did_change_non_success_count,
                "excluded_did_change_terminal_reasons": excluded_did_change_terminal_reasons,
                "promoted_save_cycle_sample_count": promoted_save_cycle_sample_count,
                "did_save_materialization_sample_count": did_save_materialization_count,
                "save_fastlane_slow_first_publish_count": slow_first_publish_count,
                "save_fastlane_slow_first_publish_elapsed_count": slow_first_publish_elapsed_count,
                "save_fastlane_slow_first_publish_syntax_query_count": slow_first_publish_syntax_query_count,
                "semantic_query_dominates_parse_exec_count": semantic_query_dominates_parse_exec_count,
                "representative_bounded_wait_shape": "detached_ready_artifacts_wins_before_canonical_timeout",
                "representative_canonical_residual_mix": if ready_install_exact_type_index_wait_classified_blocker_count > 0 {
                    "producer_lifecycle_reaches_detached_ready_then_classified_exact_type_index_blocker"
                } else {
                    "producer_lifecycle_reaches_detached_ready_then_canonical_materialization"
                },
                "post_detached_publish_shape": if detached_ready_artifacts_count > 0 {
                    "semantic_query_dominates_parse_exec_with_additional_publish_tail"
                } else {
                    "no_detached_publish_samples_on_waiting_phase_representative_slice"
                },
            },
            "aggregate": {
                "max_followup_ready_snapshot_bounded_wait_elapsed_ms": max_followup_ready_snapshot_bounded_wait_elapsed_ms,
                "max_followup_publish_elapsed_ms": max_followup_publish_elapsed_ms,
                "max_followup_publish_semantic_diagnostics_query_ms": max_followup_publish_semantic_diagnostics_query_ms,
                "max_followup_ready_snapshot_parse_exec_ms": max_followup_ready_snapshot_parse_exec_ms,
                "max_first_publish_elapsed_ms": max_first_publish_elapsed_ms,
                "max_first_publish_syntax_query_ms": max_first_publish_syntax_query_ms,
                "max_followup_save_fastlane_gate_wait_ms": max_followup_save_fastlane_gate_wait_ms,
                "max_followup_admission_queue_wait_ms": max_followup_admission_queue_wait_ms,
                "max_followup_runtime_queue_wait_ms": max_followup_runtime_queue_wait_ms,
                "max_followup_publish_runtime_queue_wait_ms": max_followup_publish_runtime_queue_wait_ms,
                "max_followup_publish_blocking_queue_wait_ms": max_followup_publish_blocking_queue_wait_ms,
                "max_followup_publish_wait_for_file_version_ms": max_followup_publish_wait_for_file_version_ms,
                "max_followup_publish_snapshot_with_deps_ms": max_followup_publish_snapshot_with_deps_ms,
                "max_followup_publish_publish_wait_ms": max_followup_publish_publish_wait_ms,
                "max_ready_install_exact_type_index_wait_elapsed_ms": max_ready_install_exact_type_index_wait_elapsed_ms,
                "max_pure_did_change_ready_install_exact_type_index_wait_elapsed_ms": max_pure_did_change_ready_install_exact_type_index_wait_elapsed_ms,
                "did_change_ready_snapshot_materialization_histogram_count": did_change_materialization_histogram_count,
                "did_change_ready_snapshot_materialization_p50_ms": did_change_materialization_p50_ms,
                "did_change_ready_snapshot_materialization_p95_ms": did_change_materialization_p95_ms,
                "successful_pure_did_change_materialization_sample_count": successful_pure_did_change_materialization_sample_count,
                "successful_pure_did_change_materialization_p50_ms": successful_pure_did_change_materialization_p50_ms,
                "successful_pure_did_change_materialization_p95_ms": successful_pure_did_change_materialization_p95_ms,
            },
            "contract": {
                "canonical_ready_install_type_index_resolution": if ready_install_exact_type_index_wait_contract_approved_count == SAVE_CYCLE_COUNT as u64 {
                    "approved"
                } else {
                    "gap"
                },
                "did_change_materialization_within_baseline": did_change_materialization_within_baseline,
                "later_save_cycle_blocker_can_mask_did_change_baseline": false,
                "successful_pure_did_change_materialization_sample_count": successful_pure_did_change_materialization_sample_count,
                "excluded_did_change_non_success_count": excluded_did_change_non_success_count,
            },
            "comparison": {
                "max_first_publish_elapsed_vs_ceiling_delta_ms": max_first_publish_elapsed_ms
                    .map(|value| value as i64 - REFACTOR54_FIRST_PUBLISH_ELAPSED_MAX_MS as i64),
                "max_first_publish_syntax_query_vs_ceiling_delta_ms": max_first_publish_syntax_query_ms
                    .map(|value| value as i64 - REFACTOR54_FIRST_PUBLISH_SYNTAX_QUERY_MAX_MS as i64),
                "max_followup_publish_elapsed_vs_baseline_ceiling_delta_ms": max_followup_publish_elapsed_ms
                    .map(|value| value as i64 - BASELINE_FOLLOWUP_PUBLISH_ELAPSED_MAX_MS as i64),
                "max_followup_ready_snapshot_parse_exec_vs_baseline_ceiling_delta_ms": max_followup_ready_snapshot_parse_exec_ms
                    .map(|value| value as i64 - BASELINE_READY_SNAPSHOT_PARSE_EXEC_MAX_MS as i64),
                "did_change_ready_snapshot_materialization_p50_vs_baseline_delta_ms": did_change_materialization_p50_ms - BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
                "did_change_ready_snapshot_materialization_p95_vs_baseline_delta_ms": did_change_materialization_p95_ms - BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
                "successful_pure_did_change_materialization_p50_vs_baseline_delta_ms": successful_pure_did_change_materialization_p50_ms - BASELINE_DID_CHANGE_MATERIALIZATION_P50_MS,
                "successful_pure_did_change_materialization_p95_vs_baseline_delta_ms": successful_pure_did_change_materialization_p95_ms - BASELINE_DID_CHANGE_MATERIALIZATION_P95_MS,
            },
            "representative_cycle": representative_cycle,
            "cycles": cycles,
        });
        let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("backend crate must live under the workspace root");
        let report_path = std::env::var(
            "BSL_V2_REAL_CONF_BIG_REPRESENTATIVE_SAVE_FOLLOWUP_BUNDLE_REPORT",
        )
        .map(std::path::PathBuf::from)
        .map(|path| {
            if path.is_relative() {
                workspace_root.join(path)
            } else {
                path
            }
        })
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("perf")
                .join("reports")
                .join(format!(
                    "{change_id}-real-conf-big-diagnostics-representative-save-followup-bundle-live.json"
                ))
        });
        if let Some(parent) = report_path.parent() {
            std::fs::create_dir_all(parent)
                .expect("failed to create directory for p56 representative save-followup bundle report");
        }
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&report)
                .expect("serialize p56 representative save-followup bundle report"),
        )
        .expect("write p56 representative save-followup bundle report");
        println!("{PROFILE_NAME}_path={}", report_path.display());

        live_transport_close_document(&mut harness, &uri).await;
        drop(server);
        harness.shutdown().await;
    });
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
}
