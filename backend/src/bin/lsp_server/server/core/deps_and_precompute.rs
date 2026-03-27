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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionArtifactWaitOutcomeV2 {
    HeadReady,
    ExactReady,
    Deadline,
    ObservedVersionMismatch,
}

impl CompletionArtifactWaitOutcomeV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::HeadReady => "head_ready",
            Self::ExactReady => "exact_ready",
            Self::Deadline => "deadline",
            Self::ObservedVersionMismatch => "observed_version_mismatch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionArtifactPollTraceV2 {
    pub poll_count: u64,
    pub poll_elapsed: Duration,
    pub observed_file_version: Option<i32>,
    pub head_ready: Option<bool>,
    pub exact_ready: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionArtifactWaitTraceV2 {
    pub outcome: CompletionArtifactWaitOutcomeV2,
    pub poll_trace: CompletionArtifactPollTraceV2,
}

#[cfg(test)]
fn test_type_index_precompute_delay() -> Option<std::time::Duration> {
    std::env::var("BSL_TEST_TYPE_INDEX_PRECOMPUTE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(std::time::Duration::from_millis)
}

#[cfg(not(test))]
fn test_type_index_precompute_delay() -> Option<std::time::Duration> {
    None
}

#[cfg(test)]
fn test_type_index_precompute_post_compute_delay() -> Option<std::time::Duration> {
    std::env::var("BSL_TEST_TYPE_INDEX_PRECOMPUTE_POST_COMPUTE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map(std::time::Duration::from_millis)
}

#[cfg(not(test))]
fn test_type_index_precompute_post_compute_delay() -> Option<std::time::Duration> {
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeIndexPrecomputeWaiterActionV2 {
    None,
    Joined,
    Promoted,
}

impl TypeIndexPrecomputeWaiterActionV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Joined => "joined",
            Self::Promoted => "promoted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExactTypeIndexMatchingTaskStateV2 {
    Matching,
    WrongVersion,
    Missing,
}

impl ExactTypeIndexMatchingTaskStateV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Matching => "matching",
            Self::WrongVersion => "wrong_version",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactTypeIndexWaitTraceV2 {
    pub outcome: ExactTypeIndexWaitOutcomeV2,
    pub waiter_action: TypeIndexPrecomputeWaiterActionV2,
    pub matching_task_state: Option<ExactTypeIndexMatchingTaskStateV2>,
    pub task_phase: Option<TypeIndexPrecomputePhaseV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeIndexPrecomputePhaseV2 {
    WaitingForVersion = 1,
    Snapshotting = 2,
    WaitingCpuPermit = 3,
    Computing = 4,
    Completed = 5,
}

impl TypeIndexPrecomputePhaseV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::WaitingForVersion => "waiting_for_version",
            Self::Snapshotting => "snapshotting",
            Self::WaitingCpuPermit => "waiting_cpu_permit",
            Self::Computing => "computing",
            Self::Completed => "completed",
        }
    }

    pub(super) fn as_u8(self) -> u8 {
        self as u8
    }

    pub(super) fn from_atomic(value: u8) -> Self {
        match value {
            1 => Self::WaitingForVersion,
            2 => Self::Snapshotting,
            3 => Self::WaitingCpuPermit,
            4 => Self::Computing,
            _ => Self::Completed,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ExactTypeIndexMatchingTaskTraceV2 {
    matching_task_state: ExactTypeIndexMatchingTaskStateV2,
    task_phase: Option<TypeIndexPrecomputePhaseV2>,
}

fn type_index_precompute_debounce_duration() -> Duration {
    Duration::from_millis(25)
}

impl BslLanguageServer {
    fn is_completed_retained_type_index_task(
        task: &super::super::TypeIndexPrecomputeTaskV2,
    ) -> bool {
        task.handle.is_finished()
            && matches!(
                TypeIndexPrecomputePhaseV2::from_atomic(task.phase.load(Ordering::Relaxed)),
                TypeIndexPrecomputePhaseV2::Completed
            )
    }

    pub(crate) async fn cleanup_completed_type_index_precompute_task_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
    ) {
        let mut tasks = self.type_index_precompute_tasks_v2.lock().await;
        let should_remove = tasks.get(&file_id).is_some_and(|task| {
            Self::is_completed_retained_type_index_task(task)
                && expected_version
                    .map(|version| task.supersession_key.requested_version == version)
                    .unwrap_or(true)
        });
        if should_remove {
            let _ = tasks.remove(&file_id);
        }
    }

    async fn snapshot_for_completion_wait_v2(&self) -> bsl_analysis_v2::AnalysisV2 {
        self.analysis_v2
            .completion_current_revision_snapshot_for_origin_and_operation(
                bsl_runtime::application::ObservabilityOrigin::Lsp,
                bsl_runtime::application::SemanticOperation::Completion,
            )
            .await
            .analysis
    }

    fn spawn_type_index_precompute_task_v2(
        &self,
        supersession_key: super::super::TypeIndexPrecomputeSupersessionKeyV2,
        work_class: bsl_runtime::application::CpuWorkClass,
        scheduled_at: Instant,
    ) -> super::super::TypeIndexPrecomputeTaskV2 {
        let task_id = self
            .next_type_index_precompute_task_id
            .fetch_add(1, Ordering::Relaxed);
        let phase = Arc::new(std::sync::atomic::AtomicU8::new(
            TypeIndexPrecomputePhaseV2::WaitingForVersion.as_u8(),
        ));
        let active_requested_version = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let phase_for_task = Arc::clone(&phase);
        let active_requested_version_for_task = Arc::clone(&active_requested_version);
        let server = self.clone();
        let file_id = supersession_key.file_id;
        let handle = tokio::spawn(async move {
            server
                .run_type_index_precompute_task_v2(
                    file_id,
                    task_id,
                    work_class,
                    Arc::clone(&phase_for_task),
                    Arc::clone(&active_requested_version_for_task),
                )
                .await;
        });

        super::super::TypeIndexPrecomputeTaskV2 {
            task_id,
            supersession_key,
            work_class,
            phase,
            active_requested_version,
            scheduled_at,
            handle,
        }
    }

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
        let scheduled_at = Instant::now();
        let mut tasks = self.type_index_precompute_tasks_v2.lock().await;
        if let Some(task) = tasks.get_mut(&file_id) {
            if task.supersession_key == supersession_key {
                return;
            }
            let retained_completed_task = Self::is_completed_retained_type_index_task(task);
            if matches!(
                task.work_class,
                bsl_runtime::application::CpuWorkClass::Background
            ) && !retained_completed_task {
                task.supersession_key = supersession_key;
                task.scheduled_at = scheduled_at;
                return;
            }
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

        tasks.insert(
            file_id,
            self.spawn_type_index_precompute_task_v2(
                supersession_key,
                bsl_runtime::application::CpuWorkClass::Background,
                scheduled_at,
            ),
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

    pub(crate) async fn has_matching_type_index_precompute_task_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
    ) -> bool {
        let tasks = self.type_index_precompute_tasks_v2.lock().await;
        tasks.get(&file_id).is_some_and(|task| {
            expected_version
                .map(|version| task.supersession_key.requested_version == version)
                .unwrap_or(true)
        })
    }

    pub(crate) async fn wait_for_current_type_index_serve_only_ready_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
        max_wait: std::time::Duration,
    ) -> ExactTypeIndexWaitTraceV2 {
        let deadline = tokio::time::Instant::now() + max_wait;
        let mut waiter_action = TypeIndexPrecomputeWaiterActionV2::None;
        loop {
            let analysis = self.snapshot_for_completion_wait_v2().await;
            let observed_version = analysis.file_version(file_id).ok().flatten();
            let exact_ready = expected_version
                .is_none_or(|version| observed_version == Some(version))
                && analysis
                    .current_type_index_serve_only_ready(file_id)
                    .unwrap_or(false);
            if exact_ready {
                let matching_task_trace = self
                    .current_exact_wait_matching_task_trace_v2(file_id, expected_version)
                    .await;
                self.cleanup_completed_type_index_precompute_task_v2(file_id, expected_version)
                    .await;
                if !matches!(waiter_action, TypeIndexPrecomputeWaiterActionV2::None) {
                    self.coordinator
                        .record_intellisense_v2_completion_exact_type_index_wait_ready_after_wait();
                }
                return ExactTypeIndexWaitTraceV2 {
                    outcome: ExactTypeIndexWaitOutcomeV2::Ready,
                    waiter_action,
                    matching_task_state: Some(matching_task_trace.matching_task_state),
                    task_phase: matching_task_trace.task_phase,
                };
            }

            if matches!(waiter_action, TypeIndexPrecomputeWaiterActionV2::None) {
                waiter_action = self
                    .promote_type_index_precompute_for_waiter_v2(file_id, expected_version)
                    .await;
                match waiter_action {
                    TypeIndexPrecomputeWaiterActionV2::Joined => self
                        .coordinator
                        .record_intellisense_v2_completion_exact_type_index_wait_join(),
                    TypeIndexPrecomputeWaiterActionV2::Promoted => self
                        .coordinator
                        .record_intellisense_v2_completion_exact_type_index_wait_promotion(),
                    TypeIndexPrecomputeWaiterActionV2::None => {}
                }
            }

            let matching_task_trace = self
                .current_exact_wait_matching_task_trace_v2(file_id, expected_version)
                .await;
            if matches!(
                matching_task_trace.matching_task_state,
                ExactTypeIndexMatchingTaskStateV2::WrongVersion
            ) {
                return ExactTypeIndexWaitTraceV2 {
                    outcome: ExactTypeIndexWaitOutcomeV2::TaskPresentWrongVersion,
                    waiter_action,
                    matching_task_state: Some(matching_task_trace.matching_task_state),
                    task_phase: matching_task_trace.task_phase,
                };
            }
            let observed_version_mismatch =
                expected_version.is_some_and(|version| observed_version != Some(version));
            if matches!(
                matching_task_trace.matching_task_state,
                ExactTypeIndexMatchingTaskStateV2::Missing
            ) {
                return ExactTypeIndexWaitTraceV2 {
                    outcome: if observed_version_mismatch {
                        ExactTypeIndexWaitOutcomeV2::ObservedVersionMismatch
                    } else {
                        ExactTypeIndexWaitOutcomeV2::NoMatchingTask
                    },
                    waiter_action,
                    matching_task_state: Some(matching_task_trace.matching_task_state),
                    task_phase: matching_task_trace.task_phase,
                };
            }
            if tokio::time::Instant::now() >= deadline {
                return ExactTypeIndexWaitTraceV2 {
                    outcome: if observed_version_mismatch {
                        ExactTypeIndexWaitOutcomeV2::ObservedVersionMismatch
                    } else {
                        ExactTypeIndexWaitOutcomeV2::Deadline
                    },
                    waiter_action,
                    matching_task_state: Some(matching_task_trace.matching_task_state),
                    task_phase: matching_task_trace.task_phase,
                };
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    pub(crate) async fn wait_for_current_completion_artifact_ready_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
        max_wait: std::time::Duration,
    ) -> CompletionArtifactWaitTraceV2 {
        let deadline = tokio::time::Instant::now() + max_wait;
        let started = Instant::now();
        let mut poll_count = 0_u64;
        loop {
            let analysis = self.snapshot_for_completion_wait_v2().await;
            let observed_version = analysis.file_version(file_id).ok().flatten();
            let version_matches = expected_version
                .map(|version| observed_version == Some(version))
                .unwrap_or(true);
            poll_count = poll_count.saturating_add(1);
            let head_ready = analysis
                .current_completion_head_ready(file_id)
                .ok()
                .unwrap_or(false);
            let exact_ready = analysis
                .current_type_index_serve_only_ready(file_id)
                .ok()
                .unwrap_or(false);
            let poll_trace = CompletionArtifactPollTraceV2 {
                poll_count,
                poll_elapsed: started.elapsed(),
                observed_file_version: observed_version,
                head_ready: Some(head_ready),
                exact_ready: Some(exact_ready),
            };

            if version_matches && exact_ready {
                self.cleanup_completed_type_index_precompute_task_v2(file_id, expected_version)
                    .await;
            }

            if version_matches && head_ready {
                return CompletionArtifactWaitTraceV2 {
                    outcome: CompletionArtifactWaitOutcomeV2::HeadReady,
                    poll_trace,
                };
            }
            if version_matches && exact_ready {
                return CompletionArtifactWaitTraceV2 {
                    outcome: CompletionArtifactWaitOutcomeV2::ExactReady,
                    poll_trace,
                };
            }

            if tokio::time::Instant::now() >= deadline {
                return CompletionArtifactWaitTraceV2 {
                    outcome: if expected_version
                        .is_some_and(|version| observed_version != Some(version))
                    {
                        CompletionArtifactWaitOutcomeV2::ObservedVersionMismatch
                    } else {
                        CompletionArtifactWaitOutcomeV2::Deadline
                    },
                    poll_trace,
                };
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    pub(super) async fn promote_type_index_precompute_for_waiter_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
    ) -> TypeIndexPrecomputeWaiterActionV2 {
        let Some(expected_version) = expected_version else {
            return TypeIndexPrecomputeWaiterActionV2::None;
        };

        let mut tasks = self.type_index_precompute_tasks_v2.lock().await;
        let Some(task) = tasks.get(&file_id) else {
            return TypeIndexPrecomputeWaiterActionV2::None;
        };
        if task.supersession_key.requested_version != expected_version {
            return TypeIndexPrecomputeWaiterActionV2::None;
        }
        if matches!(
            task.work_class,
            bsl_runtime::application::CpuWorkClass::Interactive
        ) {
            return TypeIndexPrecomputeWaiterActionV2::Joined;
        }
        let phase = TypeIndexPrecomputePhaseV2::from_atomic(task.phase.load(Ordering::Relaxed));
        let active_requested_version = task.active_requested_version.load(Ordering::Relaxed);
        if matches!(
            phase,
            TypeIndexPrecomputePhaseV2::Computing | TypeIndexPrecomputePhaseV2::Completed
        ) && (active_requested_version == expected_version || active_requested_version == 0)
        {
            return TypeIndexPrecomputeWaiterActionV2::Joined;
        }

        let previous = tasks
            .remove(&file_id)
            .expect("matching type-index precompute task must exist");
        previous.handle.abort();
        tasks.insert(
            file_id,
            self.spawn_type_index_precompute_task_v2(
                previous.supersession_key,
                bsl_runtime::application::CpuWorkClass::Interactive,
                previous.scheduled_at,
            ),
        );
        TypeIndexPrecomputeWaiterActionV2::Promoted
    }

    async fn current_exact_wait_matching_task_trace_v2(
        &self,
        file_id: V2FileId,
        expected_version: Option<i32>,
    ) -> ExactTypeIndexMatchingTaskTraceV2 {
        let tasks = self.type_index_precompute_tasks_v2.lock().await;
        match tasks.get(&file_id) {
            Some(task) => {
                let matching_task_state = if expected_version
                    .map(|version| task.supersession_key.requested_version == version)
                    .unwrap_or(true)
                {
                    ExactTypeIndexMatchingTaskStateV2::Matching
                } else {
                    ExactTypeIndexMatchingTaskStateV2::WrongVersion
                };
                ExactTypeIndexMatchingTaskTraceV2 {
                    matching_task_state,
                    task_phase: Some(TypeIndexPrecomputePhaseV2::from_atomic(
                        task.phase.load(Ordering::Relaxed),
                    )),
                }
            }
            None => ExactTypeIndexMatchingTaskTraceV2 {
                matching_task_state: ExactTypeIndexMatchingTaskStateV2::Missing,
                task_phase: None,
            },
        }
    }

    async fn current_type_index_precompute_task_state_v2(
        &self,
        file_id: V2FileId,
        task_id: u64,
    ) -> Option<(super::super::TypeIndexPrecomputeSupersessionKeyV2, Instant)> {
        let tasks = self.type_index_precompute_tasks_v2.lock().await;
        let task = tasks.get(&file_id)?;
        if task.task_id != task_id {
            return None;
        }
        Some((task.supersession_key, task.scheduled_at))
    }

    async fn run_type_index_precompute_task_v2(
        &self,
        file_id: V2FileId,
        task_id: u64,
        work_class: bsl_runtime::application::CpuWorkClass,
        phase: Arc<std::sync::atomic::AtomicU8>,
        active_requested_version: Arc<std::sync::atomic::AtomicI32>,
    ) {
        loop {
            let file_still_open = self
                .latest_received_file_versions_v2
                .read()
                .await
                .contains_key(&file_id);
            if !file_still_open {
                return;
            }

            let Some((supersession_key, scheduled_at)) = self
                .current_type_index_precompute_task_state_v2(file_id, task_id)
                .await
            else {
                return;
            };

            if matches!(
                work_class,
                bsl_runtime::application::CpuWorkClass::Background
            ) {
                let delay = type_index_precompute_debounce_duration();
                if delay > Duration::ZERO {
                    tokio::time::sleep(delay).await;
                }
                let Some((current_key, current_scheduled_at)) = self
                    .current_type_index_precompute_task_state_v2(file_id, task_id)
                    .await
                else {
                    return;
                };
                if current_key != supersession_key || current_scheduled_at != scheduled_at {
                    continue;
                }
            }

            active_requested_version.store(supersession_key.requested_version, Ordering::Relaxed);
            self.execute_type_index_precompute_once_v2(
                supersession_key,
                work_class,
                Arc::clone(&phase),
                scheduled_at,
            )
            .await;
            active_requested_version.store(0, Ordering::Relaxed);

            let exact_ready_observed = {
                let analysis = self.snapshot_for_completion_wait_v2().await;
                analysis.file_version(file_id).ok().flatten()
                    == Some(supersession_key.requested_version)
                    && analysis
                        .current_type_index_serve_only_ready(file_id)
                        .ok()
                        .unwrap_or(false)
            };

            let mut tasks = self.type_index_precompute_tasks_v2.lock().await;
            let Some(task) = tasks.get(&file_id) else {
                return;
            };
            if task.task_id != task_id {
                return;
            }
            if task.supersession_key == supersession_key && task.work_class == work_class {
                if exact_ready_observed {
                    tasks.remove(&file_id);
                }
                return;
            }
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
        work_class: bsl_runtime::application::CpuWorkClass,
        phase: Arc<std::sync::atomic::AtomicU8>,
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
        phase.store(
            TypeIndexPrecomputePhaseV2::WaitingForVersion.as_u8(),
            Ordering::Relaxed,
        );
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

        phase.store(
            TypeIndexPrecomputePhaseV2::Snapshotting.as_u8(),
            Ordering::Relaxed,
        );
        let (analysis, _index_snapshot, _deps_id) = self.analysis_v2.snapshot_with_deps().await;
        if self
            .type_index_precompute_checkpoint_v2(key, "before_compute")
            .await
        {
            return;
        }

        phase.store(
            TypeIndexPrecomputePhaseV2::WaitingCpuPermit.as_u8(),
            Ordering::Relaxed,
        );
        let phase_for_compute = Arc::clone(&phase);
        let precompute =
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                work_class,
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                Some(self.coordinator.as_ref()),
                move || {
                    phase_for_compute.store(
                        TypeIndexPrecomputePhaseV2::Computing.as_u8(),
                        Ordering::Relaxed,
                    );
                    if let Some(delay) = test_type_index_precompute_delay() {
                        std::thread::sleep(delay);
                    }
                    analysis.precompute_type_index_for_file(
                        key.file_id,
                        Some(key.requested_version),
                        queue_wait_ms,
                    )
                },
            )
            .await;
        phase.store(
            TypeIndexPrecomputePhaseV2::Completed.as_u8(),
            Ordering::Relaxed,
        );
        if let Some(delay) = test_type_index_precompute_post_compute_delay() {
            tokio::time::sleep(delay).await;
        }

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
                if result.reason_code
                    == bsl_analysis_v2::TypeIndexPrecomputeReasonCode::TypeIndexPrecomputeExactStored
                {
                    if let Some(file_version) = result.file_version {
                        let (analysis_after_store, _index_snapshot_after_store, deps_id_after_store) =
                            self.analysis_v2.snapshot_with_deps().await;
                        let observed_version_after_store =
                            analysis_after_store.file_version(key.file_id).ok().flatten();
                        let settings_id_after_store = analysis_after_store.settings_id().ok();
                        let exact_ready_after_store = analysis_after_store
                            .current_type_index_serve_only_ready(key.file_id)
                            .ok()
                            .unwrap_or(false);
                        if observed_version_after_store == Some(file_version)
                            && exact_ready_after_store
                        {
                            let _ = self
                                .record_completion_head_to_exact_upgrade_if_pending_v2(
                                    key.file_id,
                                    file_version,
                                    &deps_id_after_store,
                                    settings_id_after_store.as_ref(),
                                )
                                .await;
                        }
                    }
                }
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
}
