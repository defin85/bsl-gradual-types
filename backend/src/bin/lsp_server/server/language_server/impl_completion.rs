use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionTimelineStageStatus {
    Completed,
    Cancelled,
    Failed,
    Skipped,
}

impl CompletionTimelineStageStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionPrepareGuardResult<T> {
    Prepared(T),
    TimedOut,
    Aborted(&'static str),
}

impl<T> CompletionPrepareGuardResult<T> {
    #[cfg(test)]
    fn trace_outcome(&self) -> String {
        match self {
            Self::Prepared(_) => "prepared".to_string(),
            Self::TimedOut => "timeout".to_string(),
            Self::Aborted(reason) => format!("aborted:{reason}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionRouteKind {
    HeadHit,
    ExactHit,
}

#[derive(Debug, Clone)]
struct CompletionRouteObservation {
    kind: CompletionRouteKind,
    file_version: i32,
    deps_id: bsl_analysis_v2::DepsSnapshotId,
    settings_id: Option<bsl_analysis_v2::SettingsId>,
    exact_ready: bool,
}

struct CompletionPreparedSnapshot {
    kind: &'static str,
    context: bsl_runtime::application::ExecutionContext,
    expected_version: i32,
    snapshot: Option<bsl_runtime::application::CompletionCurrentRevisionSnapshot>,
    wait_elapsed: Option<std::time::Duration>,
    snapshot_elapsed: std::time::Duration,
    wait_for_file_version_runtime: Option<bsl_runtime::application::WaitForFileVersionRuntimeTrace>,
    snapshot_with_deps_runtime: Option<bsl_runtime::application::SnapshotWithDepsRuntimeTrace>,
    timeout_attribution: Option<bsl_runtime::application::PrepareTimeoutAttributionTrace>,
    observed_file_version: Option<i32>,
    file_content_override: Option<Arc<str>>,
    file_path_override: Option<Arc<str>>,
    head_owner_type_hints_override: Option<Vec<bsl_shared::domain::types::TypeResolution>>,
    head_ready_override: bool,
    exact_ready_override: bool,
    deps_override: Option<Arc<bsl_analysis_v2::SemanticDeps>>,
    deps_id_override: Option<bsl_analysis_v2::DepsSnapshotId>,
    index_snapshot_override: Option<Arc<bsl_runtime::system::IndexSnapshot>>,
    settings_id_override: Option<bsl_analysis_v2::SettingsId>,
}

impl CompletionPreparedSnapshot {
    fn from_lightweight(
        context: bsl_runtime::application::ExecutionContext,
        prepared: bsl_runtime::application::PreparedCompletionFirstResponse,
        expected_version: i32,
    ) -> Self {
        Self {
            kind: "lightweight_current_revision",
            context,
            expected_version,
            snapshot: None,
            wait_elapsed: prepared.wait_elapsed,
            snapshot_elapsed: prepared.snapshot_elapsed,
            wait_for_file_version_runtime: prepared.wait_for_file_version_runtime,
            snapshot_with_deps_runtime: None,
            timeout_attribution: prepared.timeout_attribution,
            observed_file_version: prepared.observed_file_version,
            file_content_override: prepared.support.file_content,
            file_path_override: prepared.support.file_path,
            head_owner_type_hints_override: Some(prepared.support.head_owner_type_hints),
            head_ready_override: prepared.support.head_ready,
            exact_ready_override: prepared.support.exact_ready,
            deps_override: prepared.support.deps,
            deps_id_override: Some(prepared.support.deps_id),
            index_snapshot_override: Some(prepared.support.index_snapshot),
            settings_id_override: prepared.support.settings_id,
        }
    }

    fn from_exact_stateful(
        context: bsl_runtime::application::ExecutionContext,
        prepared: bsl_runtime::application::PreparedOperationSnapshot,
        expected_version: i32,
    ) -> Self {
        Self {
            kind: "exact_stateful",
            context,
            expected_version,
            snapshot: Some(
                bsl_runtime::application::CompletionCurrentRevisionSnapshot {
                    analysis: prepared.snapshot.analysis,
                    deps_id: prepared.snapshot.deps_id,
                    index_snapshot: prepared.index_snapshot,
                },
            ),
            wait_elapsed: prepared.wait_elapsed,
            snapshot_elapsed: prepared.snapshot_elapsed,
            wait_for_file_version_runtime: prepared.wait_for_file_version_runtime,
            snapshot_with_deps_runtime: Some(prepared.snapshot_with_deps_runtime),
            timeout_attribution: prepared.timeout_attribution,
            observed_file_version: prepared.observed_file_version,
            file_content_override: None,
            file_path_override: None,
            head_owner_type_hints_override: None,
            head_ready_override: false,
            exact_ready_override: false,
            deps_override: None,
            deps_id_override: None,
            index_snapshot_override: None,
            settings_id_override: None,
        }
    }

    fn from_shadow_head_fast_path(
        context: bsl_runtime::application::ExecutionContext,
        snapshot: bsl_runtime::application::CompletionCurrentRevisionSnapshot,
        expected_version: i32,
        file_content: Arc<str>,
        file_path: Arc<str>,
        head_owner_type_hints: Vec<bsl_shared::domain::types::TypeResolution>,
        snapshot_elapsed: std::time::Duration,
    ) -> Self {
        let deps_override = snapshot.analysis.deps_data().ok();
        let settings_id_override = snapshot.analysis.settings_id().ok();
        let exact_ready_override = snapshot
            .analysis
            .current_type_index_serve_only_ready(context.file_id)
            .ok()
            .unwrap_or(false);
        Self {
            kind: "lightweight_current_revision",
            context,
            expected_version,
            snapshot: None,
            wait_elapsed: None,
            snapshot_elapsed,
            wait_for_file_version_runtime: None,
            snapshot_with_deps_runtime: None,
            timeout_attribution: None,
            observed_file_version: Some(expected_version),
            file_content_override: Some(file_content),
            file_path_override: Some(file_path),
            head_owner_type_hints_override: Some(head_owner_type_hints),
            head_ready_override: true,
            exact_ready_override,
            deps_override,
            deps_id_override: Some(snapshot.deps_id),
            index_snapshot_override: Some(snapshot.index_snapshot),
            settings_id_override,
        }
    }

    fn from_shadow_head_support_bundle_fast_path(
        context: bsl_runtime::application::ExecutionContext,
        bundle: bsl_runtime::application::CompletionSupportBundle,
        expected_version: i32,
        file_content: Arc<str>,
        file_path: Arc<str>,
        head_owner_type_hints: Vec<bsl_shared::domain::types::TypeResolution>,
        snapshot_elapsed: std::time::Duration,
    ) -> Self {
        let settings_id = context.settings.settings_id.clone();
        Self {
            kind: "lightweight_current_revision",
            context,
            expected_version,
            snapshot: None,
            wait_elapsed: None,
            snapshot_elapsed,
            wait_for_file_version_runtime: None,
            snapshot_with_deps_runtime: None,
            timeout_attribution: None,
            observed_file_version: Some(expected_version),
            file_content_override: Some(file_content),
            file_path_override: Some(file_path),
            head_owner_type_hints_override: Some(head_owner_type_hints),
            head_ready_override: true,
            exact_ready_override: false,
            deps_override: Some(bundle.deps),
            deps_id_override: Some(bundle.deps_id),
            index_snapshot_override: Some(bundle.index_snapshot),
            settings_id_override: Some(settings_id),
        }
    }
}

#[derive(Debug, Clone)]
struct CompletionTimelineCapture {
    request_id: Option<String>,
    uri: String,
    trigger_mode: String,
    started_at_ms: u64,
    transport_received_at_ms: Option<u64>,
    transport_received_at_ms_provenance: Option<String>,
    jsonrpc_dispatch_received_at_ms: Option<u64>,
    request_context_call_entered_at_ms: Option<u64>,
    pre_method_attribution_provenance: Option<String>,
    service_future_created_at_ms: Option<u64>,
    service_future_first_poll_entered_at_ms: Option<u64>,
    service_future_first_poll_outcome: Option<String>,
    service_future_first_wake_scheduled_at_ms: Option<u64>,
    service_scope_entered_at_ms: Option<u64>,
    method_entered_at_ms: Option<u64>,
    handler_entered_at_ms: Option<u64>,
    response_sent_at_ms: Option<u64>,
    cancel_observed_at_ms: Option<u64>,
    timeline_cursor_ms: u64,
    prepare_details: Option<crate::types::CompletionTimelinePrepareDetailsTrace>,
    turn_attribution: Option<crate::types::CompletionTimelineTurnAttributionTrace>,
    stages: Vec<crate::types::CompletionTimelineStageTrace>,
}

#[derive(Debug, Clone, Copy)]
struct CompletionResponseBuildBreakdown {
    snapshot_read: std::time::Duration,
    collect: std::time::Duration,
    rank: std::time::Duration,
    format: std::time::Duration,
}

impl CompletionTimelineCapture {
    fn new(
        request_id: Option<String>,
        uri: &Url,
        trigger_mode: &str,
        method_entered_at_ms: u64,
        handler_entered_at_ms: u64,
    ) -> Self {
        Self {
            request_id,
            uri: uri.to_string(),
            trigger_mode: trigger_mode.to_string(),
            started_at_ms: method_entered_at_ms,
            transport_received_at_ms: None,
            transport_received_at_ms_provenance: None,
            jsonrpc_dispatch_received_at_ms: None,
            request_context_call_entered_at_ms: None,
            pre_method_attribution_provenance: None,
            service_future_created_at_ms: None,
            service_future_first_poll_entered_at_ms: None,
            service_future_first_poll_outcome: None,
            service_future_first_wake_scheduled_at_ms: None,
            service_scope_entered_at_ms: None,
            method_entered_at_ms: Some(method_entered_at_ms),
            handler_entered_at_ms: Some(handler_entered_at_ms),
            response_sent_at_ms: None,
            cancel_observed_at_ms: None,
            timeline_cursor_ms: 0,
            prepare_details: None,
            turn_attribution: None,
            stages: Vec::new(),
        }
    }

    fn duration_to_ms(duration: std::time::Duration) -> u64 {
        duration.as_millis().min(u64::MAX as u128) as u64
    }

    fn push_stage_ms(
        &mut self,
        name: &str,
        status: CompletionTimelineStageStatus,
        duration_ms: u64,
    ) {
        let stage = crate::types::CompletionTimelineStageTrace {
            name: name.to_string(),
            status: status.as_str().to_string(),
            started_offset_ms: self.timeline_cursor_ms,
            duration_ms,
        };
        self.timeline_cursor_ms = self.timeline_cursor_ms.saturating_add(duration_ms);
        self.stages.push(stage);
    }

    fn push_stage(
        &mut self,
        name: &str,
        status: CompletionTimelineStageStatus,
        duration: std::time::Duration,
    ) {
        self.push_stage_ms(name, status, Self::duration_to_ms(duration));
    }

    fn push_completed_stage_ms(&mut self, name: &str, duration_ms: u64) {
        self.push_stage_ms(name, CompletionTimelineStageStatus::Completed, duration_ms);
    }

    fn push_completed_stage(&mut self, name: &str, duration: std::time::Duration) {
        self.push_stage(name, CompletionTimelineStageStatus::Completed, duration);
    }

    fn push_response_build_stage(
        &mut self,
        response_build_duration: std::time::Duration,
        breakdown: Option<CompletionResponseBuildBreakdown>,
    ) {
        let response_build_ms = Self::duration_to_ms(response_build_duration);
        let Some(breakdown) = breakdown else {
            self.push_completed_stage_ms("response_build", response_build_ms);
            return;
        };

        let stage_durations_ms = [
            (
                "snapshot_read",
                Self::duration_to_ms(breakdown.snapshot_read),
            ),
            ("collect", Self::duration_to_ms(breakdown.collect)),
            ("rank", Self::duration_to_ms(breakdown.rank)),
            ("format", Self::duration_to_ms(breakdown.format)),
        ];
        let breakdown_total_ms = stage_durations_ms
            .iter()
            .fold(0_u64, |acc, (_, duration_ms)| {
                acc.saturating_add(*duration_ms)
            });

        // Fail-closed: avoid duplicating aggregate + nested stages when breakdown is inconsistent.
        if breakdown_total_ms == 0 || breakdown_total_ms > response_build_ms {
            self.push_completed_stage_ms("response_build", response_build_ms);
            return;
        }

        for (name, duration_ms) in stage_durations_ms {
            if duration_ms > 0 {
                self.push_completed_stage_ms(name, duration_ms);
            }
        }
        let response_build_other_ms = response_build_ms.saturating_sub(breakdown_total_ms);
        if response_build_other_ms > 0 {
            self.push_completed_stage_ms("response_build_other", response_build_other_ms);
        }
    }

    fn push_terminal_stage(&mut self, outcome: &str) {
        let status = match outcome {
            "cancelled" | "superseded" => CompletionTimelineStageStatus::Cancelled,
            "fail_closed" | "handler_error" => CompletionTimelineStageStatus::Failed,
            "skipped" => CompletionTimelineStageStatus::Skipped,
            _ => CompletionTimelineStageStatus::Completed,
        };
        self.push_stage("terminal", status, std::time::Duration::from_millis(0));
    }

    fn set_turn_attribution(
        &mut self,
        attribution: crate::types::CompletionTimelineTurnAttributionTrace,
    ) {
        self.turn_attribution = Some(attribution);
    }

    fn set_transport_received_at_ms(&mut self, transport_received_at_ms: u64) {
        self.transport_received_at_ms = Some(transport_received_at_ms);
    }

    fn set_transport_received_at_ms_provenance(&mut self, provenance: impl Into<String>) {
        self.transport_received_at_ms_provenance = Some(provenance.into());
    }

    fn set_jsonrpc_dispatch_received_at_ms(&mut self, jsonrpc_dispatch_received_at_ms: u64) {
        self.jsonrpc_dispatch_received_at_ms = Some(jsonrpc_dispatch_received_at_ms);
    }

    fn set_request_context_call_entered_at_ms(&mut self, request_context_call_entered_at_ms: u64) {
        self.request_context_call_entered_at_ms = Some(request_context_call_entered_at_ms);
    }

    fn set_pre_method_attribution_provenance(&mut self, provenance: impl Into<String>) {
        self.pre_method_attribution_provenance = Some(provenance.into());
    }

    fn set_service_future_created_at_ms(&mut self, service_future_created_at_ms: u64) {
        self.service_future_created_at_ms = Some(service_future_created_at_ms);
    }

    fn set_service_future_first_poll_entered_at_ms(
        &mut self,
        service_future_first_poll_entered_at_ms: u64,
    ) {
        self.service_future_first_poll_entered_at_ms =
            Some(service_future_first_poll_entered_at_ms);
    }

    fn set_service_future_first_poll_outcome(
        &mut self,
        service_future_first_poll_outcome: impl Into<String>,
    ) {
        self.service_future_first_poll_outcome = Some(service_future_first_poll_outcome.into());
    }

    fn set_service_future_first_wake_scheduled_at_ms(
        &mut self,
        service_future_first_wake_scheduled_at_ms: u64,
    ) {
        self.service_future_first_wake_scheduled_at_ms =
            Some(service_future_first_wake_scheduled_at_ms);
    }

    fn set_service_scope_entered_at_ms(&mut self, service_scope_entered_at_ms: u64) {
        self.service_scope_entered_at_ms = Some(service_scope_entered_at_ms);
    }

    #[cfg(test)]
    fn set_method_entered_at_ms(&mut self, method_entered_at_ms: u64) {
        self.method_entered_at_ms = Some(method_entered_at_ms);
        self.started_at_ms = method_entered_at_ms;
    }

    #[cfg(test)]
    fn set_handler_entered_at_ms(&mut self, handler_entered_at_ms: u64) {
        self.handler_entered_at_ms = Some(handler_entered_at_ms);
    }

    fn set_response_sent_at_ms(&mut self, response_sent_at_ms: u64) {
        self.response_sent_at_ms = Some(response_sent_at_ms);
    }

    fn observe_cancel_at_ms(&mut self, cancel_observed_at_ms: u64) {
        if self.cancel_observed_at_ms.is_none() {
            self.cancel_observed_at_ms = Some(cancel_observed_at_ms);
        }
    }

    fn server_edge_details_trace(
        &self,
    ) -> Option<crate::types::CompletionTimelineServerEdgeDetailsTrace> {
        let transport_received_at_ms = self.transport_received_at_ms?;
        let transport_received_at_ms_provenance = self
            .transport_received_at_ms_provenance
            .clone()
            .unwrap_or_else(|| "request_context_call_entry".to_string());
        let jsonrpc_dispatch_received_at_ms = self.jsonrpc_dispatch_received_at_ms;
        let request_context_call_entered_at_ms = self.request_context_call_entered_at_ms;
        let service_future_created_at_ms = self.service_future_created_at_ms;
        let service_future_first_poll_entered_at_ms = self.service_future_first_poll_entered_at_ms;
        let service_future_first_poll_outcome = self.service_future_first_poll_outcome.clone();
        let service_future_first_wake_scheduled_at_ms =
            self.service_future_first_wake_scheduled_at_ms;
        let service_scope_entered_at_ms = self.service_scope_entered_at_ms;
        let method_entered_at_ms = self.method_entered_at_ms;
        let handler_entered_at_ms = self.handler_entered_at_ms?;
        let response_sent_at_ms = self.response_sent_at_ms?;
        let cancel_observed_at_ms = self.cancel_observed_at_ms;
        Some(crate::types::CompletionTimelineServerEdgeDetailsTrace {
            transport_received_at_ms,
            transport_received_at_ms_provenance,
            jsonrpc_dispatch_received_at_ms,
            pre_method_attribution_provenance: self
                .pre_method_attribution_provenance
                .clone()
                .unwrap_or_else(|| "unavailable".to_string()),
            service_future_created_at_ms,
            service_future_first_poll_entered_at_ms,
            service_future_first_poll_outcome,
            service_future_first_wake_scheduled_at_ms,
            service_scope_entered_at_ms,
            method_entered_at_ms,
            handler_entered_at_ms,
            response_sent_at_ms,
            cancel_observed_at_ms,
            dispatch_to_request_context_wait_ms: jsonrpc_dispatch_received_at_ms
                .zip(request_context_call_entered_at_ms)
                .map(
                    |(jsonrpc_dispatch_received_at_ms, request_context_call_entered_at_ms)| {
                        request_context_call_entered_at_ms
                            .saturating_sub(jsonrpc_dispatch_received_at_ms)
                    },
                ),
            transport_to_service_future_wait_ms: service_future_created_at_ms.map(
                |service_future_created_at_ms| {
                    service_future_created_at_ms.saturating_sub(transport_received_at_ms)
                },
            ),
            service_future_to_scope_wait_ms: service_future_created_at_ms
                .zip(service_scope_entered_at_ms)
                .map(
                    |(service_future_created_at_ms, service_scope_entered_at_ms)| {
                        service_scope_entered_at_ms.saturating_sub(service_future_created_at_ms)
                    },
                ),
            service_future_to_first_poll_wait_ms: service_future_created_at_ms
                .zip(service_future_first_poll_entered_at_ms)
                .map(
                    |(service_future_created_at_ms, service_future_first_poll_entered_at_ms)| {
                        service_future_first_poll_entered_at_ms
                            .saturating_sub(service_future_created_at_ms)
                    },
                ),
            first_poll_to_first_wake_wait_ms: service_future_first_poll_entered_at_ms
                .zip(service_future_first_wake_scheduled_at_ms)
                .map(
                    |(
                        service_future_first_poll_entered_at_ms,
                        service_future_first_wake_scheduled_at_ms,
                    )| {
                        service_future_first_wake_scheduled_at_ms
                            .saturating_sub(service_future_first_poll_entered_at_ms)
                    },
                ),
            transport_to_service_scope_wait_ms: service_scope_entered_at_ms.map(
                |service_scope_entered_at_ms| {
                    service_scope_entered_at_ms.saturating_sub(transport_received_at_ms)
                },
            ),
            service_scope_to_method_wait_ms: service_scope_entered_at_ms
                .zip(method_entered_at_ms)
                .map(|(service_scope_entered_at_ms, method_entered_at_ms)| {
                    method_entered_at_ms.saturating_sub(service_scope_entered_at_ms)
                }),
            transport_to_method_wait_ms: method_entered_at_ms.map(|method_entered_at_ms| {
                method_entered_at_ms.saturating_sub(transport_received_at_ms)
            }),
            method_prelude_exec_ms: method_entered_at_ms.map(|method_entered_at_ms| {
                handler_entered_at_ms.saturating_sub(method_entered_at_ms)
            }),
            transport_to_handler_wait_ms: handler_entered_at_ms
                .saturating_sub(transport_received_at_ms),
            server_handler_exec_ms: response_sent_at_ms.saturating_sub(handler_entered_at_ms),
            cancel_observed_after_handler_enter_ms: cancel_observed_at_ms
                .map(|cancel_at| cancel_at.saturating_sub(handler_entered_at_ms)),
        })
    }

    fn prepare_details_mut(&mut self) -> &mut crate::types::CompletionTimelinePrepareDetailsTrace {
        self.prepare_details.get_or_insert_with(Default::default)
    }

    fn set_prepare_wait_budget(&mut self, wait_budget: Option<std::time::Duration>) {
        self.prepare_details_mut().wait_budget_ms =
            wait_budget.map(CompletionTimelineCapture::duration_to_ms);
    }

    fn set_prepare_min_file_version(&mut self, min_file_version: Option<i32>) {
        self.prepare_details_mut().min_file_version = min_file_version;
    }

    fn set_prepare_shadow_version_at_start(&mut self, shadow_version: Option<i32>) {
        self.prepare_details_mut().shadow_version_at_start = shadow_version;
    }

    fn set_prepare_outcome(&mut self, outcome: &str) {
        self.prepare_details_mut().outcome = Some(outcome.to_string());
    }

    fn set_prepare_kind(&mut self, kind: &str) {
        self.prepare_details_mut().kind = Some(kind.to_string());
    }

    fn set_prepare_route(&mut self, route: &str) {
        self.prepare_details_mut().route = Some(route.to_string());
    }

    fn set_prepare_fail_closed_cause(&mut self, cause: &str) {
        self.prepare_details_mut().fail_closed_cause = Some(cause.to_string());
    }

    fn set_prepare_guard_outcome(&mut self, outcome: impl Into<String>) {
        self.prepare_details_mut().guard_outcome = Some(outcome.into());
    }

    fn set_prepare_observed_file_version(&mut self, observed_file_version: Option<i32>) {
        self.prepare_details_mut().observed_file_version = observed_file_version;
    }

    fn set_prepare_wait_elapsed(&mut self, wait_elapsed: Option<std::time::Duration>) {
        self.prepare_details_mut().wait_elapsed_ms =
            wait_elapsed.map(CompletionTimelineCapture::duration_to_ms);
    }

    fn set_prepare_snapshot_elapsed(&mut self, snapshot_elapsed: std::time::Duration) {
        self.prepare_details_mut().snapshot_elapsed_ms =
            Some(CompletionTimelineCapture::duration_to_ms(snapshot_elapsed));
    }

    fn set_prepare_apply_age_at_start(&mut self, apply_age: Option<std::time::Duration>) {
        self.prepare_details_mut().apply_age_at_start_ms =
            apply_age.map(CompletionTimelineCapture::duration_to_ms);
    }

    fn set_prepare_apply_age_at_terminal(&mut self, apply_age: Option<std::time::Duration>) {
        self.prepare_details_mut().apply_age_at_terminal_ms =
            apply_age.map(CompletionTimelineCapture::duration_to_ms);
    }

    fn set_prepare_progress_snapshot(
        &mut self,
        progress: &bsl_runtime::application::PrepareStatefulProgressSnapshot,
    ) {
        let progress_trace = self
            .prepare_details_mut()
            .progress
            .get_or_insert_with(Default::default);
        progress_trace.phase = progress.phase.map(str::to_string);
        progress_trace.phase_started_offset_ms = progress
            .phase_started_offset
            .map(CompletionTimelineCapture::duration_to_ms);
        progress_trace.wait_completed_offset_ms = progress
            .wait_completed_offset
            .map(CompletionTimelineCapture::duration_to_ms);
        progress_trace.snapshot_completed_offset_ms = progress
            .snapshot_completed_offset
            .map(CompletionTimelineCapture::duration_to_ms);
    }

    fn set_prepare_wait_for_file_version_runtime(
        &mut self,
        trace: Option<bsl_runtime::application::WaitForFileVersionRuntimeTrace>,
    ) {
        let Some(trace) = trace else {
            return;
        };
        self.prepare_details_mut().wait_for_file_version_runtime =
            Some(crate::types::CompletionTimelinePrepareRuntimeTrace {
                queue_wait_ms: trace
                    .queue_wait_elapsed
                    .map(CompletionTimelineCapture::duration_to_ms),
                exec_ms: trace
                    .exec_elapsed
                    .map(CompletionTimelineCapture::duration_to_ms),
                wake_wait_ms: trace
                    .wake_wait_elapsed
                    .map(CompletionTimelineCapture::duration_to_ms),
                resolution: trace
                    .resolution
                    .map(|resolution| resolution.as_str().to_string()),
            });
    }

    fn set_prepare_snapshot_with_deps_runtime(
        &mut self,
        trace: bsl_runtime::application::SnapshotWithDepsRuntimeTrace,
    ) {
        self.prepare_details_mut().snapshot_with_deps_runtime =
            Some(crate::types::CompletionTimelinePrepareRuntimeTrace {
                queue_wait_ms: trace
                    .queue_wait_elapsed
                    .map(CompletionTimelineCapture::duration_to_ms),
                exec_ms: trace
                    .exec_elapsed
                    .map(CompletionTimelineCapture::duration_to_ms),
                wake_wait_ms: None,
                resolution: None,
            });
    }

    fn set_prepare_snapshot_with_deps_timeout_runtime(
        &mut self,
        trace: Option<bsl_runtime::application::SnapshotWithDepsTimeoutRuntimeTrace>,
    ) {
        self.prepare_details_mut()
            .snapshot_with_deps_timeout_runtime =
            trace.map(
                |trace| crate::types::CompletionTimelinePrepareRuntimeTrace {
                    queue_wait_ms: trace
                        .queue_wait_elapsed
                        .map(CompletionTimelineCapture::duration_to_ms),
                    exec_ms: trace
                        .exec_elapsed
                        .map(CompletionTimelineCapture::duration_to_ms),
                    wake_wait_ms: trace
                        .wake_wait_elapsed
                        .map(CompletionTimelineCapture::duration_to_ms),
                    resolution: Some(trace.resolution.as_str().to_string()),
                },
            );
    }

    fn set_prepare_timeout_attribution(
        &mut self,
        trace: bsl_runtime::application::PrepareTimeoutAttributionTrace,
    ) {
        self.prepare_details_mut().timeout_attribution = Some(
            crate::types::CompletionTimelinePrepareTimeoutAttributionTrace {
                source: trace.source.as_str().to_string(),
                phase: trace.phase.to_string(),
                budget_ms: CompletionTimelineCapture::duration_to_ms(trace.budget),
                elapsed_ms: CompletionTimelineCapture::duration_to_ms(trace.elapsed),
                overshoot_ms: CompletionTimelineCapture::duration_to_ms(trace.overshoot),
            },
        );
    }

    fn exact_wait_details_mut(
        &mut self,
    ) -> &mut crate::types::CompletionTimelineExactWaitDetailsTrace {
        self.prepare_details_mut()
            .exact_wait
            .get_or_insert_with(Default::default)
    }

    fn set_exact_wait_head_ready_before_wait(&mut self, ready: bool) {
        self.exact_wait_details_mut().head_ready_before_wait = Some(ready);
    }

    fn set_exact_wait_exact_ready_before_wait(&mut self, ready: bool) {
        self.exact_wait_details_mut().exact_ready_before_wait = Some(ready);
    }

    fn set_exact_wait_current_revision_head_owner_hints_ready(&mut self, ready: bool) {
        self.exact_wait_details_mut()
            .current_revision_head_owner_hints_ready = Some(ready);
    }

    fn set_exact_wait_artifact_outcome(&mut self, outcome: &str) {
        self.exact_wait_details_mut().artifact_wait_outcome = Some(outcome.to_string());
    }

    fn set_exact_wait_type_index_outcome(&mut self, outcome: &str) {
        self.exact_wait_details_mut().type_index_wait_outcome = Some(outcome.to_string());
    }

    fn set_exact_wait_type_index_waiter_action(&mut self, action: &str) {
        self.exact_wait_details_mut().type_index_waiter_action = Some(action.to_string());
    }

    fn set_exact_wait_matching_task_state(&mut self, state: &str) {
        self.exact_wait_details_mut().matching_task_state = Some(state.to_string());
    }

    fn set_exact_wait_task_phase(&mut self, phase: &str) {
        self.exact_wait_details_mut().task_phase = Some(phase.to_string());
    }

    fn set_exact_wait_artifact_poll(
        &mut self,
        trace: super::super::core::CompletionArtifactPollTraceV2,
    ) {
        self.exact_wait_details_mut().artifact_poll =
            Some(crate::types::CompletionTimelineExactArtifactPollTrace {
                poll_count: trace.poll_count,
                poll_elapsed_ms: CompletionTimelineCapture::duration_to_ms(trace.poll_elapsed),
                observed_file_version: trace.observed_file_version,
                head_ready: trace.head_ready,
                exact_ready: trace.exact_ready,
            });
    }

    fn into_trace(
        self,
        trace_id: String,
        total_duration: std::time::Duration,
        outcome: &str,
    ) -> crate::types::CompletionTimelineTrace {
        let total_duration_ms = Self::duration_to_ms(total_duration);
        let max_stage_end_ms = self
            .stages
            .iter()
            .map(|stage| stage.started_offset_ms.saturating_add(stage.duration_ms))
            .max()
            .unwrap_or(0);
        let total_duration_ms = total_duration_ms.max(max_stage_end_ms);
        let dominant_stage = self
            .stages
            .iter()
            .filter(|stage| stage.status != "skipped")
            .max_by_key(|stage| stage.duration_ms)
            .map(|stage| stage.name.clone());
        let server_edge_details = self.server_edge_details_trace();

        crate::types::CompletionTimelineTrace {
            trace_id,
            request_id: self.request_id,
            uri: self.uri,
            trigger_mode: self.trigger_mode,
            outcome: outcome.to_string(),
            started_at_ms: self.started_at_ms,
            total_duration_ms,
            dominant_stage,
            prepare_details: self.prepare_details,
            server_edge_details,
            turn_attribution: self.turn_attribution,
            stages: self.stages,
        }
    }
}

fn turn_holder_snapshot_to_trace(
    holder: super::super::completion_dispatcher::CompletionTurnHolderSnapshot,
) -> crate::types::CompletionTimelineTurnHolderTrace {
    crate::types::CompletionTimelineTurnHolderTrace {
        request_id: holder.request_id,
        file_seq: holder.file_seq,
        request_epoch: holder.request_epoch,
        trigger_mode: holder.trigger_mode,
        version_hint: holder.version_hint,
        age_ms: CompletionTimelineCapture::duration_to_ms(holder.age),
    }
}

fn dispatch_attribution_to_trace(
    attribution: super::super::completion_dispatcher::CompletionDispatchAttributionSnapshot,
) -> crate::types::CompletionTimelineTurnAttributionTrace {
    crate::types::CompletionTimelineTurnAttributionTrace {
        request_file_seq: attribution.request_file_seq,
        request_epoch: attribution.request_epoch,
        queue_outcome: attribution.queue_outcome.as_str().to_string(),
        turn_wait_outcome: None,
        dispatcher_resolution_latency_ms: None,
        queue_capacity: attribution.queue_capacity,
        queue_depth_before_enqueue: attribution.queue_depth_before_enqueue,
        queue_depth_after_enqueue: attribution.queue_depth_after_enqueue,
        queued_completion_ahead_count: attribution.queued_completion_ahead_count,
        did_change_ahead_count: attribution.did_change_ahead_count,
        active_completion_count: attribution.active_completion_count,
        dropped_completion_file_seq: attribution.dropped_completion_file_seq,
        active_holder: attribution.active_holder.map(turn_holder_snapshot_to_trace),
        queued_completion_ahead: attribution
            .queued_completion_ahead
            .map(turn_holder_snapshot_to_trace),
    }
}

fn derive_pre_method_attribution_provenance(
    current_request_id: Option<&str>,
    current_request_received_at_ms: Option<u64>,
    exact_request_context: Option<&super::super::request_context::PendingCompletionRequestContext>,
    fallback_request_context: Option<
        &super::super::request_context::PendingCompletionRequestContext,
    >,
) -> &'static str {
    if current_request_id.is_some() && current_request_received_at_ms.is_some() {
        "same_request_authoritative"
    } else if exact_request_context.is_some() {
        "same_request_authoritative"
    } else if fallback_request_context.is_some() {
        "best_effort_fallback"
    } else {
        "unavailable"
    }
}

fn completion_public_timeline_outcome(outcome: &str) -> &'static str {
    match outcome {
        "ok_non_empty" => "ok_non_empty",
        "ok_empty" => "ok_empty",
        "cancelled" => "cancelled",
        "superseded" => "superseded",
        "handler_error" => "handler_error",
        "wait_not_ready"
        | "missing_file_content"
        | "missing_file_path"
        | "missing_deps"
        | "missing_ir"
        | "fallback_unavailable"
        | "queue_rejected" => "fail_closed",
        _ => "fail_closed",
    }
}

fn completion_public_fail_closed_reason(outcome: &str) -> Option<&'static str> {
    match outcome {
        "missing_ir" => Some("missing_canonical_ir"),
        "fallback_unavailable" | "wait_not_ready" => Some("missing_semantic_index"),
        "superseded" => Some("superseded_revision"),
        "cancelled" => Some("cancelled"),
        "missing_deps" | "missing_file_content" | "missing_file_path" | "queue_rejected" => {
            Some("unavailable_by_contract")
        }
        _ => None,
    }
}

fn completion_prepare_error_outcome(
    outcome: bsl_runtime::application::SemanticOutcome,
) -> &'static str {
    match outcome {
        bsl_runtime::application::SemanticOutcome::StaleVersion => "superseded",
        bsl_runtime::application::SemanticOutcome::MissingDeps => "missing_deps",
        bsl_runtime::application::SemanticOutcome::Cancelled => "cancelled",
        _ => "wait_not_ready",
    }
}

fn observe_cancelled_timeline_outcome(
    timeline_capture: &mut CompletionTimelineCapture,
    outcome: &str,
) {
    if outcome == "cancelled" {
        timeline_capture.observe_cancel_at_ms(super::super::unix_timestamp_ms());
    }
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_completion_prepare_abort(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    request_id: Option<&str>,
    request_epoch: u64,
    cancellation_token: Option<&super::super::completion_cancellation::CompletionCancellationToken>,
    cancel_event_emitted: &mut bool,
) -> &'static str {
    const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);

    loop {
        if let Some(outcome) = super::helpers::completion_checkpoint_outcome(
            server,
            file_id,
            request_id,
            request_epoch,
            cancellation_token,
            "prepare",
            cancel_event_emitted,
        )
        .await
        {
            return outcome;
        }

        if let Some(token) = cancellation_token {
            tokio::select! {
                _ = token.cancelled() => {}
                _ = tokio::time::sleep(POLL_INTERVAL) => {}
            }
        } else {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
}

async fn run_completion_prepare_guard<T, PF, AF>(
    prepare_future: PF,
    prepare_timeout: Option<std::time::Duration>,
    abort_future: Option<AF>,
) -> CompletionPrepareGuardResult<T>
where
    PF: std::future::Future<Output = T>,
    AF: std::future::Future<Output = &'static str>,
{
    let mut prepare_future = std::pin::pin!(prepare_future);

    match (prepare_timeout, abort_future) {
        (Some(timeout), Some(abort_future)) => {
            let mut abort_future = std::pin::pin!(abort_future);
            let mut timeout_sleep = std::pin::pin!(tokio::time::sleep(timeout));
            tokio::select! {
                prepared = &mut prepare_future => CompletionPrepareGuardResult::Prepared(prepared),
                outcome = &mut abort_future => CompletionPrepareGuardResult::Aborted(outcome),
                _ = &mut timeout_sleep => CompletionPrepareGuardResult::TimedOut,
            }
        }
        (Some(timeout), None) => {
            let mut timeout_sleep = std::pin::pin!(tokio::time::sleep(timeout));
            tokio::select! {
                prepared = &mut prepare_future => CompletionPrepareGuardResult::Prepared(prepared),
                _ = &mut timeout_sleep => CompletionPrepareGuardResult::TimedOut,
            }
        }
        (None, Some(abort_future)) => {
            let mut abort_future = std::pin::pin!(abort_future);
            tokio::select! {
                prepared = &mut prepare_future => CompletionPrepareGuardResult::Prepared(prepared),
                outcome = &mut abort_future => CompletionPrepareGuardResult::Aborted(outcome),
            }
        }
        (None, None) => CompletionPrepareGuardResult::Prepared(prepare_future.await),
    }
}

async fn completion_apply_age_for_file(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
) -> Option<std::time::Duration> {
    server
        .latest_apply_enqueued_at_v2
        .read()
        .await
        .get(&file_id)
        .copied()
        .map(|enqueued_at| enqueued_at.elapsed())
}

impl BslLanguageServer {
    pub(super) async fn lsp_completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        let method_entered_at_ms = super::super::unix_timestamp_ms();
        let started = Instant::now();
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let trigger_mode = completion_trigger_mode_label(params.context.as_ref());
        let trigger_char_hint = completion_trigger_character(params.context.as_ref());
        let shadow_internal_request =
            completion_is_shadow_internal_request(params.context.as_ref());
        let current_request_id = super::super::request_context::current_request_id();
        let current_request_received_at_ms =
            super::super::request_context::current_request_received_at_ms();
        let current_request_jsonrpc_dispatch_received_at_ms =
            super::super::request_context::current_request_jsonrpc_dispatch_received_at_ms();
        let current_request_service_future_created_at_ms =
            super::super::request_context::current_request_service_future_created_at_ms();
        let current_request_service_scope_entered_at_ms =
            super::super::request_context::current_request_service_scope_entered_at_ms();
        let exact_request_context = current_request_id.as_deref().and_then(|request_id| {
            super::super::request_context::take_completion_request_context_by_request_id(request_id)
        });
        let fallback_request_context = if current_request_id.is_none() {
            super::super::request_context::take_completion_request_context(&uri, position)
        } else {
            None
        };
        let pending_request_context = exact_request_context
            .as_ref()
            .or(fallback_request_context.as_ref());
        let pending_request_cancelled_before_take =
            pending_request_context.is_some_and(|context| context.cancelled_before_take);
        let completion_request_id = current_request_id.clone().or_else(|| {
            pending_request_context
                .as_ref()
                .map(|context| context.request_id.clone())
        });
        if !shadow_internal_request {
            self.coordinator
                .record_intellisense_v2_completion_trigger_mode(trigger_mode);
        }

        let file_id = self.get_or_create_file_id_v2(&uri).await;
        // Serialize against prior didOpen/didChange current-revision handoff without
        // reintroducing slow parse/exact work on the completion path.
        let _text_sync_barrier = self.text_sync_v2.lock().await;
        let version_hint = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
        drop(_text_sync_barrier);
        let completion_knobs =
            bsl_runtime::application::CompletionPipelineKnobs::from_runtime_config();
        self.completion_dispatcher_v2
            .set_queue_capacity(completion_knobs.queue_capacity)
            .await;
        let routing_key = completion_canary_routing_key(
            &uri,
            position,
            trigger_mode,
            trigger_char_hint,
            version_hint,
        );
        let routing_plan = if shadow_internal_request {
            CompletionRoutingPlan {
                response_route: CompletionResponseRoute::EventDriven,
                run_shadow_event_driven: false,
            }
        } else {
            completion_routing_plan(
                completion_knobs.mode,
                completion_knobs.canary_percent,
                &routing_key,
            )
        };

        if routing_plan.run_shadow_event_driven {
            let mut shadow_params = params.clone();
            let shadow_trigger = completion_shadow_internal_trigger_value(trigger_char_hint);
            if let Some(context) = shadow_params.context.as_mut() {
                context.trigger_character = Some(shadow_trigger);
            } else {
                shadow_params.context = Some(CompletionContext {
                    trigger_kind: CompletionTriggerKind::INVOKED,
                    trigger_character: Some(shadow_trigger),
                });
            }
            let shadow_server = self.clone();
            tokio::spawn(async move {
                let _ = shadow_server.completion(shadow_params).await;
            });
        }

        let event_driven_guards_enabled = routing_plan.response_route.event_driven_guards_enabled();
        let completion_observability_mode = completion_observability_mode_label(
            routing_plan.response_route,
            shadow_internal_request,
        );
        let handler_entered_at_ms = super::super::unix_timestamp_ms();
        let mut timeline_capture = CompletionTimelineCapture::new(
            completion_request_id.clone(),
            &uri,
            trigger_mode,
            method_entered_at_ms,
            handler_entered_at_ms,
        );
        let pre_method_attribution_provenance = derive_pre_method_attribution_provenance(
            current_request_id.as_deref(),
            current_request_received_at_ms,
            exact_request_context.as_ref(),
            fallback_request_context.as_ref(),
        );
        timeline_capture.set_pre_method_attribution_provenance(pre_method_attribution_provenance);
        let request_context_call_entered_at_ms = current_request_received_at_ms
            .or_else(|| pending_request_context.and_then(|context| context.request_received_at_ms));
        if let Some(request_context_call_entered_at_ms) = request_context_call_entered_at_ms {
            timeline_capture
                .set_request_context_call_entered_at_ms(request_context_call_entered_at_ms);
        }
        if let Some(jsonrpc_dispatch_received_at_ms) =
            current_request_jsonrpc_dispatch_received_at_ms.or_else(|| {
                pending_request_context.and_then(|context| context.jsonrpc_dispatch_received_at_ms)
            })
        {
            timeline_capture.set_transport_received_at_ms_provenance("jsonrpc_dispatch_received");
            timeline_capture.set_jsonrpc_dispatch_received_at_ms(jsonrpc_dispatch_received_at_ms);
            timeline_capture.set_transport_received_at_ms(jsonrpc_dispatch_received_at_ms);
        } else {
            timeline_capture.set_transport_received_at_ms_provenance("request_context_call_entry");
            timeline_capture.set_transport_received_at_ms(
                request_context_call_entered_at_ms.unwrap_or(method_entered_at_ms),
            );
        }
        if let Some(service_future_created_at_ms) = current_request_service_future_created_at_ms
            .or_else(|| {
                pending_request_context.and_then(|context| context.service_future_created_at_ms)
            })
        {
            timeline_capture.set_service_future_created_at_ms(service_future_created_at_ms);
        }
        if let Some(service_scope_entered_at_ms) = current_request_service_scope_entered_at_ms
            .or_else(|| {
                pending_request_context.and_then(|context| context.service_scope_entered_at_ms)
            })
        {
            timeline_capture.set_service_scope_entered_at_ms(service_scope_entered_at_ms);
        }
        timeline_capture.set_prepare_min_file_version(version_hint);
        timeline_capture.set_prepare_shadow_version_at_start(
            self.latest_document_shadow_state_v2
                .read()
                .await
                .get(&file_id)
                .map(|state| state.version),
        );
        let (
            completion_ticket,
            completion_turn_outcome,
            _completion_request_registration,
            completion_cancellation_token,
            mut completion_drop_guard,
            mut completion_active_turn_guard,
            completion_dispatch_attribution,
        ) = if event_driven_guards_enabled {
            let completion_request_metadata =
                super::super::completion_dispatcher::CompletionRequestMetadata {
                    request_id: completion_request_id.clone(),
                    version_hint,
                    trigger_mode: trigger_mode.to_string(),
                };
            let completion_dispatch = self
                .completion_dispatcher_v2
                .emit_completion_request_with_turn(
                    file_id,
                    completion_request_id.clone(),
                    version_hint,
                    trigger_mode.to_string(),
                )
                .await;
            let completion_ticket = completion_dispatch.ticket;
            let superseded_active_request_ids = completion_dispatch.superseded_active_request_ids;
            let completion_request_registration = completion_request_id.clone().map(|request_id| {
                self.completion_cancellation_registry_v2.register_request(
                    request_id,
                    file_id,
                    completion_ticket.request_epoch,
                )
            });
            let mut completion_dispatch_attribution =
                dispatch_attribution_to_trace(completion_dispatch.attribution);
            let completion_cancellation_token = completion_request_registration
                .as_ref()
                .map(|registration| registration.token());
            if pending_request_cancelled_before_take {
                if let Some(token) = completion_cancellation_token.as_ref() {
                    token.cancel();
                }
            }
            let completion_drop_guard = Some(CompletionRequestDropCancelGuard::new(
                completion_request_id.clone(),
                Arc::clone(&self.completion_cancellation_registry_v2),
                Arc::clone(&self.completion_dispatcher_v2),
            ));
            for stale_request_id in superseded_active_request_ids {
                if self
                    .completion_cancellation_registry_v2
                    .cancel_request(&stale_request_id)
                    .is_some()
                {
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        superseded_request_id = %stale_request_id,
                        request_epoch = completion_ticket.request_epoch,
                        request_id = ?completion_request_id,
                        "completion dispatcher proactively cancelled older active completion"
                    );
                }
            }
            if completion_queue_enqueue_failed(completion_ticket.queue_outcome) {
                debug!(
                    uri = %uri,
                    file_id = file_id.0,
                    file_seq = completion_ticket.file_seq,
                    request_epoch = completion_ticket.request_epoch,
                    request_id = ?completion_request_id,
                    queue_outcome = ?completion_ticket.queue_outcome,
                    "completion dispatcher dropped completion event"
                );
            }
            let completion_turn_outcome =
                if completion_queue_enqueue_failed(completion_ticket.queue_outcome) {
                    super::super::completion_dispatcher::CompletionTurnOutcome::QueueRejected
                } else if let Some(turn_waiter) = completion_dispatch.turn_waiter {
                    let turn_wait_started = Instant::now();
                    let turn_resolution = turn_waiter.wait().await;
                    let turn_wait_elapsed = turn_wait_started.elapsed();
                    self.coordinator
                        .record_completion_stage_latency("turn_wait", turn_wait_elapsed);
                    timeline_capture.push_completed_stage("turn_wait", turn_wait_elapsed);
                    completion_dispatch_attribution.turn_wait_outcome =
                        Some(turn_resolution.outcome.as_str().to_string());
                    completion_dispatch_attribution.dispatcher_resolution_latency_ms =
                        turn_resolution
                            .dispatcher_resolution_latency
                            .map(CompletionTimelineCapture::duration_to_ms);
                    turn_resolution.outcome
                } else {
                    completion_dispatch_attribution.turn_wait_outcome = Some(
                        super::super::completion_dispatcher::CompletionTurnOutcome::QueueRejected
                            .as_str()
                            .to_string(),
                    );
                    super::super::completion_dispatcher::CompletionTurnOutcome::QueueRejected
                };
            let completion_active_turn_guard = if matches!(
                completion_turn_outcome,
                super::super::completion_dispatcher::CompletionTurnOutcome::Ready
            ) {
                let _ = self
                    .completion_dispatcher_v2
                    .mark_completion_active(file_id, completion_ticket, completion_request_metadata)
                    .await;
                Some(super::helpers::CompletionActiveTurnGuard::new(
                    file_id,
                    completion_ticket.file_seq,
                    Arc::clone(&self.completion_dispatcher_v2),
                ))
            } else {
                None
            };
            (
                completion_ticket,
                Some(completion_turn_outcome),
                completion_request_registration,
                completion_cancellation_token,
                completion_drop_guard,
                completion_active_turn_guard,
                Some(completion_dispatch_attribution),
            )
        } else {
            (
                super::super::completion_dispatcher::DispatchTicket {
                    file_seq: 0,
                    request_epoch: 0,
                    queue_outcome:
                        super::super::completion_dispatcher::QueueEnqueueOutcome::Enqueued,
                },
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };
        if let Some(turn_attribution) = completion_dispatch_attribution {
            timeline_capture.set_turn_attribution(turn_attribution);
        }
        let snippet_support = *self.completion_snippet_support.read().await;
        #[cfg(test)]
        if let Some(delay_ms) = std::env::var("BSL_TEST_COMPLETION_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
        {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        let mut completion_outcome: Option<&'static str> = None;
        let mut observed_file_version_for_completion: Option<i32> = None;
        let mut member_access_observed = false;
        let mut cancel_event_emitted = false;
        let mut completion_route: Option<CompletionRouteObservation> = None;
        let mut completion = 'completion_flow: {
            if let Some(turn_outcome) = completion_turn_outcome {
                match turn_outcome {
                    super::super::completion_dispatcher::CompletionTurnOutcome::Ready => {}
                    super::super::completion_dispatcher::CompletionTurnOutcome::SupersededBeforeStart => {
                        completion_outcome = Some("superseded");
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    super::super::completion_dispatcher::CompletionTurnOutcome::QueueRejected => {
                        completion_outcome = Some("queue_rejected");
                        break 'completion_flow Some(completion_incomplete_empty_response());
        }
    }
            }

            async fn try_prepare_shadow_head_fast_path(
                server: &BslLanguageServer,
                uri: &Url,
                file_id: bsl_analysis_v2::FileId,
                position: Position,
                flow_sensitive: bool,
                completion_mode: Option<&'static str>,
            ) -> Result<Option<CompletionPreparedSnapshot>, bsl_runtime::application::SemanticOutcome>
            {
                let expected_version = server
                    .resolve_or_seed_min_file_version_v2(
                        uri,
                        file_id,
                        bsl_runtime::application::SemanticOperation::Completion,
                    )
                    .await?;
                let Some(shadow_state) = server
                    .latest_document_shadow_state_v2
                    .read()
                    .await
                    .get(&file_id)
                    .cloned()
                else {
                    return Ok(None);
                };
                if shadow_state.version < expected_version {
                    return Ok(None);
                }
                let file_path = match uri.to_file_path() {
                    Ok(path) => Arc::<str>::from(path.to_string_lossy().to_string()),
                    Err(_) => Arc::<str>::from(uri.to_string()),
                };

                let context = server
                    .build_execution_context_v2_with_completion_mode(
                        bsl_runtime::application::SemanticOperation::Completion,
                        file_id,
                        Some(expected_version),
                        flow_sensitive,
                        completion_mode,
                    )
                    .await;
                let support_bundle_started = Instant::now();
                let support_bundle = server.analysis_v2.completion_support_bundle();
                let contextual_owner_type_hints = support_bundle
                    .deps
                    .resolver
                    .as_ref()
                    .map(|resolver| {
                        bsl_runtime::application::completion_member_access_owner_type_hints_from_head_receiver(
                            shadow_state.text.as_ref(),
                            position.line,
                            position.character,
                            file_path.as_ref(),
                            resolver.as_ref(),
                            support_bundle.deps.repository.as_ref(),
                        )
                    })
                    .unwrap_or_default();
                if !contextual_owner_type_hints.is_empty() {
                    return Ok(Some(
                        CompletionPreparedSnapshot::from_shadow_head_support_bundle_fast_path(
                            context,
                            support_bundle,
                            expected_version,
                            shadow_state.text,
                            file_path,
                            contextual_owner_type_hints,
                            support_bundle_started.elapsed(),
                        ),
                    ));
                }
                let snapshot_started = Instant::now();
                let snapshot = server
                    .analysis_v2
                    .completion_current_revision_snapshot_for_origin_and_operation(
                        context.origin,
                        context.operation,
                    )
                    .await;
                let snapshot_elapsed = snapshot_started.elapsed();
                let Some(settings_id) = snapshot.analysis.settings_id().ok() else {
                    return Ok(None);
                };
                let head_owner_type_hints = bsl_runtime::application::
        completion_member_access_owner_type_hints_from_completion_head_for_version(
            &snapshot.analysis,
            file_id,
            expected_version,
            &snapshot.deps_id,
            &settings_id,
            shadow_state.text.as_ref(),
            position.line,
            position.character,
        );
                if head_owner_type_hints.is_empty() {
                    return Ok(None);
                }

                Ok(Some(
                    CompletionPreparedSnapshot::from_shadow_head_fast_path(
                        context,
                        snapshot,
                        expected_version,
                        shadow_state.text,
                        file_path,
                        head_owner_type_hints,
                        snapshot_elapsed,
                    ),
                ))
            }
            if pending_request_cancelled_before_take {
                if event_driven_guards_enabled {
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "before_sync_globals",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                } else {
                    observe_cancelled_timeline_outcome(&mut timeline_capture, "cancelled");
                    completion_outcome = Some("cancelled");
                    break 'completion_flow Some(completion_incomplete_empty_response());
                }
            }
            let first_completion_for_file = {
                let mut seen = self.completion_seen_files_v2.write().await;
                seen.insert(file_id)
            };
            self.coordinator
                .record_intellisense_v2_completion_temperature(if first_completion_for_file {
                    "first"
                } else {
                    "warm"
                });
            let sync_globals_started = Instant::now();
            self.sync_v2_globals().await;
            let sync_globals_elapsed = sync_globals_started.elapsed();
            self.coordinator
                .record_completion_stage_latency("sync_globals", sync_globals_elapsed);
            timeline_capture.push_completed_stage("sync_globals", sync_globals_elapsed);

            let empty = || Some(completion_empty_response(false));
            let mut member_access_request = trigger_char_hint == Some('.');
            if !member_access_request {
                let shadow_text = {
                    let shadow = self.latest_document_shadow_state_v2.read().await;
                    shadow.get(&file_id).map(|state| state.text.clone())
                };
                if let Some(text) = shadow_text {
                    member_access_request = completion_request_targets_member_access(
                        text.as_ref(),
                        position,
                        trigger_char_hint,
                    );
                }
            }

            let include_flow_sensitive = {
                let settings = self.settings.read().await;
                settings.enable_flow_sensitive
            };

            if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                event_driven_guards_enabled,
                self,
                file_id,
                completion_request_id.as_deref(),
                completion_ticket.request_epoch,
                completion_cancellation_token.as_ref(),
                "before_prepare",
                &mut cancel_event_emitted,
            )
            .await
            {
                observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                completion_outcome = Some(outcome);
                break 'completion_flow Some(completion_incomplete_empty_response());
            }

            let prepare_apply_age_at_start = completion_apply_age_for_file(self, file_id).await;
            timeline_capture.set_prepare_apply_age_at_start(prepare_apply_age_at_start);
            if let Some(apply_age) = prepare_apply_age_at_start {
                self.coordinator
                    .record_completion_stage_latency("prepare_apply_age_at_start", apply_age);
            }
            let prepare_started = Instant::now();
            let prepare_timeout =
                bsl_runtime::application::intellisense_v2::interactive_freshness_knobs(
                    bsl_runtime::application::SemanticOperation::Completion,
                    Some(self.coordinator.as_ref()),
                )
                .map(|knobs| knobs.wait_budget);
            timeline_capture.set_prepare_wait_budget(prepare_timeout);
            timeline_capture.set_prepare_kind(if member_access_request {
                "lightweight_current_revision"
            } else {
                "exact_stateful"
            });
            let prepare_progress = bsl_runtime::application::PrepareStatefulProgress::new();
            let guarded_prepare = if member_access_request {
                match try_prepare_shadow_head_fast_path(
                    self,
                    &uri,
                    file_id,
                    position,
                    include_flow_sensitive,
                    Some(completion_observability_mode),
                )
                .await
                {
                    Ok(Some(prepared)) => CompletionPrepareGuardResult::Prepared(Ok(prepared)),
                    Ok(None) => match run_completion_prepare_guard(
                        self.prepare_lsp_completion_first_response_v2_with_completion_mode_and_progress(
                            &uri,
                            file_id,
                            position,
                            include_flow_sensitive,
                            Some(completion_observability_mode),
                            Some(&prepare_progress),
                        ),
                        prepare_timeout,
                        event_driven_guards_enabled.then(|| {
                            wait_for_completion_prepare_abort(
                                self,
                                file_id,
                                completion_request_id.as_deref(),
                                completion_ticket.request_epoch,
                                completion_cancellation_token.as_ref(),
                                &mut cancel_event_emitted,
                            )
                        }),
                    )
                    .await
                    {
                        CompletionPrepareGuardResult::Prepared(prepared) => {
                            CompletionPrepareGuardResult::Prepared(prepared.map(
                                |(context, prepared, expected_version)| {
                                    CompletionPreparedSnapshot::from_lightweight(
                                        context,
                                        prepared,
                                        expected_version,
                                    )
                                },
                            ))
                        }
                        CompletionPrepareGuardResult::TimedOut => {
                            CompletionPrepareGuardResult::TimedOut
                        }
                        CompletionPrepareGuardResult::Aborted(outcome) => {
                            CompletionPrepareGuardResult::Aborted(outcome)
                        }
                    },
                    Err(outcome) => CompletionPrepareGuardResult::Prepared(Err(outcome)),
                }
            } else {
                match run_completion_prepare_guard(
                    self.prepare_lsp_stateful_operation_v2_with_completion_mode_and_progress(
                        &uri,
                        file_id,
                        bsl_runtime::application::SemanticOperation::Completion,
                        include_flow_sensitive,
                        Some(completion_observability_mode),
                        Some(&prepare_progress),
                    ),
                    prepare_timeout,
                    event_driven_guards_enabled.then(|| {
                        wait_for_completion_prepare_abort(
                            self,
                            file_id,
                            completion_request_id.as_deref(),
                            completion_ticket.request_epoch,
                            completion_cancellation_token.as_ref(),
                            &mut cancel_event_emitted,
                        )
                    }),
                )
                .await
                {
                    CompletionPrepareGuardResult::Prepared(prepared) => {
                        CompletionPrepareGuardResult::Prepared(prepared.map(
                            |(context, prepared, expected_version)| {
                                CompletionPreparedSnapshot::from_exact_stateful(
                                    context,
                                    prepared,
                                    expected_version,
                                )
                            },
                        ))
                    }
                    CompletionPrepareGuardResult::TimedOut => {
                        CompletionPrepareGuardResult::TimedOut
                    }
                    CompletionPrepareGuardResult::Aborted(outcome) => {
                        CompletionPrepareGuardResult::Aborted(outcome)
                    }
                }
            };
            let prepare_elapsed = prepare_started.elapsed();
            let prepare_progress_snapshot = prepare_progress.snapshot();
            timeline_capture.set_prepare_progress_snapshot(&prepare_progress_snapshot);
            self.coordinator
                .record_completion_stage_latency("prepare_stateful", prepare_elapsed);
            let prepare_apply_age_at_terminal = completion_apply_age_for_file(self, file_id).await;
            timeline_capture.set_prepare_apply_age_at_terminal(prepare_apply_age_at_terminal);
            if let Some(apply_age) = prepare_apply_age_at_terminal {
                self.coordinator
                    .record_completion_stage_latency("prepare_apply_age_at_terminal", apply_age);
            }

            let prepared = match guarded_prepare {
                CompletionPrepareGuardResult::Prepared(prepared) => {
                    timeline_capture.set_prepare_guard_outcome("prepared");
                    timeline_capture.push_completed_stage("prepare_stateful", prepare_elapsed);
                    prepared
                }
                CompletionPrepareGuardResult::TimedOut => {
                    timeline_capture.set_prepare_guard_outcome("timeout");
                    timeline_capture.set_prepare_outcome("wait_not_ready");
                    timeline_capture.set_prepare_fail_closed_cause("prepare_timeout");
                    if let Some(timeout_runtime) =
                        prepare_progress_snapshot.snapshot_with_deps_timeout_runtime
                    {
                        timeline_capture
                            .set_prepare_snapshot_with_deps_timeout_runtime(Some(timeout_runtime));
                    } else if prepare_progress_snapshot.phase == Some("snapshot_with_deps") {
                        timeline_capture.set_prepare_snapshot_with_deps_timeout_runtime(Some(
                            bsl_runtime::application::SnapshotWithDepsTimeoutRuntimeTrace {
                                queue_wait_elapsed: None,
                                exec_elapsed: None,
                                wake_wait_elapsed: None,
                                resolution:
                                    bsl_runtime::application::SnapshotWithDepsTimeoutResolutionKind::Unavailable,
                            },
                        ));
                    }
                    if let Some(prepare_timeout) = prepare_timeout {
                        timeline_capture.set_prepare_timeout_attribution(
                            bsl_runtime::application::PrepareTimeoutAttributionTrace::new(
                                bsl_runtime::application::PrepareTimeoutSourceKind::PrepareGuard,
                                prepare_progress_snapshot.phase.unwrap_or("unavailable"),
                                prepare_timeout,
                                prepare_elapsed,
                            ),
                        );
                    }
                    self.coordinator
                        .record_intellisense_v2_interactive_wait_budget_exhausted();
                    self.coordinator
                        .record_intellisense_v2_completion_fail_closed_cause("prepare_timeout");
                    self.coordinator
                        .record_intellisense_v2_completion_fallback_unavailable();
                    timeline_capture.push_stage(
                        "prepare_stateful",
                        CompletionTimelineStageStatus::Failed,
                        prepare_elapsed,
                    );
                    completion_outcome = Some("wait_not_ready");
                    break 'completion_flow Some(completion_incomplete_empty_response());
                }
                CompletionPrepareGuardResult::Aborted(outcome) => {
                    timeline_capture.set_prepare_guard_outcome(format!("aborted:{outcome}"));
                    timeline_capture.set_prepare_outcome(outcome);
                    observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                    let stage_status = match outcome {
                        "cancelled" | "superseded" => CompletionTimelineStageStatus::Cancelled,
                        _ => CompletionTimelineStageStatus::Failed,
                    };
                    timeline_capture.push_stage("prepare_stateful", stage_status, prepare_elapsed);
                    completion_outcome = Some(outcome);
                    break 'completion_flow Some(completion_incomplete_empty_response());
                }
            };

            match prepared {
                Ok(mut prepared) => {
                    let context = prepared.context.clone();
                    let expected_version = prepared.expected_version;
                    timeline_capture.set_prepare_kind(prepared.kind);
                    timeline_capture.set_prepare_outcome("ready");
                    timeline_capture
                        .set_prepare_observed_file_version(prepared.observed_file_version);
                    timeline_capture.set_prepare_wait_elapsed(prepared.wait_elapsed);
                    timeline_capture.set_prepare_snapshot_elapsed(prepared.snapshot_elapsed);
                    timeline_capture.set_prepare_wait_for_file_version_runtime(
                        prepared.wait_for_file_version_runtime,
                    );
                    if let Some(snapshot_with_deps_runtime) = prepared.snapshot_with_deps_runtime {
                        timeline_capture
                            .set_prepare_snapshot_with_deps_runtime(snapshot_with_deps_runtime);
                    }
                    if let Some(timeout_attribution) = prepared.timeout_attribution {
                        timeline_capture.set_prepare_timeout_attribution(timeout_attribution);
                    }
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "wait",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    let (snapshot_file_bytes, snapshot_file_lines) = prepared
                        .file_content_override
                        .as_ref()
                        .map(|text| (text.len(), text.lines().count()))
                        .or_else(|| {
                            prepared
                                .snapshot
                                .as_ref()
                                .and_then(|snapshot| {
                                    snapshot.analysis.file_text(file_id).ok().flatten()
                                })
                                .map(|text| (text.len(), text.lines().count()))
                        })
                        .unwrap_or((0, 0));
                    self.coordinator
                        .record_intellisense_v2_payload_shape_with_origin(
                            context.origin.as_str(),
                            context.operation.as_str(),
                            bsl_runtime::application::ObservabilityStage::RuntimeSnapshotWithDeps
                                .as_str(),
                            snapshot_file_bytes,
                            snapshot_file_lines,
                        );
                    if let Some(wait_elapsed) = prepared.wait_elapsed {
                        if let Some(threshold) =
                            super::super::intellisense_v2_slow_wait_warn_threshold()
                        {
                            if wait_elapsed >= threshold {
                                warn!(
                                    uri = %uri,
                                    file_id = file_id.0,
                                    expected_version,
                                    wait_ms = wait_elapsed.as_millis(),
                                    threshold_ms = threshold.as_millis(),
                                    "Completion v2: wait_for_file_version is slow"
                                );
                            }
                        }
                    }
                    if let Some(threshold) =
                        super::super::intellisense_v2_slow_snapshot_warn_threshold()
                    {
                        if prepared.snapshot_elapsed >= threshold {
                            warn!(
                                uri = %uri,
                                file_id = file_id.0,
                                snapshot_ms = prepared.snapshot_elapsed.as_millis(),
                                file_bytes = snapshot_file_bytes,
                                file_lines = snapshot_file_lines,
                                threshold_ms = threshold.as_millis(),
                                "Completion v2: snapshot acquisition is slow"
                            );
                        }
                    }
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "snapshot",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }

                    let mut refreshed_snapshot_after_wait = None;
                    let exact_wait_budget =
                        bsl_runtime::application::intellisense_v2::interactive_freshness_knobs(
                            bsl_runtime::application::SemanticOperation::Completion,
                            Some(self.coordinator.as_ref()),
                        )
                        .map(|knobs| knobs.wait_budget)
                        .unwrap_or_default();
                    let mut head_ready = member_access_request
                        && (prepared.head_ready_override
                            || prepared.snapshot.as_ref().is_some_and(|snapshot| {
                                snapshot
                                    .analysis
                                    .current_completion_head_ready(file_id)
                                    .ok()
                                    .unwrap_or(false)
                            }));
                    let exact_ready_before_wait = prepared.exact_ready_override
                        || prepared.snapshot.as_ref().is_some_and(|snapshot| {
                            snapshot
                                .analysis
                                .current_type_index_serve_only_ready(file_id)
                                .ok()
                                .unwrap_or(false)
                        });
                    if member_access_request {
                        timeline_capture.set_exact_wait_head_ready_before_wait(head_ready);
                        timeline_capture
                            .set_exact_wait_exact_ready_before_wait(exact_ready_before_wait);
                    }
                    let mut exact_hit_candidate = false;
                    if member_access_request && !head_ready && !exact_ready_before_wait {
                        if let Some(apply_age) = completion_apply_age_for_file(self, file_id).await
                        {
                            self.coordinator.record_completion_stage_latency(
                                "exact_wait_apply_age_at_start",
                                apply_age,
                            );
                        }
                        let exact_wait_started = Instant::now();
                        let artifact_wait_outcome = self
                            .wait_for_current_completion_artifact_ready_v2(
                                file_id,
                                Some(expected_version),
                                exact_wait_budget,
                            )
                            .await;
                        timeline_capture
                            .set_exact_wait_artifact_poll(artifact_wait_outcome.poll_trace);
                        let exact_wait_elapsed = exact_wait_started.elapsed();
                        self.coordinator.record_completion_stage_latency(
                            "wait_exact_type_index",
                            exact_wait_elapsed,
                        );
                        timeline_capture
                            .push_completed_stage("wait_exact_type_index", exact_wait_elapsed);

                        match artifact_wait_outcome.outcome {
                            super::super::core::CompletionArtifactWaitOutcomeV2::HeadReady => {
                                timeline_capture.set_exact_wait_artifact_outcome(
                                    super::super::core::CompletionArtifactWaitOutcomeV2::HeadReady
                                        .as_str(),
                                );
                                head_ready = true;
                                if prepared.snapshot.is_none() {
                                    match self
                                        .analysis_v2
                                        .prepare_completion_first_response_with_progress(
                                            &context,
                                            Some(self.coordinator.as_ref()),
                                            None,
                                            position.line,
                                            position.character,
                                        )
                                        .await
                                    {
                                        Ok(refreshed_prepared) => {
                                            prepared = CompletionPreparedSnapshot::from_lightweight(
                                                context.clone(),
                                                refreshed_prepared,
                                                expected_version,
                                            );
                                        }
                                        Err(outcome) => {
                                            let outcome =
                                                completion_prepare_error_outcome(outcome);
                                            observe_cancelled_timeline_outcome(
                                                &mut timeline_capture,
                                                outcome,
                                            );
                                            completion_outcome = Some(outcome);
                                            break 'completion_flow Some(
                                                completion_incomplete_empty_response(),
                                            );
                                        }
                                    }
                                }
                            }
                            super::super::core::CompletionArtifactWaitOutcomeV2::ExactReady => {
                                timeline_capture.set_exact_wait_artifact_outcome(
                                    super::super::core::CompletionArtifactWaitOutcomeV2::ExactReady
                                        .as_str(),
                                );
                                if let Some(apply_age) =
                                    completion_apply_age_for_file(self, file_id).await
                                {
                                    self.coordinator.record_completion_stage_latency(
                                        "exact_wait_apply_age_at_terminal",
                                        apply_age,
                                    );
                                }
                                self.coordinator
                                    .record_intellisense_v2_completion_exact_type_index_wait_outcome(
                                        super::super::core::ExactTypeIndexWaitOutcomeV2::Ready
                                            .as_str(),
                                    );
                                exact_hit_candidate = true;
                                refreshed_snapshot_after_wait = Some(
                                    self.analysis_v2
                                        .completion_current_revision_snapshot_for_origin_and_operation(
                                            context.origin,
                                            context.operation,
                                        )
                                        .await,
                                );
                            }
                            super::super::core::CompletionArtifactWaitOutcomeV2::Deadline
                            | super::super::core::CompletionArtifactWaitOutcomeV2::ObservedVersionMismatch => {
                                timeline_capture
                                    .set_exact_wait_artifact_outcome(artifact_wait_outcome.outcome.as_str());
                                if let Some(apply_age) =
                                    completion_apply_age_for_file(self, file_id).await
                                {
                                    self.coordinator.record_completion_stage_latency(
                                        "exact_wait_apply_age_at_terminal",
                                        apply_age,
                                    );
                                }
                                let terminal_outcome = if matches!(
                                    artifact_wait_outcome.outcome,
                                    super::super::core::CompletionArtifactWaitOutcomeV2::ObservedVersionMismatch
                                ) {
                                    super::super::core::ExactTypeIndexWaitOutcomeV2::ObservedVersionMismatch
                                } else {
                                    super::super::core::ExactTypeIndexWaitOutcomeV2::Deadline
                                };
                                self.coordinator
                                    .record_intellisense_v2_completion_exact_type_index_wait_outcome(
                                        terminal_outcome.as_str(),
                                    );
                                if terminal_outcome
                                    == super::super::core::ExactTypeIndexWaitOutcomeV2::Deadline
                                {
                                    timeline_capture.set_prepare_fail_closed_cause("exact_deadline");
                                    self.coordinator
                                        .record_intellisense_v2_completion_fail_closed_cause(
                                            "exact_deadline",
                                        );
                                }
                                self.coordinator
                                    .record_intellisense_v2_completion_fallback_unavailable();
                                completion_outcome.get_or_insert("wait_not_ready");
                                break 'completion_flow Some(completion_empty_response(false));
                            }
                        }
                    }
                    let current_revision_head_owner_type_hints = if member_access_request
                        && head_ready
                    {
                        prepared
                            .head_owner_type_hints_override
                            .clone()
                            .or_else(|| {
                                prepared.snapshot.as_ref().map(|snapshot| {
                                    completion_member_access_owner_type_hints_from_current_revision_head(
                                        &snapshot.analysis,
                                        file_id,
                                        position,
                                    )
                                })
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    let head_route_candidate =
                        member_access_request && !current_revision_head_owner_type_hints.is_empty();
                    if member_access_request {
                        timeline_capture.set_exact_wait_current_revision_head_owner_hints_ready(
                            !current_revision_head_owner_type_hints.is_empty(),
                        );
                    }
                    if member_access_request
                        && current_revision_head_owner_type_hints.is_empty()
                        && !exact_ready_before_wait
                        && !exact_hit_candidate
                    {
                        if let Some(apply_age) = completion_apply_age_for_file(self, file_id).await
                        {
                            self.coordinator.record_completion_stage_latency(
                                "exact_wait_apply_age_at_start",
                                apply_age,
                            );
                        }
                        let exact_wait_started = Instant::now();
                        let exact_wait = self
                            .wait_for_current_type_index_serve_only_ready_v2(
                                file_id,
                                Some(expected_version),
                                exact_wait_budget,
                            )
                            .await;
                        let exact_wait_elapsed = exact_wait_started.elapsed();
                        self.coordinator.record_completion_stage_latency(
                            "wait_exact_type_index",
                            exact_wait_elapsed,
                        );
                        timeline_capture
                            .push_completed_stage("wait_exact_type_index", exact_wait_elapsed);
                        timeline_capture
                            .set_exact_wait_type_index_outcome(exact_wait.outcome.as_str());
                        timeline_capture.set_exact_wait_type_index_waiter_action(
                            exact_wait.waiter_action.as_str(),
                        );
                        if let Some(matching_task_state) = exact_wait.matching_task_state {
                            timeline_capture
                                .set_exact_wait_matching_task_state(matching_task_state.as_str());
                        }
                        if let Some(task_phase) = exact_wait.task_phase {
                            timeline_capture.set_exact_wait_task_phase(task_phase.as_str());
                        }

                        if exact_wait.outcome
                            != super::super::core::ExactTypeIndexWaitOutcomeV2::Ready
                        {
                            if let Some(apply_age) =
                                completion_apply_age_for_file(self, file_id).await
                            {
                                self.coordinator.record_completion_stage_latency(
                                    "exact_wait_apply_age_at_terminal",
                                    apply_age,
                                );
                            }
                            self.coordinator
                                .record_intellisense_v2_completion_exact_type_index_wait_outcome(
                                    exact_wait.outcome.as_str(),
                                );
                            if exact_wait.outcome
                                == super::super::core::ExactTypeIndexWaitOutcomeV2::Deadline
                            {
                                timeline_capture.set_prepare_fail_closed_cause("exact_deadline");
                                self.coordinator
                                    .record_intellisense_v2_completion_fail_closed_cause(
                                        "exact_deadline",
                                    );
                            }
                            self.coordinator
                                .record_intellisense_v2_completion_fallback_unavailable();
                            completion_outcome.get_or_insert("wait_not_ready");
                            break 'completion_flow Some(completion_empty_response(false));
                        }

                        let snapshot_after_wait = self
                            .analysis_v2
                            .completion_current_revision_snapshot_for_origin_and_operation(
                                context.origin,
                                context.operation,
                            )
                            .await;
                        let exact_ready_after_wait = snapshot_after_wait
                            .analysis
                            .current_type_index_serve_only_ready(file_id)
                            .ok()
                            .unwrap_or(false);
                        if !exact_ready_after_wait {
                            let terminal_outcome = if snapshot_after_wait
                                .analysis
                                .file_version(file_id)
                                .ok()
                                .flatten()
                                != Some(expected_version)
                            {
                                super::super::core::ExactTypeIndexWaitOutcomeV2::ObservedVersionMismatch
                            } else {
                                super::super::core::ExactTypeIndexWaitOutcomeV2::Deadline
                            };
                            if let Some(apply_age) =
                                completion_apply_age_for_file(self, file_id).await
                            {
                                self.coordinator.record_completion_stage_latency(
                                    "exact_wait_apply_age_at_terminal",
                                    apply_age,
                                );
                            }
                            self.coordinator
                                .record_intellisense_v2_completion_exact_type_index_wait_outcome(
                                    terminal_outcome.as_str(),
                                );
                            if terminal_outcome
                                == super::super::core::ExactTypeIndexWaitOutcomeV2::Deadline
                            {
                                timeline_capture.set_prepare_fail_closed_cause("exact_deadline");
                                self.coordinator
                                    .record_intellisense_v2_completion_fail_closed_cause(
                                        "exact_deadline",
                                    );
                            }
                            self.coordinator
                                .record_intellisense_v2_completion_fallback_unavailable();
                            completion_outcome.get_or_insert("wait_not_ready");
                            break 'completion_flow Some(completion_empty_response(false));
                        }
                        if let Some(apply_age) = completion_apply_age_for_file(self, file_id).await
                        {
                            self.coordinator.record_completion_stage_latency(
                                "exact_wait_apply_age_at_terminal",
                                apply_age,
                            );
                        }
                        self.coordinator
                            .record_intellisense_v2_completion_exact_type_index_wait_outcome(
                                super::super::core::ExactTypeIndexWaitOutcomeV2::Ready.as_str(),
                            );
                        exact_hit_candidate = true;
                        refreshed_snapshot_after_wait = Some(snapshot_after_wait);
                    }

                    let query_bundle_started = Instant::now();
                    let exact_snapshot_for_query = if head_route_candidate {
                        None
                    } else if let Some(snapshot) = refreshed_snapshot_after_wait {
                        Some(snapshot)
                    } else if let Some(snapshot) = prepared.snapshot.take() {
                        Some(snapshot)
                    } else {
                        Some(
                            self.analysis_v2
                                .completion_current_revision_snapshot_for_origin_and_operation(
                                    context.origin,
                                    context.operation,
                                )
                                .await,
                        )
                    };
                    let (
                        file_content,
                        file_path,
                        mut member_access_owner_type_hints,
                        deps,
                        ir_program,
                        index_snapshot,
                        observed_deps_id,
                        observed_settings_id,
                        observed_file_version,
                    ) = {
                        let file_content_override = prepared.file_content_override.clone();
                        let file_path_override = prepared.file_path_override.clone();
                        let observed_file_version_override = prepared.observed_file_version;
                        if head_route_candidate {
                            let index_snapshot = prepared
                                .index_snapshot_override
                                .clone()
                                .expect("lightweight completion prepare must carry index snapshot");
                            let observed_deps_id = prepared
                                .deps_id_override
                                .clone()
                                .expect("lightweight completion prepare must carry deps id");
                            let observed_settings_id = prepared.settings_id_override.clone();
                            debug!(
                                "Completion v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                                uri,
                                file_id.0,
                                observed_file_version_override,
                                Some(observed_deps_id.as_str()),
                                observed_settings_id.as_ref().map(|v| v.as_str()),
                                index_snapshot.id.as_str(),
                            );
                            (
                                file_content_override,
                                file_path_override,
                                current_revision_head_owner_type_hints.clone(),
                                prepared.deps_override.clone(),
                                None,
                                index_snapshot,
                                observed_deps_id,
                                observed_settings_id,
                                observed_file_version_override,
                            )
                        } else {
                            let snapshot = exact_snapshot_for_query
                                .expect("exact completion route must carry a fresh snapshot");
                            let analysis = snapshot.analysis;
                            let index_snapshot = snapshot.index_snapshot;
                            let observed_deps_id = snapshot.deps_id;
                            let observed_file_version = observed_file_version_override
                                .or_else(|| analysis.file_version(file_id).ok().flatten());
                            let observed_settings_id = prepared
                                .settings_id_override
                                .clone()
                                .or_else(|| analysis.settings_id().ok());
                            debug!(
                            "Completion v2 observed: uri={}, file_id={}, file_version={:?}, deps_id={:?}, settings_id={:?}, index_snapshot_id={}",
                                uri,
                                file_id.0,
                                observed_file_version,
                                Some(observed_deps_id.as_str()),
                                observed_settings_id.as_ref().map(|v| v.as_str()),
                                index_snapshot.id.as_str(),
                        );
                            match analysis.file_text_len(file_id) {
                                Ok(Some(len)) => debug!(
                                    "Completion v2 (salsa) active: uri={}, file_id={}, text_len={}",
                                    uri, file_id.0, len
                                ),
                                Ok(None) => debug!(
                                    "Completion v2 (salsa) active: uri={}, file_id={} (file not found)",
                                    uri, file_id.0
                                ),
                                Err(_) => debug!(
                                    "Completion v2 (salsa) cancelled: uri={}, file_id={}",
                                    uri, file_id.0
                                ),
                            }

                            let observed_byte_offset = analysis
                                .utf16_position_to_byte_offset(
                                    file_id,
                                    position.line,
                                    position.character,
                                )
                                .ok()
                                .flatten();
                            let observed_point = analysis
                                .utf16_position_to_point(file_id, position.line, position.character)
                                .ok()
                                .flatten();
                            debug!(
                            "Completion v2 positioning: uri={}, file_id={}, lsp=({}:{}) -> byte_offset={:?}, point={:?}",
                            uri,
                            file_id.0,
                            position.line,
                            position.character,
                            observed_byte_offset,
                            observed_point,
                        );
                            let context_for_query = context.clone();
                            let coordinator_for_query = self.coordinator.clone();
                            let uri_for_query = uri.clone();
                            let observed_deps_id_for_query = observed_deps_id.clone();
                            let cancellation_token_for_query =
                                completion_cancellation_token.clone();
                            let query_result =
                                bsl_runtime::application::spawn_bounded_blocking_with_class_observed_origin(
                                    bsl_runtime::application::CpuWorkClass::Interactive,
                                    context_for_query.origin.as_str(),
                                    Some(self.coordinator.as_ref()),
                                    move || {
                                    let deps_and_file_snapshot_started = Instant::now();
                                    let file_content = analysis.file_text(file_id).ok().flatten();
                                    let file_path = analysis.file_path(file_id).ok().flatten();
                                    let deps = analysis.deps_data().ok();
                                    let member_access_owner_type_hints = file_content
                                        .as_deref()
                                        .map(|text| {
                                            if completion_request_targets_member_access(
                                                text,
                                                position,
                                                trigger_char_hint,
                                            ) {
                                                completion_member_access_owner_type_hints_at_position(
                                                    &analysis,
                                                    file_id,
                                                    text,
                                                    position,
                                                    Some(coordinator_for_query.as_ref()),
                                                )
                                            } else {
                                                Vec::new()
                                            }
                                        })
                                        .unwrap_or_default();
                                    coordinator_for_query.record_completion_stage_latency(
                                        "query_bundle_deps_and_file_snapshot",
                                        deps_and_file_snapshot_started.elapsed(),
                                    );
                                    if cancellation_token_for_query
                                        .as_ref()
                                        .is_some_and(|token| token.is_cancelled())
                                    {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_result(
                                                "cancelled",
                                            );
                                        return (
                                            file_content,
                                            file_path,
                                            member_access_owner_type_hints,
                                            deps,
                                            None,
                                            false,
                                            true,
                                        );
                                    }

                                    #[cfg(test)]
                                    if let Some(delay_ms) = std::env::var(
                                        "BSL_TEST_COMPLETION_IR_QUERY_DELAY_MS",
                                    )
                                    .ok()
                                    .and_then(|value| value.parse::<u64>().ok())
                                    .filter(|value| *value > 0)
                                    {
                                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                                    }

                                    let ir_started = Instant::now();
                                    let ir_query =
                                        bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                                            &context_for_query,
                                            &analysis,
                                            Some(coordinator_for_query.as_ref()),
                                            file_id,
                                        );
                                    let ir_elapsed = ir_started.elapsed();
                                    let ir_outcome =
                                        bsl_runtime::application::classify_optional_query(&ir_query);
                                    if let Some(threshold) =
                                        super::super::intellisense_v2_slow_query_warn_threshold()
                                    {
                                        if ir_elapsed >= threshold {
                                            warn!(
                                                uri = %uri_for_query,
                                                file_id = file_id.0,
                                                ir_ms = ir_elapsed.as_millis(),
                                                threshold_ms = threshold.as_millis(),
                                                "Completion v2: ir query is slow"
                                            );
                                        }
                                    }

                                    let (ir_program, ir_cancelled_after_retry) = match ir_query {
                                        Ok(program) => (program, false),
                                        Err(first_cancelled) => {
                                            // One fast retry mitigates transient cancellation races between
                                            // rapid didChange updates and completion query execution.
                                            let retry_started = Instant::now();
                                            let ir_retry =
                                                bsl_runtime::application::IntellisenseV2Facade::run_ir_query_singleflight(
                                                    &context_for_query,
                                                    &analysis,
                                                    Some(coordinator_for_query.as_ref()),
                                                    file_id,
                                                );
                                            let retry_elapsed = retry_started.elapsed();
                                            if let Some(threshold) =
                                                super::super::intellisense_v2_slow_query_warn_threshold()
                                            {
                                                if retry_elapsed >= threshold {
                                                    warn!(
                                                        uri = %uri_for_query,
                                                        file_id = file_id.0,
                                                        ir_retry_ms = retry_elapsed.as_millis(),
                                                        threshold_ms = threshold.as_millis(),
                                                        "Completion v2: ir retry query is slow"
                                                    );
                                                }
                                            }
                                            match ir_retry {
                                                Ok(program) => {
                                                    debug!(
                                                        uri = %uri_for_query,
                                                        file_id = file_id.0,
                                                        "Completion v2: recovered from transient ir cancellation via retry"
                                                    );
                                                    (program, false)
                                                }
                                                Err(retry_cancelled) => {
                                                    debug!(
                                                        uri = %uri_for_query,
                                                        file_id = file_id.0,
                                                        first_error = ?first_cancelled,
                                                        retry_error = ?retry_cancelled,
                                                        ir_outcome = ir_outcome.as_str(),
                                                        "Completion v2: ir query cancelled after retry"
                                                    );
                                                    (None, true)
                                                }
                                            }
                                        }
                                    };
                                    if cancellation_token_for_query
                                        .as_ref()
                                        .is_some_and(|token| token.is_cancelled())
                                    {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_result(
                                                "cancelled",
                                            );
                                        return (
                                            file_content,
                                            file_path,
                                            member_access_owner_type_hints,
                                            deps,
                                            ir_program,
                                            ir_cancelled_after_retry,
                                            true,
                                        );
                                    }
                                    if bsl_runtime::system::global_runtime_config()
                                        .get_bool(
                                            bsl_runtime::system::RuntimeKey::IntellisenseV2P4Smoke,
                                        )
                                        .unwrap_or(false)
                                    {
                                        match ir_program.as_ref() {
                                            Some(program) => debug!(
                                                "Completion v2 ir: uri={}, file_id={}, deps_id={:?}, nodes={}",
                                                uri_for_query,
                                                file_id.0,
                                                Some(observed_deps_id_for_query.as_str()),
                                                program.nodes.len()
                                            ),
                                            None => debug!(
                                                "Completion v2 ir: uri={}, file_id={} (unavailable)",
                                                uri_for_query, file_id.0
                                            ),
                                        }
                                    }

                                    if cancellation_token_for_query
                                        .as_ref()
                                        .is_some_and(|token| token.is_cancelled())
                                    {
                                        coordinator_for_query
                                            .record_intellisense_v2_completion_owner_hint_result(
                                                "cancelled",
                                            );
                                        return (
                                            file_content,
                                            file_path,
                                            member_access_owner_type_hints,
                                            deps,
                                            ir_program,
                                            ir_cancelled_after_retry,
                                            true,
                                        );
                                    }

                                    (
                                        file_content,
                                        file_path,
                                        member_access_owner_type_hints,
                                        deps,
                                        ir_program,
                                        ir_cancelled_after_retry,
                                        false,
                                    )
                                },
                                )
                                .await;

                            let (
                                file_content,
                                file_path,
                                member_access_owner_type_hints,
                                deps,
                                ir_program,
                                ir_cancelled_after_retry,
                                query_checkpoint_cancelled,
                            ) = match query_result {
                                Ok(result) => result,
                                Err(join_error) => {
                                    warn!(
                                        uri = %uri,
                                        file_id = file_id.0,
                                        error = %join_error,
                                        "Completion v2: interactive query task failed"
                                    );
                                    (None, None, Vec::new(), None, None, true, true)
                                }
                            };
                            if (ir_cancelled_after_retry || query_checkpoint_cancelled)
                                && completion_outcome.is_none()
                            {
                                timeline_capture
                                    .observe_cancel_at_ms(super::super::unix_timestamp_ms());
                                completion_outcome = Some("cancelled");
                            }
                            if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                                event_driven_guards_enabled,
                                self,
                                file_id,
                                completion_request_id.as_deref(),
                                completion_ticket.request_epoch,
                                completion_cancellation_token.as_ref(),
                                "ir",
                                &mut cancel_event_emitted,
                            )
                            .await
                            {
                                observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                                completion_outcome = Some(outcome);
                                break 'completion_flow Some(completion_incomplete_empty_response());
                            }

                            (
                                file_content,
                                file_path,
                                member_access_owner_type_hints,
                                deps,
                                ir_program,
                                index_snapshot,
                                observed_deps_id,
                                observed_settings_id,
                                observed_file_version,
                            )
                        }
                    };
                    let query_bundle_elapsed = query_bundle_started.elapsed();
                    self.coordinator
                        .record_completion_stage_latency("query_bundle", query_bundle_elapsed);
                    timeline_capture.push_completed_stage("query_bundle", query_bundle_elapsed);
                    observed_file_version_for_completion = observed_file_version;
                    let member_access_context = file_content
                        .as_deref()
                        .map(|text| {
                            completion_request_targets_member_access(
                                text,
                                position,
                                trigger_char_hint,
                            )
                        })
                        .unwrap_or(member_access_request);
                    member_access_observed = member_access_context;
                    if member_access_context && member_access_owner_type_hints.is_empty() {
                        member_access_owner_type_hints =
                            current_revision_head_owner_type_hints.clone();
                    }
                    if member_access_context {
                        if head_route_candidate {
                            if let Some(file_version) = observed_file_version {
                                completion_route = Some(CompletionRouteObservation {
                                    kind: CompletionRouteKind::HeadHit,
                                    file_version,
                                    deps_id: observed_deps_id.clone(),
                                    settings_id: observed_settings_id.clone(),
                                    exact_ready: exact_ready_before_wait,
                                });
                            }
                        } else if exact_hit_candidate {
                            if let Some(file_version) = observed_file_version {
                                completion_route = Some(CompletionRouteObservation {
                                    kind: CompletionRouteKind::ExactHit,
                                    file_version,
                                    deps_id: observed_deps_id.clone(),
                                    settings_id: observed_settings_id.clone(),
                                    exact_ready: true,
                                });
                            }
                        }
                    }
                    if member_access_context && member_access_owner_type_hints.is_empty() {
                        self.coordinator
                            .record_intellisense_v2_completion_fallback_unavailable();
                        completion_outcome.get_or_insert("wait_not_ready");
                        break 'completion_flow Some(completion_empty_response(false));
                    }
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "collect",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }

                    let response_build_started = Instant::now();
                    let mut completion_response = match (file_content, file_path, deps, ir_program)
                    {
                        (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                            crate::handlers::handle_completion_v2_with_trigger_hint_and_owner_hints(
                                file_content,
                                file_path,
                                Some(ir_program),
                                member_access_owner_type_hints,
                                deps,
                                position,
                                &uri,
                                index_snapshot.as_ref(),
                                snippet_support,
                                include_flow_sensitive,
                                trigger_char_hint,
                            )
                            .await
                        }
                        (Some(file_content), Some(file_path), Some(deps), None)
                            if member_access_context
                                && !member_access_owner_type_hints.is_empty() =>
                        {
                            crate::handlers::handle_completion_v2_with_trigger_hint_and_owner_hints(
                                file_content,
                                file_path,
                                None,
                                member_access_owner_type_hints,
                                deps,
                                position,
                                &uri,
                                index_snapshot.as_ref(),
                                snippet_support,
                                include_flow_sensitive,
                                trigger_char_hint,
                            )
                            .await
                        }
                        (None, _, _, _) => {
                            completion_outcome.get_or_insert("missing_file_content");
                            empty()
                        }
                        (Some(_), None, _, _) => {
                            completion_outcome.get_or_insert("missing_file_path");
                            empty()
                        }
                        (Some(_), Some(_), None, _) => {
                            completion_outcome.get_or_insert("missing_deps");
                            empty()
                        }
                        (Some(file_content), Some(file_path), Some(deps), None) => {
                            let (fallback_outcome, response) = resolve_completion_without_ir(
                                self,
                                file_id,
                                observed_deps_id.clone(),
                                observed_settings_id.clone(),
                                observed_file_version,
                                member_access_context,
                                file_content,
                                file_path,
                                member_access_owner_type_hints,
                                deps,
                                position,
                                &uri,
                                index_snapshot.as_ref(),
                                snippet_support,
                                include_flow_sensitive,
                                trigger_char_hint,
                            )
                            .await;
                            completion_outcome.get_or_insert(fallback_outcome);
                            response
                        }
                    };
                    if let (Some(response), Some(file_version)) =
                        (completion_response.as_mut(), observed_file_version)
                    {
                        crate::handlers::attach_completion_resolve_context(
                            &mut response.response,
                            &uri,
                            file_version,
                            &observed_deps_id,
                            observed_settings_id.as_ref(),
                        );
                    }
                    let response_build_elapsed = response_build_started.elapsed();
                    self.coordinator
                        .record_completion_stage_latency("response_build", response_build_elapsed);
                    let response_build_breakdown = completion_response
                        .as_ref()
                        .and_then(|response| response.stats.as_ref())
                        .map(|stats| CompletionResponseBuildBreakdown {
                            snapshot_read: stats.stage_snapshot_read,
                            collect: stats.stage_collect,
                            rank: stats.stage_rank,
                            format: stats.stage_format,
                        });
                    timeline_capture.push_response_build_stage(
                        response_build_elapsed,
                        response_build_breakdown,
                    );
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "rank",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
                        event_driven_guards_enabled,
                        self,
                        file_id,
                        completion_request_id.as_deref(),
                        completion_ticket.request_epoch,
                        completion_cancellation_token.as_ref(),
                        "format",
                        &mut cancel_event_emitted,
                    )
                    .await
                    {
                        observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    completion_response
                }
                Err(outcome) => {
                    let prepare_outcome = completion_prepare_error_outcome(outcome);
                    timeline_capture.set_prepare_outcome(prepare_outcome);
                    completion_outcome = Some(prepare_outcome);
                    debug!(
                        uri = %uri,
                        file_id = file_id.0,
                        outcome = outcome.as_str(),
                        "Completion v2: stateful operation not ready"
                    );
                    empty()
                }
            }
        };
        let elapsed = started.elapsed();
        self.coordinator.record_completion_latency(elapsed);
        if let Some(outcome) = completion_checkpoint_outcome_if_enabled(
            event_driven_guards_enabled,
            self,
            file_id,
            completion_request_id.as_deref(),
            completion_ticket.request_epoch,
            completion_cancellation_token.as_ref(),
            "publish",
            &mut cancel_event_emitted,
        )
        .await
        {
            observe_cancelled_timeline_outcome(&mut timeline_capture, outcome);
            completion_outcome = Some(outcome);
            completion = Some(completion_incomplete_empty_response());
        }

        super::impl_completion_helpers::observe_completion_result_metrics(
            self,
            &completion,
            &mut completion_outcome,
            super::impl_completion_helpers::CompletionResultMetricsContext {
                member_access_observed,
                trigger_mode,
                observed_file_version_for_completion,
                file_id,
                position: &position,
            },
        )
        .await;

        if let Some(reason) = completion_outcome.and_then(completion_public_fail_closed_reason) {
            self.coordinator
                .record_intellisense_v2_interactive_fail_closed_reason("lsp", "completion", reason);
        }

        let timeline_outcome =
            completion_public_timeline_outcome(completion_outcome.unwrap_or("ok_empty"));
        observe_cancelled_timeline_outcome(&mut timeline_capture, timeline_outcome);
        if matches!(timeline_outcome, "ok_non_empty" | "ok_empty") {
            if let Some(route) = completion_route.take() {
                match route.kind {
                    CompletionRouteKind::HeadHit => {
                        timeline_capture.set_prepare_route("head_hit");
                        self.record_completion_head_hit_v2(
                            file_id,
                            route.file_version,
                            route.deps_id,
                            route.settings_id,
                            route.exact_ready,
                        )
                        .await;
                    }
                    CompletionRouteKind::ExactHit => {
                        timeline_capture.set_prepare_route("exact_hit");
                        self.record_completion_exact_hit_v2(
                            file_id,
                            route.file_version,
                            route.deps_id,
                            route.settings_id,
                        )
                        .await;
                    }
                }
            }
        }
        timeline_capture.push_terminal_stage(timeline_outcome);
        if let Some(service_future_first_poll_entered_at_ms) =
            super::super::request_context::current_request_service_future_first_poll_entered_at_ms()
                .or_else(|| {
                    pending_request_context
                        .and_then(|context| context.service_future_first_poll_entered_at_ms)
                })
        {
            timeline_capture.set_service_future_first_poll_entered_at_ms(
                service_future_first_poll_entered_at_ms,
            );
        }
        if let Some(service_future_first_poll_outcome) =
            super::super::request_context::current_request_service_future_first_poll_outcome()
                .or_else(|| {
                    pending_request_context
                        .and_then(|context| context.service_future_first_poll_outcome.clone())
                })
        {
            timeline_capture
                .set_service_future_first_poll_outcome(service_future_first_poll_outcome);
        }
        if let Some(service_future_first_wake_scheduled_at_ms) =
            super::super::request_context::current_request_service_future_first_wake_scheduled_at_ms(
            )
            .or_else(|| {
                pending_request_context
                    .and_then(|context| context.service_future_first_wake_scheduled_at_ms)
            })
        {
            timeline_capture.set_service_future_first_wake_scheduled_at_ms(
                service_future_first_wake_scheduled_at_ms,
            );
        }
        timeline_capture.set_response_sent_at_ms(super::super::unix_timestamp_ms());
        if !shadow_internal_request {
            if let Some(server_edge_details) = timeline_capture.server_edge_details_trace() {
                self.coordinator.record_completion_stage_latency(
                    "transport_to_handler_wait",
                    std::time::Duration::from_millis(
                        server_edge_details.transport_to_handler_wait_ms,
                    ),
                );
                self.coordinator.record_completion_stage_latency(
                    "server_handler_exec",
                    std::time::Duration::from_millis(server_edge_details.server_handler_exec_ms),
                );
                if let Some(cancel_observed_after_handler_enter_ms) =
                    server_edge_details.cancel_observed_after_handler_enter_ms
                {
                    self.coordinator.record_completion_stage_latency(
                        "cancel_observed_after_handler_enter",
                        std::time::Duration::from_millis(cancel_observed_after_handler_enter_ms),
                    );
                    self.coordinator
                        .record_intellisense_v2_completion_cancel_observed();
                }
            }
        }
        if !shadow_internal_request {
            let trace = timeline_capture.into_trace(
                self.next_completion_timeline_trace_id(),
                elapsed,
                timeline_outcome,
            );
            self.record_completion_timeline_trace(trace).await;
        }

        if let Some(outcome) = completion_outcome {
            self.coordinator
                .record_intellisense_v2_completion_outcome(outcome);
        }

        if let Some(drop_guard) = completion_drop_guard.as_mut() {
            drop_guard.disarm();
        }
        drop(completion_active_turn_guard.take());
        Ok(completion.map(|result| result.response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::request_context;
    use std::future::pending;
    use tower_lsp::lsp_types::Url;

    fn sample_capture() -> CompletionTimelineCapture {
        let uri = Url::parse("file:///completion_timeline_capture_test.bsl").expect("test uri");
        CompletionTimelineCapture::new(
            Some("req-1".to_string()),
            &uri,
            "invoked",
            1_699_999_995,
            1_700_000_000,
        )
    }

    #[test]
    fn completion_timeline_total_duration_is_never_less_than_stage_end() {
        let mut capture = sample_capture();
        capture.push_completed_stage("sync_globals", std::time::Duration::from_millis(7));
        capture.push_completed_stage("query_bundle", std::time::Duration::from_millis(11));

        let trace = capture.into_trace(
            "trace-1".to_string(),
            std::time::Duration::from_millis(5),
            "ok_empty",
        );
        assert_eq!(trace.total_duration_ms, 18);
    }

    #[test]
    fn response_build_breakdown_replaces_aggregate_without_double_counting() {
        let mut capture = sample_capture();
        capture.push_completed_stage("prepare_stateful", std::time::Duration::from_millis(5));
        capture.push_response_build_stage(
            std::time::Duration::from_millis(10),
            Some(CompletionResponseBuildBreakdown {
                snapshot_read: std::time::Duration::from_millis(2),
                collect: std::time::Duration::from_millis(3),
                rank: std::time::Duration::from_millis(1),
                format: std::time::Duration::from_millis(1),
            }),
        );

        let stage_names: Vec<&str> = capture
            .stages
            .iter()
            .map(|stage| stage.name.as_str())
            .collect();
        assert!(!stage_names.contains(&"response_build"));
        assert!(stage_names.contains(&"snapshot_read"));
        assert!(stage_names.contains(&"collect"));
        assert!(stage_names.contains(&"rank"));
        assert!(stage_names.contains(&"format"));
        assert!(stage_names.contains(&"response_build_other"));

        let max_stage_end = capture
            .stages
            .iter()
            .map(|stage| stage.started_offset_ms.saturating_add(stage.duration_ms))
            .max()
            .unwrap_or(0);
        assert_eq!(max_stage_end, 15);
    }

    #[test]
    fn response_build_breakdown_falls_back_to_aggregate_when_inconsistent() {
        let mut capture = sample_capture();
        capture.push_response_build_stage(
            std::time::Duration::from_millis(3),
            Some(CompletionResponseBuildBreakdown {
                snapshot_read: std::time::Duration::from_millis(2),
                collect: std::time::Duration::from_millis(2),
                rank: std::time::Duration::from_millis(1),
                format: std::time::Duration::from_millis(1),
            }),
        );
        assert_eq!(capture.stages.len(), 1);
        assert_eq!(capture.stages[0].name, "response_build");
        assert_eq!(capture.stages[0].duration_ms, 3);
    }

    #[test]
    fn terminal_stage_marks_superseded_as_cancelled() {
        let mut capture = sample_capture();
        capture.push_terminal_stage("superseded");
        assert_eq!(capture.stages.len(), 1);
        assert_eq!(capture.stages[0].name, "terminal");
        assert_eq!(capture.stages[0].status, "cancelled");
    }

    #[test]
    fn terminal_stage_marks_fail_closed_as_failed() {
        let mut capture = sample_capture();
        capture.push_terminal_stage("fail_closed");
        assert_eq!(capture.stages.len(), 1);
        assert_eq!(capture.stages[0].name, "terminal");
        assert_eq!(capture.stages[0].status, "failed");
    }

    #[test]
    fn server_edge_details_are_derived_from_transport_handler_and_response_timestamps() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_990);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_future_created_at_ms(1_699_999_999_991);
        capture.set_service_scope_entered_at_ms(1_699_999_999_992);
        capture.set_method_entered_at_ms(1_699_999_999_995);
        capture.set_handler_entered_at_ms(1_700_000_000_000);
        capture.set_response_sent_at_ms(1_700_000_000_025);

        let trace = capture.into_trace(
            "trace-server-edge".to_string(),
            std::time::Duration::from_millis(25),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(details.transport_received_at_ms, 1_699_999_999_990);
        assert_eq!(
            details.transport_received_at_ms_provenance,
            "request_context_call_entry"
        );
        assert_eq!(details.jsonrpc_dispatch_received_at_ms, None);
        assert_eq!(details.dispatch_to_request_context_wait_ms, None);
        assert_eq!(
            details.service_future_created_at_ms,
            Some(1_699_999_999_991)
        );
        assert_eq!(details.service_scope_entered_at_ms, Some(1_699_999_999_992));
        assert_eq!(details.method_entered_at_ms, Some(1_699_999_999_995));
        assert_eq!(details.handler_entered_at_ms, 1_700_000_000_000);
        assert_eq!(details.response_sent_at_ms, 1_700_000_000_025);
        assert_eq!(details.transport_to_service_future_wait_ms, Some(1));
        assert_eq!(details.service_future_to_scope_wait_ms, Some(1));
        assert_eq!(details.service_future_first_poll_entered_at_ms, None);
        assert_eq!(details.service_future_to_first_poll_wait_ms, None);
        assert_eq!(details.service_future_first_poll_outcome, None);
        assert_eq!(details.service_future_first_wake_scheduled_at_ms, None);
        assert_eq!(details.first_poll_to_first_wake_wait_ms, None);
        assert_eq!(details.transport_to_service_scope_wait_ms, Some(2));
        assert_eq!(details.service_scope_to_method_wait_ms, Some(3));
        assert_eq!(details.transport_to_method_wait_ms, Some(5));
        assert_eq!(details.method_prelude_exec_ms, Some(5));
        assert_eq!(details.transport_to_handler_wait_ms, 10);
        assert_eq!(details.server_handler_exec_ms, 25);
        assert_eq!(details.cancel_observed_at_ms, None);
        assert_eq!(details.cancel_observed_after_handler_enter_ms, None);
    }

    #[test]
    fn server_edge_details_derive_first_poll_and_first_wake_split_when_present() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_990);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_future_created_at_ms(1_699_999_999_995);
        capture.set_service_future_first_poll_entered_at_ms(1_700_000_000_000);
        capture.set_service_future_first_poll_outcome("pending");
        capture.set_service_future_first_wake_scheduled_at_ms(1_700_000_000_007);
        capture.set_service_scope_entered_at_ms(1_700_000_000_009);
        capture.set_method_entered_at_ms(1_700_000_000_012);
        capture.set_handler_entered_at_ms(1_700_000_000_020);
        capture.set_response_sent_at_ms(1_700_000_000_040);

        let trace = capture.into_trace(
            "trace-service-future-first-poll".to_string(),
            std::time::Duration::from_millis(50),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(
            details.service_future_created_at_ms,
            Some(1_699_999_999_995)
        );
        assert_eq!(
            details.service_future_first_poll_entered_at_ms,
            Some(1_700_000_000_000)
        );
        assert_eq!(details.service_future_to_first_poll_wait_ms, Some(5));
        assert_eq!(
            details.service_future_first_poll_outcome.as_deref(),
            Some("pending")
        );
        assert_eq!(
            details.service_future_first_wake_scheduled_at_ms,
            Some(1_700_000_000_007)
        );
        assert_eq!(details.first_poll_to_first_wake_wait_ms, Some(7));
        assert_eq!(details.service_future_to_scope_wait_ms, Some(14));
    }

    #[test]
    fn server_edge_details_do_not_fabricate_first_wake_split_when_first_poll_is_ready() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_990);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_future_created_at_ms(1_699_999_999_995);
        capture.set_service_future_first_poll_entered_at_ms(1_700_000_000_000);
        capture.set_service_future_first_poll_outcome("ready");
        capture.set_service_scope_entered_at_ms(1_700_000_000_000);
        capture.set_method_entered_at_ms(1_700_000_000_004);
        capture.set_handler_entered_at_ms(1_700_000_000_006);
        capture.set_response_sent_at_ms(1_700_000_000_015);

        let trace = capture.into_trace(
            "trace-service-future-first-poll-ready".to_string(),
            std::time::Duration::from_millis(25),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(
            details.service_future_first_poll_entered_at_ms,
            Some(1_700_000_000_000)
        );
        assert_eq!(details.service_future_to_first_poll_wait_ms, Some(5));
        assert_eq!(
            details.service_future_first_poll_outcome.as_deref(),
            Some("ready")
        );
        assert_eq!(details.service_future_first_wake_scheduled_at_ms, None);
        assert_eq!(details.first_poll_to_first_wake_wait_ms, None);
    }

    #[test]
    fn server_edge_details_keep_first_cancel_observation_and_derive_late_cancel_delta() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_995);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_future_created_at_ms(1_699_999_999_995);
        capture.set_service_scope_entered_at_ms(1_699_999_999_996);
        capture.set_method_entered_at_ms(1_699_999_999_998);
        capture.set_handler_entered_at_ms(1_700_000_000_000);
        capture.observe_cancel_at_ms(1_700_000_000_012);
        capture.observe_cancel_at_ms(1_700_000_000_018);
        capture.set_response_sent_at_ms(1_700_000_000_030);

        let trace = capture.into_trace(
            "trace-cancel-edge".to_string(),
            std::time::Duration::from_millis(30),
            "cancelled",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(details.transport_to_service_future_wait_ms, Some(0));
        assert_eq!(details.service_future_to_scope_wait_ms, Some(1));
        assert_eq!(details.transport_to_service_scope_wait_ms, Some(1));
        assert_eq!(details.service_scope_to_method_wait_ms, Some(2));
        assert_eq!(details.transport_to_method_wait_ms, Some(3));
        assert_eq!(details.method_prelude_exec_ms, Some(2));
        assert_eq!(details.transport_to_handler_wait_ms, 5);
        assert_eq!(details.server_handler_exec_ms, 30);
        assert_eq!(details.cancel_observed_at_ms, Some(1_700_000_000_012));
        assert_eq!(details.cancel_observed_after_handler_enter_ms, Some(12));
    }

    #[test]
    fn server_edge_details_do_not_fabricate_service_future_split_when_timestamp_is_absent() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_995);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_scope_entered_at_ms(1_699_999_999_996);
        capture.set_method_entered_at_ms(1_699_999_999_998);
        capture.set_handler_entered_at_ms(1_700_000_000_000);
        capture.set_response_sent_at_ms(1_700_000_000_030);

        let trace = capture.into_trace(
            "trace-no-service-future-split".to_string(),
            std::time::Duration::from_millis(30),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(details.service_future_created_at_ms, None);
        assert_eq!(details.transport_to_service_future_wait_ms, None);
        assert_eq!(details.service_future_to_scope_wait_ms, None);
    }

    #[test]
    fn server_edge_details_include_pre_method_attribution_provenance() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_995);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_scope_entered_at_ms(1_699_999_999_996);
        capture.set_method_entered_at_ms(1_699_999_999_998);
        capture.set_handler_entered_at_ms(1_700_000_000_000);
        capture.set_response_sent_at_ms(1_700_000_000_030);
        capture.set_pre_method_attribution_provenance("same_request_authoritative");

        let trace = capture.into_trace(
            "trace-pre-method-provenance".to_string(),
            std::time::Duration::from_millis(30),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(
            details.pre_method_attribution_provenance,
            "same_request_authoritative"
        );
    }

    #[test]
    fn server_edge_details_use_outer_dispatch_timestamp_as_transport_anchor_when_available() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_990);
        capture.set_transport_received_at_ms_provenance("jsonrpc_dispatch_received");
        capture.set_jsonrpc_dispatch_received_at_ms(1_699_999_999_990);
        capture.set_request_context_call_entered_at_ms(1_699_999_999_992);
        capture.set_service_future_created_at_ms(1_699_999_999_995);
        capture.set_service_scope_entered_at_ms(1_699_999_999_999);
        capture.set_method_entered_at_ms(1_700_000_000_004);
        capture.set_handler_entered_at_ms(1_700_000_000_006);
        capture.set_response_sent_at_ms(1_700_000_000_020);

        let trace = capture.into_trace(
            "trace-jsonrpc-dispatch-edge".to_string(),
            std::time::Duration::from_millis(30),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(details.transport_received_at_ms, 1_699_999_999_990);
        assert_eq!(
            details.transport_received_at_ms_provenance,
            "jsonrpc_dispatch_received"
        );
        assert_eq!(
            details.jsonrpc_dispatch_received_at_ms,
            Some(1_699_999_999_990)
        );
        assert_eq!(details.dispatch_to_request_context_wait_ms, Some(2));
        assert_eq!(details.transport_to_service_future_wait_ms, Some(5));
        assert_eq!(details.service_future_to_scope_wait_ms, Some(4));
        assert_eq!(details.transport_to_service_scope_wait_ms, Some(9));
        assert_eq!(details.transport_to_method_wait_ms, Some(14));
    }

    #[test]
    fn pre_method_attribution_provenance_stays_fail_closed_for_overlapping_completion() {
        let uri = Url::parse("file:///request_context_overlap_provenance.bsl").expect("url");
        let position = tower_lsp::lsp_types::Position::new(8, 4);
        request_context::record_completion_request_id_for_testing(&uri, position, "req-1");
        request_context::record_completion_request_id_for_testing(&uri, position, "req-2");

        let second_request_context =
            request_context::take_completion_request_context_by_request_id("req-2")
                .expect("second request context");
        let first_request_context =
            request_context::take_completion_request_context(&uri, position)
                .expect("first request context");

        assert_eq!(
            derive_pre_method_attribution_provenance(
                Some("req-2"),
                Some(1_700_000_000_020),
                Some(&second_request_context),
                None,
            ),
            "same_request_authoritative"
        );
        assert_eq!(
            derive_pre_method_attribution_provenance(
                None,
                None,
                None,
                Some(&first_request_context)
            ),
            "best_effort_fallback"
        );
        assert_eq!(
            derive_pre_method_attribution_provenance(None, None, None, None),
            "unavailable"
        );

        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_700_000_000_100);
        capture.set_transport_received_at_ms_provenance("request_context_call_entry");
        capture.set_service_scope_entered_at_ms(1_700_000_000_104);
        capture.set_method_entered_at_ms(1_700_000_000_140);
        capture.set_handler_entered_at_ms(1_700_000_000_141);
        capture.set_response_sent_at_ms(1_700_000_000_220);
        capture.set_pre_method_attribution_provenance("best_effort_fallback");

        let trace = capture.into_trace(
            "trace-overlap-provenance".to_string(),
            std::time::Duration::from_millis(120),
            "ok_non_empty",
        );
        let details = trace
            .server_edge_details
            .expect("server_edge_details must be present");
        assert_eq!(
            details.pre_method_attribution_provenance,
            "best_effort_fallback"
        );
    }

    #[test]
    fn prepare_runtime_drilldown_is_serialised_into_trace() {
        let mut capture = sample_capture();
        capture.set_prepare_kind("lightweight_current_revision");
        capture.set_prepare_wait_for_file_version_runtime(Some(
            bsl_runtime::application::WaitForFileVersionRuntimeTrace {
                queue_wait_elapsed: Some(std::time::Duration::from_millis(11)),
                exec_elapsed: Some(std::time::Duration::from_millis(2)),
                wake_wait_elapsed: Some(std::time::Duration::from_millis(97)),
                resolution: Some(
                    bsl_runtime::application::WaitForFileVersionResolutionKind::Waiter,
                ),
            },
        ));
        capture.set_prepare_snapshot_with_deps_runtime(
            bsl_runtime::application::SnapshotWithDepsRuntimeTrace {
                queue_wait_elapsed: Some(std::time::Duration::from_millis(5)),
                exec_elapsed: Some(std::time::Duration::from_millis(7)),
            },
        );

        let trace = capture.into_trace(
            "trace-prepare-runtime".to_string(),
            std::time::Duration::from_millis(25),
            "ok_non_empty",
        );
        let prepare_details = trace
            .prepare_details
            .expect("prepare_details must be present");
        assert_eq!(
            prepare_details.kind.as_deref(),
            Some("lightweight_current_revision")
        );
        let wait_runtime = prepare_details
            .wait_for_file_version_runtime
            .expect("wait runtime must be present");
        assert_eq!(wait_runtime.queue_wait_ms, Some(11));
        assert_eq!(wait_runtime.exec_ms, Some(2));
        assert_eq!(wait_runtime.wake_wait_ms, Some(97));
        assert_eq!(wait_runtime.resolution.as_deref(), Some("waiter"));
        let snapshot_runtime = prepare_details
            .snapshot_with_deps_runtime
            .expect("snapshot runtime must be present");
        assert_eq!(snapshot_runtime.queue_wait_ms, Some(5));
        assert_eq!(snapshot_runtime.exec_ms, Some(7));
        assert_eq!(snapshot_runtime.wake_wait_ms, None);
        assert_eq!(snapshot_runtime.resolution, None);
    }

    #[test]
    fn snapshot_timeout_runtime_is_serialised_into_trace() {
        let mut capture = sample_capture();
        capture.set_prepare_snapshot_with_deps_timeout_runtime(Some(
            bsl_runtime::application::SnapshotWithDepsTimeoutRuntimeTrace {
                queue_wait_elapsed: Some(std::time::Duration::from_millis(19)),
                exec_elapsed: Some(std::time::Duration::from_millis(87)),
                wake_wait_elapsed: Some(std::time::Duration::from_millis(401)),
                resolution:
                    bsl_runtime::application::SnapshotWithDepsTimeoutResolutionKind::WakeWait,
            },
        ));

        let trace = capture.into_trace(
            "trace-snapshot-timeout-runtime".to_string(),
            std::time::Duration::from_millis(507),
            "fail_closed",
        );
        let snapshot_timeout_runtime = trace
            .prepare_details
            .and_then(|prepare| prepare.snapshot_with_deps_timeout_runtime)
            .expect("snapshot_with_deps_timeout_runtime must be present");
        assert_eq!(snapshot_timeout_runtime.queue_wait_ms, Some(19));
        assert_eq!(snapshot_timeout_runtime.exec_ms, Some(87));
        assert_eq!(snapshot_timeout_runtime.wake_wait_ms, Some(401));
        assert_eq!(
            snapshot_timeout_runtime.resolution.as_deref(),
            Some("wake_wait")
        );
    }

    #[test]
    fn exact_wait_task_state_drilldown_is_serialised_into_trace() {
        let mut capture = sample_capture();
        capture.set_exact_wait_type_index_outcome("deadline");
        capture.set_exact_wait_type_index_waiter_action("promoted");
        capture.set_exact_wait_matching_task_state("matching");
        capture.set_exact_wait_task_phase("waiting_cpu_permit");

        let trace = capture.into_trace(
            "trace-exact-wait".to_string(),
            std::time::Duration::from_millis(25),
            "fail_closed",
        );
        let exact_wait = trace
            .prepare_details
            .and_then(|prepare| prepare.exact_wait)
            .expect("exact_wait must be present");
        assert_eq!(
            exact_wait.type_index_wait_outcome.as_deref(),
            Some("deadline")
        );
        assert_eq!(
            exact_wait.type_index_waiter_action.as_deref(),
            Some("promoted")
        );
        assert_eq!(exact_wait.matching_task_state.as_deref(), Some("matching"));
        assert_eq!(exact_wait.task_phase.as_deref(), Some("waiting_cpu_permit"));
    }

    #[test]
    fn prepare_timeout_attribution_is_serialised_into_trace() {
        let mut capture = sample_capture();
        capture.set_prepare_timeout_attribution(
            bsl_runtime::application::PrepareTimeoutAttributionTrace::new(
                bsl_runtime::application::PrepareTimeoutSourceKind::PrepareGuard,
                "wait_for_file_version",
                std::time::Duration::from_millis(120),
                std::time::Duration::from_millis(2986),
            ),
        );

        let trace = capture.into_trace(
            "trace-prepare-timeout".to_string(),
            std::time::Duration::from_millis(2986),
            "fail_closed",
        );
        let timeout_attribution = trace
            .prepare_details
            .and_then(|prepare| prepare.timeout_attribution)
            .expect("timeout_attribution must be present");
        assert_eq!(timeout_attribution.source, "prepare_guard");
        assert_eq!(timeout_attribution.phase, "wait_for_file_version");
        assert_eq!(timeout_attribution.budget_ms, 120);
        assert_eq!(timeout_attribution.elapsed_ms, 2986);
        assert_eq!(timeout_attribution.overshoot_ms, 2866);
    }

    #[test]
    fn exact_wait_artifact_poll_is_serialised_into_trace() {
        let mut capture = sample_capture();
        capture.set_exact_wait_artifact_poll(crate::server::core::CompletionArtifactPollTraceV2 {
            poll_count: 14,
            poll_elapsed: std::time::Duration::from_millis(155),
            observed_file_version: Some(9),
            head_ready: Some(false),
            exact_ready: Some(false),
        });

        let trace = capture.into_trace(
            "trace-artifact-poll".to_string(),
            std::time::Duration::from_millis(155),
            "fail_closed",
        );
        let artifact_poll = trace
            .prepare_details
            .and_then(|prepare| prepare.exact_wait)
            .and_then(|exact_wait| exact_wait.artifact_poll)
            .expect("artifact_poll must be present");
        assert_eq!(artifact_poll.poll_count, 14);
        assert_eq!(artifact_poll.poll_elapsed_ms, 155);
        assert_eq!(artifact_poll.observed_file_version, Some(9));
        assert_eq!(artifact_poll.head_ready, Some(false));
        assert_eq!(artifact_poll.exact_ready, Some(false));
    }

    #[test]
    fn public_timeline_outcome_collapses_legacy_fail_closed_labels() {
        for outcome in [
            "wait_not_ready",
            "missing_file_content",
            "missing_file_path",
            "missing_deps",
            "missing_ir",
            "fallback_unavailable",
            "queue_rejected",
        ] {
            assert_eq!(completion_public_timeline_outcome(outcome), "fail_closed");
        }
    }

    #[tokio::test]
    async fn prepare_guard_times_out_pending_prepare() {
        let guarded = run_completion_prepare_guard(
            pending::<()>(),
            Some(std::time::Duration::from_millis(20)),
            Option::<std::future::Ready<&'static str>>::None,
        )
        .await;

        assert!(matches!(guarded, CompletionPrepareGuardResult::TimedOut));
        assert_eq!(guarded.trace_outcome(), "timeout");
    }

    #[tokio::test]
    async fn prepare_guard_returns_abort_outcome_before_timeout() {
        let (tx, rx) = tokio::sync::oneshot::channel::<&'static str>();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let _ = tx.send("cancelled");
        });

        let guarded = run_completion_prepare_guard(
            pending::<()>(),
            Some(std::time::Duration::from_secs(1)),
            Some(async move { rx.await.expect("abort outcome") }),
        )
        .await;

        assert!(matches!(
            guarded,
            CompletionPrepareGuardResult::Aborted("cancelled")
        ));
        assert_eq!(guarded.trace_outcome(), "aborted:cancelled");
    }

    #[tokio::test]
    async fn prepare_guard_marks_completed_prepare_branch() {
        let guarded = run_completion_prepare_guard(
            async { 42_u32 },
            Some(std::time::Duration::from_secs(1)),
            Option::<std::future::Ready<&'static str>>::None,
        )
        .await;

        match guarded {
            CompletionPrepareGuardResult::Prepared(value) => assert_eq!(value, 42),
            other => panic!("expected prepared branch, got {other:?}"),
        }
        assert_eq!(guarded.trace_outcome(), "prepared");
    }
}
