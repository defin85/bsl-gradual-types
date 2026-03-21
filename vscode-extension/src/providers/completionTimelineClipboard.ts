import {
    CompletionProbeFeedViewModel,
    CompletionProbeViewModel,
    CompletionTimelinePanelState,
    CompletionTimelineTraceViewModel,
    getAverageTraceProvenanceNotice,
} from './completionTimelineModel';
import {
    buildCompletionTraceBottleneckVerdicts,
    formatDispatcherAttributionTrace,
    formatExactArtifactPollTrace,
    formatExactWaitTrace,
    formatPrepareProgressTrace,
    formatPrepareRuntimeTrace,
    formatPrepareTimeoutAttributionTrace,
} from './completionTimelineDrilldown';

export type CompletionTimelineClipboardMode = 'all' | 'average';

export function formatVisibleCompletionTimelineForClipboard(
    state: CompletionTimelinePanelState,
    mode: CompletionTimelineClipboardMode
): string | null {
    const header = [
        'Completion Timeline',
        `mode=${mode}`,
        `updated=${new Date(resolveUpdatedAtMs(state)).toLocaleString()}`,
    ].join(' | ');

    const sections = [
        formatServerTimelineSectionForClipboard(state, mode),
        formatClientProbeFeedForClipboard(state.client_probe_feed),
    ].filter((section): section is string => Boolean(section));

    if (sections.length === 0) {
        return null;
    }

    return [header, ...sections].join('\n\n');
}

export function formatSelectedCompletionTraceForClipboard(
    state: CompletionTimelinePanelState,
    traceId: string
): string | null {
    if (state.kind !== 'ready') {
        return null;
    }

    const candidates = state.average_trace
        ? [...state.traces, state.average_trace]
        : state.traces;
    const trace = candidates.find((item) => item.trace_id === traceId);
    if (!trace) {
        return null;
    }

    return formatCompletionTimelineTraceForClipboard(trace);
}

export function formatCompletionTimelineTraceForClipboard(
    trace: CompletionTimelineTraceViewModel
): string {
    const headerParts = [`${trace.trace_id} (${trace.trigger_mode})`];
    if (typeof trace.sample_count === 'number') {
        headerParts.push(`sample=${trace.sample_count}`);
    }

    const summaryParts = [
        `outcome=${trace.outcome}`,
        `total=${trace.total_duration_ms}ms`,
    ];
    if (trace.dominant_stage) {
        summaryParts.push(`dominant=${trace.dominant_stage}`);
    }

    const lines = [
        headerParts.join(' | '),
        `request=${trace.request_id ?? 'n/a'} | started=${new Date(trace.started_at_ms).toLocaleTimeString()} | uri=${trace.uri}`,
        summaryParts.join(' | '),
    ];

    if (trace.unattributed_overhead_ms > 0) {
        lines.push(
            `unattributed_overhead=${trace.unattributed_overhead_ms}ms | max_stage_end=${trace.max_stage_end_ms}ms`
        );
    }
    const averageTraceNotice = getAverageTraceProvenanceNotice(trace);
    if (averageTraceNotice) {
        lines.push(averageTraceNotice);
    }

    if (trace.server_edge_details) {
        const detailsBits = [
            `transport_received_at_ms=${trace.server_edge_details.transport_received_at_ms}`,
            ...(trace.server_edge_details.transport_received_at_ms_provenance
                ? [`transport_received_at_ms_provenance=${trace.server_edge_details.transport_received_at_ms_provenance}`]
                : []),
            ...(typeof trace.server_edge_details.jsonrpc_dispatch_received_at_ms === 'number'
                ? [`jsonrpc_dispatch_received_at_ms=${trace.server_edge_details.jsonrpc_dispatch_received_at_ms}`]
                : []),
            ...(typeof trace.server_edge_details.service_future_created_at_ms === 'number'
                ? [`service_future_created_at_ms=${trace.server_edge_details.service_future_created_at_ms}`]
                : []),
            ...(trace.server_edge_details.pre_method_attribution_provenance
                ? [`pre_method_attribution_provenance=${trace.server_edge_details.pre_method_attribution_provenance}`]
                : []),
            ...(typeof trace.server_edge_details.service_scope_entered_at_ms === 'number'
                ? [`service_scope_entered_at_ms=${trace.server_edge_details.service_scope_entered_at_ms}`]
                : []),
            ...(typeof trace.server_edge_details.method_entered_at_ms === 'number'
                ? [`method_entered_at_ms=${trace.server_edge_details.method_entered_at_ms}`]
                : []),
            `handler_entered_at_ms=${trace.server_edge_details.handler_entered_at_ms}`,
            `response_sent_at_ms=${trace.server_edge_details.response_sent_at_ms}`,
            ...(typeof trace.server_edge_details.transport_to_service_scope_wait_ms === 'number'
                ? [`transport_to_service_scope_wait_ms=${trace.server_edge_details.transport_to_service_scope_wait_ms}`]
                : []),
            ...(typeof trace.server_edge_details.dispatch_to_request_context_wait_ms === 'number'
                ? [`dispatch_to_request_context_wait_ms=${trace.server_edge_details.dispatch_to_request_context_wait_ms}`]
                : []),
            ...(typeof trace.server_edge_details.transport_to_service_future_wait_ms === 'number'
                ? [`transport_to_service_future_wait_ms=${trace.server_edge_details.transport_to_service_future_wait_ms}`]
                : []),
            ...(typeof trace.server_edge_details.service_future_to_scope_wait_ms === 'number'
                ? [`service_future_to_scope_wait_ms=${trace.server_edge_details.service_future_to_scope_wait_ms}`]
                : []),
            ...(typeof trace.server_edge_details.service_scope_to_method_wait_ms === 'number'
                ? [`service_scope_to_method_wait_ms=${trace.server_edge_details.service_scope_to_method_wait_ms}`]
                : []),
            ...(typeof trace.server_edge_details.transport_to_method_wait_ms === 'number'
                ? [`transport_to_method_wait_ms=${trace.server_edge_details.transport_to_method_wait_ms}`]
                : []),
            ...(typeof trace.server_edge_details.method_prelude_exec_ms === 'number'
                ? [`method_prelude_exec_ms=${trace.server_edge_details.method_prelude_exec_ms}`]
                : []),
            `transport_to_handler_wait_ms=${trace.server_edge_details.transport_to_handler_wait_ms}`,
            `server_handler_exec_ms=${trace.server_edge_details.server_handler_exec_ms}`,
        ];
        if (typeof trace.server_edge_details.cancel_observed_at_ms === 'number') {
            detailsBits.push(
                `cancel_observed_at_ms=${trace.server_edge_details.cancel_observed_at_ms}`
            );
        }
        if (
            typeof trace.server_edge_details.cancel_observed_after_handler_enter_ms === 'number'
        ) {
            detailsBits.push(
                `cancel_observed_after_handler_enter_ms=${trace.server_edge_details.cancel_observed_after_handler_enter_ms}`
            );
        }
        lines.push(detailsBits.join(' | '));
    }
    if (trace.prepare_details) {
        const detailsBits: string[] = [];
        if (typeof trace.prepare_details.wait_budget_ms === 'number') {
            detailsBits.push(`prepare_wait_budget_ms=${trace.prepare_details.wait_budget_ms}`);
        }
        if (trace.prepare_details.guard_outcome) {
            detailsBits.push(`prepare_guard_outcome=${trace.prepare_details.guard_outcome}`);
        }
        if (trace.prepare_details.outcome) {
            detailsBits.push(`prepare_outcome=${trace.prepare_details.outcome}`);
        }
        if (trace.prepare_details.route) {
            detailsBits.push(`completion_route=${trace.prepare_details.route}`);
        }
        if (trace.prepare_details.fail_closed_cause) {
            detailsBits.push(`fail_closed_cause=${trace.prepare_details.fail_closed_cause}`);
        }
        if (typeof trace.prepare_details.min_file_version === 'number') {
            detailsBits.push(`prepare_min_file_version=${trace.prepare_details.min_file_version}`);
        }
        if (typeof trace.prepare_details.shadow_version_at_start === 'number') {
            detailsBits.push(
                `prepare_shadow_version_at_start=${trace.prepare_details.shadow_version_at_start}`
            );
        }
        if (typeof trace.prepare_details.observed_file_version === 'number') {
            detailsBits.push(
                `prepare_observed_file_version=${trace.prepare_details.observed_file_version}`
            );
        }
        if (typeof trace.prepare_details.wait_elapsed_ms === 'number') {
            detailsBits.push(`prepare_wait_elapsed_ms=${trace.prepare_details.wait_elapsed_ms}`);
        }
        if (typeof trace.prepare_details.snapshot_elapsed_ms === 'number') {
            detailsBits.push(
                `prepare_snapshot_elapsed_ms=${trace.prepare_details.snapshot_elapsed_ms}`
            );
        }
        if (typeof trace.prepare_details.apply_age_at_start_ms === 'number') {
            detailsBits.push(
                `prepare_apply_age_at_start_ms=${trace.prepare_details.apply_age_at_start_ms}`
            );
        }
        if (typeof trace.prepare_details.apply_age_at_terminal_ms === 'number') {
            detailsBits.push(
                `prepare_apply_age_at_terminal_ms=${trace.prepare_details.apply_age_at_terminal_ms}`
            );
        }
        if (detailsBits.length > 0) {
            lines.push(detailsBits.join(' | '));
        }
        const bottleneckVerdicts = buildCompletionTraceBottleneckVerdicts(trace);
        for (const verdict of bottleneckVerdicts) {
            lines.push(`bottleneck_verdict=${verdict}`);
        }
        const progressTrace = formatPrepareProgressTrace(trace.prepare_details.progress);
        if (progressTrace) {
            lines.push(progressTrace);
        }
        const waitRuntimeTrace = formatPrepareRuntimeTrace(
            'wait_for_file_version_runtime',
            trace.prepare_details.wait_for_file_version_runtime
        );
        if (waitRuntimeTrace) {
            lines.push(waitRuntimeTrace);
        }
        const snapshotRuntimeTrace = formatPrepareRuntimeTrace(
            'snapshot_with_deps_runtime',
            trace.prepare_details.snapshot_with_deps_runtime
        );
        if (snapshotRuntimeTrace) {
            lines.push(snapshotRuntimeTrace);
        }
        const snapshotTimeoutRuntimeTrace = formatPrepareRuntimeTrace(
            'snapshot_with_deps_timeout_runtime',
            trace.prepare_details.snapshot_with_deps_timeout_runtime
        );
        if (snapshotTimeoutRuntimeTrace) {
            lines.push(snapshotTimeoutRuntimeTrace);
        }
        const timeoutAttributionTrace = formatPrepareTimeoutAttributionTrace(
            trace.prepare_details.timeout_attribution
        );
        if (timeoutAttributionTrace) {
            lines.push(timeoutAttributionTrace);
        }
        const exactWaitTrace = formatExactWaitTrace(trace.prepare_details.exact_wait);
        if (exactWaitTrace) {
            lines.push(exactWaitTrace);
        }
        const artifactPollTrace = formatExactArtifactPollTrace(
            trace.prepare_details.exact_wait?.artifact_poll
        );
        if (artifactPollTrace) {
            lines.push(artifactPollTrace);
        }
    }
    if (trace.turn_attribution) {
        const turn = trace.turn_attribution;
        const turnBits = [
            `turn_request_file_seq=${turn.request_file_seq}`,
            `turn_request_epoch=${turn.request_epoch}`,
            `queue_outcome=${turn.queue_outcome}`,
            `queue_depth=${turn.queue_depth_before_enqueue}->${turn.queue_depth_after_enqueue}/${turn.queue_capacity}`,
            `queued_completion_ahead=${turn.queued_completion_ahead_count}`,
            `did_change_ahead=${turn.did_change_ahead_count}`,
            `active_completion_count=${turn.active_completion_count}`,
        ];
        if (turn.turn_wait_outcome) {
            turnBits.push(`turn_wait_outcome=${turn.turn_wait_outcome}`);
        }
        const dispatcherTrace = formatDispatcherAttributionTrace(turn);
        if (dispatcherTrace) {
            turnBits.push(dispatcherTrace);
        }
        if (turn.dropped_completion_file_seq.length > 0) {
            turnBits.push(`dropped_completion_file_seq=${turn.dropped_completion_file_seq.join(',')}`);
        }
        lines.push(turnBits.join(' | '));
        if (turn.active_holder) {
            lines.push(
                formatTurnHolderLine('active_holder', turn.active_holder)
            );
        }
        if (turn.queued_completion_ahead) {
            lines.push(
                formatTurnHolderLine('queued_completion_ahead', turn.queued_completion_ahead)
            );
        }
    }

    for (const stage of trace.stages) {
        const dominant = stage.is_dominant ? ' | dominant' : '';
        lines.push(
            `${stage.name} | ${stage.status} | ${stage.started_offset_ms}ms -> ${stage.end_offset_ms}ms (${stage.duration_ms}ms, ${stage.duration_percent.toFixed(1)}%)${dominant}`
        );
    }

    return lines.join('\n');
}

function formatTurnHolderLine(
    label: string,
    holder: NonNullable<CompletionTimelineTraceViewModel['turn_attribution']>['active_holder']
): string {
    const requestId = holder?.request_id ?? 'n/a';
    const versionHint = typeof holder?.version_hint === 'number'
        ? ` | version_hint=${holder.version_hint}`
        : '';
    return `${label} | request=${requestId} | file_seq=${holder?.file_seq} | epoch=${holder?.request_epoch} | trigger=${holder?.trigger_mode}${versionHint} | age=${holder?.age_ms}ms`;
}

function resolveUpdatedAtMs(state: CompletionTimelinePanelState): number {
    if (state.kind === 'ready') {
        return state.updated_at_ms;
    }

    return state.client_probe_feed.updated_at_ms;
}

function formatServerTimelineSectionForClipboard(
    state: CompletionTimelinePanelState,
    mode: CompletionTimelineClipboardMode
): string | null {
    const lines = ['Server Timeline'];

    if (state.kind === 'unsupported') {
        lines.push(state.message);
        return lines.join('\n');
    }

    if (state.kind === 'error') {
        lines.push(`Failed to load server timeline: ${state.message}`);
        return lines.join('\n');
    }

    lines.push(`contract=v${state.version}`);
    if (state.version < 7) {
        lines.push('v7 pre-method and snapshot overshoot attribution fields are unavailable by design on this payload.');
    }
    if (state.version < 8) {
        lines.push('v8 trustworthy pre-method attribution provenance is unavailable by design on this payload.');
    }
    if (state.version < 9) {
        lines.push('v9 pre-service-scope split is unavailable by design on this payload.');
    }
    if (state.version < 10) {
        lines.push('v10 dispatch split is unavailable by design on this payload.');
    }
    const traces = mode === 'average'
        ? (state.average_trace ? [state.average_trace] : [])
        : state.traces;
    if (traces.length === 0) {
        lines.push('No server traces visible.');
        return lines.join('\n');
    }

    return [...lines, ...traces.map(formatCompletionTimelineTraceForClipboard)].join('\n\n');
}

function formatClientProbeFeedForClipboard(
    feed: CompletionProbeFeedViewModel
): string {
    const lines = [
        'Client Probe Feed | local-only debug data',
        `updated=${new Date(feed.updated_at_ms).toLocaleString()}`,
        'Client probes are extension-local debug records and do not replace server timeline stages, routes, or outcomes.',
    ];

    if (feed.probes.length === 0) {
        lines.push('No client probes recorded yet.');
        return lines.join('\n');
    }

    return [...lines, ...feed.probes.map(formatClientProbeForClipboard)].join('\n\n');
}

function formatClientProbeForClipboard(
    probe: CompletionProbeViewModel
): string {
    const triggerCharacter = probe.trigger_character
        ? ` | trigger_character=${probe.trigger_character}`
        : '';
    const didChangeDelta = probe.time_since_last_did_change_sent_ms === 'unknown'
        ? 'unknown'
        : `${probe.time_since_last_did_change_sent_ms}ms`;
    const incompleteSuffix = typeof probe.is_incomplete === 'boolean'
        ? ` | is_incomplete=${probe.is_incomplete}`
        : '';
    const supersededBySuffix = probe.superseded_by_probe_id
        ? ` | superseded_by_probe_id=${probe.superseded_by_probe_id}`
        : '';
    const supersededAfterSuffix = typeof probe.superseded_after_ms === 'number'
        ? ` | superseded_after_ms=${probe.superseded_after_ms}ms`
        : '';
    const dispatchDeltaMs = Math.max(
        0,
        probe.lsp_request_started_at_ms - probe.request_started_at_ms
    );
    const lspRoundtripMs = Math.max(
        0,
        probe.lsp_response_received_at_ms - probe.lsp_request_started_at_ms
    );
    const postResponseMs = Math.max(
        0,
        probe.request_completed_at_ms - probe.lsp_response_received_at_ms
    );

    return [
        `${probe.probe_id} (${probe.trigger_mode})`,
        `started=${new Date(probe.request_started_at_ms).toLocaleTimeString()} | uri=${probe.uri} | document_version=${probe.document_version} | document_version_at_terminal=${probe.document_version_at_terminal}`,
        `client_terminal_state=${probe.client_terminal_state} | client_duration=${probe.client_duration_ms}ms | cancel_reason_hint=${probe.cancel_reason_hint}${supersededBySuffix}${supersededAfterSuffix}`,
        `result_kind=${probe.result_kind} | item_count_bucket=${probe.item_count_bucket}${incompleteSuffix}`,
        `transport_dispatch_delta_ms=${dispatchDeltaMs} | lsp_roundtrip_ms=${lspRoundtripMs} | client_post_response_ms=${postResponseMs}`,
        `time_since_last_local_edit_ms=${probe.time_since_last_local_edit_ms} | time_since_last_did_change_sent_ms=${didChangeDelta} | did_change_count_during_probe=${probe.did_change_count_during_probe}`,
        `cursor_moved_during_probe=${probe.cursor_moved_during_probe} | active_completion_count_at_start=${probe.active_completion_count_at_start} | same_uri_probe_overlap_count=${probe.same_uri_probe_overlap_count} | newer_probe_started_before_terminal=${probe.newer_probe_started_before_terminal}`,
        `is_after_dot=${probe.is_after_dot}${triggerCharacter} | identifier_tail_length=${probe.identifier_tail_length}`,
    ].join('\n');
}
