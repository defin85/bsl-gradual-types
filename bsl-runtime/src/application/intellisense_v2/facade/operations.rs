use super::*;

#[cfg(debug_assertions)]
async fn maybe_inject_completion_current_revision_snapshot_delay_for_test() {
    let delay_ms = std::env::var("BSL_TEST_COMPLETION_CURRENT_REVISION_SNAPSHOT_DELAY_MS")
        .ok()
        .or_else(|| std::env::var("BSL_TEST_AGED_NON_MEMBER_EXACT_REPROBE_DELAY_MS").ok())
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    let Some(delay_ms) = delay_ms else {
        return;
    };
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
}

#[cfg(not(debug_assertions))]
async fn maybe_inject_completion_current_revision_snapshot_delay_for_test() {}

impl IntellisenseV2Facade {
    pub fn completion_support_bundle(&self) -> CompletionSupportBundle {
        let snapshot = self.inner.completion_deps_index_snapshot.load_full();
        CompletionSupportBundle {
            deps: snapshot.deps.clone(),
            deps_id: snapshot.deps_id.clone(),
            index_snapshot: snapshot.index_snapshot.clone(),
        }
    }

    fn operation_requires_exact_type_index(operation: SemanticOperation) -> bool {
        matches!(
            operation,
            SemanticOperation::Completion
                | SemanticOperation::Hover
                | SemanticOperation::Members
                | SemanticOperation::TypeAtPosition
                | SemanticOperation::SignatureHelp
                | SemanticOperation::Definition
        )
    }

    fn should_eager_warm_exact_type_index(context: &ExecutionContext) -> bool {
        if !Self::operation_requires_exact_type_index(context.operation) {
            return false;
        }

        // LSP interactive semantics use bounded current-revision readiness checks and explicit
        // fail-closed handling at the feature layer. Stateful prepare must not silently
        // materialize the exact artifact on the request path; otherwise cache-miss probes for
        // completion, hover, definition, signature help, and type-at-position turn into hidden
        // cold rebuilds instead of observing readiness as-is.
        !matches!(context.origin, ObservabilityOrigin::Lsp)
    }

    fn classify_completion_first_response_readiness(
        analysis: &AnalysisV2,
        file_id: FileId,
    ) -> CompletionFirstResponseReadiness {
        let exact_ready = analysis
            .current_type_index_serve_only_ready(file_id)
            .ok()
            .unwrap_or(false);
        if exact_ready {
            return CompletionFirstResponseReadiness::ExactReady;
        }

        let head_ready = analysis
            .current_completion_head_ready(file_id)
            .ok()
            .unwrap_or(false);
        if head_ready {
            CompletionFirstResponseReadiness::HeadReady
        } else {
            CompletionFirstResponseReadiness::NotReady
        }
    }

    pub async fn snapshot_for_origin_and_operation(
        &self,
        origin: ObservabilityOrigin,
        operation: SemanticOperation,
    ) -> SemanticSnapshot {
        let queue_priority = RuntimeQueuePriority::for_operation(operation);
        let snapshot_with_deps = self
            .snapshot_with_deps_with_priority(origin, queue_priority, None)
            .await;
        SemanticSnapshot {
            analysis: snapshot_with_deps.analysis,
            deps_id: snapshot_with_deps.deps_id,
        }
    }

    pub async fn snapshot_for_operation(&self, operation: SemanticOperation) -> SemanticSnapshot {
        self.snapshot_for_origin_and_operation(ObservabilityOrigin::Runtime, operation)
            .await
    }

    pub async fn wait_for_file_version_for_operation(
        &self,
        origin: ObservabilityOrigin,
        operation: SemanticOperation,
        file_id: FileId,
        min_version: i32,
    ) -> bool {
        self.wait_for_file_version_with_priority(
            origin,
            RuntimeQueuePriority::for_operation(operation),
            file_id,
            min_version,
        )
        .await
        .ready
    }

    pub async fn completion_current_revision_snapshot_for_origin_and_operation(
        &self,
        origin: ObservabilityOrigin,
        operation: SemanticOperation,
    ) -> CompletionCurrentRevisionSnapshot {
        maybe_inject_completion_current_revision_snapshot_delay_for_test().await;
        let queue_priority = RuntimeQueuePriority::for_operation(operation);
        for _ in 0..4 {
            let deps_index_snapshot = self.inner.completion_deps_index_snapshot.load_full();
            let analysis = self.snapshot_with_priority(queue_priority).await;
            let analysis_deps_id = analysis.deps_id().ok();
            if analysis_deps_id.as_ref() == Some(&deps_index_snapshot.deps_id) {
                return CompletionCurrentRevisionSnapshot {
                    analysis,
                    deps_id: deps_index_snapshot.deps_id.clone(),
                    index_snapshot: deps_index_snapshot.index_snapshot.clone(),
                };
            }

            let refreshed_deps_index_snapshot =
                self.inner.completion_deps_index_snapshot.load_full();
            if analysis_deps_id.as_ref() == Some(&refreshed_deps_index_snapshot.deps_id) {
                return CompletionCurrentRevisionSnapshot {
                    analysis,
                    deps_id: refreshed_deps_index_snapshot.deps_id.clone(),
                    index_snapshot: refreshed_deps_index_snapshot.index_snapshot.clone(),
                };
            }
            tokio::task::yield_now().await;
        }

        let snapshot = self
            .snapshot_with_deps_with_priority(origin, queue_priority, None)
            .await;
        CompletionCurrentRevisionSnapshot {
            analysis: snapshot.analysis,
            deps_id: snapshot.deps_id,
            index_snapshot: snapshot.index_snapshot,
        }
    }

    pub async fn prepare_completion_first_response(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
        line: u32,
        column: u32,
    ) -> Result<PreparedCompletionFirstResponse, SemanticOutcome> {
        self.prepare_completion_first_response_with_progress(
            context,
            observability,
            None,
            line,
            column,
        )
        .await
    }

    pub async fn prepare_completion_first_response_with_progress(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
        progress: Option<&PrepareStatefulProgress>,
        line: u32,
        column: u32,
    ) -> Result<PreparedCompletionFirstResponse, SemanticOutcome> {
        debug_assert_eq!(context.operation, SemanticOperation::Completion);

        let interactive_knobs = interactive_freshness_knobs(context.operation, observability);
        let queue_priority = RuntimeQueuePriority::for_operation(context.operation);
        let mut wait_budget_exhausted = false;
        let mut timeout_attribution = None;

        let (wait_elapsed, wait_for_file_version_runtime) = if let Some(min_file_version) =
            context.min_file_version
        {
            if let Some(progress) = progress {
                progress.mark_phase("wait_for_file_version");
            }
            let started = Instant::now();
            let wait_result = if let Some(knobs) = interactive_knobs {
                match tokio::time::timeout(
                    knobs.wait_budget,
                    self.wait_for_file_version_with_priority(
                        context.origin,
                        queue_priority,
                        context.file_id,
                        min_file_version,
                    ),
                )
                .await
                {
                    Ok(wait_result) => Some(wait_result),
                    Err(_) => {
                        wait_budget_exhausted = true;
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_interactive_wait_budget_exhausted();
                        }
                        None
                    }
                }
            } else {
                Some(
                    self.wait_for_file_version_with_priority(
                        context.origin,
                        queue_priority,
                        context.file_id,
                        min_file_version,
                    )
                    .await,
                )
            };
            let elapsed = started.elapsed();
            if wait_budget_exhausted {
                if let Some(knobs) = interactive_knobs {
                    timeout_attribution = Some(PrepareTimeoutAttributionTrace::new(
                        PrepareTimeoutSourceKind::InteractiveWaitBudget,
                        "wait_for_file_version",
                        knobs.wait_budget,
                        elapsed,
                    ));
                }
            }
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    context.completion_mode,
                    elapsed,
                );
            }
            if let Some(progress) = progress {
                progress.mark_wait_completed();
            }
            let wait_ok = wait_result
                .as_ref()
                .map(|reply| reply.ready)
                .unwrap_or(true);
            if !wait_ok {
                if let Some(progress) = progress {
                    progress.mark_phase("stale_version");
                }
                return Err(SemanticOutcome::StaleVersion);
            }
            (Some(elapsed), wait_result.map(|reply| reply.trace))
        } else {
            (None, None)
        };

        if let Some(progress) = progress {
            progress.mark_phase("snapshot_current_revision");
        }
        let snapshot_started = Instant::now();
        let snapshot = self
            .completion_current_revision_snapshot_for_origin_and_operation(
                context.origin,
                context.operation,
            )
            .await;
        if let Some(progress) = progress {
            progress.mark_snapshot_completed();
            progress.mark_phase("deps_guard");
        }

        if let Some(expected_deps_id) = context.expected_deps_id.as_ref() {
            if expected_deps_id != &snapshot.deps_id {
                if let Some(coordinator) = observability {
                    coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        context.completion_mode,
                        snapshot_started.elapsed(),
                    );
                }
                if let Some(progress) = progress {
                    progress.mark_phase("missing_deps");
                }
                return Err(SemanticOutcome::MissingDeps);
            }
        }

        let observed_file_version = snapshot
            .analysis
            .file_version(context.file_id)
            .ok()
            .flatten();
        if let (Some(min_file_version), Some(_knobs)) =
            (context.min_file_version, interactive_knobs)
        {
            if wait_budget_exhausted {
                let record_fallback_unavailable = || {
                    if let Some(coordinator) = observability {
                        coordinator.record_intellisense_v2_completion_fallback_unavailable();
                    }
                };

                if let Some(observed_version) = observed_file_version {
                    if observed_version < min_file_version {
                        let lag_versions = min_file_version.saturating_sub(observed_version);
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_revision_lag(lag_versions);
                        }
                        record_fallback_unavailable();
                        if let Some(coordinator) = observability {
                            coordinator
                                .record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                                    context.origin.as_str(),
                                    context.operation.as_str(),
                                    context.completion_mode,
                                    snapshot_started.elapsed(),
                                );
                        }
                        if let Some(progress) = progress {
                            progress.mark_phase("stale_version");
                        }
                        return Err(SemanticOutcome::StaleVersion);
                    }
                } else {
                    record_fallback_unavailable();
                    if let Some(coordinator) = observability {
                        coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                            context.origin.as_str(),
                            context.operation.as_str(),
                            context.completion_mode,
                            snapshot_started.elapsed(),
                        );
                    }
                    if let Some(progress) = progress {
                        progress.mark_phase("stale_version");
                    }
                    return Err(SemanticOutcome::StaleVersion);
                }
            } else if observed_file_version.is_some_and(|version| version < min_file_version) {
                if let Some(coordinator) = observability {
                    coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        context.completion_mode,
                        snapshot_started.elapsed(),
                    );
                }
                if let Some(progress) = progress {
                    progress.mark_phase("stale_version");
                }
                return Err(SemanticOutcome::StaleVersion);
            }
        }

        let snapshot_elapsed = snapshot_started.elapsed();
        if let Some(coordinator) = observability {
            coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                context.origin.as_str(),
                context.operation.as_str(),
                context.completion_mode,
                snapshot_elapsed,
            );
        }
        if let Some(progress) = progress {
            progress.mark_phase("ready");
        }

        let settings_id = snapshot.analysis.settings_id().ok();
        let file_content = snapshot.analysis.file_text(context.file_id).ok().flatten();
        let file_path = snapshot.analysis.file_path(context.file_id).ok().flatten();
        let head_ready = snapshot
            .analysis
            .current_completion_head_ready(context.file_id)
            .ok()
            .unwrap_or(false);
        let exact_ready = snapshot
            .analysis
            .current_type_index_serve_only_ready(context.file_id)
            .ok()
            .unwrap_or(false);
        let head_owner_type_hints = if head_ready {
            observed_file_version
                .zip(settings_id.as_ref())
                .zip(file_content.as_deref())
                .map(
                    |((file_version, settings_id), file_content)| {
                        crate::application::type_system::
                            completion_member_access_owner_type_hints_from_completion_head_for_version(
                                &snapshot.analysis,
                                context.file_id,
                                crate::application::type_system::
                                    CompletionHeadTypeHintsForVersionRequest {
                                        file_version,
                                        deps_id: &snapshot.deps_id,
                                        settings_id,
                                        file_content,
                                        line,
                                        column,
                                    },
                            )
                    },
                )
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let deps = {
            let support_bundle = self.completion_support_bundle();
            if support_bundle.deps_id == snapshot.deps_id {
                Some(support_bundle.deps)
            } else {
                snapshot.analysis.deps_data().ok()
            }
        };

        Ok(PreparedCompletionFirstResponse {
            readiness: Self::classify_completion_first_response_readiness(
                &snapshot.analysis,
                context.file_id,
            ),
            support: CompletionFirstResponseSupport {
                deps,
                deps_id: snapshot.deps_id,
                index_snapshot: snapshot.index_snapshot,
                settings_id,
                file_content,
                file_path,
                head_owner_type_hints,
                head_ready,
                exact_ready,
            },
            wait_elapsed,
            snapshot_elapsed,
            wait_for_file_version_runtime,
            timeout_attribution,
            wait_budget_exhausted,
            observed_file_version,
        })
    }

    /// Canonical stateful operation preparation for adapters:
    /// wait-for-version -> snapshot-with-deps -> deps guard check.
    pub async fn prepare_stateful_operation(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        self.prepare_stateful_operation_with_admission_lane_and_progress(
            context,
            observability,
            None,
            None,
        )
        .await
    }

    pub async fn prepare_stateful_operation_with_progress(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
        progress: Option<&PrepareStatefulProgress>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        self.prepare_stateful_operation_with_admission_lane_and_progress(
            context,
            observability,
            None,
            progress,
        )
        .await
    }

    pub async fn prepare_stateful_operation_with_admission_lane(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
        admission_lane: Option<AdmissionLane>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        self.prepare_stateful_operation_with_admission_lane_and_progress(
            context,
            observability,
            admission_lane,
            None,
        )
        .await
    }

    pub async fn prepare_stateful_operation_with_admission_lane_and_progress(
        &self,
        context: &ExecutionContext,
        observability: Option<&SystemCoordinator>,
        admission_lane: Option<AdmissionLane>,
        progress: Option<&PrepareStatefulProgress>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        let interactive_knobs = interactive_freshness_knobs(context.operation, observability);
        let fastpath_preconditions = completion_fastpath_preconditions(
            context.operation,
            context.completion_large_churn_active,
            context.min_file_version,
            context.expected_deps_id.is_some(),
            interactive_knobs.is_some(),
        );
        let queue_priority = RuntimeQueuePriority::for_operation(context.operation);
        let mut wait_budget_exhausted = false;
        let stale_served = false;
        let mut timeout_attribution = None;

        let (wait_elapsed, wait_for_file_version_runtime) = if let Some(min_file_version) =
            context.min_file_version
        {
            if let Some(progress) = progress {
                progress.mark_phase("wait_for_file_version");
            }
            let started = Instant::now();
            let wait_result = if let Some(knobs) = interactive_knobs {
                match tokio::time::timeout(
                    knobs.wait_budget,
                    self.wait_for_file_version_with_priority_and_lane(
                        context.origin,
                        queue_priority,
                        context.file_id,
                        min_file_version,
                        admission_lane,
                    ),
                )
                .await
                {
                    Ok(wait_result) => Some(wait_result),
                    Err(_) => {
                        wait_budget_exhausted = true;
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_interactive_wait_budget_exhausted();
                        }
                        None
                    }
                }
            } else {
                Some(
                    self.wait_for_file_version_with_priority_and_lane(
                        context.origin,
                        queue_priority,
                        context.file_id,
                        min_file_version,
                        admission_lane,
                    )
                    .await,
                )
            };
            let elapsed = started.elapsed();
            if wait_budget_exhausted {
                if let Some(knobs) = interactive_knobs {
                    timeout_attribution = Some(PrepareTimeoutAttributionTrace::new(
                        PrepareTimeoutSourceKind::InteractiveWaitBudget,
                        "wait_for_file_version",
                        knobs.wait_budget,
                        elapsed,
                    ));
                }
            }
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_wait_for_file_version_with_origin_and_mode(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    context.completion_mode,
                    elapsed,
                );
            }
            if let Some(progress) = progress {
                progress.mark_wait_completed();
            }
            let wait_ok = wait_result
                .as_ref()
                .map(|reply| reply.ready)
                .unwrap_or(true);
            if !wait_ok {
                if let Some(progress) = progress {
                    progress.mark_phase("stale_version");
                }
                return Err(SemanticOutcome::StaleVersion);
            }
            (Some(elapsed), wait_result.map(|reply| reply.trace))
        } else {
            (None, None)
        };

        if let Some(progress) = progress {
            progress.mark_phase("snapshot_with_deps");
            progress.mark_snapshot_with_deps_queue_wait();
        }
        let snapshot_started = Instant::now();
        let snapshot_with_deps = self
            .snapshot_with_deps_with_priority_and_lane(
                context.origin,
                queue_priority,
                admission_lane,
                progress.cloned(),
            )
            .await;
        let snapshot_with_deps_runtime = snapshot_with_deps.trace;
        let analysis = snapshot_with_deps.analysis;
        let index_snapshot = snapshot_with_deps.index_snapshot;
        let deps_id = snapshot_with_deps.deps_id;
        if let Some(progress) = progress {
            progress.mark_snapshot_completed();
            progress.mark_phase("deps_guard");
        }

        if let Some(expected_deps_id) = context.expected_deps_id.as_ref() {
            if expected_deps_id != &deps_id {
                if let Some(coordinator) = observability {
                    coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        context.completion_mode,
                        snapshot_started.elapsed(),
                    );
                }
                if let Some(progress) = progress {
                    progress.mark_phase("missing_deps");
                }
                return Err(SemanticOutcome::MissingDeps);
            }
        }

        let observed_file_version = analysis.file_version(context.file_id).ok().flatten();
        if let (Some(min_file_version), Some(knobs)) = (context.min_file_version, interactive_knobs)
        {
            if wait_budget_exhausted {
                let completion_fallback_metric_enabled =
                    fastpath_preconditions.operation_is_completion;
                let record_completion_fallback_unavailable = || {
                    if completion_fallback_metric_enabled {
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_completion_fallback_unavailable();
                        }
                    }
                };

                if let Some(observed_version) = observed_file_version {
                    if observed_version < min_file_version {
                        let lag_versions = min_file_version.saturating_sub(observed_version);
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_revision_lag(lag_versions);
                        }
                        let _ = knobs;
                        record_completion_fallback_unavailable();
                        if let Some(coordinator) = observability {
                            coordinator
                                .record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                                    context.origin.as_str(),
                                    context.operation.as_str(),
                                    context.completion_mode,
                                    snapshot_started.elapsed(),
                                );
                        }
                        if let Some(progress) = progress {
                            progress.mark_phase("stale_version");
                        }
                        return Err(SemanticOutcome::StaleVersion);
                    }
                } else {
                    record_completion_fallback_unavailable();
                    if let Some(coordinator) = observability {
                        coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                            context.origin.as_str(),
                            context.operation.as_str(),
                            context.completion_mode,
                            snapshot_started.elapsed(),
                        );
                    }
                    if let Some(progress) = progress {
                        progress.mark_phase("stale_version");
                    }
                    return Err(SemanticOutcome::StaleVersion);
                }
            } else if observed_file_version.is_some_and(|version| version < min_file_version) {
                if let Some(coordinator) = observability {
                    coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        context.completion_mode,
                        snapshot_started.elapsed(),
                    );
                }
                if let Some(progress) = progress {
                    progress.mark_phase("stale_version");
                }
                return Err(SemanticOutcome::StaleVersion);
            }
        }

        if Self::should_eager_warm_exact_type_index(context) {
            if let Some(progress) = progress {
                progress.mark_phase("exact_type_index_warm");
            }
            if let Some(file_version) = observed_file_version {
                let exact_ready = analysis
                    .current_type_index_serve_only_ready(context.file_id)
                    .ok()
                    .unwrap_or(false);
                if !exact_ready {
                    let precompute_started = Instant::now();
                    if let Ok(precompute) = analysis.precompute_type_index_for_file(
                        context.file_id,
                        Some(file_version),
                        0,
                    ) {
                        if let Some(coordinator) = observability {
                            coordinator.record_intellisense_v2_type_index_reason(
                                precompute.reason_code.as_str(),
                            );
                            if precompute.stats.evicted_per_file_window_total > 0 {
                                coordinator.record_intellisense_v2_type_index_reason(
                                    bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedPerFileWindow
                                        .as_str(),
                                );
                            }
                            if precompute.stats.evicted_global_guard_total > 0 {
                                coordinator.record_intellisense_v2_type_index_reason(
                                    bsl_analysis_v2::TypeIndexArtifactReasonCode::TypeIndexArtifactEvictedGlobalGuard
                                        .as_str(),
                                );
                            }
                            coordinator.record_intellisense_v2_runtime_exec_latency_with_origin(
                                context.origin.as_str(),
                                "type_index_precompute",
                                precompute_started.elapsed(),
                            );
                        }
                    }
                }
            }
        }

        let snapshot_elapsed = snapshot_started.elapsed();
        if let Some(coordinator) = observability {
            coordinator.record_intellisense_v2_snapshot_latency_with_origin_and_mode(
                context.origin.as_str(),
                context.operation.as_str(),
                context.completion_mode,
                snapshot_elapsed,
            );
        }
        if let Some(progress) = progress {
            progress.mark_phase("ready");
        }

        Ok(PreparedOperationSnapshot {
            snapshot: SemanticSnapshot { analysis, deps_id },
            index_snapshot,
            wait_elapsed,
            snapshot_elapsed,
            wait_for_file_version_runtime,
            snapshot_with_deps_runtime,
            timeout_attribution,
            wait_budget_exhausted,
            stale_served,
            completion_churn_fastpath_active: fastpath_preconditions.churn_aware_fastpath_active(),
            observed_file_version,
        })
    }

    /// Canonical ephemeral operation preparation for one-shot adapters:
    /// snapshot build -> deps guard check.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_ephemeral_operation(
        context: &ExecutionContext,
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        index_snapshot: Arc<IndexSnapshot>,
        file_text: Arc<str>,
        file_version: i32,
        file_path: Arc<str>,
        observability: Option<&SystemCoordinator>,
    ) -> Result<PreparedOperationSnapshot, SemanticOutcome> {
        let snapshot_started = Instant::now();
        let snapshot = Self::ephemeral_snapshot(
            deps_id,
            deps,
            index_snapshot.clone(),
            context.settings.clone(),
            context.file_id,
            file_text,
            file_version,
            file_path,
        );
        let snapshot_elapsed = snapshot_started.elapsed();
        if let Some(coordinator) = observability {
            coordinator.record_intellisense_v2_snapshot_latency_with_origin(
                context.origin.as_str(),
                context.operation.as_str(),
                snapshot_elapsed,
            );
        }

        if let Some(expected_deps_id) = context.expected_deps_id.as_ref() {
            if expected_deps_id != &snapshot.deps_id {
                return Err(SemanticOutcome::MissingDeps);
            }
        }

        if Self::operation_requires_exact_type_index(context.operation) {
            let _ = snapshot.analysis.precompute_type_index_for_file(
                context.file_id,
                Some(file_version),
                0,
            );
        }

        Ok(PreparedOperationSnapshot {
            snapshot,
            index_snapshot,
            wait_elapsed: None,
            snapshot_elapsed,
            wait_for_file_version_runtime: None,
            snapshot_with_deps_runtime: SnapshotWithDepsRuntimeTrace::default(),
            timeout_attribution: None,
            wait_budget_exhausted: false,
            stale_served: false,
            completion_churn_fastpath_active: false,
            observed_file_version: Some(file_version),
        })
    }

    /// Run an optional semantic query with shared stage-level observability hooks.
    pub fn run_optional_query<T, E, F>(
        context: &ExecutionContext,
        stage: ObservabilityStage,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        query: F,
    ) -> Result<Option<T>, E>
    where
        F: FnOnce(&AnalysisV2) -> Result<Option<T>, E>,
    {
        let started = Instant::now();
        let raw_result = query(analysis);
        let elapsed = started.elapsed();
        let query_cancelled = raw_result.is_err();
        let report_cancelled =
            query_cancelled && !matches!(context.cancellation, CancellationPolicy::Ignore);
        let result = match raw_result {
            Ok(value) => Ok(value),
            Err(err) => match context.cancellation {
                CancellationPolicy::RespectClientAbort => Err(err),
                CancellationPolicy::BestEffort | CancellationPolicy::Ignore => Ok(None),
            },
        };

        if let Some(coordinator) = observability {
            match stage {
                ObservabilityStage::IrQuery => {
                    coordinator.record_intellisense_v2_ir_query_latency_with_origin_and_mode(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        context.completion_mode,
                        elapsed,
                    );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_ir_query_cancelled_with_origin_and_mode(
                            context.origin.as_str(),
                            context.operation.as_str(),
                            context.completion_mode,
                        );
                    }
                }
                ObservabilityStage::SyntaxDiagnosticsQuery => {
                    let syntax_mode = analysis
                        .syntax_diagnostics_observability_mode(context.file_id)
                        .ok()
                        .flatten()
                        .unwrap_or("other");
                    coordinator
                        .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
                            context.origin.as_str(),
                            syntax_mode,
                            elapsed,
                        );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_query_cancelled_with_origin(
                            context.origin.as_str(),
                            "syntax",
                        );
                    }
                }
                ObservabilityStage::SemanticDiagnosticsQuery => {
                    coordinator
                        .record_intellisense_v2_semantic_diagnostics_query_latency_with_origin(
                            context.origin.as_str(),
                            elapsed,
                        );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_query_cancelled_with_origin(
                            context.origin.as_str(),
                            "semantic",
                        );
                    }
                }
                ObservabilityStage::ParseResultQuery => {
                    coordinator
                        .record_intellisense_v2_parse_result_query_latency_with_origin_operation_and_mode(
                            context.origin.as_str(),
                            context.operation.as_str(),
                            context.completion_mode,
                            elapsed,
                        );
                    if report_cancelled {
                        coordinator.record_intellisense_v2_query_cancelled_with_origin_and_mode(
                            context.origin.as_str(),
                            "other",
                            context.completion_mode,
                        );
                    }
                }
                ObservabilityStage::RuntimeQueueWait
                | ObservabilityStage::RuntimeWaitForFileVersion
                | ObservabilityStage::RuntimeSnapshotWithDeps => {}
            }
        }

        result
    }

    /// Run parse_result according to centralized lazy policy and shared stage hooks.
    pub fn run_parse_result_query<T, E, F>(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        ir_available: bool,
        observability: Option<&SystemCoordinator>,
        query: F,
    ) -> Result<Option<T>, E>
    where
        F: FnOnce(&AnalysisV2) -> Result<Option<T>, E>,
    {
        if !should_query_parse_result(context.operation, ir_available) {
            return Ok(None);
        }
        Self::run_optional_query(
            context,
            ObservabilityStage::ParseResultQuery,
            analysis,
            observability,
            query,
        )
    }

    pub fn run_ir_query_singleflight(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
    ) -> Result<Option<Arc<SemanticProgram>>, SingleflightQueryError> {
        Self::run_ir_query_singleflight_with_cancellation(
            context,
            analysis,
            observability,
            file_id,
            None,
        )
    }

    pub fn run_ir_query_singleflight_with_cancellation(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
        external_cancellation: Option<bsl_analysis_v2::ExternalCancellationCheck>,
    ) -> Result<Option<Arc<SemanticProgram>>, SingleflightQueryError> {
        let key = Self::singleflight_revision_key(analysis, file_id, SingleflightQueryKind::Ir);
        Self::run_optional_query(
            context,
            ObservabilityStage::IrQuery,
            analysis,
            observability,
            |_analysis| {
                if let Some(key) = key {
                    Self::run_singleflight_query_with_wait_cancellation(
                        &IR_FLIGHTS,
                        key,
                        context.origin,
                        SingleflightQueryKind::Ir,
                        observability,
                        external_cancellation.as_ref(),
                        || {
                            bsl_analysis_v2::with_external_cancellation_check(
                                external_cancellation.clone(),
                                || {
                                    analysis
                                        .ir(file_id)
                                        .map_err(|_| SingleflightQueryError::Cancelled)
                                },
                            )
                        },
                    )
                } else {
                    if let Some(coordinator) = observability {
                        coordinator
                            .record_intellisense_v2_singleflight_key_unavailable_with_origin(
                                context.origin.as_str(),
                                SingleflightQueryKind::Ir.as_str(),
                            );
                    }
                    bsl_analysis_v2::with_external_cancellation_check(external_cancellation, || {
                        analysis
                            .ir(file_id)
                            .map_err(|_| SingleflightQueryError::Cancelled)
                    })
                }
            },
        )
    }

    pub fn run_parse_result_query_singleflight(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        ir_available: bool,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
    ) -> Result<Option<Arc<bsl_syntax::ast::ParseResult>>, SingleflightQueryError> {
        if !should_query_parse_result(context.operation, ir_available) {
            return Ok(None);
        }
        let key =
            Self::singleflight_revision_key(analysis, file_id, SingleflightQueryKind::ParseResult);
        Self::run_optional_query(
            context,
            ObservabilityStage::ParseResultQuery,
            analysis,
            observability,
            |_analysis| {
                if let Some(key) = key {
                    Self::run_singleflight_query(
                        &PARSE_RESULT_FLIGHTS,
                        key,
                        context.origin,
                        SingleflightQueryKind::ParseResult,
                        observability,
                        || {
                            analysis
                                .parse_result(file_id)
                                .map_err(|_| SingleflightQueryError::Cancelled)
                        },
                    )
                } else {
                    if let Some(coordinator) = observability {
                        coordinator
                            .record_intellisense_v2_singleflight_key_unavailable_with_origin(
                                context.origin.as_str(),
                                SingleflightQueryKind::ParseResult.as_str(),
                            );
                    }
                    analysis
                        .parse_result(file_id)
                        .map_err(|_| SingleflightQueryError::Cancelled)
                }
            },
        )
    }

    pub fn run_syntax_diagnostics_query_singleflight(
        context: &ExecutionContext,
        analysis: &AnalysisV2,
        observability: Option<&SystemCoordinator>,
        file_id: FileId,
    ) -> Result<Option<Arc<Vec<ParseError>>>, SingleflightQueryError> {
        let key = Self::singleflight_revision_key(
            analysis,
            file_id,
            SingleflightQueryKind::SyntaxDiagnostics,
        );
        Self::run_optional_query(
            context,
            ObservabilityStage::SyntaxDiagnosticsQuery,
            analysis,
            observability,
            |_analysis| {
                if let Some(key) = key {
                    Self::run_singleflight_query(
                        &SYNTAX_DIAGNOSTICS_FLIGHTS,
                        key,
                        context.origin,
                        SingleflightQueryKind::SyntaxDiagnostics,
                        observability,
                        || {
                            analysis
                                .syntax_diagnostics(file_id)
                                .map_err(|_| SingleflightQueryError::Cancelled)
                        },
                    )
                } else {
                    if let Some(coordinator) = observability {
                        coordinator
                            .record_intellisense_v2_singleflight_key_unavailable_with_origin(
                                context.origin.as_str(),
                                SingleflightQueryKind::SyntaxDiagnostics.as_str(),
                            );
                    }
                    analysis
                        .syntax_diagnostics(file_id)
                        .map_err(|_| SingleflightQueryError::Cancelled)
                }
            },
        )
    }

    fn singleflight_revision_key(
        analysis: &AnalysisV2,
        file_id: FileId,
        query_kind: SingleflightQueryKind,
    ) -> Option<SingleflightRevisionKey> {
        let file_version = analysis.file_version(file_id).ok().flatten()?;
        let file_signature = Self::singleflight_file_signature(analysis, file_id)?;
        let (deps_id, settings_id) = if Self::singleflight_requires_snapshot_identity(query_kind) {
            (
                Some(analysis.deps_id().ok()?),
                Some(analysis.settings_id().ok()?),
            )
        } else {
            (None, None)
        };
        Some(SingleflightRevisionKey {
            file_id,
            file_version,
            file_signature,
            deps_id,
            settings_id,
            query_kind,
        })
    }

    pub(super) fn singleflight_requires_snapshot_identity(
        query_kind: SingleflightQueryKind,
    ) -> bool {
        matches!(query_kind, SingleflightQueryKind::Ir)
    }

    fn singleflight_file_signature(analysis: &AnalysisV2, file_id: FileId) -> Option<String> {
        if let Some(path) = analysis.file_path(file_id).ok().flatten() {
            return Some(format!("path:{path}"));
        }
        let text = analysis.file_text(file_id).ok().flatten()?;
        Some(format!("text:{}", blake3::hash(text.as_bytes()).to_hex()))
    }

    pub(super) fn run_singleflight_query<T>(
        flights: &OnceLock<SingleflightMap<T>>,
        key: SingleflightRevisionKey,
        origin: ObservabilityOrigin,
        query_kind: SingleflightQueryKind,
        observability: Option<&SystemCoordinator>,
        query: impl FnOnce() -> Result<Option<T>, SingleflightQueryError>,
    ) -> Result<Option<T>, SingleflightQueryError>
    where
        T: Clone,
    {
        Self::run_singleflight_query_with_wait_cancellation(
            flights,
            key,
            origin,
            query_kind,
            observability,
            None,
            query,
        )
    }

    fn run_singleflight_query_with_wait_cancellation<T>(
        flights: &OnceLock<SingleflightMap<T>>,
        key: SingleflightRevisionKey,
        origin: ObservabilityOrigin,
        query_kind: SingleflightQueryKind,
        observability: Option<&SystemCoordinator>,
        wait_cancellation: Option<&bsl_analysis_v2::ExternalCancellationCheck>,
        query: impl FnOnce() -> Result<Option<T>, SingleflightQueryError>,
    ) -> Result<Option<T>, SingleflightQueryError>
    where
        T: Clone,
    {
        let flights = flights.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        let (flight, is_leader) = {
            let mut guard = flights
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(existing) = guard.get(&key) {
                (existing.clone(), false)
            } else {
                let created = Arc::new(SingleflightFlight::new());
                guard.insert(key.clone(), created.clone());
                (created, true)
            }
        };

        if is_leader {
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_singleflight_leader_with_origin(
                    origin.as_str(),
                    query_kind.as_str(),
                );
            }
            let result = match catch_unwind(AssertUnwindSafe(query)) {
                Ok(result) => result,
                Err(_panic_payload) => Err(SingleflightQueryError::Cancelled),
            };
            {
                let mut state = flight
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.terminal_outcome = Some(match &result {
                    Ok(value) => SingleflightTerminalOutcome::Success(value.clone()),
                    Err(err) => SingleflightTerminalOutcome::Error(*err),
                });
                state.in_progress = false;
            }
            flight.cv.notify_all();
            let mut guard = flights
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.remove(&key);
            result
        } else {
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_singleflight_shared_with_origin(
                    origin.as_str(),
                    query_kind.as_str(),
                );
            }
            let wait_started = Instant::now();
            let mut state = flight
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            while state.in_progress {
                if wait_cancellation.is_some_and(|check| check.is_cancelled()) {
                    return Err(SingleflightQueryError::Cancelled);
                }
                if wait_cancellation.is_some() {
                    let (next_state, _) = flight
                        .cv
                        .wait_timeout(state, std::time::Duration::from_millis(10))
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state = next_state;
                } else {
                    state = flight
                        .cv
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
            }
            if let Some(coordinator) = observability {
                coordinator.record_intellisense_v2_singleflight_wait_latency_with_origin(
                    origin.as_str(),
                    query_kind.as_str(),
                    wait_started.elapsed(),
                );
            }
            match state.terminal_outcome.clone() {
                Some(SingleflightTerminalOutcome::Success(shared)) => Ok(shared),
                Some(SingleflightTerminalOutcome::Error(err)) => Err(err),
                None => Err(SingleflightQueryError::Cancelled),
            }
        }
    }

    /// One-shot helper for ephemeral adapters (e.g. web handlers).
    /// Builds a semantic snapshot without creating a long-lived writer-thread runtime.
    #[allow(clippy::too_many_arguments)]
    pub fn ephemeral_snapshot(
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
        _index_snapshot: Arc<IndexSnapshot>,
        settings: ExecutionSettings,
        file_id: FileId,
        file_text: Arc<str>,
        file_version: i32,
        file_path: Arc<str>,
    ) -> SemanticSnapshot {
        let mut host = AnalysisHostV2::default();
        host.apply_change(Change::SetDepsSnapshot {
            deps_id: deps_id.clone(),
            deps,
        });
        host.apply_change(Change::SetSettingsSnapshot {
            settings_id: settings.settings_id,
            diagnostics_detail_level: settings.diagnostics_detail_level,
        });
        host.apply_change(Change::SetFile {
            file_id,
            text: file_text,
            version: file_version,
            path: file_path,
        });

        SemanticSnapshot {
            analysis: host.snapshot(),
            deps_id,
        }
    }
}
