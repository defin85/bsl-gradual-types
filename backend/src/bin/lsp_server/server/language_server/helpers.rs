use super::*;

pub(super) fn effective_include_flow_sensitive(
    request_override: Option<bool>,
    enable_flow_sensitive_setting: bool,
) -> bool {
    request_override.unwrap_or(enable_flow_sensitive_setting)
}

pub(super) fn lsp_fail_closed_reason_from_prepare_outcome(
    outcome: bsl_runtime::application::SemanticOutcome,
) -> &'static str {
    match outcome {
        bsl_runtime::application::SemanticOutcome::StaleVersion => "superseded_revision",
        bsl_runtime::application::SemanticOutcome::Cancelled => "cancelled",
        bsl_runtime::application::SemanticOutcome::MissingDeps => "unavailable_by_contract",
        _ => "missing_canonical_ir",
    }
}

pub(super) fn record_lsp_interactive_fail_closed_reason(
    coordinator: &bsl_runtime::system::SystemCoordinator,
    operation: &str,
    reason: &str,
) {
    coordinator.record_intellisense_v2_interactive_fail_closed_reason("lsp", operation, reason);
}

#[derive(Debug, Clone, Default)]
pub(super) struct RequestServerEdgeTraceInputs {
    pub adapter_read_at_ms: Option<u64>,
    pub transport_received_at_ms: Option<u64>,
    pub transport_received_at_ms_provenance: Option<String>,
    pub jsonrpc_dispatch_received_at_ms: Option<u64>,
    pub transport_slot_released_at_ms: Option<u64>,
    pub request_context_call_entered_at_ms: Option<u64>,
    pub pre_method_attribution_provenance: Option<String>,
    pub service_future_created_at_ms: Option<u64>,
    pub service_future_first_poll_entered_at_ms: Option<u64>,
    pub service_future_first_poll_outcome: Option<String>,
    pub service_future_first_wake_scheduled_at_ms: Option<u64>,
    pub first_poll_contention_attribution:
        Option<crate::types::CompletionTimelineFirstPollContentionAttributionTrace>,
    pub first_poll_contention_contenders:
        Option<Vec<crate::types::CompletionTimelineFirstPollContentionContenderTrace>>,
    pub service_scope_entered_at_ms: Option<u64>,
    pub method_entered_at_ms: Option<u64>,
    pub handler_entered_at_ms: Option<u64>,
    pub response_sent_at_ms: Option<u64>,
    pub response_output_enqueue_completed_at_ms: Option<u64>,
    pub response_output_write_started_at_ms: Option<u64>,
    pub response_output_encode_completed_at_ms: Option<u64>,
    pub response_flush_completed_at_ms: Option<u64>,
    pub cancel_observed_at_ms: Option<u64>,
}

pub(super) fn build_server_edge_details_trace(
    inputs: &RequestServerEdgeTraceInputs,
) -> Option<crate::types::CompletionTimelineServerEdgeDetailsTrace> {
    let adapter_read_at_ms = inputs.adapter_read_at_ms;
    let transport_received_at_ms = inputs.transport_received_at_ms?;
    let transport_received_at_ms_provenance = inputs
        .transport_received_at_ms_provenance
        .clone()
        .unwrap_or_else(|| "request_context_call_entry".to_string());
    let jsonrpc_dispatch_received_at_ms = inputs.jsonrpc_dispatch_received_at_ms;
    let transport_slot_released_at_ms = inputs.transport_slot_released_at_ms;
    let request_context_call_entered_at_ms = inputs.request_context_call_entered_at_ms;
    let service_future_created_at_ms = inputs.service_future_created_at_ms;
    let service_future_first_poll_entered_at_ms = inputs.service_future_first_poll_entered_at_ms;
    let service_future_first_poll_outcome = inputs.service_future_first_poll_outcome.clone();
    let service_future_first_wake_scheduled_at_ms =
        inputs.service_future_first_wake_scheduled_at_ms;
    let first_poll_contention_attribution = inputs.first_poll_contention_attribution.clone();
    let first_poll_contention_contenders = inputs.first_poll_contention_contenders.clone();
    let service_scope_entered_at_ms = inputs.service_scope_entered_at_ms;
    let method_entered_at_ms = inputs.method_entered_at_ms;
    let handler_entered_at_ms = inputs.handler_entered_at_ms?;
    let response_sent_at_ms = inputs.response_sent_at_ms?;
    let response_output_enqueue_completed_at_ms = inputs.response_output_enqueue_completed_at_ms;
    let response_output_write_started_at_ms = inputs.response_output_write_started_at_ms;
    let response_output_encode_completed_at_ms = inputs.response_output_encode_completed_at_ms;
    let response_flush_completed_at_ms = inputs.response_flush_completed_at_ms;
    let cancel_observed_at_ms = inputs.cancel_observed_at_ms;

    Some(crate::types::CompletionTimelineServerEdgeDetailsTrace {
        adapter_read_at_ms,
        transport_received_at_ms,
        transport_received_at_ms_provenance,
        jsonrpc_dispatch_received_at_ms,
        transport_slot_released_at_ms,
        pre_method_attribution_provenance: inputs
            .pre_method_attribution_provenance
            .clone()
            .unwrap_or_else(|| "unavailable".to_string()),
        service_future_created_at_ms,
        service_future_first_poll_entered_at_ms,
        service_future_first_poll_outcome,
        service_future_first_wake_scheduled_at_ms,
        first_poll_contention_attribution,
        first_poll_contention_contenders,
        service_scope_entered_at_ms,
        method_entered_at_ms,
        handler_entered_at_ms,
        response_sent_at_ms,
        response_output_enqueue_completed_at_ms,
        response_output_write_started_at_ms,
        response_output_encode_completed_at_ms,
        response_flush_completed_at_ms,
        cancel_observed_at_ms,
        dispatch_to_request_context_wait_ms: jsonrpc_dispatch_received_at_ms
            .zip(request_context_call_entered_at_ms)
            .map(|(dispatch_ms, request_context_ms)| {
                request_context_ms.saturating_sub(dispatch_ms)
            }),
        adapter_to_dispatch_wait_ms: adapter_read_at_ms
            .zip(jsonrpc_dispatch_received_at_ms)
            .map(|(adapter_read_ms, dispatch_ms)| dispatch_ms.saturating_sub(adapter_read_ms)),
        transport_to_slot_release_wait_ms: transport_slot_released_at_ms
            .map(|slot_release_ms| slot_release_ms.saturating_sub(transport_received_at_ms)),
        transport_to_service_future_wait_ms: service_future_created_at_ms
            .map(|service_future_ms| service_future_ms.saturating_sub(transport_received_at_ms)),
        service_future_to_scope_wait_ms: service_future_created_at_ms
            .zip(service_scope_entered_at_ms)
            .map(|(service_future_ms, service_scope_ms)| {
                service_scope_ms.saturating_sub(service_future_ms)
            }),
        service_future_to_first_poll_wait_ms: service_future_created_at_ms
            .zip(service_future_first_poll_entered_at_ms)
            .map(|(service_future_ms, first_poll_ms)| {
                first_poll_ms.saturating_sub(service_future_ms)
            }),
        first_poll_to_first_wake_wait_ms: service_future_first_poll_entered_at_ms
            .zip(service_future_first_wake_scheduled_at_ms)
            .map(|(first_poll_ms, first_wake_ms)| first_wake_ms.saturating_sub(first_poll_ms)),
        transport_to_service_scope_wait_ms: service_scope_entered_at_ms
            .map(|service_scope_ms| service_scope_ms.saturating_sub(transport_received_at_ms)),
        service_scope_to_method_wait_ms: service_scope_entered_at_ms
            .zip(method_entered_at_ms)
            .map(|(service_scope_ms, method_ms)| method_ms.saturating_sub(service_scope_ms)),
        transport_to_method_wait_ms: method_entered_at_ms
            .map(|method_ms| method_ms.saturating_sub(transport_received_at_ms)),
        method_prelude_exec_ms: method_entered_at_ms
            .map(|method_ms| handler_entered_at_ms.saturating_sub(method_ms)),
        slot_release_to_handler_wait_ms: transport_slot_released_at_ms
            .map(|slot_release_ms| handler_entered_at_ms.saturating_sub(slot_release_ms)),
        slot_release_to_response_wait_ms: transport_slot_released_at_ms
            .map(|slot_release_ms| response_sent_at_ms.saturating_sub(slot_release_ms)),
        transport_to_handler_wait_ms: handler_entered_at_ms
            .saturating_sub(transport_received_at_ms),
        server_handler_exec_ms: response_sent_at_ms.saturating_sub(handler_entered_at_ms),
        response_ready_to_output_enqueue_wait_ms: response_output_enqueue_completed_at_ms
            .map(|enqueue_completed_at_ms| {
                enqueue_completed_at_ms.saturating_sub(response_sent_at_ms)
            }),
        response_output_queue_wait_ms: response_output_enqueue_completed_at_ms
            .zip(response_output_write_started_at_ms)
            .map(|(enqueue_completed_at_ms, write_started_at_ms)| {
                write_started_at_ms.saturating_sub(enqueue_completed_at_ms)
            }),
        response_output_encode_exec_ms: response_output_write_started_at_ms
            .zip(response_output_encode_completed_at_ms)
            .map(|(write_started_at_ms, encode_completed_at_ms)| {
                encode_completed_at_ms.saturating_sub(write_started_at_ms)
            }),
        response_output_write_and_flush_exec_ms: response_output_encode_completed_at_ms
            .zip(response_flush_completed_at_ms)
            .map(|(encode_completed_at_ms, flush_completed_at_ms)| {
                flush_completed_at_ms.saturating_sub(encode_completed_at_ms)
            }),
        response_ready_to_flush_wait_ms: response_flush_completed_at_ms
            .map(|flush_completed_at_ms| flush_completed_at_ms.saturating_sub(response_sent_at_ms)),
        cancel_observed_after_handler_enter_ms: cancel_observed_at_ms
            .map(|cancel_ms| cancel_ms.saturating_sub(handler_entered_at_ms)),
    })
}

#[cfg(test)]
pub(super) fn record_current_request_server_edge_trace_for_testing(
    method: &str,
    uri: &Url,
    method_entered_at_ms: u64,
    handler_entered_at_ms: u64,
    response_sent_at_ms: u64,
) {
    let request_id = super::super::request_context::current_request_id();
    let request_context_call_entered_at_ms =
        super::super::request_context::current_request_received_at_ms();
    let inputs = RequestServerEdgeTraceInputs {
        adapter_read_at_ms: None,
        transport_received_at_ms: request_context_call_entered_at_ms,
        transport_received_at_ms_provenance: request_context_call_entered_at_ms
            .map(|_| "request_context_call_entry".to_string()),
        jsonrpc_dispatch_received_at_ms:
            super::super::request_context::current_request_jsonrpc_dispatch_received_at_ms(),
        transport_slot_released_at_ms: None,
        request_context_call_entered_at_ms,
        pre_method_attribution_provenance: Some(
            if request_id.is_some() && request_context_call_entered_at_ms.is_some() {
                "same_request_authoritative".to_string()
            } else {
                "unavailable".to_string()
            },
        ),
        service_future_created_at_ms:
            super::super::request_context::current_request_service_future_created_at_ms(),
        service_future_first_poll_entered_at_ms:
            super::super::request_context::current_request_service_future_first_poll_entered_at_ms(),
        service_future_first_poll_outcome:
            super::super::request_context::current_request_service_future_first_poll_outcome(),
        service_future_first_wake_scheduled_at_ms:
            super::super::request_context::current_request_service_future_first_wake_scheduled_at_ms(
            ),
        first_poll_contention_attribution: super::super::request_context::
            current_request_service_future_first_poll_contention_attribution(),
        first_poll_contention_contenders: super::super::request_context::
            current_request_service_future_first_poll_contention_contenders(),
        service_scope_entered_at_ms:
            super::super::request_context::current_request_service_scope_entered_at_ms(),
        method_entered_at_ms: Some(method_entered_at_ms),
        handler_entered_at_ms: Some(handler_entered_at_ms),
        response_sent_at_ms: Some(response_sent_at_ms),
        response_output_enqueue_completed_at_ms: None,
        response_output_write_started_at_ms: None,
        response_output_encode_completed_at_ms: None,
        response_flush_completed_at_ms: None,
        cancel_observed_at_ms: None,
    };
    let Some(server_edge_details) = build_server_edge_details_trace(&inputs) else {
        return;
    };
    super::super::request_context::record_request_server_edge_trace_for_testing(
        request_id.as_deref(),
        method,
        uri,
        server_edge_details,
    );
}

pub(super) fn should_schedule_profile(
    trigger: bsl_runtime::application::DiagnosticsTrigger,
    profile: bsl_runtime::application::DiagnosticsProfile,
    flow_sensitive_enabled: bool,
) -> bool {
    if matches!(
        profile,
        bsl_runtime::application::DiagnosticsProfile::IdleHeavy
    ) && !flow_sensitive_enabled
    {
        return matches!(
            trigger,
            bsl_runtime::application::DiagnosticsTrigger::DidSave
                | bsl_runtime::application::DiagnosticsTrigger::Idle
        );
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LargeChurnTransition {
    None,
    Entered,
    Exited,
}

pub(super) fn should_defer_heavy_diagnostics_for_large_churn(
    trigger: bsl_runtime::application::DiagnosticsTrigger,
    profile: bsl_runtime::application::DiagnosticsProfile,
    large_churn_active: bool,
) -> bool {
    large_churn_active
        && matches!(
            trigger,
            bsl_runtime::application::DiagnosticsTrigger::DidChange
        )
        && !matches!(profile, bsl_runtime::application::DiagnosticsProfile::Fast)
}

pub(super) fn lsp_range_change_to_parser_edit(
    change: &TextDocumentContentChangeEvent,
) -> Option<bsl_runtime::system::parser_coordinator::TextEdit> {
    let range = change.range?;
    Some(bsl_runtime::system::parser_coordinator::TextEdit {
        start_line: range.start.line,
        start_utf16_column: range.start.character,
        old_end_line: range.end.line,
        old_end_utf16_column: range.end.character,
        new_text: change.text.clone(),
    })
}

pub(super) fn unix_time_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(super) fn changed_range_footprint_bytes(
    range: &bsl_runtime::system::parser_coordinator::ParseChangedRange,
) -> usize {
    let old_span =
        usize::try_from(range.old_end_byte.saturating_sub(range.start_byte)).unwrap_or(0);
    let new_span =
        usize::try_from(range.new_end_byte.saturating_sub(range.start_byte)).unwrap_or(0);
    old_span.max(new_span)
}

pub(super) fn advance_large_churn_state(
    state: &mut super::super::ScaleAwareChurnStateV2,
    now: Instant,
    is_large_document: bool,
    knobs: bsl_runtime::application::ScaleAwareDiagnosticsKnobs,
) -> LargeChurnTransition {
    if now.duration_since(state.window_started_at) > knobs.churn_window {
        state.window_started_at = now;
        state.changes_in_window = 0;
    }

    state.changes_in_window = state.changes_in_window.saturating_add(1);
    let was_active = state.large_churn_active;
    let is_churn = state.changes_in_window >= knobs.churn_min_changes;
    state.large_churn_active = knobs.enabled && is_large_document && is_churn;

    match (was_active, state.large_churn_active) {
        (false, true) => LargeChurnTransition::Entered,
        (true, false) => LargeChurnTransition::Exited,
        _ => LargeChurnTransition::None,
    }
}

pub(super) fn completion_trigger_mode_label(context: Option<&CompletionContext>) -> &'static str {
    match context.map(|ctx| ctx.trigger_kind) {
        Some(CompletionTriggerKind::TRIGGER_CHARACTER) => "trigger_character",
        Some(CompletionTriggerKind::INVOKED) => "invoked",
        Some(CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS) => "trigger_for_incomplete",
        Some(_) => "other",
        None => "none",
    }
}

pub(super) const COMPLETION_SHADOW_INTERNAL_TRIGGER_MARKER: &str = "__bsl_shadow_internal__";

pub(super) fn completion_shadow_internal_trigger_payload(value: &str) -> Option<Option<char>> {
    let payload = value.strip_prefix(COMPLETION_SHADOW_INTERNAL_TRIGGER_MARKER)?;
    let payload = payload.strip_prefix(':')?;
    let codepoint = payload.parse::<u32>().ok()?;
    if codepoint == 0 {
        Some(None)
    } else {
        char::from_u32(codepoint).map(Some)
    }
}

pub(super) fn completion_shadow_internal_trigger_value(trigger_char_hint: Option<char>) -> String {
    format!(
        "{}:{}",
        COMPLETION_SHADOW_INTERNAL_TRIGGER_MARKER,
        trigger_char_hint.map(u32::from).unwrap_or(0),
    )
}

pub(super) fn completion_is_shadow_internal_request(context: Option<&CompletionContext>) -> bool {
    context
        .and_then(|ctx| ctx.trigger_character.as_deref())
        .is_some_and(|value| completion_shadow_internal_trigger_payload(value).is_some())
}

pub(super) fn completion_trigger_character(context: Option<&CompletionContext>) -> Option<char> {
    context
        .and_then(|ctx| ctx.trigger_character.as_deref())
        .and_then(|value| {
            completion_shadow_internal_trigger_payload(value)
                .unwrap_or_else(|| value.chars().next())
        })
}

pub(super) fn is_completion_identifier_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

pub(super) fn completion_request_targets_member_access(
    text: &str,
    position: Position,
    trigger_char_hint: Option<char>,
) -> bool {
    if trigger_char_hint == Some('.') {
        return true;
    }

    let Some(line_text) = text.lines().nth(position.line as usize) else {
        return false;
    };
    let column_index =
        bsl_backend::system::positioning::utf16_to_byte_offset(line_text, position.character);
    let line_prefix = line_text.get(..column_index).unwrap_or(line_text);
    let line_prefix = if line_text
        .get(column_index..)
        .and_then(|tail| tail.chars().next())
        == Some('.')
    {
        format!("{line_prefix}.")
    } else {
        line_prefix.to_string()
    };

    let trimmed = line_prefix.trim_end();
    let Some(dot_pos) = trimmed.rfind('.') else {
        return false;
    };
    let after_dot = trimmed[dot_pos + 1..].trim_start();
    after_dot.is_empty() || after_dot.chars().all(is_completion_identifier_char)
}

pub(super) fn completion_member_access_owner_type_hints_at_position(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    file_content: &str,
    position: Position,
    coordinator: Option<&bsl_runtime::system::SystemCoordinator>,
) -> Vec<bsl_shared::domain::types::TypeResolution> {
    let Some(_line_text) = file_content.lines().nth(position.line as usize) else {
        if let Some(coordinator) = coordinator {
            coordinator.record_intellisense_v2_completion_owner_hint_result("no_line");
        }
        return Vec::new();
    };
    let resolutions =
        bsl_runtime::application::completion_member_access_owner_type_hints_from_analysis(
            analysis,
            file_id,
            file_content,
            position.line,
            position.character,
        );

    if let Some(coordinator) = coordinator {
        coordinator.record_intellisense_v2_completion_owner_hint_lookup_path("direct");
    }

    if let Some(coordinator) = coordinator {
        coordinator.record_intellisense_v2_completion_owner_hint_result(
            if resolutions.is_empty() {
                "type_miss"
            } else {
                "type_hit"
            },
        );
    }

    resolutions
}

pub(super) fn completion_member_access_owner_type_hints_from_current_revision_head(
    analysis: &bsl_analysis_v2::AnalysisV2,
    file_id: bsl_analysis_v2::FileId,
    position: Position,
) -> Vec<bsl_shared::domain::types::TypeResolution> {
    let Some(file_content) = analysis.file_text(file_id).ok().flatten() else {
        return Vec::new();
    };

    bsl_runtime::application::completion_member_access_owner_type_hints_from_completion_head(
        analysis,
        file_id,
        file_content.as_ref(),
        position.line,
        position.character,
    )
}

pub(super) fn completion_labels_fingerprint(response: &CompletionResponse) -> Vec<String> {
    const PARITY_LABELS_LIMIT: usize = 64;

    let mut labels = BTreeSet::new();
    let push_label = |set: &mut BTreeSet<String>, label: &str| {
        if set.len() >= PARITY_LABELS_LIMIT {
            return;
        }
        let normalized = label.trim().to_lowercase();
        if normalized.is_empty() {
            return;
        }
        set.insert(normalized);
    };

    match response {
        CompletionResponse::List(list) => {
            for item in &list.items {
                push_label(&mut labels, &item.label);
            }
        }
        CompletionResponse::Array(items) => {
            for item in items {
                push_label(&mut labels, &item.label);
            }
        }
    }

    labels.into_iter().collect()
}

pub(super) fn completion_labels_overlap_ratio(lhs: &[String], rhs: &[String]) -> f64 {
    if lhs.is_empty() || rhs.is_empty() {
        return 0.0;
    }

    let left: BTreeSet<&str> = lhs.iter().map(String::as_str).collect();
    let right: BTreeSet<&str> = rhs.iter().map(String::as_str).collect();
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

pub(super) fn completion_parity_overlap_bucket(overlap_ratio: f64) -> &'static str {
    if overlap_ratio <= 0.0 {
        "none"
    } else if overlap_ratio < 0.3 {
        "low"
    } else {
        "high"
    }
}

pub(super) fn completion_publish_allowed(
    request_epoch: u64,
    latest_request_epoch: Option<u64>,
) -> bool {
    match latest_request_epoch {
        Some(latest_epoch) => latest_epoch == request_epoch,
        None => true,
    }
}

pub(super) fn completion_queue_enqueue_failed(
    outcome: super::super::completion_dispatcher::QueueEnqueueOutcome,
) -> bool {
    matches!(
        outcome,
        super::super::completion_dispatcher::QueueEnqueueOutcome::Full
            | super::super::completion_dispatcher::QueueEnqueueOutcome::Closed
    )
}

pub(super) fn completion_empty_response(
    is_incomplete: bool,
) -> crate::handlers::CompletionResponseWithStats {
    crate::handlers::CompletionResponseWithStats {
        response: CompletionResponse::List(CompletionList {
            is_incomplete,
            items: Vec::new(),
        }),
        stats: None,
        had_error: false,
    }
}

pub(super) fn completion_incomplete_empty_response() -> crate::handlers::CompletionResponseWithStats
{
    completion_empty_response(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompletionResponseRoute {
    Legacy,
    EventDriven,
}

impl CompletionResponseRoute {
    pub(super) fn event_driven_guards_enabled(self) -> bool {
        matches!(self, Self::EventDriven)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CompletionRoutingPlan {
    pub(super) response_route: CompletionResponseRoute,
    pub(super) run_shadow_event_driven: bool,
}

pub(super) fn completion_dispatch_enabled_for_mode(
    mode: bsl_runtime::application::CompletionMode,
) -> bool {
    !matches!(mode, bsl_runtime::application::CompletionMode::Off)
}

pub(super) fn completion_canary_routing_key(
    uri: &Url,
    position: Position,
    trigger_mode: &str,
    trigger_char_hint: Option<char>,
    version_hint: Option<i32>,
) -> String {
    let trigger_char_code = trigger_char_hint.map(u32::from).unwrap_or(0);
    format!(
        "{}:{}:{}:{}:{}:{}",
        uri,
        position.line,
        position.character,
        trigger_mode,
        trigger_char_code,
        version_hint.unwrap_or(i32::MIN),
    )
}

pub(super) fn completion_route_canary_event_driven(routing_key: &str, canary_percent: u8) -> bool {
    if canary_percent == 0 {
        return false;
    }
    if canary_percent >= 100 {
        return true;
    }
    (hash_content(routing_key) % 100) < u64::from(canary_percent)
}

pub(super) fn completion_routing_plan(
    mode: bsl_runtime::application::CompletionMode,
    canary_percent: u8,
    routing_key: &str,
) -> CompletionRoutingPlan {
    match mode {
        bsl_runtime::application::CompletionMode::Off => CompletionRoutingPlan {
            response_route: CompletionResponseRoute::Legacy,
            run_shadow_event_driven: false,
        },
        bsl_runtime::application::CompletionMode::Shadow => CompletionRoutingPlan {
            response_route: CompletionResponseRoute::Legacy,
            run_shadow_event_driven: true,
        },
        bsl_runtime::application::CompletionMode::Canary => CompletionRoutingPlan {
            response_route: if completion_route_canary_event_driven(routing_key, canary_percent) {
                CompletionResponseRoute::EventDriven
            } else {
                CompletionResponseRoute::Legacy
            },
            run_shadow_event_driven: false,
        },
        bsl_runtime::application::CompletionMode::On => CompletionRoutingPlan {
            response_route: CompletionResponseRoute::EventDriven,
            run_shadow_event_driven: false,
        },
    }
}

pub(super) fn completion_observability_mode_label(
    response_route: CompletionResponseRoute,
    shadow_internal_request: bool,
) -> &'static str {
    if shadow_internal_request {
        "shadow"
    } else if response_route.event_driven_guards_enabled() {
        "event_driven"
    } else {
        "legacy"
    }
}

pub(super) struct CompletionRequestDropCancelGuard {
    request_id: Option<String>,
    cancellation_registry:
        Arc<super::super::completion_cancellation::CompletionCancellationRegistry>,
    dispatcher: Arc<super::super::completion_dispatcher::CompletionDispatcherRegistry>,
    disarmed: bool,
}

impl CompletionRequestDropCancelGuard {
    pub(super) fn new(
        request_id: Option<String>,
        cancellation_registry: Arc<
            super::super::completion_cancellation::CompletionCancellationRegistry,
        >,
        dispatcher: Arc<super::super::completion_dispatcher::CompletionDispatcherRegistry>,
    ) -> Self {
        Self {
            request_id,
            cancellation_registry,
            dispatcher,
            disarmed: false,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for CompletionRequestDropCancelGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        let Some(request_id) = self.request_id.clone() else {
            return;
        };
        let Some(entry) = self.cancellation_registry.cancel_request(&request_id) else {
            return;
        };
        let dispatcher = Arc::clone(&self.dispatcher);
        tokio::spawn(async move {
            let _ = dispatcher
                .cancel_pre_active_completion(entry.file_id, entry.request_epoch)
                .await;
            let _ = dispatcher.emit_cancel(entry.file_id, request_id).await;
        });
    }
}

pub(super) struct CompletionActiveTurnGuard {
    file_id: bsl_analysis_v2::FileId,
    file_seq: u64,
    dispatcher: Arc<super::super::completion_dispatcher::CompletionDispatcherRegistry>,
}

impl CompletionActiveTurnGuard {
    pub(super) fn new(
        file_id: bsl_analysis_v2::FileId,
        file_seq: u64,
        dispatcher: Arc<super::super::completion_dispatcher::CompletionDispatcherRegistry>,
    ) -> Self {
        Self {
            file_id,
            file_seq,
            dispatcher,
        }
    }
}

impl Drop for CompletionActiveTurnGuard {
    fn drop(&mut self) {
        let dispatcher = Arc::clone(&self.dispatcher);
        let file_id = self.file_id;
        let file_seq = self.file_seq;
        tokio::spawn(async move {
            let _ = dispatcher.mark_completion_inactive(file_id, file_seq).await;
        });
    }
}

pub(super) async fn completion_checkpoint_outcome(
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    request_id: Option<&str>,
    request_epoch: u64,
    cancellation_token: Option<&super::super::completion_cancellation::CompletionCancellationToken>,
    checkpoint: &'static str,
    cancel_event_emitted: &mut bool,
) -> Option<&'static str> {
    if cancellation_token.is_some_and(|token| token.is_cancelled()) {
        if let Some(request_id) = request_id {
            if !*cancel_event_emitted {
                let cancel_ticket = server
                    .completion_dispatcher_v2
                    .emit_cancel(file_id, request_id.to_string())
                    .await;
                *cancel_event_emitted = true;
                if completion_queue_enqueue_failed(cancel_ticket.queue_outcome) {
                    debug!(
                        file_id = file_id.0,
                        file_seq = cancel_ticket.file_seq,
                        request_epoch = cancel_ticket.request_epoch,
                        request_id = request_id,
                        queue_outcome = ?cancel_ticket.queue_outcome,
                        checkpoint,
                        "completion dispatcher dropped cancel checkpoint event"
                    );
                }
            }
        }
        return Some("cancelled");
    }

    let latest_request_epoch = server
        .completion_dispatcher_v2
        .latest_request_epoch(file_id)
        .await;
    if !completion_publish_allowed(request_epoch, latest_request_epoch) {
        return Some("superseded");
    }

    None
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn completion_checkpoint_outcome_if_enabled(
    event_driven_guards_enabled: bool,
    server: &BslLanguageServer,
    file_id: bsl_analysis_v2::FileId,
    request_id: Option<&str>,
    request_epoch: u64,
    cancellation_token: Option<&super::super::completion_cancellation::CompletionCancellationToken>,
    checkpoint: &'static str,
    cancel_event_emitted: &mut bool,
) -> Option<&'static str> {
    if !event_driven_guards_enabled {
        return None;
    }
    completion_checkpoint_outcome(
        server,
        file_id,
        request_id,
        request_epoch,
        cancellation_token,
        checkpoint,
        cancel_event_emitted,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn resolve_completion_without_ir(
    _server: &BslLanguageServer,
    _file_id: bsl_analysis_v2::FileId,
    _observed_deps_id: bsl_analysis_v2::DepsSnapshotId,
    _observed_settings_id: Option<bsl_analysis_v2::SettingsId>,
    _observed_file_version: Option<i32>,
    _member_access_context: bool,
    _file_content: Arc<str>,
    _file_path: Arc<str>,
    _member_access_owner_type_hints: Vec<bsl_shared::domain::types::TypeResolution>,
    _deps: Arc<bsl_analysis_v2::SemanticDeps>,
    _position: Position,
    _uri: &Url,
    _index_snapshot: &bsl_backend::system::IndexSnapshot,
    _snippet_support: bool,
    _include_flow_sensitive: bool,
    _trigger_char_hint: Option<char>,
) -> (
    &'static str,
    Option<crate::handlers::CompletionResponseWithStats>,
) {
    let decision =
        bsl_runtime::application::completion_missing_ir_policy_decision(false, false, false, false);

    match decision {
        bsl_runtime::application::CompletionMissingIrPolicyDecision::FailClosedUnavailable => {
            ("missing_ir", Some(completion_empty_response(false)))
        }
    }
}

pub(super) async fn resolve_cache_config_path(
    params: &ExecuteCommandParams,
    config: &tokio::sync::RwLock<Option<LspConfig>>,
) -> JsonRpcResult<String> {
    if !params.arguments.is_empty() {
        let request: CacheCommandParams = serde_json::from_value(params.arguments[0].clone())
            .map_err(|e| {
                tower_lsp::jsonrpc::Error::invalid_params(format!("Invalid parameters: {}", e))
            })?;
        if let Some(path) = request.configuration_path {
            return Ok(path);
        }
    }

    let config_guard = config.read().await;
    if let Some(cfg) = config_guard.as_ref() {
        if let Some(path) = cfg.configuration_path.clone() {
            return Ok(path);
        }
    }

    Err(tower_lsp::jsonrpc::Error::invalid_params(
        "Missing configuration path",
    ))
}

pub(super) fn normalize_lsp_config(config: &mut LspConfig) {
    config.platform_docs_archive = normalize_optional_string(config.platform_docs_archive.clone());
    config.configuration_path = normalize_optional_string(config.configuration_path.clone());
    config.platform_version = normalize_optional_string(config.platform_version.clone());
}

pub(super) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
