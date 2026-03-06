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

#[derive(Debug, Clone)]
struct CompletionTimelineCapture {
    request_id: Option<String>,
    uri: String,
    trigger_mode: String,
    started_at_ms: u64,
    timeline_cursor_ms: u64,
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
            timeline_cursor_ms: 0,
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
            "handler_error"
            | "queue_rejected"
            | "wait_not_ready"
            | "missing_deps"
            | "missing_file_content"
            | "missing_file_path" => CompletionTimelineStageStatus::Failed,
            "skipped" => CompletionTimelineStageStatus::Skipped,
            _ => CompletionTimelineStageStatus::Completed,
        };
        self.push_stage("terminal", status, std::time::Duration::from_millis(0));
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

        crate::types::CompletionTimelineTrace {
            trace_id,
            request_id: self.request_id,
            uri: self.uri,
            trigger_mode: self.trigger_mode,
            outcome: outcome.to_string(),
            started_at_ms: self.started_at_ms,
            total_duration_ms,
            dominant_stage,
            stages: self.stages,
        }
    }
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
        let mut timeline_capture = CompletionTimelineCapture::new(
            completion_request_id.clone(),
            &uri,
            trigger_mode,
            super::super::unix_timestamp_ms(),
        );
        let (
            completion_ticket,
            completion_turn_outcome,
            _completion_request_registration,
            completion_cancellation_token,
            mut completion_drop_guard,
        ) = if event_driven_guards_enabled {
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
            let completion_request_registration = completion_request_id.clone().map(|request_id| {
                self.completion_cancellation_registry_v2.register_request(
                    request_id,
                    file_id,
                    completion_ticket.request_epoch,
                )
            });
            let completion_cancellation_token = completion_request_registration
                .as_ref()
                .map(|registration| registration.token());
            let completion_drop_guard = Some(CompletionRequestDropCancelGuard::new(
                completion_request_id.clone(),
                Arc::clone(&self.completion_cancellation_registry_v2),
                Arc::clone(&self.completion_dispatcher_v2),
            ));
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
                    turn_outcome
                } else {
                    super::super::completion_dispatcher::CompletionTurnOutcome::QueueRejected
                };
            (
                completion_ticket,
                Some(completion_turn_outcome),
                completion_request_registration,
                completion_cancellation_token,
                completion_drop_guard,
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
            )
        };
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
            let extract_non_empty_items =
                |response: &crate::handlers::CompletionResponseWithStats| match &response.response {
                    CompletionResponse::List(list) if !list.items.is_empty() => {
                        Some(list.items.clone())
                    }
                    CompletionResponse::Array(items) if !items.is_empty() => Some(items.clone()),
                    _ => None,
                };
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

            let prepare_started = Instant::now();
            let prepared = self
                .prepare_lsp_stateful_operation_v2_with_completion_mode(
                    &uri,
                    file_id,
                    bsl_runtime::application::SemanticOperation::Completion,
                    include_flow_sensitive,
                    Some(completion_observability_mode),
                )
                .await;
            let prepare_elapsed = prepare_started.elapsed();
            self.coordinator
                .record_completion_stage_latency("prepare_stateful", prepare_elapsed);
            timeline_capture.push_completed_stage("prepare_stateful", prepare_elapsed);

            match prepared {
                Ok((context, prepared, expected_version)) => {
                    let force_incomplete_due_stale = prepared.stale_served;
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
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }

                    if prepared.completion_churn_fastpath_active
                        && prepared.wait_budget_exhausted
                        && prepared.stale_served
                    {
                        let observed_deps_id = prepared.snapshot.deps_id.clone();
                        let observed_settings_id = prepared.snapshot.analysis.settings_id().ok();
                        let observed_file_version = prepared
                            .snapshot
                            .analysis
                            .file_version(file_id)
                            .ok()
                            .flatten();
                        observed_file_version_for_completion = observed_file_version;

                        let (strict_stale_cached_items, relaxed_stale_cached_items) =
                            completion_cached_stale_items(
                                self,
                                file_id,
                                &observed_deps_id,
                                observed_settings_id.as_ref(),
                                observed_file_version,
                            )
                            .await;
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
                            completion_outcome = Some(outcome);
                            break 'completion_flow Some(completion_incomplete_empty_response());
                        }

                        if let Some(items) =
                            strict_stale_cached_items.or(relaxed_stale_cached_items)
                        {
                            completion_outcome.get_or_insert("degraded_incomplete");
                            if !shadow_internal_request {
                                spawn_completion_refresh_after_stale_fastpath(
                                    self.clone(),
                                    params.clone(),
                                    trigger_char_hint,
                                );
                            }
                            break 'completion_flow Some(completion_response_with_cached_items(
                                items,
                            ));
                        }

                        if !shadow_internal_request {
                            spawn_completion_refresh_after_stale_fastpath(
                                self.clone(),
                                params.clone(),
                                trigger_char_hint,
                            );
                        }
                        self.coordinator
                            .record_intellisense_v2_completion_fallback_unavailable();
                        completion_outcome.get_or_insert("fallback_unavailable");
                        break 'completion_flow Some(
                            crate::handlers::build_keyword_degraded_completion(snippet_support),
                        );
                    }

                    let query_bundle_started = Instant::now();
                    let (
                        file_content,
                        file_path,
                        parse_result,
                        member_access_owner_type_hint,
                        deps,
                        ir_program,
                        index_snapshot,
                        observed_deps_id,
                        observed_settings_id,
                        observed_file_version,
                    ) = {
                        let analysis = prepared.snapshot.analysis;
                        let index_snapshot = prepared.snapshot.index_snapshot;
                        let member_access_request_for_query = member_access_request;
                        let last_apply_enqueued_at = self
                            .latest_apply_enqueued_at_v2
                            .read()
                            .await
                            .get(&file_id)
                            .copied();
                        let apply_age_at_query_start_ms =
                            last_apply_enqueued_at.map(|started_at| {
                                query_bundle_started
                                    .saturating_duration_since(started_at)
                                    .as_millis()
                            });

                        let observed_file_version = analysis.file_version(file_id).ok().flatten();
                        let observed_deps_id = prepared.snapshot.deps_id;
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

                        let context_for_query = context.clone();
                        let coordinator_for_query = self.coordinator.clone();
                        let uri_for_query = uri.clone();
                        let observed_deps_id_for_query = observed_deps_id.clone();
                        let cancellation_token_for_query = completion_cancellation_token.clone();
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
                                            None,
                                            None,
                                            deps,
                                            None,
                                            false,
                                            true,
                                        );
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
                                            None,
                                            None,
                                            deps,
                                            ir_program,
                                            ir_cancelled_after_retry,
                                            true,
                                        );
                                    }
                                    // Strict serve-only completion path: do not run
                                    // parse_result query in interactive request flow.
                                    let parse_result: Option<Arc<bsl_syntax::ast::ParseResult>> =
                                        None;

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

                                    if bsl_runtime::system::global_runtime_config()
                                        .get_bool(
                                            bsl_runtime::system::RuntimeKey::IntellisenseV2P3Smoke,
                                        )
                                        .unwrap_or(false)
                                    {
                                        match parse_result.as_ref() {
                                            Some(parsed) => debug!(
                                                "Completion v2 parse_result: uri={}, file_id={}, syntax_errors={}",
                                                uri_for_query,
                                                file_id.0,
                                                parsed.syntax_errors.len()
                                            ),
                                            None => debug!(
                                                "Completion v2 parse_result: uri={}, file_id={} (unavailable)",
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
                                            parse_result,
                                            None,
                                            deps,
                                            ir_program,
                                            ir_cancelled_after_retry,
                                            true,
                                        );
                                    }

                                    let member_access_owner_type_hint =
                                        super::impl_completion_helpers::compute_member_access_owner_hint(
                                            &analysis,
                                            file_id,
                                            position,
                                            member_access_request_for_query,
                                            file_content.as_deref(),
                                            coordinator_for_query.as_ref(),
                                            apply_age_at_query_start_ms,
                                        );

                                    (
                                        file_content,
                                        file_path,
                                        parse_result,
                                        member_access_owner_type_hint,
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
                            parse_result,
                            member_access_owner_type_hint,
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
                                (None, None, None, None, None, None, true, true)
                            }
                        };
                        if (ir_cancelled_after_retry || query_checkpoint_cancelled)
                            && completion_outcome.is_none()
                        {
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
                            completion_outcome = Some(outcome);
                            break 'completion_flow Some(completion_incomplete_empty_response());
                        }

                        (
                            file_content,
                            file_path,
                            parse_result,
                            member_access_owner_type_hint,
                            deps,
                            ir_program,
                            index_snapshot,
                            observed_deps_id,
                            observed_settings_id,
                            observed_file_version,
                        )
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
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }

                    let response_build_started = Instant::now();
                    let mut completion_response = match (file_content, file_path, deps, ir_program)
                    {
                        (Some(file_content), Some(file_path), Some(deps), Some(ir_program)) => {
                            crate::handlers::handle_completion_v2_with_trigger_hint(
                                file_content,
                                file_path,
                                ir_program,
                                parse_result,
                                member_access_owner_type_hint,
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
                                parse_result,
                                member_access_owner_type_hint,
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
                        completion_outcome = Some(outcome);
                        break 'completion_flow Some(completion_incomplete_empty_response());
                    }
                    if force_incomplete_due_stale {
                        if let Some(response) = completion_response.as_mut() {
                            if let CompletionResponse::List(list) = &mut response.response {
                                list.is_incomplete = true;
                            }
                        }
                    }
                    if !matches!(completion_outcome, Some("cancelled" | "superseded")) {
                        let cache_store_started = Instant::now();
                        if let (Some(settings_id), Some(file_version), Some(response_items)) = (
                            observed_settings_id.clone(),
                            observed_file_version,
                            completion_response
                                .as_ref()
                                .and_then(extract_non_empty_items),
                        ) {
                            self.completion_stale_fallback_cache_v2
                                .write()
                                .await
                                .insert(
                                    file_id,
                                    CompletionStaleFallbackCacheEntryV2 {
                                        deps_id: observed_deps_id,
                                        settings_id,
                                        file_version,
                                        items: response_items,
                                    },
                                );
                        }
                        let cache_store_elapsed = cache_store_started.elapsed();
                        self.coordinator
                            .record_completion_stage_latency("cache_store", cache_store_elapsed);
                        timeline_capture.push_completed_stage("cache_store", cache_store_elapsed);
                    }
                    completion_response
                }
                Err(outcome) => {
                    completion_outcome = Some("wait_not_ready");
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

        let timeline_outcome = completion_outcome.unwrap_or("ok_empty");
        timeline_capture.push_terminal_stage(timeline_outcome);
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
        Ok(completion.map(|result| result.response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
