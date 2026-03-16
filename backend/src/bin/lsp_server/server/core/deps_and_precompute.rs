use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExactTypeIndexWaitOutcomeV2 {
    Ready,
    Deadline,
    NoMatchingTask,
    TaskPresentWrongVersion,
    ObservedVersionMismatch,
}

impl ExactTypeIndexWaitOutcomeV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Deadline => "deadline",
            Self::NoMatchingTask => "no_matching_task",
            Self::TaskPresentWrongVersion => "task_present_wrong_version",
            Self::ObservedVersionMismatch => "observed_version_mismatch",
        }
    }
}

impl BslLanguageServer {
    pub(crate) async fn sync_v2_globals(&self) {
        let settings = self.settings.read().await.clone();
        let settings_id = compute_settings_id_v2(&settings);
        let diagnostics_detail_level =
            bsl_shared::formatting::DetailLevel::parse(&settings.diagnostics.detail_level);

        let mut changes = Vec::new();

        {
            let mut last_settings_id = self.last_settings_id_v2.write().await;
            if last_settings_id.as_ref() != Some(&settings_id) {
                *last_settings_id = Some(settings_id.clone());
                changes.push(bsl_analysis_v2::Change::SetSettingsSnapshot {
                    settings_id,
                    diagnostics_detail_level,
                });
            }
        }

        self.analysis_v2.apply_changes(changes);
    }

    pub(crate) async fn deps_update_v2(
        &self,
        reason: &str,
        platform_docs_root: Option<PathBuf>,
        config_root: Option<PathBuf>,
    ) {
        let build_started = Instant::now();
        let coordinator = self.coordinator.clone();
        let bundle_result = tokio::task::spawn_blocking(move || {
            build_deps_bundle_v2(
                coordinator.as_ref(),
                platform_docs_root.as_deref(),
                config_root.as_deref(),
            )
        })
        .await;

        let bundle = match bundle_result {
            Ok(Ok(bundle)) => bundle,
            Ok(Err(err)) => {
                let elapsed = build_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_deps_update_build_latency(elapsed);
                self.coordinator.record_intellisense_v2_deps_update_error();
                warn!(
                    "deps_update_v2 build failed: reason={}, error={}",
                    reason, err
                );
                return;
            }
            Err(err) => {
                let elapsed = build_started.elapsed();
                self.coordinator
                    .record_intellisense_v2_deps_update_build_latency(elapsed);
                self.coordinator.record_intellisense_v2_deps_update_error();
                warn!(
                    "deps_update_v2 build join failed: reason={}, error={}",
                    reason, err
                );
                return;
            }
        };

        let build_elapsed = build_started.elapsed();
        self.coordinator
            .record_intellisense_v2_deps_update_build_latency(build_elapsed);

        self.apply_deps_bundle_v2(reason, bundle).await;
    }

    pub(crate) async fn apply_deps_bundle_v2(&self, reason: &str, bundle: DepsBundleV2) {
        let apply_started = Instant::now();
        let ok = self
            .analysis_v2
            .apply_deps_bundle(
                bundle.deps_id.clone(),
                bundle.semantic_deps.clone(),
                bundle.index_snapshot.clone(),
            )
            .await;
        let apply_elapsed = apply_started.elapsed();
        self.coordinator
            .record_intellisense_v2_deps_update_apply_latency(apply_elapsed);

        if !ok {
            self.coordinator.record_intellisense_v2_deps_update_error();
            warn!(
                "deps_update_v2 apply failed: reason={}, deps_id={}, index_snapshot_id={}",
                reason,
                bundle.deps_id.as_str(),
                bundle.meta.index_snapshot_id
            );
            return;
        }

        {
            let mut last_deps_id = self.last_deps_id_v2.write().await;
            *last_deps_id = Some(bundle.deps_id.clone());
        }

        self.coordinator
            .record_intellisense_v2_deps_update_success();
        info!(
            "deps_update_v2 applied: reason={}, deps_id={}, index_snapshot_id={}, platform_version={}, platform_fp={}, config_fp={}, strict_fingerprint={}",
            reason,
            bundle.deps_id.as_str(),
            bundle.meta.index_snapshot_id,
            bundle.meta.platform_version,
            bundle.meta.platform_fingerprint.as_deref().unwrap_or("none"),
            bundle.meta.config_fingerprint.as_deref().unwrap_or("none"),
            bundle.meta.strict_fingerprint
        );
    }

    pub(crate) async fn schedule_type_index_precompute_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
    ) {
        let supersession_key = super::super::TypeIndexPrecomputeSupersessionKeyV2 {
            file_id,
            requested_version,
        };
        let mut tasks = self.type_index_precompute_tasks_v2.lock().await;
        if tasks
            .get(&file_id)
            .is_some_and(|task| task.supersession_key == supersession_key)
        {
            self.coordinator.record_intellisense_v2_type_index_reason(
                bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeQueueSaturated
                    .as_str(),
            );
            return;
        }

        if let Some(previous) = tasks.remove(&file_id) {
            debug!(
                file_id = file_id.0,
                previous_version = previous.supersession_key.requested_version,
                requested_version,
                reason_code =
                    bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeCancelled
                        .as_str(),
                "Event-driven type_index precompute superseded: abort previous task"
            );
            self.coordinator.record_intellisense_v2_type_index_reason(
                bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeCancelled
                    .as_str(),
            );
            previous.handle.abort();
        }

        let server = self.clone();
        let handle = tokio::spawn(async move {
            let enqueued_at = Instant::now();
            server
                .execute_type_index_precompute_once_v2(supersession_key, enqueued_at)
                .await;
            server
                .finalize_type_index_precompute_task_v2(supersession_key)
                .await;
        });
        tasks.insert(
            file_id,
            super::super::TypeIndexPrecomputeTaskV2 {
                supersession_key,
                handle,
            },
        );
    }

    pub(crate) async fn cancel_type_index_precompute_v2(&self, file_id: V2FileId) {
        let task = self
            .type_index_precompute_tasks_v2
            .lock()
            .await
            .remove(&file_id);
        if let Some(task) = task {
            debug!(
                file_id = file_id.0,
                requested_version = task.supersession_key.requested_version,
                reason_code =
                    bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeCancelled
                        .as_str(),
                "Event-driven type_index precompute cancelled on file cleanup"
            );
            self.coordinator.record_intellisense_v2_type_index_reason(
                bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeCancelled
                    .as_str(),
            );
            task.handle.abort();
        }
    }

    pub(crate) async fn wait_for_current_type_index_serve_only_ready_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
        max_wait: std::time::Duration,
    ) -> ExactTypeIndexWaitOutcomeV2 {
        let deadline = tokio::time::Instant::now() + max_wait;
        loop {
            let analysis = self.analysis_v2.snapshot().await;
            let observed_version = analysis.file_version(file_id).ok().flatten();
            let exact_ready = expected_version
                .is_none_or(|version| observed_version == Some(version))
                && analysis
                    .current_type_index_serve_only_ready(file_id)
                    .unwrap_or(false);
            if exact_ready {
                return ExactTypeIndexWaitOutcomeV2::Ready;
            }

            enum MatchingTaskState {
                Matching,
                WrongVersion,
                Missing,
            }

            let matching_task_state = {
                let tasks = self.type_index_precompute_tasks_v2.lock().await;
                match tasks.get(&file_id) {
                    Some(task) => {
                        if expected_version
                            .map(|version| task.supersession_key.requested_version == version)
                            .unwrap_or(true)
                        {
                            MatchingTaskState::Matching
                        } else {
                            MatchingTaskState::WrongVersion
                        }
                    }
                    None => MatchingTaskState::Missing,
                }
            };
            if matches!(matching_task_state, MatchingTaskState::WrongVersion) {
                return ExactTypeIndexWaitOutcomeV2::TaskPresentWrongVersion;
            }
            let observed_version_mismatch =
                expected_version.is_some_and(|version| observed_version != Some(version));
            if matches!(matching_task_state, MatchingTaskState::Missing) {
                return if observed_version_mismatch {
                    ExactTypeIndexWaitOutcomeV2::ObservedVersionMismatch
                } else {
                    ExactTypeIndexWaitOutcomeV2::NoMatchingTask
                };
            }
            if tokio::time::Instant::now() >= deadline {
                return if observed_version_mismatch {
                    ExactTypeIndexWaitOutcomeV2::ObservedVersionMismatch
                } else {
                    ExactTypeIndexWaitOutcomeV2::Deadline
                };
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    async fn type_index_precompute_checkpoint_v2(
        &self,
        key: super::super::TypeIndexPrecomputeSupersessionKeyV2,
        stage: &'static str,
    ) -> bool {
        let current_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&key.file_id)
            .copied();
        if current_version == Some(key.requested_version) {
            return false;
        }
        debug!(
            file_id = key.file_id.0,
            requested_version = key.requested_version,
            current_version,
            stage,
            reason_code =
                bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeSuperseded
                    .as_str(),
            "Event-driven type_index precompute checkpoint superseded"
        );
        self.coordinator.record_intellisense_v2_type_index_reason(
            bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeSuperseded.as_str(),
        );
        true
    }

    async fn execute_type_index_precompute_once_v2(
        &self,
        key: super::super::TypeIndexPrecomputeSupersessionKeyV2,
        enqueued_at: Instant,
    ) {
        if self
            .type_index_precompute_checkpoint_v2(key, "before_queue")
            .await
        {
            return;
        }

        let queue_wait = enqueued_at.elapsed();
        let queue_wait_ms = queue_wait.as_millis();
        self.coordinator
            .record_intellisense_v2_runtime_queue_wait_latency_with_origin(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                "type_index_precompute",
                queue_wait,
            );

        // The precompute task is spawned right after enqueueing didOpen/didChange into the
        // analysis runtime. Wait until the runtime actually applies the requested revision,
        // otherwise snapshot_with_deps() may observe an older version and treat the precompute
        // as spuriously superseded.
        if !self
            .analysis_v2
            .wait_for_file_version(key.file_id, key.requested_version)
            .await
        {
            debug!(
                file_id = key.file_id.0,
                requested_version = key.requested_version,
                "Event-driven type_index precompute stopped before runtime reached requested version"
            );
            return;
        }
        if self
            .type_index_precompute_checkpoint_v2(key, "after_wait_for_file_version")
            .await
        {
            return;
        }

        let (analysis, _index_snapshot, _deps_id) = self.analysis_v2.snapshot_with_deps().await;
        if self
            .type_index_precompute_checkpoint_v2(key, "before_compute")
            .await
        {
            return;
        }

        let precompute =
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                bsl_runtime::application::CpuWorkClass::Background,
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    analysis.precompute_type_index_for_file(
                        key.file_id,
                        Some(key.requested_version),
                        queue_wait_ms,
                    )
                },
            )
            .await;

        match precompute {
            Ok(Ok(result)) => {
                self.coordinator
                    .record_intellisense_v2_type_index_reason(result.reason_code.as_str());
                if result.stats.evicted_per_file_window_total > 0 {
                    self.coordinator.record_intellisense_v2_type_index_reason(
                        bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedPerFileWindow
                            .as_str(),
                    );
                }
                if result.stats.evicted_global_guard_total > 0 {
                    self.coordinator.record_intellisense_v2_type_index_reason(
                        bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedGlobalGuard
                            .as_str(),
                    );
                }
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute",
                        duration_from_millis_u128(result.stats.exec_ms),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_build",
                        duration_from_millis_u128(result.stats.build_ms),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_ir",
                        duration_from_millis_u128(result.stats.ir_ms),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_ast_to_ir",
                        duration_from_millis_u128(result.stats.ast_to_ir_convert_ms),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_semantic_facts",
                        duration_from_millis_u128(result.stats.semantic_facts_materialize_ms),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_semantic_facts_seed_module_context",
                        duration_from_millis_u128(
                            result.stats.semantic_facts_seed_module_context_ms,
                        ),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_semantic_facts_local_function_summaries",
                        duration_from_millis_u128(
                            result.stats.semantic_facts_local_function_summaries_ms,
                        ),
                    );
                self.coordinator
                    .record_intellisense_v2_runtime_exec_latency_with_origin(
                        bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                        "type_index_precompute_semantic_facts_visit_statements",
                        duration_from_millis_u128(result.stats.semantic_facts_visit_statements_ms),
                    );
                debug!(
                    file_id = key.file_id.0,
                    requested_version = key.requested_version,
                    observed_version = result.file_version,
                    reason_code = result.reason_code.as_str(),
                    queue_wait_ms = result.stats.queue_wait_ms,
                    exec_ms = result.stats.exec_ms,
                    ir_ms = result.stats.ir_ms,
                    ast_to_ir_convert_ms = result.stats.ast_to_ir_convert_ms,
                    semantic_facts_materialize_ms = result.stats.semantic_facts_materialize_ms,
                    semantic_facts_seed_module_context_ms =
                        result.stats.semantic_facts_seed_module_context_ms,
                    semantic_facts_local_function_summaries_ms =
                        result.stats.semantic_facts_local_function_summaries_ms,
                    semantic_facts_visit_statements_ms =
                        result.stats.semantic_facts_visit_statements_ms,
                    semantic_facts_visit_callable_body_ms =
                        result.stats.semantic_facts_visit_callable_body_ms,
                    semantic_facts_visit_callable_body_count =
                        result.stats.semantic_facts_visit_callable_body_count,
                    semantic_facts_merge_control_flow_env_ms =
                        result.stats.semantic_facts_merge_control_flow_env_ms,
                    semantic_facts_merge_control_flow_env_count =
                        result.stats.semantic_facts_merge_control_flow_env_count,
                    semantic_facts_statement_count = result.stats.semantic_facts_statement_count,
                    semantic_facts_local_function_summary_count =
                        result.stats.semantic_facts_local_function_summary_count,
                    semantic_facts_index_entry_count =
                        result.stats.semantic_facts_index_entry_count,
                    build_ms = result.stats.build_ms,
                    evicted_per_file_window_total = result.stats.evicted_per_file_window_total,
                    evicted_global_guard_total = result.stats.evicted_global_guard_total,
                    "Event-driven type_index precompute finished"
                );
            }
            Ok(Err(_cancelled)) => {
                self.coordinator.record_intellisense_v2_type_index_reason(
                    bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeCancelled
                        .as_str(),
                );
                debug!(
                    file_id = key.file_id.0,
                    requested_version = key.requested_version,
                    reason_code = bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeCancelled.as_str(),
                    "Event-driven type_index precompute cancelled"
                );
            }
            Err(join_error) => {
                warn!(
                    file_id = key.file_id.0,
                    requested_version = key.requested_version,
                    error = %join_error,
                    "Event-driven type_index precompute task failed"
                );
            }
        }
    }

    async fn finalize_type_index_precompute_task_v2(
        &self,
        key: super::super::TypeIndexPrecomputeSupersessionKeyV2,
    ) {
        let mut tasks = self.type_index_precompute_tasks_v2.lock().await;
        if tasks
            .get(&key.file_id)
            .is_some_and(|task| task.supersession_key == key)
        {
            tasks.remove(&key.file_id);
        }
    }
}
