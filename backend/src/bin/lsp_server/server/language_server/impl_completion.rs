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

#[derive(Debug, Clone)]
struct CompletionTimelineCapture {
    request_id: Option<String>,
    uri: String,
    trigger_mode: String,
    started_at_ms: u64,
    transport_received_at_ms: Option<u64>,
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
    fn new(request_id: Option<String>, uri: &Url, trigger_mode: &str, started_at_ms: u64) -> Self {
        Self {
            request_id,
            uri: uri.to_string(),
            trigger_mode: trigger_mode.to_string(),
            started_at_ms,
            transport_received_at_ms: None,
            handler_entered_at_ms: Some(started_at_ms),
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

    #[cfg(test)]
    fn set_handler_entered_at_ms(&mut self, handler_entered_at_ms: u64) {
        self.handler_entered_at_ms = Some(handler_entered_at_ms);
        self.started_at_ms = handler_entered_at_ms;
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
        let handler_entered_at_ms = self.handler_entered_at_ms?;
        let response_sent_at_ms = self.response_sent_at_ms?;
        let cancel_observed_at_ms = self.cancel_observed_at_ms;
        Some(crate::types::CompletionTimelineServerEdgeDetailsTrace {
            transport_received_at_ms,
            handler_entered_at_ms,
            response_sent_at_ms,
            cancel_observed_at_ms,
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
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let trigger_mode = completion_trigger_mode_label(params.context.as_ref());
        let trigger_char_hint = completion_trigger_character(params.context.as_ref());
        let shadow_internal_request =
            completion_is_shadow_internal_request(params.context.as_ref());
        let completion_request_id = super::super::request_context::current_request_id()
            .or_else(|| super::super::request_context::take_completion_request_id(&uri, position));
        if !shadow_internal_request {
            self.coordinator
                .record_intellisense_v2_completion_trigger_mode(trigger_mode);
        }

        let file_id = self.get_or_create_file_id_v2(&uri).await;
        let version_hint = self
            .latest_received_file_versions_v2
            .read()
            .await
            .get(&file_id)
            .copied();
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
        let started = Instant::now();
        let handler_entered_at_ms = super::super::unix_timestamp_ms();
        let mut timeline_capture = CompletionTimelineCapture::new(
            completion_request_id.clone(),
            &uri,
            trigger_mode,
            handler_entered_at_ms,
        );
        timeline_capture.set_transport_received_at_ms(
            super::super::request_context::current_request_received_at_ms()
                .unwrap_or(handler_entered_at_ms),
        );
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
                    let turn_outcome = turn_waiter.wait().await;
                    let turn_wait_elapsed = turn_wait_started.elapsed();
                    self.coordinator
                        .record_completion_stage_latency("turn_wait", turn_wait_elapsed);
                    timeline_capture.push_completed_stage("turn_wait", turn_wait_elapsed);
                    completion_dispatch_attribution.turn_wait_outcome =
                        Some(turn_outcome.as_str().to_string());
                    turn_outcome
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
            let prepare_abort = event_driven_guards_enabled.then(|| {
                wait_for_completion_prepare_abort(
                    self,
                    file_id,
                    completion_request_id.as_deref(),
                    completion_ticket.request_epoch,
                    completion_cancellation_token.as_ref(),
                    &mut cancel_event_emitted,
                )
            });
            let guarded_prepare = run_completion_prepare_guard(
                self.prepare_lsp_stateful_operation_v2_with_completion_mode(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Completion,
                    include_flow_sensitive,
                    Some(completion_observability_mode),
                ),
                prepare_timeout,
                prepare_abort,
            )
            .await;
            let prepare_elapsed = prepare_started.elapsed();
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
                Ok((context, prepared, expected_version)) => {
                    timeline_capture.set_prepare_outcome("ready");
                    timeline_capture.set_prepare_observed_file_version(
                        prepared
                            .snapshot
                            .analysis
                            .file_version(file_id)
                            .ok()
                            .flatten(),
                    );
                    timeline_capture.set_prepare_wait_elapsed(prepared.wait_elapsed);
                    timeline_capture.set_prepare_snapshot_elapsed(prepared.snapshot_elapsed);
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
                        .snapshot
                        .analysis
                        .file_text(file_id)
                        .ok()
                        .flatten()
                        .map(|text| (text.len(), text.lines().count()))
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
                        && prepared
                            .snapshot
                            .analysis
                            .current_completion_head_ready(file_id)
                            .ok()
                            .unwrap_or(false);
                    let exact_ready_before_wait = prepared
                        .snapshot
                        .analysis
                        .current_type_index_serve_only_ready(file_id)
                        .ok()
                        .unwrap_or(false);
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
                        let exact_wait_elapsed = exact_wait_started.elapsed();
                        self.coordinator.record_completion_stage_latency(
                            "wait_exact_type_index",
                            exact_wait_elapsed,
                        );
                        timeline_capture
                            .push_completed_stage("wait_exact_type_index", exact_wait_elapsed);

                        match artifact_wait_outcome {
                            super::super::core::CompletionArtifactWaitOutcomeV2::HeadReady => {
                                head_ready = true;
                            }
                            super::super::core::CompletionArtifactWaitOutcomeV2::ExactReady => {
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
                                refreshed_snapshot_after_wait =
                                    Some(self.analysis_v2.snapshot_with_deps().await);
                            }
                            super::super::core::CompletionArtifactWaitOutcomeV2::Deadline
                            | super::super::core::CompletionArtifactWaitOutcomeV2::ObservedVersionMismatch => {
                                if let Some(apply_age) =
                                    completion_apply_age_for_file(self, file_id).await
                                {
                                    self.coordinator.record_completion_stage_latency(
                                        "exact_wait_apply_age_at_terminal",
                                        apply_age,
                                    );
                                }
                                let terminal_outcome = if matches!(
                                    artifact_wait_outcome,
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
                    let current_revision_head_owner_type_hints =
                        if member_access_request && head_ready {
                            completion_member_access_owner_type_hints_from_current_revision_head(
                                &prepared.snapshot.analysis,
                                file_id,
                                position,
                            )
                        } else {
                            Vec::new()
                        };
                    let head_route_candidate =
                        member_access_request && !current_revision_head_owner_type_hints.is_empty();
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
                        let exact_wait_outcome = self
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

                        if exact_wait_outcome
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
                                    exact_wait_outcome.as_str(),
                                );
                            if exact_wait_outcome
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

                        let (analysis_after_wait, index_snapshot_after_wait, deps_id_after_wait) =
                            self.analysis_v2.snapshot_with_deps().await;
                        let exact_ready_after_wait = analysis_after_wait
                            .current_type_index_serve_only_ready(file_id)
                            .ok()
                            .unwrap_or(false);
                        if !exact_ready_after_wait {
                            let terminal_outcome = if analysis_after_wait
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
                        refreshed_snapshot_after_wait = Some((
                            analysis_after_wait,
                            index_snapshot_after_wait,
                            deps_id_after_wait,
                        ));
                    }

                    let query_bundle_started = Instant::now();
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
                        let (analysis, index_snapshot, observed_deps_id) =
                            refreshed_snapshot_after_wait.unwrap_or((
                                prepared.snapshot.analysis,
                                prepared.index_snapshot,
                                prepared.snapshot.deps_id,
                            ));
                        let observed_file_version = analysis.file_version(file_id).ok().flatten();
                        let observed_settings_id = analysis.settings_id().ok();
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

                        if head_route_candidate {
                            let file_content = analysis.file_text(file_id).ok().flatten();
                            let file_path = analysis.file_path(file_id).ok().flatten();
                            let deps = analysis.deps_data().ok();
                            (
                                file_content,
                                file_path,
                                current_revision_head_owner_type_hints.clone(),
                                deps,
                                None,
                                index_snapshot,
                                observed_deps_id,
                                observed_settings_id,
                                observed_file_version,
                            )
                        } else {
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
                        std::time::Duration::from_millis(
                            cancel_observed_after_handler_enter_ms,
                        ),
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
    use std::future::pending;
    use tower_lsp::lsp_types::Url;

    fn sample_capture() -> CompletionTimelineCapture {
        let uri = Url::parse("file:///completion_timeline_capture_test.bsl").expect("test uri");
        CompletionTimelineCapture::new(Some("req-1".to_string()), &uri, "invoked", 1_700_000_000)
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
        assert_eq!(details.handler_entered_at_ms, 1_700_000_000_000);
        assert_eq!(details.response_sent_at_ms, 1_700_000_000_025);
        assert_eq!(details.transport_to_handler_wait_ms, 10);
        assert_eq!(details.server_handler_exec_ms, 25);
        assert_eq!(details.cancel_observed_at_ms, None);
        assert_eq!(details.cancel_observed_after_handler_enter_ms, None);
    }

    #[test]
    fn server_edge_details_keep_first_cancel_observation_and_derive_late_cancel_delta() {
        let mut capture = sample_capture();
        capture.set_transport_received_at_ms(1_699_999_999_995);
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
        assert_eq!(details.transport_to_handler_wait_ms, 5);
        assert_eq!(details.server_handler_exec_ms, 30);
        assert_eq!(details.cancel_observed_at_ms, Some(1_700_000_000_012));
        assert_eq!(details.cancel_observed_after_handler_enter_ms, Some(12));
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
