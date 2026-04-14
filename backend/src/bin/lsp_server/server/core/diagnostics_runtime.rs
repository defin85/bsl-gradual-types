use super::*;

#[cfg(debug_assertions)]
fn maybe_inject_save_fastlane_shadow_parse_delay_for_test() {
    let delay_ms = std::env::var("BSL_TEST_SAVE_FASTLANE_PARSE_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    let Some(delay_ms) = delay_ms else {
        return;
    };
    std::thread::sleep(Duration::from_millis(delay_ms));
}

#[cfg(not(debug_assertions))]
fn maybe_inject_save_fastlane_shadow_parse_delay_for_test() {}

enum SaveFastlaneFirstPublishWaitOutcome {
    Published,
    NotPublished,
}

pub(crate) const SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET: Duration =
    Duration::from_millis(3_500);
pub(crate) const SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET: Duration =
    Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadyParseSnapshotProbeSlotV2 {
    ZeroBudget,
    BoundedWait,
    ReliefValve,
}

impl ReadyParseSnapshotProbeSlotV2 {
    fn as_str(self) -> &'static str {
        match self {
            Self::ZeroBudget => "zero_budget",
            Self::BoundedWait => "bounded_wait",
            Self::ReliefValve => "relief_valve",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadyParseSnapshotProbeOutcomeV2 {
    Ready,
    NotReady,
    GenerationMismatch,
    VersionMismatch,
    Timeout,
    Cancelled,
    Superseded,
}

impl ReadyParseSnapshotProbeOutcomeV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NotReady => "not_ready",
            Self::GenerationMismatch => "generation_mismatch",
            Self::VersionMismatch => "version_mismatch",
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone)]
struct ReadyParseSnapshotProbeResultV2 {
    outcome: ReadyParseSnapshotProbeOutcomeV2,
    state: Option<super::super::ReadyParseSnapshotStateV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadySnapshotTaskStateV2 {
    Absent,
    InFlightSameVersion,
    InFlightOtherVersion,
    ReadySameVersion,
}

impl ReadySnapshotTaskStateV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::InFlightSameVersion => "in_flight_same_version",
            Self::InFlightOtherVersion => "in_flight_other_version",
            Self::ReadySameVersion => "ready_same_version",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiagnosticsSaveFollowupBranchContextV2 {
    ready_snapshot_task_state: ReadySnapshotTaskStateV2,
    shadow_state_available: bool,
    shadow_text_hash: Option<[u8; 32]>,
    ready_snapshot_phase_attribution: Option<DiagnosticsReadySnapshotPhaseAttributionV2>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadySnapshotReliefValveOutcomeV2 {
    EngagedHelped,
    EngagedTimedOut,
    EngagedVersionMismatch,
    EngagedGenerationMismatch,
    EngagedCancelled,
    EngagedSuperseded,
    SkippedNotExactStillCurrent,
    SkippedRuntimeQueueWait,
    SkippedApplyLag,
    SkippedTimeoutPhaseUnavailable,
    SkippedTimeoutPhaseWaiting,
}

impl ReadySnapshotReliefValveOutcomeV2 {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::EngagedHelped => "engaged_helped",
            Self::EngagedTimedOut => "engaged_timed_out",
            Self::EngagedVersionMismatch => "engaged_version_mismatch",
            Self::EngagedGenerationMismatch => "engaged_generation_mismatch",
            Self::EngagedCancelled => "engaged_cancelled",
            Self::EngagedSuperseded => "engaged_superseded",
            Self::SkippedNotExactStillCurrent => "skipped_not_exact_still_current",
            Self::SkippedRuntimeQueueWait => "skipped_runtime_queue_wait",
            Self::SkippedApplyLag => "skipped_apply_lag",
            Self::SkippedTimeoutPhaseUnavailable => "skipped_timeout_phase_unavailable",
            Self::SkippedTimeoutPhaseWaiting => "skipped_timeout_phase_waiting",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiagnosticsReadySnapshotPhaseAttributionV2 {
    pub(crate) timeout_phase: Option<&'static str>,
    pub(crate) timeout_phase_elapsed_ms: Option<u64>,
    pub(crate) parse_exec_ms: Option<u64>,
    pub(crate) post_parse_pre_materialization_ms: Option<u64>,
    pub(crate) ready_install_ms: Option<u64>,
    pub(crate) document_symbol_side_work_ms: Option<u64>,
    pub(crate) dominant_phase: Option<&'static str>,
    pub(crate) dominant_phase_ms: Option<u64>,
}

impl DiagnosticsReadySnapshotPhaseAttributionV2 {
    pub(crate) fn from_completed(
        attribution: &super::super::ReadyParseSnapshotPhaseAttributionV2,
    ) -> Option<Self> {
        let (dominant_phase, dominant_phase_ms) = attribution.dominant_phase().unwrap_or(("", 0));
        let has_any = attribution.parse_exec_ms.is_some()
            || attribution.post_parse_pre_materialization_ms.is_some()
            || attribution.ready_install_ms.is_some()
            || attribution.document_symbol_side_work_ms.is_some();
        has_any.then_some(Self {
            timeout_phase: None,
            timeout_phase_elapsed_ms: None,
            parse_exec_ms: attribution.parse_exec_ms,
            post_parse_pre_materialization_ms: attribution.post_parse_pre_materialization_ms,
            ready_install_ms: attribution.ready_install_ms,
            document_symbol_side_work_ms: attribution.document_symbol_side_work_ms,
            dominant_phase: (dominant_phase_ms > 0).then_some(dominant_phase),
            dominant_phase_ms: (dominant_phase_ms > 0).then_some(dominant_phase_ms),
        })
    }

    pub(crate) fn from_snapshot(
        snapshot: &super::super::ReadyParseSnapshotPhaseAttributionSnapshotV2,
        include_timeout_phase: bool,
    ) -> Option<Self> {
        let has_any = snapshot.current_phase.is_some()
            || snapshot.completed.parse_exec_ms.is_some()
            || snapshot
                .completed
                .post_parse_pre_materialization_ms
                .is_some()
            || snapshot.completed.ready_install_ms.is_some()
            || snapshot.completed.document_symbol_side_work_ms.is_some();
        if !has_any {
            return None;
        }
        let dominant = snapshot.dominant_phase();
        Some(Self {
            timeout_phase: include_timeout_phase
                .then(|| snapshot.current_phase.map(|phase| phase.as_str()))
                .flatten(),
            timeout_phase_elapsed_ms: include_timeout_phase
                .then_some(snapshot.current_phase_elapsed_ms)
                .flatten(),
            parse_exec_ms: snapshot.completed.parse_exec_ms.or_else(|| {
                matches!(
                    snapshot.current_phase,
                    Some(super::super::ReadyParseSnapshotAttributionPhaseV2::ParseExec)
                )
                .then_some(snapshot.current_phase_elapsed_ms)
                .flatten()
            }),
            post_parse_pre_materialization_ms: snapshot
                .completed
                .post_parse_pre_materialization_ms
                .or_else(|| {
                    matches!(
                        snapshot.current_phase,
                        Some(
                            super::super::ReadyParseSnapshotAttributionPhaseV2::PostParsePreMaterialization
                        )
                    )
                    .then_some(snapshot.current_phase_elapsed_ms)
                    .flatten()
                }),
            ready_install_ms: snapshot.completed.ready_install_ms.or_else(|| {
                matches!(
                    snapshot.current_phase,
                    Some(super::super::ReadyParseSnapshotAttributionPhaseV2::ReadyInstall)
                )
                .then_some(snapshot.current_phase_elapsed_ms)
                .flatten()
            }),
            document_symbol_side_work_ms: snapshot
                .completed
                .document_symbol_side_work_ms
                .or_else(|| {
                    matches!(
                        snapshot.current_phase,
                        Some(
                            super::super::ReadyParseSnapshotAttributionPhaseV2::DocumentSymbolSideWork
                        )
                    )
                    .then_some(snapshot.current_phase_elapsed_ms)
                    .flatten()
                }),
            dominant_phase: dominant.map(|(phase, _)| phase),
            dominant_phase_ms: dominant.map(|(_, duration_ms)| duration_ms),
        })
    }

    fn has_late_exact_timeout_phase(self) -> bool {
        matches!(
            self.timeout_phase,
            Some("parse_exec")
                | Some("post_parse_pre_materialization")
                | Some("ready_install")
                | Some("document_symbol_side_work")
        )
    }
}

struct SaveFollowupReadyArtifactsReply {
    diagnostics: Vec<tower_lsp::lsp_types::Diagnostic>,
    observed_deps_id: String,
    observed_settings_id: String,
    runtime_queue_wait: Option<Duration>,
    apply_lag: Option<Duration>,
    syntax_elapsed: Option<Duration>,
    semantic_elapsed: Option<Duration>,
    syntax_work_mode: Option<&'static str>,
    semantic_path: Option<&'static str>,
    semantic_parse_source: Option<&'static str>,
    semantic_ir_source: Option<&'static str>,
}

enum SaveFollowupReadyArtifactsAttemptV2 {
    Executed(bsl_runtime::application::DiagnosticsDisposition),
    ProbeMiss(ReadyParseSnapshotProbeOutcomeV2),
}

enum DidSaveFollowupAdmissionOutcome {
    Admitted {
        guard: DidSaveFollowupSlotGuard,
        queue_wait_elapsed: Option<Duration>,
    },
    Disposition {
        disposition: bsl_runtime::application::DiagnosticsDisposition,
        queue_wait_elapsed: Option<Duration>,
    },
}

pub(crate) struct DidSaveFollowupSlotGuard {
    server: BslLanguageServer,
    admitted_at: Instant,
    released: std::sync::atomic::AtomicBool,
}

impl DidSaveFollowupSlotGuard {
    fn new(server: BslLanguageServer) -> Self {
        Self {
            server,
            admitted_at: Instant::now(),
            released: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn release(&self) {
        if self.released.swap(true, Ordering::SeqCst) {
            return;
        }

        let quota = bsl_runtime::application::did_save_followup_lane_quota();
        let (active_slots, queue_depth) = {
            let mut state = self
                .server
                .diagnostics_did_save_followup_lane_v2
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.active_slots = state.active_slots.saturating_sub(1);
            (state.active_slots, state.queued_set.len())
        };
        self.server
            .record_did_save_followup_lane_saturation_v2(quota, active_slots, queue_depth);
        self.server
            .coordinator
            .record_intellisense_v2_runtime_lane_exec_latency_with_origin(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                bsl_runtime::application::AdmissionLane::DidSaveFollowup.as_str(),
                self.admitted_at.elapsed(),
            );
        self.server
            .diagnostics_did_save_followup_lane_notify_v2
            .notify_waiters();
    }
}

impl Drop for DidSaveFollowupSlotGuard {
    fn drop(&mut self) {
        self.release();
    }
}

impl BslLanguageServer {
    fn sum_nonzero_durations<I>(durations: I) -> Option<Duration>
    where
        I: IntoIterator<Item = Option<Duration>>,
    {
        let total = durations
            .into_iter()
            .flatten()
            .fold(Duration::ZERO, |acc, duration| acc.saturating_add(duration));
        (total > Duration::ZERO).then_some(total)
    }

    fn diagnostics_save_timeline_cycle_key_for_supersession_key(
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
    ) -> Option<super::super::DiagnosticsSaveTimelineCycleKey> {
        supersession_key
            .save_cycle_sequence
            .map(
                |save_cycle_sequence| super::super::DiagnosticsSaveTimelineCycleKey {
                    file_id: supersession_key.file_id,
                    diagnostics_generation: supersession_key.diagnostics_generation,
                    save_cycle_sequence,
                    requested_version: supersession_key.requested_version,
                },
            )
    }

    fn record_did_save_followup_lane_saturation_v2(
        &self,
        quota: usize,
        active_slots: usize,
        queue_depth: usize,
    ) {
        let origin = bsl_runtime::application::ObservabilityOrigin::Lsp.as_str();
        let lane = bsl_runtime::application::AdmissionLane::DidSaveFollowup.as_str();
        self.coordinator
            .record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
                origin,
                lane,
                "quota",
                quota as f64,
            );
        self.coordinator
            .record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
                origin,
                lane,
                "active_slots",
                active_slots as f64,
            );
        self.coordinator
            .record_intellisense_v2_runtime_lane_saturation_gauge_with_origin(
                origin,
                lane,
                "queue_depth",
                queue_depth as f64,
            );
    }

    async fn acquire_did_save_followup_lane_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> DidSaveFollowupAdmissionOutcome {
        let queue_wait_started = Instant::now();
        let mut queued = false;
        let origin = bsl_runtime::application::ObservabilityOrigin::Lsp.as_str();
        let lane = bsl_runtime::application::AdmissionLane::DidSaveFollowup.as_str();

        loop {
            if let Some(disposition) = self
                .diagnostics_checkpoint_v2(supersession_key, trigger, cancel_token)
                .await
            {
                let queue_wait_elapsed = queued.then_some(queue_wait_started.elapsed());
                if let Some(queue_wait_elapsed) = queue_wait_elapsed {
                    self.coordinator
                        .record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
                            origin,
                            lane,
                            queue_wait_elapsed,
                        );
                }
                self.remove_did_save_followup_lane_queue_entry_v2(supersession_key.file_id);
                return DidSaveFollowupAdmissionOutcome::Disposition {
                    disposition,
                    queue_wait_elapsed,
                };
            }

            let quota = bsl_runtime::application::did_save_followup_lane_quota();
            let (should_wait, (active_slots, queue_depth)) = {
                let mut state = self
                    .diagnostics_did_save_followup_lane_v2
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());

                if quota == 0 {
                    state.queued_set.remove(&supersession_key.file_id);
                    state
                        .queued_files
                        .retain(|file_id| *file_id != supersession_key.file_id);
                    let active_slots = state.active_slots;
                    let queue_depth = state.queued_set.len();
                    (false, (active_slots, queue_depth))
                } else {
                    let front_file = state.queued_files.front().copied();
                    let can_admit = state.active_slots < quota
                        && (front_file.is_none() || front_file == Some(supersession_key.file_id));
                    if can_admit {
                        if front_file == Some(supersession_key.file_id) {
                            let _ = state.queued_files.pop_front();
                            state.queued_set.remove(&supersession_key.file_id);
                        }
                        state.active_slots = state.active_slots.saturating_add(1);
                        let active_slots = state.active_slots;
                        let queue_depth = state.queued_set.len();
                        (false, (active_slots, queue_depth))
                    } else {
                        if state.queued_set.insert(supersession_key.file_id) {
                            state.queued_files.push_back(supersession_key.file_id);
                        }
                        let active_slots = state.active_slots;
                        let queue_depth = state.queued_set.len();
                        queued = true;
                        (true, (active_slots, queue_depth))
                    }
                }
            };
            self.record_did_save_followup_lane_saturation_v2(quota, active_slots, queue_depth);

            if quota == 0 {
                let queue_wait_elapsed = queued.then_some(queue_wait_started.elapsed());
                if let Some(queue_wait_elapsed) = queue_wait_elapsed {
                    self.coordinator
                        .record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
                            origin,
                            lane,
                            queue_wait_elapsed,
                        );
                }
                self.diagnostics_did_save_followup_lane_notify_v2
                    .notify_waiters();
                return DidSaveFollowupAdmissionOutcome::Disposition {
                    disposition: bsl_runtime::application::DiagnosticsDisposition::DisabledByConfig,
                    queue_wait_elapsed,
                };
            }

            if !should_wait {
                let queue_wait_elapsed = queued.then_some(queue_wait_started.elapsed());
                if let Some(queue_wait_elapsed) = queue_wait_elapsed {
                    self.coordinator
                        .record_intellisense_v2_runtime_lane_queue_wait_latency_with_origin(
                            origin,
                            lane,
                            queue_wait_elapsed,
                        );
                }
                return DidSaveFollowupAdmissionOutcome::Admitted {
                    guard: DidSaveFollowupSlotGuard::new(self.clone()),
                    queue_wait_elapsed,
                };
            }

            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                supersession_key,
                "runtime_queue_wait",
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );

            tokio::select! {
                _ = self.diagnostics_did_save_followup_lane_notify_v2.notified() => {}
                _ = tokio::time::sleep(Duration::from_millis(25)) => {}
            }
        }
    }

    fn remove_did_save_followup_lane_queue_entry_v2(&self, file_id: V2FileId) {
        let quota = bsl_runtime::application::did_save_followup_lane_quota();
        let snapshot = {
            let mut state = self
                .diagnostics_did_save_followup_lane_v2
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let removed = state.queued_set.remove(&file_id);
            if removed {
                state
                    .queued_files
                    .retain(|queued_file_id| *queued_file_id != file_id);
                Some((state.active_slots, state.queued_set.len()))
            } else {
                None
            }
        };
        if let Some((active_slots, queue_depth)) = snapshot {
            self.record_did_save_followup_lane_saturation_v2(quota, active_slots, queue_depth);
            self.diagnostics_did_save_followup_lane_notify_v2
                .notify_waiters();
        }
    }

    async fn finalize_diagnostics_save_profile_result_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        disposition: bsl_runtime::application::DiagnosticsDisposition,
        publish_kind: Option<&'static str>,
        runtime_queue_wait_ms: Option<Duration>,
        apply_lag_ms: Option<Duration>,
        blocking_queue_wait_ms: Option<Duration>,
        wait_for_file_version_ms: Option<Duration>,
        snapshot_with_deps_ms: Option<Duration>,
        syntax_diagnostics_query_ms: Option<Duration>,
        semantic_diagnostics_query_ms: Option<Duration>,
        publish_wait_ms: Option<Duration>,
        syntax_work_mode: Option<&'static str>,
        semantic_path: Option<&'static str>,
        semantic_parse_source: Option<&'static str>,
        semantic_ir_source: Option<&'static str>,
        pipeline_started: Instant,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        if !matches!(
            trigger,
            bsl_runtime::application::DiagnosticsTrigger::DidSave
        ) {
            return disposition;
        }

        let runtime_queue_wait_ms = duration_to_nonzero_ms(runtime_queue_wait_ms);
        let apply_lag_ms = duration_to_nonzero_ms(apply_lag_ms);
        let publish = (matches!(
            disposition,
            bsl_runtime::application::DiagnosticsDisposition::Published
        ) || publish_kind.is_some()
            || runtime_queue_wait_ms.is_some()
            || apply_lag_ms.is_some()
            || syntax_work_mode.is_some()
            || semantic_path.is_some()
            || semantic_parse_source.is_some()
            || semantic_ir_source.is_some()
            || blocking_queue_wait_ms.is_some()
            || wait_for_file_version_ms.is_some()
            || snapshot_with_deps_ms.is_some()
            || syntax_diagnostics_query_ms.is_some()
            || semantic_diagnostics_query_ms.is_some()
            || publish_wait_ms.is_some())
        .then(|| crate::types::DiagnosticsSaveTimelinePublishTrace {
            profile: supersession_key.profile.as_str().to_string(),
            publish_kind: publish_kind.unwrap_or("unknown").to_string(),
            outcome: disposition.as_str().to_string(),
            elapsed_ms: pipeline_started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            syntax_work_mode: syntax_work_mode.map(str::to_string),
            semantic_path: semantic_path.map(str::to_string),
            semantic_parse_source: semantic_parse_source.map(str::to_string),
            semantic_ir_source: semantic_ir_source.map(str::to_string),
            runtime_queue_wait_ms,
            apply_lag_ms,
            blocking_queue_wait_ms: blocking_queue_wait_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            wait_for_file_version_ms: wait_for_file_version_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            snapshot_with_deps_ms: snapshot_with_deps_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            syntax_diagnostics_query_ms: syntax_diagnostics_query_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            semantic_diagnostics_query_ms: semantic_diagnostics_query_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
            publish_wait_ms: publish_wait_ms
                .map(|value| value.as_millis().min(u64::MAX as u128) as u64),
        });

        let Some(cycle_key) =
            Self::diagnostics_save_timeline_cycle_key_for_supersession_key(supersession_key)
        else {
            return disposition;
        };

        self.record_diagnostics_save_timeline_profile_result(
            uri,
            cycle_key,
            super::super::DiagnosticsSaveTimelineProfileResult {
                profile: supersession_key.profile,
                disposition,
                publish,
            },
        );
        disposition
    }

    fn diagnostics_publish_rank(profile: bsl_runtime::application::DiagnosticsProfile) -> u8 {
        match profile {
            bsl_runtime::application::DiagnosticsProfile::SaveFastlane => 1,
            _ => 2,
        }
    }

    pub(crate) async fn cancel_diagnostics_v2(&self, file_id: V2FileId) {
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        let keys: Vec<super::super::DiagnosticsTaskKeyV2> = tasks
            .keys()
            .copied()
            .filter(|key| key.file_id == file_id)
            .collect();
        for key in keys {
            if let Some(task) = tasks.remove(&key) {
                task.cancel_token
                    .cancel(super::super::DiagnosticsCancellationReasonV2::ClientCancel);
                self.record_diagnostics_pipeline_event_v2(
                    task.trigger,
                    task.supersession_key.profile,
                    bsl_runtime::application::DiagnosticsDisposition::ClientCancel,
                );
                task.handle.abort();
            }
        }
    }

    pub(crate) async fn bump_diagnostics_generation_v2(&self, file_id: V2FileId) -> u64 {
        let mut generations = self.diagnostics_generation_v2.write().await;
        let next = generations
            .get(&file_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        generations.insert(file_id, next);
        next
    }

    pub(crate) async fn bump_diagnostics_save_cycle_sequence_v2(&self, file_id: V2FileId) -> u64 {
        let mut sequences = self.diagnostics_save_cycle_sequence_v2.write().await;
        let next = sequences
            .get(&file_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        sequences.insert(file_id, next);
        next
    }

    async fn current_diagnostics_generation_v2(&self, file_id: V2FileId) -> Option<u64> {
        self.diagnostics_generation_v2
            .read()
            .await
            .get(&file_id)
            .copied()
    }

    fn record_diagnostics_pipeline_event_v2(
        &self,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        reason: bsl_runtime::application::DiagnosticsDisposition,
    ) {
        self.coordinator
            .record_intellisense_v2_diagnostics_pipeline_event(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                trigger.as_str(),
                profile.as_str(),
                reason.as_str(),
            );
    }

    fn record_diagnostics_pipeline_publish_latency_v2(
        &self,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        duration: Duration,
    ) {
        self.coordinator
            .record_intellisense_v2_diagnostics_pipeline_publish_latency(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                trigger.as_str(),
                profile.as_str(),
                duration,
            );
    }

    fn diagnostics_cancelled_disposition_v2(
        &self,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        current_generation: Option<u64>,
        expected_generation: u64,
        current_version: Option<i32>,
        expected_version: i32,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                return token.reason().to_disposition();
            }
        }
        if current_generation != Some(expected_generation) {
            return bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
        }
        if current_version != Some(expected_version) {
            return bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
        }
        bsl_runtime::application::DiagnosticsDisposition::OtherCancel
    }

    async fn diagnostics_checkpoint_v2(
        &self,
        key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        let current_generation = self.current_diagnostics_generation_v2(key.file_id).await;
        let current_version = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&key.file_id)
            .copied();
        let disposition = if let Some(token) = cancel_token {
            if token.is_cancelled() {
                token.reason().to_disposition()
            } else {
                self.diagnostics_cancelled_disposition_v2(
                    None,
                    current_generation,
                    key.diagnostics_generation,
                    current_version,
                    key.requested_version,
                )
            }
        } else {
            self.diagnostics_cancelled_disposition_v2(
                None,
                current_generation,
                key.diagnostics_generation,
                current_version,
                key.requested_version,
            )
        };
        if matches!(
            disposition,
            bsl_runtime::application::DiagnosticsDisposition::OtherCancel
        ) && current_generation == Some(key.diagnostics_generation)
            && current_version == Some(key.requested_version)
        {
            return None;
        }
        self.record_diagnostics_pipeline_event_v2(trigger, key.profile, disposition);
        Some(disposition)
    }

    async fn diagnostics_publish_checkpoint_v2(
        &self,
        key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        observed_deps_id: Option<&str>,
        observed_settings_id: Option<&str>,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        if let Some(disposition) = self
            .diagnostics_checkpoint_v2(key, trigger, cancel_token)
            .await
        {
            return Some(disposition);
        }

        let current_deps_id = self
            .last_deps_id_v2
            .read()
            .await
            .as_ref()
            .map(|id| id.as_str().to_string());
        let current_settings_id = self
            .last_settings_id_v2
            .read()
            .await
            .as_ref()
            .map(|id| id.as_str().to_string());

        let deps_mismatch =
            observed_deps_id.is_some() && current_deps_id.as_deref() != observed_deps_id;
        let settings_mismatch = observed_settings_id.is_some()
            && current_settings_id.as_deref() != observed_settings_id;
        if deps_mismatch || settings_mismatch {
            let disposition =
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration;
            self.record_diagnostics_pipeline_event_v2(trigger, key.profile, disposition);
            return Some(disposition);
        }

        None
    }

    async fn publish_diagnostics_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        uri: &Url,
        diagnostics: Vec<tower_lsp::lsp_types::Diagnostic>,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        pipeline_started: Instant,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        let publish_rank = Self::diagnostics_publish_rank(profile);
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let diagnostics_generation = supersession_key.diagnostics_generation;
        {
            let publish_state = self.latest_diagnostics_publish_state_v2.read().await;
            if publish_state.get(&file_id).is_some_and(|state| {
                state.requested_version == requested_version
                    && state.diagnostics_generation == diagnostics_generation
                    && state.publish_rank > publish_rank
            }) {
                self.record_diagnostics_pipeline_event_v2(
                    trigger,
                    profile,
                    bsl_runtime::application::DiagnosticsDisposition::OtherCancel,
                );
                return bsl_runtime::application::DiagnosticsDisposition::OtherCancel;
            }
        }

        let diagnostics_len = diagnostics.len();
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(requested_version))
            .await;
        self.update_diagnostics_count(uri, diagnostics_len).await;
        self.latest_diagnostics_publish_state_v2
            .write()
            .await
            .insert(
                file_id,
                super::super::DiagnosticsPublishedStateV2 {
                    requested_version,
                    diagnostics_generation,
                    publish_rank,
                },
            );
        self.record_diagnostics_pipeline_publish_latency_v2(
            trigger,
            profile,
            pipeline_started.elapsed(),
        );
        self.record_diagnostics_pipeline_event_v2(
            trigger,
            profile,
            bsl_runtime::application::DiagnosticsDisposition::Published,
        );
        bsl_runtime::application::DiagnosticsDisposition::Published
    }

    async fn try_collect_save_fastlane_diagnostics_from_applied_analysis_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<(
        Vec<tower_lsp::lsp_types::Diagnostic>,
        Vec<bsl_shared::domain::types::ParseError>,
        &'static str,
        Duration,
    )> {
        tokio::time::timeout(Duration::from_millis(20), async {
            let started = Instant::now();
            let revision_state = self.analysis_v2.file_revision_state(file_id).await?;
            if revision_state.version != requested_version {
                return None;
            }

            let analysis = self.analysis_v2.snapshot().await;
            if analysis.file_version(file_id).ok().flatten() != Some(requested_version) {
                return None;
            }
            let syntax_errors = analysis.syntax_diagnostics(file_id).ok().flatten()?;
            let line_index = analysis.line_index(file_id).ok().flatten()?;
            let mode = analysis
                .syntax_diagnostics_observability_mode(file_id)
                .ok()
                .flatten()
                .unwrap_or("other");
            Some((
                syntax_errors_to_diagnostics(
                    syntax_errors.as_ref(),
                    uri,
                    analysis.file_text(file_id).ok().flatten()?.as_ref(),
                    line_index.as_ref(),
                ),
                syntax_errors.iter().cloned().collect(),
                mode,
                started.elapsed(),
            ))
        })
        .await
        .ok()
        .flatten()
    }

    fn parse_snapshot_observability_mode_v2(
        parse_snapshot: &bsl_analysis_v2::ParseSnapshot,
    ) -> &'static str {
        if parse_snapshot.incremental {
            if parse_snapshot.changed_ranges.is_empty() {
                "reused"
            } else {
                "incremental"
            }
        } else {
            "full"
        }
    }

    async fn try_collect_save_fastlane_diagnostics_from_ready_parse_snapshot_v2(
        &self,
        uri: &Url,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<(
        Vec<tower_lsp::lsp_types::Diagnostic>,
        Vec<bsl_shared::domain::types::ParseError>,
        &'static str,
        Duration,
    )> {
        let started = Instant::now();
        let ready_state = self
            .latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned()?;
        let parse_snapshot = ready_state.parse_snapshot;
        if parse_snapshot.file_version != requested_version {
            return None;
        }
        Some((
            syntax_errors_to_diagnostics(
                &parse_snapshot.parse_result.syntax_errors,
                uri,
                ready_state.text.as_ref(),
                parse_snapshot.line_index.as_ref(),
            ),
            parse_snapshot
                .parse_result
                .syntax_errors
                .iter()
                .cloned()
                .collect(),
            Self::parse_snapshot_observability_mode_v2(&parse_snapshot),
            started.elapsed(),
        ))
    }

    async fn ready_parse_snapshot_state_for_version_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<super::super::ReadyParseSnapshotStateV2> {
        self.latest_ready_parse_snapshots_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
            .filter(|state| state.parse_snapshot.file_version == requested_version)
    }

    async fn save_fastlane_syntax_artifacts_for_version_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
    ) -> Option<Arc<Vec<bsl_shared::domain::types::ParseError>>> {
        self.latest_save_fastlane_syntax_artifacts_v2
            .read()
            .await
            .get(&file_id)
            .cloned()
            .filter(|state| state.version == requested_version)
            .map(|state| state.syntax_errors)
    }

    async fn record_save_fastlane_syntax_artifacts_v2(
        &self,
        file_id: V2FileId,
        requested_version: i32,
        syntax_errors: Vec<bsl_shared::domain::types::ParseError>,
    ) {
        self.latest_save_fastlane_syntax_artifacts_v2
            .write()
            .await
            .insert(
                file_id,
                super::super::SaveFastlaneSyntaxArtifactsV2 {
                    version: requested_version,
                    syntax_errors: Arc::new(syntax_errors),
                },
            );
    }

    fn ready_parse_snapshot_probe_outcome_for_cancel_reason_v2(
        reason: super::super::DiagnosticsCancellationReasonV2,
    ) -> ReadyParseSnapshotProbeOutcomeV2 {
        match reason {
            super::super::DiagnosticsCancellationReasonV2::SupersededGeneration
            | super::super::DiagnosticsCancellationReasonV2::SupersededVersion => {
                ReadyParseSnapshotProbeOutcomeV2::Superseded
            }
            super::super::DiagnosticsCancellationReasonV2::ClientCancel
            | super::super::DiagnosticsCancellationReasonV2::OtherCancel => {
                ReadyParseSnapshotProbeOutcomeV2::Cancelled
            }
        }
    }

    pub(crate) fn ready_parse_snapshot_probe_wait_decision_v2(
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        wait_budget: Duration,
        wait_elapsed: Duration,
        cancel_reason: Option<super::super::DiagnosticsCancellationReasonV2>,
        current_generation: Option<u64>,
        latest_received_version: Option<i32>,
    ) -> Option<ReadyParseSnapshotProbeOutcomeV2> {
        if wait_budget.is_zero() {
            return Some(ReadyParseSnapshotProbeOutcomeV2::NotReady);
        }
        if wait_elapsed >= wait_budget {
            return Some(ReadyParseSnapshotProbeOutcomeV2::Timeout);
        }
        if let Some(reason) = cancel_reason {
            return Some(Self::ready_parse_snapshot_probe_outcome_for_cancel_reason_v2(reason));
        }
        if current_generation != Some(supersession_key.diagnostics_generation) {
            return Some(ReadyParseSnapshotProbeOutcomeV2::GenerationMismatch);
        }
        if latest_received_version != Some(supersession_key.requested_version) {
            return Some(ReadyParseSnapshotProbeOutcomeV2::VersionMismatch);
        }
        None
    }

    async fn wait_for_ready_parse_snapshot_probe_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        wait_budget: Duration,
        expected_text_hash: Option<[u8; 32]>,
    ) -> ReadyParseSnapshotProbeResultV2 {
        let wait_started = Instant::now();
        loop {
            if let Some(state) = self
                .ready_parse_snapshot_state_for_version_v2(
                    supersession_key.file_id,
                    supersession_key.requested_version,
                )
                .await
            {
                return ReadyParseSnapshotProbeResultV2 {
                    outcome: ReadyParseSnapshotProbeOutcomeV2::Ready,
                    state: Some(state),
                };
            }
            let cancel_reason = cancel_token
                .filter(|token| token.is_cancelled())
                .map(|token| token.reason());
            let current_generation = self
                .current_diagnostics_generation_v2(supersession_key.file_id)
                .await;
            let latest_received_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&supersession_key.file_id)
                .copied();
            if let Some(outcome) = Self::ready_parse_snapshot_probe_wait_decision_v2(
                supersession_key,
                wait_budget,
                wait_started.elapsed(),
                cancel_reason,
                current_generation,
                latest_received_version,
            ) {
                return ReadyParseSnapshotProbeResultV2 {
                    outcome,
                    state: None,
                };
            }
            let remaining_budget = wait_budget.saturating_sub(wait_started.elapsed());
            let poll_sleep = remaining_budget.min(Duration::from_millis(25));
            if let Some(task_control) = self
                .matching_background_parse_snapshot_task_control_v2(
                    supersession_key.file_id,
                    supersession_key.requested_version,
                    None,
                )
                .await
            {
                let materialized = task_control.materialized_notify.notified();
                let control = task_control.control_notify.notified();
                tokio::pin!(materialized);
                tokio::pin!(control);
                tokio::select! {
                    _ = tokio::time::sleep(poll_sleep) => {}
                    _ = &mut materialized => {}
                    _ = &mut control => {}
                }
            } else if self
                .background_parse_snapshot_task_retargeted_away_v2(
                    supersession_key.file_id,
                    supersession_key.requested_version,
                    expected_text_hash,
                )
                .await
            {
                return ReadyParseSnapshotProbeResultV2 {
                    outcome: ReadyParseSnapshotProbeOutcomeV2::VersionMismatch,
                    state: None,
                };
            } else {
                tokio::time::sleep(poll_sleep).await;
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn wait_for_ready_parse_snapshot_probe_outcome_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        wait_budget: Duration,
        expected_text_hash: Option<[u8; 32]>,
    ) -> ReadyParseSnapshotProbeOutcomeV2 {
        self.wait_for_ready_parse_snapshot_probe_v2(
            supersession_key,
            cancel_token,
            wait_budget,
            expected_text_hash,
        )
        .await
        .outcome
    }

    async fn diagnostics_save_followup_branch_context_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
    ) -> DiagnosticsSaveFollowupBranchContextV2 {
        let shadow_state = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&supersession_key.file_id)
            .cloned()
            .filter(|state| state.version == supersession_key.requested_version);
        let shadow_text_hash = shadow_state
            .as_ref()
            .map(|state| *blake3::hash(state.text.as_bytes()).as_bytes());
        let ready_state = self
            .ready_parse_snapshot_state_for_version_v2(
                supersession_key.file_id,
                supersession_key.requested_version,
            )
            .await;
        let mut exact_inflight_control = None;
        let ready_snapshot_task_state = if ready_state.is_some() {
            ReadySnapshotTaskStateV2::ReadySameVersion
        } else {
            let tasks = self.background_parse_snapshot_apply_tasks_v2.lock().await;
            match tasks.get(&supersession_key.file_id) {
                Some(task) => {
                    let target = task
                        .target
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    if target.requested_version == supersession_key.requested_version
                        && shadow_text_hash.map_or(true, |text_hash| target.text_hash == text_hash)
                        && target.source
                            != super::super::BackgroundParseSnapshotApplyTaskSourceV2::DidSave
                    {
                        exact_inflight_control = Some(Arc::clone(&task.control));
                        // refactor-17 only reorders for already-known exact-task evidence.
                        // The didSave refresh task seeded by the current save cycle does not qualify.
                        ReadySnapshotTaskStateV2::InFlightSameVersion
                    } else if target.requested_version == supersession_key.requested_version {
                        ReadySnapshotTaskStateV2::Absent
                    } else {
                        ReadySnapshotTaskStateV2::InFlightOtherVersion
                    }
                }
                None => ReadySnapshotTaskStateV2::Absent,
            }
        };
        let ready_snapshot_phase_attribution = exact_inflight_control
            .as_ref()
            .and_then(|control| {
                DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
                    &control.phase_attribution_snapshot(),
                    false,
                )
            })
            .or_else(|| {
                ready_state.as_ref().and_then(|state| {
                    DiagnosticsReadySnapshotPhaseAttributionV2::from_completed(
                        &state.phase_attribution,
                    )
                })
            });
        DiagnosticsSaveFollowupBranchContextV2 {
            ready_snapshot_task_state,
            shadow_state_available: shadow_state.is_some(),
            shadow_text_hash,
            ready_snapshot_phase_attribution,
        }
    }

    fn diagnostics_save_cycle_key_from_supersession_key_v2(
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
    ) -> Option<super::super::DiagnosticsSaveTimelineCycleKey> {
        supersession_key
            .save_cycle_sequence
            .map(
                |save_cycle_sequence| super::super::DiagnosticsSaveTimelineCycleKey {
                    file_id: supersession_key.file_id,
                    diagnostics_generation: supersession_key.diagnostics_generation,
                    save_cycle_sequence,
                    requested_version: supersession_key.requested_version,
                },
            )
    }

    fn record_diagnostics_save_followup_wait_state_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        reason: &'static str,
        runtime_queue_wait_ms: Option<Duration>,
        apply_lag_ms: Option<Duration>,
        wait_for_file_version_ms: Option<Duration>,
        snapshot_with_deps_ms: Option<Duration>,
        syntax_work_mode: Option<&'static str>,
        semantic_path: Option<&'static str>,
        semantic_parse_source: Option<&'static str>,
        semantic_ir_source: Option<&'static str>,
    ) {
        let Some(cycle_key) =
            Self::diagnostics_save_cycle_key_from_supersession_key_v2(supersession_key)
        else {
            return;
        };
        self.coordinator
            .record_intellisense_v2_diagnostics_save_followup_wait_state(reason);
        self.record_diagnostics_save_timeline_followup_wait_state(
            uri,
            cycle_key,
            reason,
            runtime_queue_wait_ms,
            apply_lag_ms,
            wait_for_file_version_ms,
            snapshot_with_deps_ms,
            syntax_work_mode,
            semantic_path,
            semantic_parse_source,
            semantic_ir_source,
        );
    }

    fn record_diagnostics_save_followup_probe_state_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        ready_snapshot_zero_probe: Option<ReadyParseSnapshotProbeOutcomeV2>,
        ready_snapshot_wait_probe: Option<ReadyParseSnapshotProbeOutcomeV2>,
        ready_snapshot_task_state: Option<ReadySnapshotTaskStateV2>,
        shadow_state_available: Option<bool>,
        ready_snapshot_phase_attribution: Option<DiagnosticsReadySnapshotPhaseAttributionV2>,
    ) {
        let Some(cycle_key) =
            Self::diagnostics_save_cycle_key_from_supersession_key_v2(supersession_key)
        else {
            return;
        };
        self.record_diagnostics_save_timeline_followup_probe_state(
            uri,
            cycle_key,
            ready_snapshot_zero_probe.map(ReadyParseSnapshotProbeOutcomeV2::as_str),
            ready_snapshot_wait_probe.map(ReadyParseSnapshotProbeOutcomeV2::as_str),
            ready_snapshot_task_state.map(ReadySnapshotTaskStateV2::as_str),
            shadow_state_available,
            ready_snapshot_phase_attribution,
        );
    }

    fn record_diagnostics_save_followup_relief_valve_state_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        outcome: ReadySnapshotReliefValveOutcomeV2,
        budget: Duration,
        elapsed: Option<Duration>,
    ) {
        let Some(cycle_key) =
            Self::diagnostics_save_cycle_key_from_supersession_key_v2(supersession_key)
        else {
            return;
        };
        self.coordinator
            .record_intellisense_v2_diagnostics_save_followup_ready_snapshot_relief_valve(
                outcome.as_str(),
                elapsed.unwrap_or_default(),
            );
        self.record_diagnostics_save_timeline_followup_relief_valve(
            uri,
            cycle_key,
            outcome.as_str(),
            budget,
            elapsed,
        );
    }

    async fn ready_snapshot_phase_attribution_for_probe_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        expected_text_hash: Option<[u8; 32]>,
        ready_state: Option<&super::super::ReadyParseSnapshotStateV2>,
        include_timeout_phase: bool,
    ) -> Option<DiagnosticsReadySnapshotPhaseAttributionV2> {
        if let Some(task_control) = self
            .matching_background_parse_snapshot_task_control_v2(
                supersession_key.file_id,
                supersession_key.requested_version,
                expected_text_hash,
            )
            .await
        {
            return DiagnosticsReadySnapshotPhaseAttributionV2::from_snapshot(
                &task_control.phase_attribution_snapshot(),
                include_timeout_phase,
            );
        }
        ready_state.and_then(|state| {
            DiagnosticsReadySnapshotPhaseAttributionV2::from_completed(&state.phase_attribution)
        })
    }

    async fn diagnostics_followup_apply_lag_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
    ) -> Option<Duration> {
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        if self
            .latest_current_revision_handoff_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            != Some(requested_version)
        {
            return None;
        }
        if self
            .analysis_v2
            .cached_file_revision_state(file_id)
            .is_some_and(|state| state.version >= requested_version)
        {
            return None;
        }
        self.latest_apply_enqueued_at_v2
            .read()
            .await
            .get(&file_id)
            .copied()
            .map(|enqueued_at| enqueued_at.elapsed())
    }

    async fn wait_for_save_fastlane_first_publish_v2(
        &self,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> SaveFastlaneFirstPublishWaitOutcome {
        let Some(cycle_key) =
            Self::diagnostics_save_timeline_cycle_key_for_supersession_key(supersession_key)
        else {
            return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
        };
        loop {
            match self.diagnostics_save_timeline_fastlane_progress(cycle_key) {
                super::DiagnosticsSaveTimelineFastlaneProgress::SuccessfulFirstPublish => {
                    return SaveFastlaneFirstPublishWaitOutcome::Published;
                }
                super::DiagnosticsSaveTimelineFastlaneProgress::TerminalWithoutPublish => {
                    return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
                }
                super::DiagnosticsSaveTimelineFastlaneProgress::Pending => {}
            }
            if cancel_token.is_some_and(|token| token.is_cancelled()) {
                return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
            }
            if self
                .current_diagnostics_generation_v2(supersession_key.file_id)
                .await
                != Some(supersession_key.diagnostics_generation)
            {
                return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
            }
            if self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&supersession_key.file_id)
                .copied()
                != Some(supersession_key.requested_version)
            {
                return SaveFastlaneFirstPublishWaitOutcome::NotPublished;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn try_execute_save_followup_from_shadow_state_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        pipeline_started: Instant,
        show_hints: bool,
        flow_sensitive_semantic: bool,
        followup_lane_guard: Option<&DidSaveFollowupSlotGuard>,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        if !matches!(
            (trigger, supersession_key.profile),
            (
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            )
        ) {
            return None;
        }

        let shadow_state = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&supersession_key.file_id)
            .cloned()?;
        if shadow_state.version != supersession_key.requested_version {
            return None;
        }

        let save_fastlane_syntax_artifacts = self
            .save_fastlane_syntax_artifacts_for_version_v2(
                supersession_key.file_id,
                supersession_key.requested_version,
            )
            .await?;
        self.coordinator
            .record_intellisense_v2_diagnostics_save_followup_semantic_path("shadow_state");
        let apply_lag = self
            .diagnostics_followup_apply_lag_v2(supersession_key)
            .await;
        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                supersession_key.file_id,
                None,
                flow_sensitive_semantic,
            )
            .await;
        let support_bundle = self.analysis_v2.completion_support_bundle();
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };
        let uri_for_blocking = uri.clone();
        let coordinator_for_blocking = self.coordinator.clone();
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let shadow_text = shadow_state.text.clone();
        let deps_id = support_bundle.deps_id.clone();
        let deps = support_bundle.deps.clone();
        let settings_id = context.settings.settings_id.clone();
        let diagnostics_detail_level = context.settings.diagnostics_detail_level;
        let context_for_blocking = context.clone();
        let path_for_blocking: Arc<str> = Arc::from(path);
        let syntax_work_mode = Some("reused");
        let semantic_path = Some("shadow_state");
        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "semantic_work",
            None,
            apply_lag,
            None,
            None,
            syntax_work_mode,
            semantic_path,
            None,
            None,
        );
        let queued_wait_server = self.clone();
        let queued_wait_uri = uri.clone();
        let queued_wait_supersession_key = *supersession_key;
        let queued_wait_apply_lag = apply_lag;
        let queued_wait_syntax_work_mode = syntax_work_mode;
        let exec_started_server = self.clone();
        let exec_started_uri = uri.clone();
        let exec_started_supersession_key = *supersession_key;
        let exec_started_apply_lag = apply_lag;
        let exec_started_syntax_work_mode = syntax_work_mode;

        let followup_call =
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
                bsl_runtime::application::CpuWorkClass::Background,
                context.origin.as_str(),
                Some(bsl_runtime::application::AdmissionLane::DidSaveFollowup),
                Some(self.coordinator.as_ref()),
                Some(move || {
                    queued_wait_server.record_diagnostics_save_followup_wait_state_v2(
                        &queued_wait_uri,
                        &queued_wait_supersession_key,
                        "runtime_queue_wait",
                        None,
                        queued_wait_apply_lag,
                        None,
                        None,
                        queued_wait_syntax_work_mode,
                        semantic_path,
                        None,
                        None,
                    );
                }),
                Some(move |queue_wait_elapsed| {
                    exec_started_server.record_diagnostics_save_followup_wait_state_v2(
                        &exec_started_uri,
                        &exec_started_supersession_key,
                        "semantic_work",
                        (queue_wait_elapsed > Duration::ZERO).then_some(queue_wait_elapsed),
                        exec_started_apply_lag,
                        None,
                        None,
                        exec_started_syntax_work_mode,
                        semantic_path,
                        None,
                        None,
                    );
                }),
                move || {
                    let mut host = bsl_analysis_v2::AnalysisHostV2::default();
                    host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
                        deps_id: deps_id.clone(),
                        deps,
                    });
                    host.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
                        settings_id: settings_id.clone(),
                        diagnostics_detail_level,
                    });
                    host.apply_change(bsl_analysis_v2::Change::SetFile {
                        file_id,
                        text: shadow_text.clone(),
                        version: requested_version,
                        path: path_for_blocking,
                    });

                    let analysis = host.snapshot();
                    let file_text = shadow_text.clone();
                    let line_index =
                        analysis
                            .line_index(file_id)
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| {
                                Arc::new(bsl_line_index::LineIndex::new(file_text.as_ref()))
                            });

                    let mut diagnostics = Vec::new();
                    diagnostics.extend(syntax_errors_to_diagnostics(
                        save_fastlane_syntax_artifacts.as_ref(),
                        &uri_for_blocking,
                        file_text.as_ref(),
                        line_index.as_ref(),
                    ));

                    let semantic_started = Instant::now();
                    let mut semantic_parse_source = None;
                    let mut semantic_ir_source = None;
                    let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                        &context_for_blocking,
                        bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                        &analysis,
                        Some(coordinator_for_blocking.as_ref()),
                        |analysis| {
                            if flow_sensitive_semantic {
                                analysis.semantic_diagnostics_flow_sensitive_profiled(file_id)
                            } else {
                                analysis.semantic_diagnostics_profiled(file_id)
                            }
                        },
                    )
                    .map_err(|_| ())?;
                    let semantic_elapsed = semantic_started.elapsed();
                    let duration_from_profile_ms =
                        |value: u128| Duration::from_millis(value.min(u64::MAX as u128) as u64);
                    if let Some(profiled) = query {
                        coordinator_for_blocking
                            .record_intellisense_v2_semantic_diagnostics_query_breakdown(
                                duration_from_profile_ms(profiled.profile.inputs_ms),
                                duration_from_profile_ms(profiled.profile.parse_result_ms),
                                duration_from_profile_ms(profiled.profile.ir_ms),
                                duration_from_profile_ms(profiled.profile.collect_ms),
                                (profiled.profile.flow_sensitive_ms > 0).then(|| {
                                    duration_from_profile_ms(profiled.profile.flow_sensitive_ms)
                                }),
                            );
                        semantic_parse_source =
                            profiled.profile.parse_source.map(|source| source.as_str());
                        semantic_ir_source =
                            profiled.profile.ir_source.map(|source| source.as_str());
                        for error in profiled.diagnostics.iter() {
                            if !show_hints
                                && matches!(
                                    error.severity,
                                    bsl_shared::domain::types::DiagnosticSeverity::Hint
                                )
                            {
                                continue;
                            }
                            diagnostics.push(semantic_error_to_diagnostic(
                                error,
                                file_text.as_ref(),
                                line_index.as_ref(),
                            ));
                        }
                    }

                    Ok::<SaveFollowupReadyArtifactsReply, ()>(SaveFollowupReadyArtifactsReply {
                        diagnostics,
                        observed_deps_id: deps_id.as_str().to_string(),
                        observed_settings_id: settings_id.as_str().to_string(),
                        runtime_queue_wait: None,
                        apply_lag: None,
                        syntax_elapsed: None,
                        semantic_elapsed: Some(semantic_elapsed),
                        syntax_work_mode,
                        semantic_path,
                        semantic_parse_source,
                        semantic_ir_source,
                    })
                },
            )
            .await;
        let runtime_queue_wait =
            Self::sum_nonzero_durations([Some(followup_call.queue_wait_elapsed)]);

        let mut reply = match followup_call.join_result {
            Ok(Ok(reply)) => reply,
            Ok(Err(())) | Err(_) => {
                return Some(
                    self.finalize_diagnostics_save_profile_result_v2(
                        uri,
                        supersession_key,
                        trigger,
                        self.diagnostics_cancelled_disposition_v2(
                            cancel_token,
                            self.current_diagnostics_generation_v2(supersession_key.file_id)
                                .await,
                            supersession_key.diagnostics_generation,
                            self.latest_received_file_versions_v2
                                .read()
                                .await
                                .get(&supersession_key.file_id)
                                .copied(),
                            supersession_key.requested_version,
                        ),
                        None,
                        runtime_queue_wait,
                        apply_lag,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        semantic_path,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await,
                );
            }
        };
        reply.runtime_queue_wait = runtime_queue_wait;
        reply.apply_lag = apply_lag;

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(
                supersession_key,
                trigger,
                cancel_token,
                Some(reply.observed_deps_id.as_str()),
                Some(reply.observed_settings_id.as_str()),
            )
            .await
        {
            return Some(
                self.finalize_diagnostics_save_profile_result_v2(
                    uri,
                    supersession_key,
                    trigger,
                    disposition,
                    None,
                    reply.runtime_queue_wait,
                    reply.apply_lag,
                    None,
                    None,
                    None,
                    reply.syntax_elapsed,
                    reply.semantic_elapsed,
                    None,
                    reply.syntax_work_mode,
                    reply.semantic_path,
                    reply.semantic_parse_source,
                    reply.semantic_ir_source,
                    pipeline_started,
                )
                .await,
            );
        }

        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "pending_publish",
            reply.runtime_queue_wait,
            reply.apply_lag,
            None,
            None,
            reply.syntax_work_mode,
            reply.semantic_path,
            reply.semantic_parse_source,
            reply.semantic_ir_source,
        );
        if let Some(guard) = followup_lane_guard {
            guard.release();
        }
        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                supersession_key,
                uri,
                reply.diagnostics,
                trigger,
                supersession_key.profile,
                pipeline_started,
            )
            .await;
        Some(
            self.finalize_diagnostics_save_profile_result_v2(
                uri,
                supersession_key,
                trigger,
                disposition,
                Some("full"),
                reply.runtime_queue_wait,
                reply.apply_lag,
                None,
                None,
                None,
                reply.syntax_elapsed,
                reply.semantic_elapsed,
                Some(publish_started.elapsed()),
                reply.syntax_work_mode,
                reply.semantic_path,
                reply.semantic_parse_source,
                reply.semantic_ir_source,
                pipeline_started,
            )
            .await,
        )
    }

    async fn try_execute_save_followup_from_ready_artifacts_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        probe_slot: ReadyParseSnapshotProbeSlotV2,
        wait_budget: Duration,
        pipeline_started: Instant,
        show_hints: bool,
        flow_sensitive_semantic: bool,
        followup_lane_guard: Option<&DidSaveFollowupSlotGuard>,
    ) -> SaveFollowupReadyArtifactsAttemptV2 {
        if !matches!(
            (trigger, supersession_key.profile),
            (
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            )
        ) {
            return SaveFollowupReadyArtifactsAttemptV2::ProbeMiss(
                ReadyParseSnapshotProbeOutcomeV2::NotReady,
            );
        }

        let expected_text_hash = self
            .latest_document_shadow_state_v2
            .read()
            .await
            .get(&supersession_key.file_id)
            .filter(|state| state.version == supersession_key.requested_version)
            .map(|state| *blake3::hash(state.text.as_bytes()).as_bytes());
        let probe_started = Instant::now();
        let probe = self
            .wait_for_ready_parse_snapshot_probe_v2(
                supersession_key,
                cancel_token,
                wait_budget,
                expected_text_hash,
            )
            .await;
        self.coordinator
            .record_intellisense_v2_diagnostics_save_followup_ready_snapshot_probe(
                probe_slot.as_str(),
                probe.outcome.as_str(),
                probe_started.elapsed(),
            );
        let branch_context = self
            .diagnostics_save_followup_branch_context_v2(supersession_key)
            .await;
        let ready_snapshot_phase_attribution = if let Some(ready_state) = probe.state.as_ref() {
            self.ready_snapshot_phase_attribution_for_probe_v2(
                supersession_key,
                expected_text_hash,
                Some(ready_state),
                false,
            )
            .await
            .or(branch_context.ready_snapshot_phase_attribution)
        } else {
            self.ready_snapshot_phase_attribution_for_probe_v2(
                supersession_key,
                expected_text_hash,
                None,
                matches!(
                    (probe_slot, probe.outcome),
                    (
                        ReadyParseSnapshotProbeSlotV2::BoundedWait,
                        ReadyParseSnapshotProbeOutcomeV2::Timeout
                    )
                ),
            )
            .await
            .or(branch_context.ready_snapshot_phase_attribution)
        };
        self.record_diagnostics_save_followup_probe_state_v2(
            uri,
            supersession_key,
            match probe_slot {
                ReadyParseSnapshotProbeSlotV2::ZeroBudget => Some(probe.outcome),
                ReadyParseSnapshotProbeSlotV2::BoundedWait
                | ReadyParseSnapshotProbeSlotV2::ReliefValve => None,
            },
            match probe_slot {
                ReadyParseSnapshotProbeSlotV2::ZeroBudget => None,
                ReadyParseSnapshotProbeSlotV2::BoundedWait => Some(probe.outcome),
                ReadyParseSnapshotProbeSlotV2::ReliefValve => None,
            },
            Some(branch_context.ready_snapshot_task_state),
            Some(branch_context.shadow_state_available),
            ready_snapshot_phase_attribution,
        );
        let Some(ready_state) = probe.state else {
            return SaveFollowupReadyArtifactsAttemptV2::ProbeMiss(probe.outcome);
        };
        self.coordinator
            .record_intellisense_v2_diagnostics_save_followup_semantic_path("ready_artifacts");
        let apply_lag = self
            .diagnostics_followup_apply_lag_v2(supersession_key)
            .await;
        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                supersession_key.file_id,
                None,
                flow_sensitive_semantic,
            )
            .await;
        let support_bundle = self.analysis_v2.completion_support_bundle();
        let path = match uri.to_file_path() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => uri.to_string(),
        };
        let profile = supersession_key.profile;
        let uri_for_blocking = uri.clone();
        let coordinator_for_blocking = self.coordinator.clone();
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let ready_text = ready_state.text.clone();
        let parse_snapshot = ready_state.parse_snapshot.clone();
        let ready_line_index = parse_snapshot.line_index.clone();
        let ready_syntax_errors = parse_snapshot.parse_result.syntax_errors.clone();
        let deps_id = support_bundle.deps_id.clone();
        let deps = support_bundle.deps.clone();
        let settings_id = context.settings.settings_id.clone();
        let diagnostics_detail_level = context.settings.diagnostics_detail_level;
        let context_for_blocking = context.clone();
        let path_for_blocking: Arc<str> = Arc::from(path);
        let semantic_path = Some("ready_artifacts");
        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "semantic_work",
            None,
            apply_lag,
            None,
            None,
            Some("reused"),
            semantic_path,
            None,
            None,
        );
        let queued_wait_server = self.clone();
        let queued_wait_uri = uri.clone();
        let queued_wait_supersession_key = *supersession_key;
        let queued_wait_apply_lag = apply_lag;
        let exec_started_server = self.clone();
        let exec_started_uri = uri.clone();
        let exec_started_supersession_key = *supersession_key;
        let exec_started_apply_lag = apply_lag;
        let followup_call =
            bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
                bsl_runtime::application::CpuWorkClass::Background,
                context.origin.as_str(),
                Some(bsl_runtime::application::AdmissionLane::DidSaveFollowup),
                Some(self.coordinator.as_ref()),
                Some(move || {
                    queued_wait_server.record_diagnostics_save_followup_wait_state_v2(
                        &queued_wait_uri,
                        &queued_wait_supersession_key,
                        "runtime_queue_wait",
                        None,
                        queued_wait_apply_lag,
                        None,
                        None,
                        Some("reused"),
                        semantic_path,
                        None,
                        None,
                    );
                }),
                Some(move |queue_wait_elapsed| {
                    exec_started_server.record_diagnostics_save_followup_wait_state_v2(
                        &exec_started_uri,
                        &exec_started_supersession_key,
                        "semantic_work",
                        (queue_wait_elapsed > Duration::ZERO).then_some(queue_wait_elapsed),
                        exec_started_apply_lag,
                        None,
                        None,
                        Some("reused"),
                        semantic_path,
                        None,
                        None,
                    );
                }),
                move || {
                    let mut host = bsl_analysis_v2::AnalysisHostV2::default();
                    host.apply_change(bsl_analysis_v2::Change::SetDepsSnapshot {
                        deps_id: deps_id.clone(),
                        deps,
                    });
                    host.apply_change(bsl_analysis_v2::Change::SetSettingsSnapshot {
                        settings_id: settings_id.clone(),
                        diagnostics_detail_level,
                    });
                    host.apply_change(bsl_analysis_v2::Change::SetFileWithSnapshot {
                        file_id,
                        text: ready_text.clone(),
                        version: requested_version,
                        path: path_for_blocking,
                        parse_snapshot,
                    });

                    let analysis = host.snapshot();
                    let file_text = ready_text.clone();
                    let line_index = ready_line_index.clone();

                    let mut diagnostics = Vec::new();
                    diagnostics.extend(syntax_errors_to_diagnostics(
                        &ready_syntax_errors,
                        &uri_for_blocking,
                        file_text.as_ref(),
                        line_index.as_ref(),
                    ));

                    let semantic_started = Instant::now();
                    let mut semantic_parse_source = None;
                    let mut semantic_ir_source = None;
                    let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                        &context_for_blocking,
                        bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                        &analysis,
                        Some(coordinator_for_blocking.as_ref()),
                        |analysis| {
                            if flow_sensitive_semantic {
                                analysis.semantic_diagnostics_flow_sensitive_profiled(file_id)
                            } else {
                                analysis.semantic_diagnostics_profiled(file_id)
                            }
                        },
                    )
                    .map_err(|_| ())?;
                    let semantic_elapsed = semantic_started.elapsed();
                    let duration_from_profile_ms =
                        |value: u128| Duration::from_millis(value.min(u64::MAX as u128) as u64);
                    if let Some(profiled) = query {
                        coordinator_for_blocking
                            .record_intellisense_v2_semantic_diagnostics_query_breakdown(
                                duration_from_profile_ms(profiled.profile.inputs_ms),
                                duration_from_profile_ms(profiled.profile.parse_result_ms),
                                duration_from_profile_ms(profiled.profile.ir_ms),
                                duration_from_profile_ms(profiled.profile.collect_ms),
                                (profiled.profile.flow_sensitive_ms > 0).then(|| {
                                    duration_from_profile_ms(profiled.profile.flow_sensitive_ms)
                                }),
                            );
                        semantic_parse_source =
                            profiled.profile.parse_source.map(|source| source.as_str());
                        semantic_ir_source =
                            profiled.profile.ir_source.map(|source| source.as_str());
                        for error in profiled.diagnostics.iter() {
                            if !show_hints
                                && matches!(
                                    error.severity,
                                    bsl_shared::domain::types::DiagnosticSeverity::Hint
                                )
                            {
                                continue;
                            }
                            diagnostics.push(semantic_error_to_diagnostic(
                                error,
                                file_text.as_ref(),
                                line_index.as_ref(),
                            ));
                        }
                    }

                    Ok::<SaveFollowupReadyArtifactsReply, ()>(SaveFollowupReadyArtifactsReply {
                        diagnostics,
                        observed_deps_id: deps_id.as_str().to_string(),
                        observed_settings_id: settings_id.as_str().to_string(),
                        runtime_queue_wait: None,
                        apply_lag: None,
                        syntax_elapsed: None,
                        semantic_elapsed: Some(semantic_elapsed),
                        syntax_work_mode: Some("reused"),
                        semantic_path,
                        semantic_parse_source,
                        semantic_ir_source,
                    })
                },
            )
            .await;
        let runtime_queue_wait =
            Self::sum_nonzero_durations([Some(followup_call.queue_wait_elapsed)]);

        let mut reply = match followup_call.join_result {
            Ok(Ok(reply)) => reply,
            Ok(Err(())) | Err(_) => {
                return SaveFollowupReadyArtifactsAttemptV2::Executed(
                    self.finalize_diagnostics_save_profile_result_v2(
                        uri,
                        supersession_key,
                        trigger,
                        self.diagnostics_cancelled_disposition_v2(
                            cancel_token,
                            self.current_diagnostics_generation_v2(supersession_key.file_id)
                                .await,
                            supersession_key.diagnostics_generation,
                            self.latest_received_file_versions_v2
                                .read()
                                .await
                                .get(&supersession_key.file_id)
                                .copied(),
                            supersession_key.requested_version,
                        ),
                        None,
                        runtime_queue_wait,
                        apply_lag,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        semantic_path,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await,
                );
            }
        };
        reply.runtime_queue_wait = runtime_queue_wait;
        reply.apply_lag = apply_lag;

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(
                supersession_key,
                trigger,
                cancel_token,
                Some(reply.observed_deps_id.as_str()),
                Some(reply.observed_settings_id.as_str()),
            )
            .await
        {
            return SaveFollowupReadyArtifactsAttemptV2::Executed(
                self.finalize_diagnostics_save_profile_result_v2(
                    uri,
                    supersession_key,
                    trigger,
                    disposition,
                    None,
                    reply.runtime_queue_wait,
                    reply.apply_lag,
                    None,
                    None,
                    None,
                    reply.syntax_elapsed,
                    reply.semantic_elapsed,
                    None,
                    reply.syntax_work_mode,
                    reply.semantic_path,
                    reply.semantic_parse_source,
                    reply.semantic_ir_source,
                    pipeline_started,
                )
                .await,
            );
        }

        self.record_diagnostics_save_followup_wait_state_v2(
            uri,
            supersession_key,
            "pending_publish",
            reply.runtime_queue_wait,
            reply.apply_lag,
            None,
            None,
            reply.syntax_work_mode,
            reply.semantic_path,
            reply.semantic_parse_source,
            reply.semantic_ir_source,
        );
        if let Some(guard) = followup_lane_guard {
            guard.release();
        }
        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                supersession_key,
                uri,
                reply.diagnostics,
                trigger,
                profile,
                pipeline_started,
            )
            .await;
        SaveFollowupReadyArtifactsAttemptV2::Executed(
            self.finalize_diagnostics_save_profile_result_v2(
                uri,
                supersession_key,
                trigger,
                disposition,
                Some("full"),
                reply.runtime_queue_wait,
                reply.apply_lag,
                None,
                None,
                None,
                reply.syntax_elapsed,
                reply.semantic_elapsed,
                Some(publish_started.elapsed()),
                reply.syntax_work_mode,
                reply.semantic_path,
                reply.semantic_parse_source,
                reply.semantic_ir_source,
                pipeline_started,
            )
            .await,
        )
    }

    pub(crate) async fn maybe_execute_save_followup_ready_snapshot_relief_valve_v2(
        &self,
        uri: &Url,
        supersession_key: &super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        pipeline_started: Instant,
        show_hints: bool,
        flow_sensitive_semantic: bool,
        followup_lane_guard: Option<&DidSaveFollowupSlotGuard>,
        followup_admission_queue_wait_elapsed: Option<Duration>,
    ) -> Option<bsl_runtime::application::DiagnosticsDisposition> {
        let branch_context = self
            .diagnostics_save_followup_branch_context_v2(supersession_key)
            .await;
        let phase_attribution = self
            .ready_snapshot_phase_attribution_for_probe_v2(
                supersession_key,
                branch_context.shadow_text_hash,
                None,
                true,
            )
            .await
            .or(branch_context.ready_snapshot_phase_attribution);
        let apply_lag = self
            .diagnostics_followup_apply_lag_v2(supersession_key)
            .await;
        let skip_outcome = if branch_context.ready_snapshot_task_state
            != ReadySnapshotTaskStateV2::InFlightSameVersion
        {
            Some(ReadySnapshotReliefValveOutcomeV2::SkippedNotExactStillCurrent)
        } else if followup_admission_queue_wait_elapsed.is_some() {
            Some(ReadySnapshotReliefValveOutcomeV2::SkippedRuntimeQueueWait)
        } else if apply_lag.is_some() {
            Some(ReadySnapshotReliefValveOutcomeV2::SkippedApplyLag)
        } else {
            match phase_attribution {
                Some(attribution) if attribution.has_late_exact_timeout_phase() => None,
                Some(attribution) if attribution.timeout_phase == Some("waiting") => {
                    Some(ReadySnapshotReliefValveOutcomeV2::SkippedTimeoutPhaseWaiting)
                }
                _ => Some(ReadySnapshotReliefValveOutcomeV2::SkippedTimeoutPhaseUnavailable),
            }
        };
        if let Some(outcome) = skip_outcome {
            self.record_diagnostics_save_followup_relief_valve_state_v2(
                uri,
                supersession_key,
                outcome,
                SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET,
                None,
            );
            return None;
        }

        let relief_started = Instant::now();
        let attempt = self
            .try_execute_save_followup_from_ready_artifacts_v2(
                uri,
                supersession_key,
                trigger,
                cancel_token,
                ReadyParseSnapshotProbeSlotV2::ReliefValve,
                SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET,
                pipeline_started,
                show_hints,
                flow_sensitive_semantic,
                followup_lane_guard,
            )
            .await;
        let relief_elapsed = relief_started.elapsed();
        match attempt {
            SaveFollowupReadyArtifactsAttemptV2::Executed(disposition) => {
                self.record_diagnostics_save_followup_relief_valve_state_v2(
                    uri,
                    supersession_key,
                    ReadySnapshotReliefValveOutcomeV2::EngagedHelped,
                    SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET,
                    Some(relief_elapsed),
                );
                Some(disposition)
            }
            SaveFollowupReadyArtifactsAttemptV2::ProbeMiss(outcome) => {
                let outcome = match outcome {
                    ReadyParseSnapshotProbeOutcomeV2::Ready
                    | ReadyParseSnapshotProbeOutcomeV2::NotReady
                    | ReadyParseSnapshotProbeOutcomeV2::Timeout => {
                        ReadySnapshotReliefValveOutcomeV2::EngagedTimedOut
                    }
                    ReadyParseSnapshotProbeOutcomeV2::VersionMismatch => {
                        ReadySnapshotReliefValveOutcomeV2::EngagedVersionMismatch
                    }
                    ReadyParseSnapshotProbeOutcomeV2::GenerationMismatch => {
                        ReadySnapshotReliefValveOutcomeV2::EngagedGenerationMismatch
                    }
                    ReadyParseSnapshotProbeOutcomeV2::Cancelled => {
                        ReadySnapshotReliefValveOutcomeV2::EngagedCancelled
                    }
                    ReadyParseSnapshotProbeOutcomeV2::Superseded => {
                        ReadySnapshotReliefValveOutcomeV2::EngagedSuperseded
                    }
                };
                self.record_diagnostics_save_followup_relief_valve_state_v2(
                    uri,
                    supersession_key,
                    outcome,
                    SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_RELIEF_VALVE_BUDGET,
                    Some(relief_elapsed),
                );
                None
            }
        }
    }

    async fn execute_save_fastlane_profile_once_v2(
        &self,
        uri: &Url,
        supersession_key: super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
        pipeline_started: Instant,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let requested_generation = supersession_key.diagnostics_generation;
        let profile = supersession_key.profile;
        let blocking_queue_wait_elapsed = None;
        let wait_for_file_version_elapsed = None;
        let snapshot_with_deps_elapsed = None;

        let (diagnostics, syntax_errors, syntax_mode, syntax_elapsed) = if let Some(result) = self
            .try_collect_save_fastlane_diagnostics_from_applied_analysis_v2(
                uri,
                file_id,
                requested_version,
            )
            .await
        {
            result
        } else if let Some(result) = self
            .try_collect_save_fastlane_diagnostics_from_ready_parse_snapshot_v2(
                uri,
                file_id,
                requested_version,
            )
            .await
        {
            result
        } else {
            let Some(shadow_state) = self
                .latest_document_shadow_state_v2
                .read()
                .await
                .get(&file_id)
                .cloned()
            else {
                let disposition = self.diagnostics_cancelled_disposition_v2(
                    cancel_token,
                    self.current_diagnostics_generation_v2(file_id).await,
                    requested_generation,
                    self.latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied(),
                    requested_version,
                );
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        None,
                        None,
                        blocking_queue_wait_elapsed,
                        wait_for_file_version_elapsed,
                        snapshot_with_deps_elapsed,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await;
            };

            if shadow_state.version != requested_version {
                let disposition =
                    bsl_runtime::application::DiagnosticsDisposition::SupersededVersion;
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        None,
                        None,
                        blocking_queue_wait_elapsed,
                        wait_for_file_version_elapsed,
                        snapshot_with_deps_elapsed,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await;
            }

            let uri_for_parse = uri.clone();
            let shadow_text = shadow_state.text;
            // Save freshness must not sit behind unrelated shared interactive blocking work.
            let syntax_result = tokio::task::spawn_blocking(move || {
                maybe_inject_save_fastlane_shadow_parse_delay_for_test();
                let started = Instant::now();
                let syntax_errors = bsl_syntax::syntax_errors_only(shadow_text.as_ref());
                let elapsed = started.elapsed();
                match syntax_errors {
                    Ok(syntax_errors) => {
                        let line_index = bsl_line_index::LineIndex::new(shadow_text.as_ref());
                        let diagnostics = syntax_errors_to_diagnostics(
                            &syntax_errors,
                            &uri_for_parse,
                            shadow_text.as_ref(),
                            &line_index,
                        );
                        Ok((syntax_errors, diagnostics, elapsed))
                    }
                    Err(err) => Err((err, elapsed)),
                }
            })
            .await;

            match syntax_result {
                Ok(Ok((syntax_errors, diagnostics, syntax_elapsed))) => {
                    (diagnostics, syntax_errors, "other", syntax_elapsed)
                }
                Ok(Err((err, syntax_elapsed))) => {
                    self.coordinator
                        .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
                            bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                            "other",
                            syntax_elapsed,
                        );
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: save_fastlane syntax parse failed"
                    );
                    let disposition = bsl_runtime::application::DiagnosticsDisposition::OtherCancel;
                    self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                    return self
                        .finalize_diagnostics_save_profile_result_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            disposition,
                            None,
                            None,
                            None,
                            blocking_queue_wait_elapsed,
                            wait_for_file_version_elapsed,
                            snapshot_with_deps_elapsed,
                            Some(syntax_elapsed),
                            None,
                            None,
                            Some("recomputed"),
                            None,
                            None,
                            None,
                            pipeline_started,
                        )
                        .await;
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: save_fastlane spawn_blocking failed"
                    );
                    let disposition = self.diagnostics_cancelled_disposition_v2(
                        cancel_token,
                        self.current_diagnostics_generation_v2(file_id).await,
                        requested_generation,
                        self.latest_received_file_versions_v2
                            .read()
                            .await
                            .get(&file_id)
                            .copied(),
                        requested_version,
                    );
                    self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                    return self
                        .finalize_diagnostics_save_profile_result_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            disposition,
                            None,
                            None,
                            None,
                            blocking_queue_wait_elapsed,
                            wait_for_file_version_elapsed,
                            snapshot_with_deps_elapsed,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            pipeline_started,
                        )
                        .await;
                }
            }
        };

        self.record_save_fastlane_syntax_artifacts_v2(file_id, requested_version, syntax_errors)
            .await;

        self.coordinator
            .record_intellisense_v2_syntax_diagnostics_query_latency_with_origin_and_mode(
                bsl_runtime::application::ObservabilityOrigin::Lsp.as_str(),
                syntax_mode,
                syntax_elapsed,
            );

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(&supersession_key, trigger, cancel_token, None, None)
            .await
        {
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    None,
                    None,
                    blocking_queue_wait_elapsed,
                    wait_for_file_version_elapsed,
                    snapshot_with_deps_elapsed,
                    Some(syntax_elapsed),
                    None,
                    None,
                    Some("recomputed"),
                    None,
                    None,
                    None,
                    pipeline_started,
                )
                .await;
        }

        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                &supersession_key,
                uri,
                diagnostics,
                trigger,
                profile,
                pipeline_started,
            )
            .await;
        self.finalize_diagnostics_save_profile_result_v2(
            uri,
            &supersession_key,
            trigger,
            disposition,
            Some("syntax_only"),
            None,
            None,
            blocking_queue_wait_elapsed,
            wait_for_file_version_elapsed,
            snapshot_with_deps_elapsed,
            Some(syntax_elapsed),
            None,
            Some(publish_started.elapsed()),
            Some("recomputed"),
            None,
            None,
            None,
            pipeline_started,
        )
        .await
    }

    fn configured_workspace_root_for_semantic_v2(
        config: Option<&crate::config::LspConfig>,
    ) -> Option<std::path::PathBuf> {
        let path = config
            .and_then(|cfg| cfg.configuration_path.as_ref())
            .map(std::path::PathBuf::from)?;
        if path.is_dir() {
            return Some(path);
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("Configuration.xml") {
            return path.parent().map(std::path::Path::to_path_buf);
        }
        None
    }

    fn resolve_path_scope_for_semantic_v2(path: &std::path::Path) -> Option<std::path::PathBuf> {
        if path.exists() {
            path.canonicalize().ok()
        } else if path.is_absolute() {
            Some(path.to_path_buf())
        } else {
            None
        }
    }

    async fn should_run_semantic_diagnostics_for_uri_v2(&self, uri: &Url) -> bool {
        let config = self.config.read().await.clone();
        let Some(config_root) = Self::configured_workspace_root_for_semantic_v2(config.as_ref())
        else {
            return true;
        };
        let Ok(document_path) = uri.to_file_path() else {
            return true;
        };
        let Some(config_root) = Self::resolve_path_scope_for_semantic_v2(&config_root) else {
            return true;
        };
        let Some(document_path) = Self::resolve_path_scope_for_semantic_v2(&document_path) else {
            return true;
        };
        let in_config_root = document_path.starts_with(&config_root);
        if !in_config_root {
            debug!(
                uri = %uri,
                document_path = %document_path.display(),
                config_root = %config_root.display(),
                "diagnostics_v2: skip semantic stage for document outside configured configurationPath"
            );
        }
        in_config_root
    }

    pub(crate) async fn run_diagnostics_profile_immediate_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        diagnostics_generation: u64,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
    ) {
        let supersession_key = super::super::DiagnosticsSupersessionKeyV2 {
            file_id,
            profile,
            diagnostics_generation,
            save_cycle_sequence: None,
            requested_version: expected_version,
        };
        let _ = self
            .execute_diagnostics_profile_once_v2(&uri, supersession_key, trigger, None)
            .await;
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn schedule_diagnostics_profile_v2(
        &self,
        uri: Url,
        file_id: V2FileId,
        expected_version: i32,
        diagnostics_generation: u64,
        save_cycle_sequence: Option<u64>,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        profile: bsl_runtime::application::DiagnosticsProfile,
        debounce: bool,
    ) {
        let slot_key = super::super::DiagnosticsTaskKeyV2 { file_id, profile };
        let supersession_key = super::super::DiagnosticsSupersessionKeyV2 {
            file_id,
            profile,
            diagnostics_generation,
            save_cycle_sequence,
            requested_version: expected_version,
        };
        let mut tasks = self.diagnostics_tasks_v2.lock().await;
        if let Some(task) = tasks.get_mut(&slot_key) {
            if task.supersession_key != supersession_key {
                let reason = super::super::DiagnosticsCancellationReasonV2::for_supersession(
                    task.supersession_key,
                    diagnostics_generation,
                    expected_version,
                );
                task.cancel_token.cancel(reason);
                self.record_diagnostics_pipeline_event_v2(
                    task.trigger,
                    task.supersession_key.profile,
                    reason.to_disposition(),
                );
                if let Some(save_cycle_sequence) = task.supersession_key.save_cycle_sequence {
                    self.record_diagnostics_save_timeline_profile_disposition(
                        &uri,
                        super::super::DiagnosticsSaveTimelineCycleKey {
                            file_id: task.supersession_key.file_id,
                            diagnostics_generation: task.supersession_key.diagnostics_generation,
                            save_cycle_sequence,
                            requested_version: task.supersession_key.requested_version,
                        },
                        task.supersession_key.profile,
                        reason.to_disposition(),
                    );
                }
                task.cancel_token = super::super::DiagnosticsCancellationTokenV2::new();
                task.supersession_key = supersession_key;
            }
            task.trigger = trigger;
            task.debounce = debounce;
            return;
        }

        let server = self.clone();
        let uri_for_task = uri.clone();
        let initial_cancel_token = super::super::DiagnosticsCancellationTokenV2::new();
        let handle = tokio::spawn(async move {
            loop {
                // If the document is already closed, stop.
                let file_still_open = server
                    .latest_received_file_versions_v2
                    .read()
                    .await
                    .contains_key(&file_id);
                if !file_still_open {
                    break;
                }

                let (supersession_key, trigger, debounce, cancel_token) = {
                    let tasks = server.diagnostics_tasks_v2.lock().await;
                    let Some(task) = tasks.get(&slot_key) else {
                        break;
                    };
                    (
                        task.supersession_key,
                        task.trigger,
                        task.debounce,
                        task.cancel_token.clone(),
                    )
                };

                // Coalesce rapid edits: while user is typing, keep moving the target forward.
                if debounce {
                    let delay = diagnostics_debounce_duration();
                    if delay != Duration::from_millis(0) {
                        tokio::time::sleep(delay).await;
                    }

                    let current_requested = {
                        let tasks = server.diagnostics_tasks_v2.lock().await;
                        let Some(task) = tasks.get(&slot_key) else {
                            break;
                        };
                        task.supersession_key
                    };
                    if current_requested != supersession_key {
                        continue;
                    }
                }

                if server
                    .diagnostics_checkpoint_v2(&supersession_key, trigger, Some(&cancel_token))
                    .await
                    .is_some()
                {
                    continue;
                }

                let _ = server
                    .execute_diagnostics_profile_once_v2(
                        &uri_for_task,
                        supersession_key,
                        trigger,
                        Some(&cancel_token),
                    )
                    .await;

                let mut tasks = server.diagnostics_tasks_v2.lock().await;
                let Some(task) = tasks.get(&slot_key) else {
                    break;
                };
                if task.supersession_key == supersession_key
                    && task.cancel_token.same_inner(&cancel_token)
                {
                    tasks.remove(&slot_key);
                    break;
                }
            }
        });

        tasks.insert(
            slot_key,
            super::super::DiagnosticsTaskV2 {
                supersession_key,
                cancel_token: initial_cancel_token,
                trigger,
                debounce,
                handle,
            },
        );
    }

    async fn execute_diagnostics_profile_once_v2(
        &self,
        uri: &Url,
        supersession_key: super::super::DiagnosticsSupersessionKeyV2,
        trigger: bsl_runtime::application::DiagnosticsTrigger,
        cancel_token: Option<&super::super::DiagnosticsCancellationTokenV2>,
    ) -> bsl_runtime::application::DiagnosticsDisposition {
        let pipeline_started = Instant::now();
        if let Some(disposition) = self
            .diagnostics_checkpoint_v2(&supersession_key, trigger, cancel_token)
            .await
        {
            return disposition;
        }
        let file_id = supersession_key.file_id;
        let requested_version = supersession_key.requested_version;
        let requested_generation = supersession_key.diagnostics_generation;
        let profile = supersession_key.profile;
        if matches!(
            profile,
            bsl_runtime::application::DiagnosticsProfile::SaveFastlane
        ) {
            return self
                .execute_save_fastlane_profile_once_v2(
                    uri,
                    supersession_key,
                    trigger,
                    cancel_token,
                    pipeline_started,
                )
                .await;
        }

        let (show_hints, flow_sensitive_enabled) = {
            let settings = self.settings.read().await;
            (
                settings.diagnostics.show_hints,
                settings.enable_flow_sensitive,
            )
        };
        let plan =
            bsl_runtime::application::diagnostics_execution_plan(profile, flow_sensitive_enabled);
        let run_semantic =
            plan.run_semantic && self.should_run_semantic_diagnostics_for_uri_v2(uri).await;
        let save_followup_from_did_save = matches!(
            (trigger, profile),
            (
                bsl_runtime::application::DiagnosticsTrigger::DidSave,
                bsl_runtime::application::DiagnosticsProfile::IdleHeavy
            )
        );
        if !plan.run_syntax && !run_semantic {
            self.record_diagnostics_pipeline_event_v2(
                trigger,
                profile,
                bsl_runtime::application::DiagnosticsDisposition::Published,
            );
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    bsl_runtime::application::DiagnosticsDisposition::Published,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    pipeline_started,
                )
                .await;
        }

        let mut followup_syntax_artifact_reuse_allowed = false;
        let mut followup_admission_queue_wait_elapsed = None;
        let mut did_save_followup_lane_guard: Option<DidSaveFollowupSlotGuard> = None;
        if save_followup_from_did_save && run_semantic {
            let applied_revision_matches_requested = || {
                self.analysis_v2
                    .cached_file_revision_state(file_id)
                    .is_some_and(|state| state.version == requested_version)
            };
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                "pending_publish",
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            let save_fastlane_first_publish_completed = matches!(
                self.wait_for_save_fastlane_first_publish_v2(&supersession_key, cancel_token)
                    .await,
                SaveFastlaneFirstPublishWaitOutcome::Published
            );
            match self
                .acquire_did_save_followup_lane_v2(uri, &supersession_key, trigger, cancel_token)
                .await
            {
                DidSaveFollowupAdmissionOutcome::Admitted {
                    guard,
                    queue_wait_elapsed,
                } => {
                    followup_admission_queue_wait_elapsed = queue_wait_elapsed;
                    did_save_followup_lane_guard = Some(guard);
                }
                DidSaveFollowupAdmissionOutcome::Disposition {
                    disposition,
                    queue_wait_elapsed,
                } => {
                    self.record_diagnostics_pipeline_event_v2(
                        trigger,
                        supersession_key.profile,
                        disposition,
                    );
                    return self
                        .finalize_diagnostics_save_profile_result_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            disposition,
                            None,
                            queue_wait_elapsed,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            pipeline_started,
                        )
                        .await;
                }
            }

            followup_syntax_artifact_reuse_allowed = save_fastlane_first_publish_completed
                && plan.run_syntax
                && self
                    .save_fastlane_syntax_artifacts_for_version_v2(file_id, requested_version)
                    .await
                    .is_some();

            if save_fastlane_first_publish_completed {
                let branch_context = self
                    .diagnostics_save_followup_branch_context_v2(&supersession_key)
                    .await;
                let prefer_inflight_exact_wait = matches!(
                    branch_context.ready_snapshot_task_state,
                    ReadySnapshotTaskStateV2::InFlightSameVersion
                );
                self.record_diagnostics_save_followup_probe_state_v2(
                    uri,
                    &supersession_key,
                    None,
                    None,
                    Some(branch_context.ready_snapshot_task_state),
                    Some(branch_context.shadow_state_available),
                    branch_context.ready_snapshot_phase_attribution,
                );
                if let SaveFollowupReadyArtifactsAttemptV2::Executed(disposition) = self
                    .try_execute_save_followup_from_ready_artifacts_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        cancel_token,
                        ReadyParseSnapshotProbeSlotV2::ZeroBudget,
                        Duration::ZERO,
                        pipeline_started,
                        show_hints,
                        plan.flow_sensitive_semantic,
                        did_save_followup_lane_guard.as_ref(),
                    )
                    .await
                {
                    return disposition;
                }
                if prefer_inflight_exact_wait {
                    let _ = self
                        .promote_background_parse_snapshot_apply_task_for_did_save_v2(
                            file_id,
                            requested_version,
                            branch_context.shadow_text_hash,
                        )
                        .await;
                    if let SaveFollowupReadyArtifactsAttemptV2::Executed(disposition) = self
                        .try_execute_save_followup_from_ready_artifacts_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            cancel_token,
                            ReadyParseSnapshotProbeSlotV2::BoundedWait,
                            SAVE_FOLLOWUP_READY_PARSE_SNAPSHOT_WAIT_BUDGET,
                            pipeline_started,
                            show_hints,
                            plan.flow_sensitive_semantic,
                            did_save_followup_lane_guard.as_ref(),
                        )
                        .await
                    {
                        return disposition;
                    }
                    if let Some(disposition) = self
                        .maybe_execute_save_followup_ready_snapshot_relief_valve_v2(
                            uri,
                            &supersession_key,
                            trigger,
                            cancel_token,
                            pipeline_started,
                            show_hints,
                            plan.flow_sensitive_semantic,
                            did_save_followup_lane_guard.as_ref(),
                            followup_admission_queue_wait_elapsed,
                        )
                        .await
                    {
                        return disposition;
                    }
                }
                if let Some(disposition) = self
                    .try_execute_save_followup_from_shadow_state_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        cancel_token,
                        pipeline_started,
                        show_hints,
                        plan.flow_sensitive_semantic,
                        did_save_followup_lane_guard.as_ref(),
                    )
                    .await
                {
                    return disposition;
                }
            }

            let wait_reason = if applied_revision_matches_requested() {
                "semantic_work"
            } else {
                "apply_lag"
            };
            let apply_lag = self
                .diagnostics_followup_apply_lag_v2(&supersession_key)
                .await;
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                wait_reason,
                followup_admission_queue_wait_elapsed,
                apply_lag,
                None,
                None,
                None,
                Some("generic_pipeline"),
                None,
                None,
            );
        }

        let context = self
            .build_execution_context_v2(
                bsl_runtime::application::SemanticOperation::Diagnostics,
                file_id,
                Some(requested_version),
                plan.flow_sensitive_semantic,
            )
            .await;
        let prepared = match if save_followup_from_did_save && run_semantic {
            self.analysis_v2
                .prepare_stateful_operation_with_admission_lane(
                    &context,
                    Some(self.coordinator.as_ref()),
                    Some(bsl_runtime::application::AdmissionLane::DidSaveFollowup),
                )
                .await
        } else {
            self.analysis_v2
                .prepare_stateful_operation(&context, Some(self.coordinator.as_ref()))
                .await
        } {
            Ok(prepared) => prepared,
            Err(outcome) => {
                let disposition = if matches!(
                    outcome,
                    bsl_runtime::application::SemanticOutcome::StaleVersion
                ) {
                    bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
                } else {
                    let current_generation = self.current_diagnostics_generation_v2(file_id).await;
                    let current_version = self
                        .latest_received_file_versions_v2
                        .read()
                        .await
                        .get(&file_id)
                        .copied();
                    self.diagnostics_cancelled_disposition_v2(
                        cancel_token,
                        current_generation,
                        requested_generation,
                        current_version,
                        requested_version,
                    )
                };
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    expected_generation = requested_generation,
                    profile = profile.as_str(),
                    trigger = trigger.as_str(),
                    outcome = outcome.as_str(),
                    "diagnostics_v2: skip publish (stateful operation not ready)"
                );
                self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        pipeline_started,
                    )
                    .await;
            }
        };

        let wait_elapsed = prepared.wait_elapsed.unwrap_or(Duration::ZERO);
        let wait_for_file_version_runtime = prepared.wait_for_file_version_runtime;
        let snapshot_with_deps_runtime = prepared.snapshot_with_deps_runtime;
        let snapshot_elapsed = prepared.snapshot_elapsed;
        let mut blocking_queue_wait_elapsed = None;
        let mut followup_runtime_queue_wait_elapsed = if save_followup_from_did_save && run_semantic
        {
            Self::sum_nonzero_durations([
                followup_admission_queue_wait_elapsed,
                wait_for_file_version_runtime.and_then(|trace| trace.queue_wait_elapsed),
                snapshot_with_deps_runtime.queue_wait_elapsed,
            ])
        } else {
            None
        };
        let followup_apply_lag_elapsed = if save_followup_from_did_save && run_semantic {
            Self::sum_nonzero_durations([
                wait_for_file_version_runtime.and_then(|trace| trace.wake_wait_elapsed)
            ])
        } else {
            None
        };
        let mut syntax_stage_elapsed: Option<Duration> = None;
        let mut semantic_stage_elapsed: Option<Duration> = None;
        let save_fastlane_syntax_artifacts = if save_followup_from_did_save
            && run_semantic
            && plan.run_syntax
            && followup_syntax_artifact_reuse_allowed
        {
            self.save_fastlane_syntax_artifacts_for_version_v2(file_id, requested_version)
                .await
        } else {
            None
        };
        let followup_syntax_work_mode =
            if save_followup_from_did_save && run_semantic && plan.run_syntax {
                Some(if save_fastlane_syntax_artifacts.is_some() {
                    "reused"
                } else {
                    "recomputed"
                })
            } else {
                None
            };
        let followup_semantic_path = if save_followup_from_did_save && run_semantic {
            Some("generic_pipeline")
        } else {
            None
        };
        let mut followup_semantic_parse_source: Option<&'static str> = None;
        let mut followup_semantic_ir_source: Option<&'static str> = None;
        if save_followup_from_did_save && run_semantic {
            self.coordinator
                .record_intellisense_v2_diagnostics_save_followup_semantic_path("generic_pipeline");
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                "semantic_work",
                followup_runtime_queue_wait_elapsed,
                followup_apply_lag_elapsed,
                Some(wait_elapsed),
                Some(snapshot_elapsed),
                followup_syntax_work_mode,
                followup_semantic_path,
                None,
                None,
            );
        }
        if wait_elapsed > Duration::ZERO {
            if let Some(threshold) = super::super::intellisense_v2_slow_client_log_threshold() {
                if wait_elapsed >= threshold {
                    self.client
                        .log_message(
                            MessageType::INFO,
                            format!(
                                "[perf] diagnostics_v2 wait_for_file_version: wait_ms={} uri={} file_id={} expected_version={} profile={}",
                                wait_elapsed.as_millis(),
                                uri,
                                file_id.0,
                                requested_version,
                                profile.as_str(),
                            ),
                        )
                        .await;
                }
            }
            if let Some(threshold) = super::super::intellisense_v2_slow_wait_warn_threshold() {
                if wait_elapsed >= threshold {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        wait_ms = wait_elapsed.as_millis(),
                        threshold_ms = threshold.as_millis(),
                        "diagnostics_v2: wait_for_file_version is slow"
                    );
                }
            }
        }

        if let Some(disposition) = self
            .diagnostics_checkpoint_v2(&supersession_key, trigger, cancel_token)
            .await
        {
            return disposition;
        }

        let mut analysis = Some(prepared.snapshot.analysis);
        let index_snapshot_id = prepared.index_snapshot.id.as_str().to_string();
        let observed_deps_id = Some(prepared.snapshot.deps_id.as_str().to_string());
        let observed_settings_id = analysis
            .as_ref()
            .and_then(|value| value.settings_id().ok().map(|id| id.as_str().to_string()));
        let file_text = analysis
            .as_ref()
            .and_then(|value| value.file_text(file_id).ok().flatten());
        let line_index = analysis
            .as_ref()
            .and_then(|value| value.line_index(file_id).ok().flatten());
        let (file_bytes, file_lines) = file_text
            .as_deref()
            .map(|text| (text.len(), text.lines().count()))
            .unwrap_or((0, 0));

        if let Some(threshold) = super::super::intellisense_v2_slow_snapshot_warn_threshold() {
            if snapshot_elapsed >= threshold {
                warn!(
                    uri = %uri,
                    file_id = file_id.0,
                    expected_version = requested_version,
                    expected_generation = requested_generation,
                    profile = profile.as_str(),
                    snapshot_ms = snapshot_elapsed.as_millis(),
                    threshold_ms = threshold.as_millis(),
                    "diagnostics_v2: snapshot acquisition is slow"
                );
            }
        }

        let mut diagnostics = Vec::new();
        let mut was_cancelled = false;

        if plan.run_syntax {
            if let Some(syntax_errors) = save_fastlane_syntax_artifacts {
                if let (Some(text), Some(index)) = (file_text.as_deref(), line_index.as_deref()) {
                    diagnostics.extend(syntax_errors_to_diagnostics(
                        syntax_errors.as_ref(),
                        uri,
                        text,
                        index,
                    ));
                }
            } else {
                self.coordinator
                    .record_intellisense_v2_payload_shape_with_origin(
                        context.origin.as_str(),
                        context.operation.as_str(),
                        bsl_runtime::application::ObservabilityStage::SyntaxDiagnosticsQuery
                            .as_str(),
                        file_bytes,
                        file_lines,
                    );
                let analysis_for_blocking = analysis
                    .take()
                    .expect("analysis snapshot must be available for syntax stage");
                let context_for_blocking = context.clone();
                let coordinator_for_blocking = self.coordinator.clone();
                let uri_for_blocking = uri.clone();
                let file_text_for_blocking = file_text.clone();
                let line_index_for_blocking = line_index.clone();
                let syntax_queue_wait_started_hook = if save_followup_from_did_save && run_semantic
                {
                    let queued_wait_server = self.clone();
                    let queued_wait_uri = uri.clone();
                    let queued_wait_supersession_key = supersession_key;
                    let queued_wait_apply_lag = followup_apply_lag_elapsed;
                    let queued_wait_runtime_queue_wait = followup_runtime_queue_wait_elapsed;
                    let queued_wait_syntax_work_mode = followup_syntax_work_mode;
                    Some(move || {
                        queued_wait_server.record_diagnostics_save_followup_wait_state_v2(
                            &queued_wait_uri,
                            &queued_wait_supersession_key,
                            "runtime_queue_wait",
                            queued_wait_runtime_queue_wait,
                            queued_wait_apply_lag,
                            Some(wait_elapsed),
                            Some(snapshot_elapsed),
                            queued_wait_syntax_work_mode,
                            followup_semantic_path,
                            None,
                            None,
                        );
                    })
                } else {
                    None
                };
                let syntax_exec_started_hook = if save_followup_from_did_save && run_semantic {
                    let exec_started_server = self.clone();
                    let exec_started_uri = uri.clone();
                    let exec_started_supersession_key = supersession_key;
                    let exec_started_apply_lag = followup_apply_lag_elapsed;
                    let exec_started_runtime_queue_wait = followup_runtime_queue_wait_elapsed;
                    let exec_started_syntax_work_mode = followup_syntax_work_mode;
                    Some(move |queue_wait_elapsed| {
                        exec_started_server.record_diagnostics_save_followup_wait_state_v2(
                            &exec_started_uri,
                            &exec_started_supersession_key,
                            "semantic_work",
                            Self::sum_nonzero_durations([
                                exec_started_runtime_queue_wait,
                                (queue_wait_elapsed > Duration::ZERO).then_some(queue_wait_elapsed),
                            ]),
                            exec_started_apply_lag,
                            Some(wait_elapsed),
                            Some(snapshot_elapsed),
                            exec_started_syntax_work_mode,
                            followup_semantic_path,
                            None,
                            None,
                        );
                    })
                } else {
                    None
                };
                let syntax_call = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
                    plan.cpu_class,
                    context_for_blocking.origin.as_str(),
                    (save_followup_from_did_save && run_semantic)
                        .then_some(bsl_runtime::application::AdmissionLane::DidSaveFollowup),
                    Some(self.coordinator.as_ref()),
                    syntax_queue_wait_started_hook,
                    syntax_exec_started_hook,
                    move || {
                        let started = Instant::now();
                        let syntax_query =
                            bsl_runtime::application::IntellisenseV2Facade::run_syntax_diagnostics_query_singleflight(
                                &context_for_blocking,
                                &analysis_for_blocking,
                                Some(coordinator_for_blocking.as_ref()),
                                file_id,
                            );
                        let elapsed = started.elapsed();
                        match syntax_query {
                            Ok(Some(syntax_errors)) => {
                                let mut diagnostics = Vec::new();
                                if let (Some(text), Some(index)) =
                                    (file_text_for_blocking.as_deref(), line_index_for_blocking.as_deref())
                                {
                                    diagnostics.extend(syntax_errors_to_diagnostics(
                                        &syntax_errors,
                                        &uri_for_blocking,
                                        text,
                                        index,
                                    ));
                                }
                                (diagnostics, false, elapsed, analysis_for_blocking)
                            }
                            Ok(None) => (Vec::new(), false, elapsed, analysis_for_blocking),
                            Err(_) => (Vec::new(), true, elapsed, analysis_for_blocking),
                        }
                    },
                )
                .await;
                let syntax_queue_wait =
                    Self::sum_nonzero_durations([Some(syntax_call.queue_wait_elapsed)]);
                blocking_queue_wait_elapsed =
                    Self::sum_nonzero_durations([blocking_queue_wait_elapsed, syntax_queue_wait]);
                if save_followup_from_did_save && run_semantic {
                    followup_runtime_queue_wait_elapsed = Self::sum_nonzero_durations([
                        followup_runtime_queue_wait_elapsed,
                        syntax_queue_wait,
                    ]);
                }
                match syntax_call.join_result {
                    Ok((syntax_diagnostics, syntax_cancelled, syntax_elapsed, next_analysis)) => {
                        analysis = Some(next_analysis);
                        diagnostics.extend(syntax_diagnostics);
                        was_cancelled |= syntax_cancelled;
                        syntax_stage_elapsed = Some(syntax_elapsed);
                        if let Some(threshold) =
                            super::super::intellisense_v2_slow_query_warn_threshold()
                        {
                            if syntax_elapsed >= threshold {
                                warn!(
                                    uri = %uri,
                                    file_id = file_id.0,
                                    expected_version = requested_version,
                                    expected_generation = requested_generation,
                                    profile = profile.as_str(),
                                    syntax_diagnostics_ms = syntax_elapsed.as_millis(),
                                    file_bytes,
                                    file_lines,
                                    threshold_ms = threshold.as_millis(),
                                    "diagnostics_v2: syntax_diagnostics query is slow"
                                );
                            }
                        }
                    }
                    Err(err) => {
                        warn!(
                            uri = %uri,
                            file_id = file_id.0,
                            expected_version = requested_version,
                            expected_generation = requested_generation,
                            profile = profile.as_str(),
                            error = ?err,
                            "diagnostics_v2: syntax spawn_blocking failed"
                        );
                        was_cancelled = true;
                    }
                }
            }
        }

        if was_cancelled {
            let current_generation = self.current_diagnostics_generation_v2(file_id).await;
            let current_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();
            let disposition = self.diagnostics_cancelled_disposition_v2(
                cancel_token,
                current_generation,
                requested_generation,
                current_version,
                requested_version,
            );
            self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    followup_runtime_queue_wait_elapsed,
                    followup_apply_lag_elapsed,
                    blocking_queue_wait_elapsed,
                    Some(wait_elapsed),
                    Some(snapshot_elapsed),
                    syntax_stage_elapsed,
                    semantic_stage_elapsed,
                    None,
                    None,
                    followup_semantic_path,
                    followup_semantic_parse_source,
                    followup_semantic_ir_source,
                    pipeline_started,
                )
                .await;
        }

        if run_semantic {
            self.coordinator
                .record_intellisense_v2_payload_shape_with_origin(
                    context.origin.as_str(),
                    context.operation.as_str(),
                    bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery.as_str(),
                    file_bytes,
                    file_lines,
                );
            if let Some(disposition) = self
                .diagnostics_checkpoint_v2(&supersession_key, trigger, cancel_token)
                .await
            {
                return self
                    .finalize_diagnostics_save_profile_result_v2(
                        uri,
                        &supersession_key,
                        trigger,
                        disposition,
                        None,
                        followup_runtime_queue_wait_elapsed,
                        followup_apply_lag_elapsed,
                        blocking_queue_wait_elapsed,
                        Some(wait_elapsed),
                        Some(snapshot_elapsed),
                        syntax_stage_elapsed,
                        semantic_stage_elapsed,
                        None,
                        followup_syntax_work_mode,
                        followup_semantic_path,
                        followup_semantic_parse_source,
                        followup_semantic_ir_source,
                        pipeline_started,
                    )
                    .await;
            }

            let analysis_for_blocking = analysis
                .take()
                .expect("analysis snapshot must be available for semantic stage");
            let context_for_blocking = context.clone();
            let coordinator_for_blocking = self.coordinator.clone();
            let file_text_for_blocking = file_text.clone();
            let line_index_for_blocking = line_index.clone();
            let semantic_flow_sensitive = plan.flow_sensitive_semantic;
            let semantic_show_hints = show_hints;
            let semantic_queue_wait_started_hook = if save_followup_from_did_save && run_semantic {
                let queued_wait_server = self.clone();
                let queued_wait_uri = uri.clone();
                let queued_wait_supersession_key = supersession_key;
                let queued_wait_apply_lag = followup_apply_lag_elapsed;
                let queued_wait_runtime_queue_wait = followup_runtime_queue_wait_elapsed;
                let queued_wait_syntax_work_mode = followup_syntax_work_mode;
                Some(move || {
                    queued_wait_server.record_diagnostics_save_followup_wait_state_v2(
                        &queued_wait_uri,
                        &queued_wait_supersession_key,
                        "runtime_queue_wait",
                        queued_wait_runtime_queue_wait,
                        queued_wait_apply_lag,
                        Some(wait_elapsed),
                        Some(snapshot_elapsed),
                        queued_wait_syntax_work_mode,
                        followup_semantic_path,
                        None,
                        None,
                    );
                })
            } else {
                None
            };
            let semantic_exec_started_hook = if save_followup_from_did_save && run_semantic {
                let exec_started_server = self.clone();
                let exec_started_uri = uri.clone();
                let exec_started_supersession_key = supersession_key;
                let exec_started_apply_lag = followup_apply_lag_elapsed;
                let exec_started_runtime_queue_wait = followup_runtime_queue_wait_elapsed;
                let exec_started_syntax_work_mode = followup_syntax_work_mode;
                Some(move |queue_wait_elapsed| {
                    exec_started_server.record_diagnostics_save_followup_wait_state_v2(
                        &exec_started_uri,
                        &exec_started_supersession_key,
                        "semantic_work",
                        Self::sum_nonzero_durations([
                            exec_started_runtime_queue_wait,
                            (queue_wait_elapsed > Duration::ZERO).then_some(queue_wait_elapsed),
                        ]),
                        exec_started_apply_lag,
                        Some(wait_elapsed),
                        Some(snapshot_elapsed),
                        exec_started_syntax_work_mode,
                        followup_semantic_path,
                        None,
                        None,
                    );
                })
            } else {
                None
            };
            let semantic_call = bsl_runtime::application::spawn_bounded_blocking_with_class_observed_call_origin_lane_hooks(
                plan.cpu_class,
                context_for_blocking.origin.as_str(),
                (save_followup_from_did_save && run_semantic)
                    .then_some(bsl_runtime::application::AdmissionLane::DidSaveFollowup),
                Some(self.coordinator.as_ref()),
                semantic_queue_wait_started_hook,
                semantic_exec_started_hook,
                move || {
                    let started = Instant::now();
                    let query = bsl_runtime::application::IntellisenseV2Facade::run_optional_query(
                        &context_for_blocking,
                        bsl_runtime::application::ObservabilityStage::SemanticDiagnosticsQuery,
                        &analysis_for_blocking,
                        Some(coordinator_for_blocking.as_ref()),
                        |analysis| {
                            if semantic_flow_sensitive {
                                analysis.semantic_diagnostics_flow_sensitive_profiled(file_id)
                            } else {
                                analysis.semantic_diagnostics_profiled(file_id)
                            }
                        },
                    );
                    let elapsed = started.elapsed();
                    let duration_from_profile_ms = |value: u128| {
                        Duration::from_millis(value.min(u64::MAX as u128) as u64)
                    };
                    match query {
                        Ok(Some(profiled)) => {
                            coordinator_for_blocking
                                .record_intellisense_v2_semantic_diagnostics_query_breakdown(
                                    duration_from_profile_ms(profiled.profile.inputs_ms),
                                    duration_from_profile_ms(profiled.profile.parse_result_ms),
                                    duration_from_profile_ms(profiled.profile.ir_ms),
                                    duration_from_profile_ms(profiled.profile.collect_ms),
                                    (profiled.profile.flow_sensitive_ms > 0).then(|| {
                                        duration_from_profile_ms(profiled.profile.flow_sensitive_ms)
                                    }),
                                );
                            let mut diagnostics = Vec::new();
                            for error in profiled.diagnostics.iter() {
                                if !semantic_show_hints
                                    && matches!(
                                        error.severity,
                                        bsl_shared::domain::types::DiagnosticSeverity::Hint
                                    )
                                {
                                    continue;
                                }
                                if let (Some(text), Some(index)) =
                                    (file_text_for_blocking.as_deref(), line_index_for_blocking.as_deref())
                                {
                                    diagnostics.push(semantic_error_to_diagnostic(error, text, index));
                                }
                            }
                            (
                                diagnostics,
                                false,
                                elapsed,
                                profiled.profile.parse_source.map(|source| source.as_str()),
                                profiled.profile.ir_source.map(|source| source.as_str()),
                            )
                        }
                        Ok(None) => (Vec::new(), false, elapsed, None, None),
                        Err(_) => (Vec::new(), true, elapsed, None, None),
                    }
                },
            )
            .await;
            let semantic_queue_wait =
                Self::sum_nonzero_durations([Some(semantic_call.queue_wait_elapsed)]);
            blocking_queue_wait_elapsed =
                Self::sum_nonzero_durations([blocking_queue_wait_elapsed, semantic_queue_wait]);
            if save_followup_from_did_save && run_semantic {
                followup_runtime_queue_wait_elapsed = Self::sum_nonzero_durations([
                    followup_runtime_queue_wait_elapsed,
                    semantic_queue_wait,
                ]);
            }
            match semantic_call.join_result {
                Ok((
                    semantic_diagnostics,
                    semantic_cancelled,
                    semantic_elapsed,
                    semantic_parse_source,
                    semantic_ir_source,
                )) => {
                    diagnostics.extend(semantic_diagnostics);
                    was_cancelled |= semantic_cancelled;
                    semantic_stage_elapsed = Some(semantic_elapsed);
                    followup_semantic_parse_source = semantic_parse_source;
                    followup_semantic_ir_source = semantic_ir_source;
                    if let Some(threshold) =
                        super::super::intellisense_v2_slow_query_warn_threshold()
                    {
                        if semantic_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                expected_version = requested_version,
                                expected_generation = requested_generation,
                                profile = profile.as_str(),
                                semantic_diagnostics_ms = semantic_elapsed.as_millis(),
                                file_bytes,
                                file_lines,
                                threshold_ms = threshold.as_millis(),
                                "diagnostics_v2: semantic_diagnostics query is slow"
                            );
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        uri = %uri,
                        file_id = file_id.0,
                        expected_version = requested_version,
                        expected_generation = requested_generation,
                        profile = profile.as_str(),
                        error = ?err,
                        "diagnostics_v2: semantic spawn_blocking failed"
                    );
                    was_cancelled = true;
                }
            }
        }

        let diagnostics_len = diagnostics.len();
        let (is_current, current_version, current_deps_id, current_settings_id, current_generation) = {
            let current_version = self
                .latest_received_file_versions_v2
                .read()
                .await
                .get(&file_id)
                .copied();
            let current_deps_id = self
                .last_deps_id_v2
                .read()
                .await
                .as_ref()
                .map(|id| id.as_str().to_string());
            let current_settings_id = self
                .last_settings_id_v2
                .read()
                .await
                .as_ref()
                .map(|id| id.as_str().to_string());
            let current_generation = self.current_diagnostics_generation_v2(file_id).await;
            let is_current = !was_cancelled
                && current_version == Some(requested_version)
                && current_deps_id == observed_deps_id
                && current_settings_id == observed_settings_id
                && current_generation == Some(requested_generation);
            (
                is_current,
                current_version,
                current_deps_id,
                current_settings_id,
                current_generation,
            )
        };

        if !is_current {
            let disposition = if was_cancelled {
                self.diagnostics_cancelled_disposition_v2(
                    cancel_token,
                    current_generation,
                    requested_generation,
                    current_version,
                    requested_version,
                )
            } else if current_generation != Some(requested_generation) {
                bsl_runtime::application::DiagnosticsDisposition::SupersededGeneration
            } else if current_version != Some(requested_version) {
                bsl_runtime::application::DiagnosticsDisposition::SupersededVersion
            } else {
                bsl_runtime::application::DiagnosticsDisposition::OtherCancel
            };
            debug!(
                uri = %uri,
                file_id = file_id.0,
                expected_version = requested_version,
                expected_generation = requested_generation,
                current_version = ?current_version,
                current_generation = ?current_generation,
                observed_deps_id = ?observed_deps_id,
                current_deps_id = ?current_deps_id,
                observed_settings_id = ?observed_settings_id,
                current_settings_id = ?current_settings_id,
                profile = profile.as_str(),
                trigger = trigger.as_str(),
                disposition = disposition.as_str(),
                "diagnostics_v2: skip publish (stale)"
            );
            self.record_diagnostics_pipeline_event_v2(trigger, profile, disposition);
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    followup_runtime_queue_wait_elapsed,
                    followup_apply_lag_elapsed,
                    blocking_queue_wait_elapsed,
                    Some(wait_elapsed),
                    Some(snapshot_elapsed),
                    syntax_stage_elapsed,
                    semantic_stage_elapsed,
                    None,
                    followup_syntax_work_mode,
                    followup_semantic_path,
                    followup_semantic_parse_source,
                    followup_semantic_ir_source,
                    pipeline_started,
                )
                .await;
        }

        if let Some(disposition) = self
            .diagnostics_publish_checkpoint_v2(
                &supersession_key,
                trigger,
                cancel_token,
                observed_deps_id.as_deref(),
                observed_settings_id.as_deref(),
            )
            .await
        {
            debug!(
                uri = %uri,
                file_id = file_id.0,
                expected_version = requested_version,
                expected_generation = requested_generation,
                profile = profile.as_str(),
                trigger = trigger.as_str(),
                disposition = disposition.as_str(),
                "diagnostics_v2: skip publish (final checkpoint)"
            );
            if let Some(guard) = did_save_followup_lane_guard.as_ref() {
                guard.release();
            }
            return self
                .finalize_diagnostics_save_profile_result_v2(
                    uri,
                    &supersession_key,
                    trigger,
                    disposition,
                    None,
                    followup_runtime_queue_wait_elapsed,
                    followup_apply_lag_elapsed,
                    blocking_queue_wait_elapsed,
                    Some(wait_elapsed),
                    Some(snapshot_elapsed),
                    syntax_stage_elapsed,
                    semantic_stage_elapsed,
                    None,
                    followup_syntax_work_mode,
                    followup_semantic_path,
                    followup_semantic_parse_source,
                    followup_semantic_ir_source,
                    pipeline_started,
                )
                .await;
        }

        debug!(
            uri = %uri,
            file_id = file_id.0,
            expected_version = requested_version,
            expected_generation = requested_generation,
            deps_id = observed_deps_id.as_deref().unwrap_or_default(),
            settings_id = observed_settings_id.as_deref().unwrap_or_default(),
            index_snapshot_id = index_snapshot_id,
            profile = profile.as_str(),
            trigger = trigger.as_str(),
            diagnostics_len,
            "diagnostics_v2: publish diagnostics"
        );

        if save_followup_from_did_save && run_semantic {
            self.record_diagnostics_save_followup_wait_state_v2(
                uri,
                &supersession_key,
                "pending_publish",
                followup_runtime_queue_wait_elapsed,
                followup_apply_lag_elapsed,
                Some(wait_elapsed),
                Some(snapshot_elapsed),
                followup_syntax_work_mode,
                followup_semantic_path,
                followup_semantic_parse_source,
                followup_semantic_ir_source,
            );
        }
        let publish_kind = if run_semantic { "full" } else { "syntax_only" };
        if let Some(guard) = did_save_followup_lane_guard.as_ref() {
            guard.release();
        }
        let publish_started = Instant::now();
        let disposition = self
            .publish_diagnostics_v2(
                &supersession_key,
                uri,
                diagnostics,
                trigger,
                profile,
                pipeline_started,
            )
            .await;
        self.finalize_diagnostics_save_profile_result_v2(
            uri,
            &supersession_key,
            trigger,
            disposition,
            Some(publish_kind),
            followup_runtime_queue_wait_elapsed,
            followup_apply_lag_elapsed,
            blocking_queue_wait_elapsed,
            Some(wait_elapsed),
            Some(snapshot_elapsed),
            syntax_stage_elapsed,
            semantic_stage_elapsed,
            Some(publish_started.elapsed()),
            followup_syntax_work_mode,
            followup_semantic_path,
            followup_semantic_parse_source,
            followup_semantic_ir_source,
            pipeline_started,
        )
        .await
    }
}
