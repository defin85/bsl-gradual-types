import {
    CompletionTimelineExactArtifactPollTrace,
    CompletionTimelineExactWaitDetailsTrace,
    CompletionTimelineFetchResult,
    CompletionTimelineFirstPollContentionContenderTrace,
    CompletionTimelineFirstPollContentionAttributionTrace,
    CompletionTimelinePreMethodAttributionProvenance,
    CompletionTimelinePrepareRuntimeTrace,
    CompletionTimelinePrepareTimeoutAttributionTrace,
    CompletionTimelineTurnAttributionTrace,
    CompletionTimelineTurnHolderTrace,
    CompletionTimelineTransportReceivedAtMsProvenance,
    CompletionTimelineTrace,
} from '../lsp/customRequests';
import { CompletionProbe, CompletionProbeTerminalState } from './completionProbe';
import {
    CompletionTraceClientIngressSupplement,
    buildCompletionTraceBottleneckVerdicts,
} from './completionTimelineDrilldown';

const PROBE_TRACE_RESPONSE_MATCH_WINDOW_MS = 5;
const MAX_SUMMARY_REQUESTS = 5;

export type ObservabilityIncidentCaptureScopeKind = 'single_uri' | 'multi_uri' | 'empty' | 'unavailable';
export type ObservabilityIncidentClientCorrelationStatus = 'correlated' | 'unavailable' | 'ambiguous';
export type ObservabilityIncidentClientCorrelationReason =
    | 'missing_server_edge'
    | 'no_probe_candidates'
    | 'timestamp_mismatch'
    | 'multiple_probe_candidates';

export interface ObservabilityIncidentCaptureScope {
    kind: ObservabilityIncidentCaptureScopeKind;
    uri?: string;
    uri_count?: number;
}

export interface ObservabilityIncidentClientCorrelation {
    status: ObservabilityIncidentClientCorrelationStatus;
    reason?: ObservabilityIncidentClientCorrelationReason;
    probe_id?: string;
    client_duration_ms?: number;
    client_terminal_state?: CompletionProbeTerminalState;
    client_to_transport_wait_ms?: number;
    server_to_client_post_response_ms?: number;
}

export interface ObservabilityIncidentExactDeadlineSummary {
    artifact_wait_outcome?: string;
    type_index_wait_outcome?: string;
    type_index_waiter_action?: string;
    matching_task_state?: string;
    task_phase?: string;
    artifact_poll?: CompletionTimelineExactArtifactPollTrace;
}

export interface ObservabilityIncidentRequestSummary {
    trace_id: string;
    request_id?: string;
    uri: string;
    trigger_mode: string;
    outcome: string;
    total_duration_ms: number;
    dominant_stage?: string;
    transport_received_at_ms_provenance?: CompletionTimelineTransportReceivedAtMsProvenance;
    jsonrpc_dispatch_received_at_ms?: number;
    service_future_created_at_ms?: number;
    service_future_first_poll_entered_at_ms?: number;
    service_future_first_poll_outcome?: string;
    service_future_first_wake_scheduled_at_ms?: number;
    first_poll_contention_attribution?: CompletionTimelineFirstPollContentionAttributionTrace;
    first_poll_contention_contenders?: CompletionTimelineFirstPollContentionContenderTrace[];
    turn_attribution?: CompletionTimelineTurnAttributionTrace;
    pre_method_attribution_provenance?: CompletionTimelinePreMethodAttributionProvenance;
    transport_to_handler_wait_ms?: number;
    dispatch_to_request_context_wait_ms?: number;
    transport_to_service_future_wait_ms?: number;
    service_future_to_scope_wait_ms?: number;
    service_future_to_first_poll_wait_ms?: number;
    first_poll_to_first_wake_wait_ms?: number;
    transport_to_service_scope_wait_ms?: number;
    service_scope_to_method_wait_ms?: number;
    transport_to_method_wait_ms?: number;
    method_prelude_exec_ms?: number;
    server_handler_exec_ms?: number;
    bottleneck_verdicts: string[];
    prepare_timeout?: CompletionTimelinePrepareTimeoutAttributionTrace;
    snapshot_with_deps_timeout_runtime?: CompletionTimelinePrepareRuntimeTrace;
    exact_deadline?: ObservabilityIncidentExactDeadlineSummary;
    client_correlation: ObservabilityIncidentClientCorrelation;
}

export interface ObservabilityIncidentRequestSection {
    captureScope: ObservabilityIncidentCaptureScope;
    requestCount: number;
    requests: ObservabilityIncidentRequestSummary[];
    gaps: string[];
}

function sanitizeFirstPollContentionContenders(
    contenders: CompletionTimelineFirstPollContentionContenderTrace[] | undefined,
    contractVersion: number
): CompletionTimelineFirstPollContentionContenderTrace[] | undefined {
    if (!contenders || contractVersion < 13) {
        return undefined;
    }
    if (contractVersion < 14) {
        return contenders.map(({ command: _command, phase: _phase, ...contender }) => contender);
    }
    if (contractVersion < 15) {
        return contenders.map(({ phase: _phase, ...contender }) => contender);
    }
    return contenders;
}

function sanitizeTurnAttribution(
    turnAttribution: CompletionTimelineTurnAttributionTrace | undefined,
    contractVersion: number
): CompletionTimelineTurnAttributionTrace | undefined {
    if (!turnAttribution) {
        return undefined;
    }
    if (contractVersion < 16) {
        const {
            turn_wait_entered_at_ms: _turnWaitEnteredAtMs,
            turn_wait_resolved_at_ms: _turnWaitResolvedAtMs,
            wake_after_turn_resolution_at_ms: _wakeAfterTurnResolutionAtMs,
            ...legacyTurnAttribution
        } = turnAttribution;
        return legacyTurnAttribution;
    }
    return turnAttribution;
}

export function buildObservabilityIncidentRequestSection(
    completionTimeline: CompletionTimelineFetchResult,
    clientProbes: CompletionProbe[]
): ObservabilityIncidentRequestSection {
    if (completionTimeline.kind !== 'ok') {
        return {
            captureScope: { kind: 'unavailable' },
            requestCount: 0,
            requests: [],
            gaps: [],
        };
    }

    const traces = completionTimeline.response.traces;
    const contractVersion = completionTimeline.response.version;
    const captureScope = buildCaptureScope(traces);
    const gaps: string[] = [];
    const unusedProbeIds = new Set(clientProbes.map((probe) => probe.probe_id));
    const requests = traces.map((trace) => {
        const clientCorrelation = buildClientCorrelation(trace, clientProbes, unusedProbeIds);
        if (clientCorrelation.status === 'correlated' && clientCorrelation.probe_id) {
            unusedProbeIds.delete(clientCorrelation.probe_id);
        } else if (clientCorrelation.status === 'ambiguous') {
            gaps.push(
                `Client/server correlation is ambiguous for trace ${trace.trace_id}: ${clientCorrelation.reason}.`
            );
        }

        return {
            trace_id: trace.trace_id,
            request_id: trace.request_id,
            uri: trace.uri,
            trigger_mode: trace.trigger_mode,
            outcome: trace.outcome,
            total_duration_ms: trace.total_duration_ms,
            dominant_stage: trace.dominant_stage,
            transport_received_at_ms_provenance:
                trace.server_edge_details?.transport_received_at_ms_provenance,
            jsonrpc_dispatch_received_at_ms:
                trace.server_edge_details?.jsonrpc_dispatch_received_at_ms,
            service_future_created_at_ms:
                trace.server_edge_details?.service_future_created_at_ms,
            service_future_first_poll_entered_at_ms:
                trace.server_edge_details?.service_future_first_poll_entered_at_ms,
            service_future_first_poll_outcome:
                trace.server_edge_details?.service_future_first_poll_outcome,
            service_future_first_wake_scheduled_at_ms:
                trace.server_edge_details?.service_future_first_wake_scheduled_at_ms,
            first_poll_contention_attribution:
                contractVersion >= 12
                    ? trace.server_edge_details?.first_poll_contention_attribution
                    : undefined,
            first_poll_contention_contenders:
                sanitizeFirstPollContentionContenders(
                    trace.server_edge_details?.first_poll_contention_contenders,
                    contractVersion
                ),
            turn_attribution: sanitizeTurnAttribution(trace.turn_attribution, contractVersion),
            pre_method_attribution_provenance:
                trace.server_edge_details?.pre_method_attribution_provenance,
            transport_to_handler_wait_ms: trace.server_edge_details?.transport_to_handler_wait_ms,
            dispatch_to_request_context_wait_ms:
                trace.server_edge_details?.dispatch_to_request_context_wait_ms,
            transport_to_service_future_wait_ms:
                trace.server_edge_details?.transport_to_service_future_wait_ms,
            service_future_to_scope_wait_ms:
                trace.server_edge_details?.service_future_to_scope_wait_ms,
            service_future_to_first_poll_wait_ms:
                trace.server_edge_details?.service_future_to_first_poll_wait_ms,
            first_poll_to_first_wake_wait_ms:
                trace.server_edge_details?.first_poll_to_first_wake_wait_ms,
            transport_to_service_scope_wait_ms:
                trace.server_edge_details?.transport_to_service_scope_wait_ms,
            service_scope_to_method_wait_ms:
                trace.server_edge_details?.service_scope_to_method_wait_ms,
            transport_to_method_wait_ms: trace.server_edge_details?.transport_to_method_wait_ms,
            method_prelude_exec_ms: trace.server_edge_details?.method_prelude_exec_ms,
            server_handler_exec_ms: trace.server_edge_details?.server_handler_exec_ms,
            bottleneck_verdicts: buildCompletionTraceBottleneckVerdicts(
                trace,
                asClientIngressSupplement(clientCorrelation)
            ),
            prepare_timeout: trace.prepare_details?.fail_closed_cause === 'prepare_timeout'
                ? trace.prepare_details.timeout_attribution
                : undefined,
            snapshot_with_deps_timeout_runtime:
                trace.prepare_details?.fail_closed_cause === 'prepare_timeout'
                    ? trace.prepare_details.snapshot_with_deps_timeout_runtime
                    : undefined,
            exact_deadline: trace.prepare_details?.fail_closed_cause === 'exact_deadline'
                ? buildExactDeadlineSummary(trace.prepare_details.exact_wait)
                : undefined,
            client_correlation: clientCorrelation,
        };
    });

    return {
        captureScope,
        requestCount: traces.length,
        requests,
        gaps,
    };
}

function asClientIngressSupplement(
    correlation: ObservabilityIncidentClientCorrelation
): CompletionTraceClientIngressSupplement {
    return {
        correlation_status: correlation.status,
        client_to_transport_wait_ms: correlation.client_to_transport_wait_ms,
    };
}

export function renderRequestScopeLine(section: ObservabilityIncidentRequestSection): string {
    const details = [`scope=${section.captureScope.kind}`];
    if (section.captureScope.uri) {
        details.push(`uri=${section.captureScope.uri}`);
    }
    if (typeof section.captureScope.uri_count === 'number' && !section.captureScope.uri) {
        details.push(`uri_count=${section.captureScope.uri_count}`);
    }
    details.push(`request_count=${section.requestCount}`);
    return `- ${details.join(' | ')}`;
}

export function renderRequestSummaryLines(section: ObservabilityIncidentRequestSection): string[] {
    if (section.requests.length === 0) {
        return ['- No authoritative request summaries were captured in this bundle.'];
    }

    const lines = section.requests.slice(0, MAX_SUMMARY_REQUESTS).map((request) => {
        const details = [
            request.trace_id,
            request.request_id ? `request=${request.request_id}` : undefined,
            `outcome=${request.outcome}`,
            `total=${request.total_duration_ms}ms`,
            request.dominant_stage ? `dominant=${request.dominant_stage}` : undefined,
            request.bottleneck_verdicts.length > 0
                ? `verdicts=${request.bottleneck_verdicts.join(',')}`
                : undefined,
            request.transport_received_at_ms_provenance
                ? `transport_received_at_ms_provenance=${request.transport_received_at_ms_provenance}`
                : undefined,
            typeof request.jsonrpc_dispatch_received_at_ms === 'number'
                ? `jsonrpc_dispatch_received_at_ms=${request.jsonrpc_dispatch_received_at_ms}`
                : undefined,
            typeof request.service_future_created_at_ms === 'number'
                ? `service_future_created_at_ms=${request.service_future_created_at_ms}`
                : undefined,
            typeof request.service_future_first_poll_entered_at_ms === 'number'
                ? `service_future_first_poll_entered_at_ms=${request.service_future_first_poll_entered_at_ms}`
                : undefined,
            request.service_future_first_poll_outcome
                ? `service_future_first_poll_outcome=${request.service_future_first_poll_outcome}`
                : undefined,
            typeof request.service_future_first_wake_scheduled_at_ms === 'number'
                ? `service_future_first_wake_scheduled_at_ms=${request.service_future_first_wake_scheduled_at_ms}`
                : undefined,
            request.first_poll_contention_attribution
                ? `first_poll_contention=${request.first_poll_contention_attribution.contender_class}:${request.first_poll_contention_attribution.uri_scope}|inflight_count=${request.first_poll_contention_attribution.inflight_count}|concurrency_level=${request.first_poll_contention_attribution.concurrency_level}${typeof request.first_poll_contention_attribution.oldest_inflight_age_ms === 'number' ? `|oldest_inflight_age_ms=${request.first_poll_contention_attribution.oldest_inflight_age_ms}` : ''}`
                : undefined,
            formatFirstPollContentionContendersForSummary(
                request.first_poll_contention_contenders
            ),
            request.turn_attribution?.turn_wait_outcome
                ? `turn_wait_outcome=${request.turn_attribution.turn_wait_outcome}`
                : undefined,
            typeof request.turn_attribution?.dispatcher_resolution_latency_ms === 'number'
                ? `dispatcher_resolution_latency_ms=${request.turn_attribution.dispatcher_resolution_latency_ms}`
                : undefined,
            typeof request.turn_attribution?.turn_wait_entered_at_ms === 'number'
                ? `turn_wait_entered_at_ms=${request.turn_attribution.turn_wait_entered_at_ms}`
                : undefined,
            typeof request.turn_attribution?.turn_wait_resolved_at_ms === 'number'
                ? `turn_wait_resolved_at_ms=${request.turn_attribution.turn_wait_resolved_at_ms}`
                : undefined,
            typeof request.turn_attribution?.wake_after_turn_resolution_at_ms === 'number'
                ? `wake_after_turn_resolution_at_ms=${request.turn_attribution.wake_after_turn_resolution_at_ms}`
                : undefined,
            formatTurnHolderForSummary(
                'queued_completion_ahead_holder',
                request.turn_attribution?.queued_completion_ahead
            ),
            formatTurnHolderForSummary(
                'active_holder',
                request.turn_attribution?.active_holder
            ),
            request.pre_method_attribution_provenance
                ? `pre_method_provenance=${request.pre_method_attribution_provenance}`
                : undefined,
            typeof request.transport_to_handler_wait_ms === 'number'
                ? `transport_to_handler_wait_ms=${request.transport_to_handler_wait_ms}`
                : undefined,
            typeof request.dispatch_to_request_context_wait_ms === 'number'
                ? `dispatch_to_request_context_wait_ms=${request.dispatch_to_request_context_wait_ms}`
                : undefined,
            typeof request.transport_to_service_future_wait_ms === 'number'
                ? `transport_to_service_future_wait_ms=${request.transport_to_service_future_wait_ms}`
                : undefined,
            typeof request.service_future_to_scope_wait_ms === 'number'
                ? `service_future_to_scope_wait_ms=${request.service_future_to_scope_wait_ms}`
                : undefined,
            typeof request.service_future_to_first_poll_wait_ms === 'number'
                ? `service_future_to_first_poll_wait_ms=${request.service_future_to_first_poll_wait_ms}`
                : undefined,
            typeof request.first_poll_to_first_wake_wait_ms === 'number'
                ? `first_poll_to_first_wake_wait_ms=${request.first_poll_to_first_wake_wait_ms}`
                : undefined,
            typeof request.transport_to_service_scope_wait_ms === 'number'
                ? `transport_to_service_scope_wait_ms=${request.transport_to_service_scope_wait_ms}`
                : undefined,
            typeof request.service_scope_to_method_wait_ms === 'number'
                ? `service_scope_to_method_wait_ms=${request.service_scope_to_method_wait_ms}`
                : undefined,
            typeof request.transport_to_method_wait_ms === 'number'
                ? `transport_to_method_wait_ms=${request.transport_to_method_wait_ms}`
                : undefined,
            typeof request.method_prelude_exec_ms === 'number'
                ? `method_prelude_exec_ms=${request.method_prelude_exec_ms}`
                : undefined,
            typeof request.server_handler_exec_ms === 'number'
                ? `server_handler_exec_ms=${request.server_handler_exec_ms}`
                : undefined,
            formatSnapshotTimeoutRuntimeForSummary(request.snapshot_with_deps_timeout_runtime),
            formatCorrelationForSummary(request.client_correlation),
        ].filter((value): value is string => Boolean(value));

        return `- ${details.join(' | ')}`;
    });

    const omittedCount = section.requests.length - MAX_SUMMARY_REQUESTS;
    if (omittedCount > 0) {
        lines.push(`- ${omittedCount} more request(s) are available in incident.json.`);
    }

    return lines;
}

function buildCaptureScope(traces: CompletionTimelineTrace[]): ObservabilityIncidentCaptureScope {
    if (traces.length === 0) {
        return { kind: 'empty' };
    }

    const uniqueUris = [...new Set(traces.map((trace) => trace.uri))];
    if (uniqueUris.length === 1) {
        return {
            kind: 'single_uri',
            uri: uniqueUris[0],
            uri_count: 1,
        };
    }

    return {
        kind: 'multi_uri',
        uri_count: uniqueUris.length,
    };
}

function formatFirstPollContentionContendersForSummary(
    contenders?: CompletionTimelineFirstPollContentionContenderTrace[]
): string | undefined {
    if (!contenders || contenders.length === 0) {
        return undefined;
    }

    const preview = contenders
        .slice(0, 3)
        .map((contender) => {
            const uri = contender.uri ?? 'unavailable';
            const method =
                contender.command
                    ? `${contender.method}:${contender.command}`
                    : contender.method;
            const phaseSuffix = contender.phase ? `[phase=${contender.phase}]` : '';
            return `${contender.request_class}:${method}${phaseSuffix}@${uri}(age_ms=${contender.age_ms})`;
        })
        .join(';');
    const omittedCount = contenders.length - 3;
    return omittedCount > 0
        ? `first_poll_contenders=${preview};+${omittedCount}_more`
        : `first_poll_contenders=${preview}`;
}

function formatTurnHolderForSummary(
    label: string,
    holder?: CompletionTimelineTurnHolderTrace
): string | undefined {
    if (!holder) {
        return undefined;
    }
    const requestId = holder.request_id ?? 'unavailable';
    const versionHint = typeof holder.version_hint === 'number'
        ? `|version_hint=${holder.version_hint}`
        : '';
    return `${label}=request:${requestId}|epoch:${holder.request_epoch}|file_seq:${holder.file_seq}|trigger:${holder.trigger_mode}${versionHint}|age_ms:${holder.age_ms}`;
}

function buildExactDeadlineSummary(
    exactWait: CompletionTimelineExactWaitDetailsTrace | undefined
): ObservabilityIncidentExactDeadlineSummary | undefined {
    if (!exactWait) {
        return undefined;
    }
    return {
        artifact_wait_outcome: exactWait.artifact_wait_outcome,
        type_index_wait_outcome: exactWait.type_index_wait_outcome,
        type_index_waiter_action: exactWait.type_index_waiter_action,
        matching_task_state: exactWait.matching_task_state,
        task_phase: exactWait.task_phase,
        artifact_poll: exactWait.artifact_poll,
    };
}

function buildClientCorrelation(
    trace: CompletionTimelineTrace,
    probes: CompletionProbe[],
    unusedProbeIds: Set<string>
): ObservabilityIncidentClientCorrelation {
    if (!trace.server_edge_details) {
        return {
            status: 'unavailable',
            reason: 'missing_server_edge',
        };
    }

    const candidates = probes.filter((probe) => {
        if (!unusedProbeIds.has(probe.probe_id)) {
            return false;
        }
        if (probe.uri !== trace.uri || probe.trigger_mode !== trace.trigger_mode) {
            return false;
        }
        return (
            Math.abs(
                probe.lsp_response_received_at_ms - trace.server_edge_details!.response_sent_at_ms
            ) <= PROBE_TRACE_RESPONSE_MATCH_WINDOW_MS
        );
    });

    if (candidates.length > 1) {
        return {
            status: 'ambiguous',
            reason: 'multiple_probe_candidates',
        };
    }

    if (candidates.length === 0) {
        const hasUriAndModeCandidates = probes.some(
            (probe) =>
                unusedProbeIds.has(probe.probe_id) &&
                probe.uri === trace.uri &&
                probe.trigger_mode === trace.trigger_mode
        );
        return {
            status: 'unavailable',
            reason: hasUriAndModeCandidates ? 'timestamp_mismatch' : 'no_probe_candidates',
        };
    }

    const probe = candidates[0];
    return {
        status: 'correlated',
        probe_id: probe.probe_id,
        client_duration_ms: probe.client_duration_ms,
        client_terminal_state: probe.client_terminal_state,
        client_to_transport_wait_ms: Math.max(
            0,
            trace.server_edge_details.transport_received_at_ms - probe.lsp_request_started_at_ms
        ),
        server_to_client_post_response_ms: Math.max(
            0,
            probe.request_completed_at_ms - trace.server_edge_details.response_sent_at_ms
        ),
    };
}

function formatCorrelationForSummary(correlation: ObservabilityIncidentClientCorrelation): string {
    switch (correlation.status) {
        case 'correlated':
            return `correlation=correlated:${correlation.probe_id}`;
        case 'ambiguous':
            return `correlation=ambiguous:${correlation.reason}`;
        default:
            return `correlation=unavailable:${correlation.reason}`;
    }
}

function formatSnapshotTimeoutRuntimeForSummary(
    runtime: CompletionTimelinePrepareRuntimeTrace | undefined
): string | undefined {
    if (!runtime?.resolution) {
        return undefined;
    }
    const details = [`snapshot_with_deps_timeout_runtime=${runtime.resolution}`];
    if (typeof runtime.queue_wait_ms === 'number') {
        details.push(`queue_wait_ms=${runtime.queue_wait_ms}`);
    }
    if (typeof runtime.exec_ms === 'number') {
        details.push(`exec_ms=${runtime.exec_ms}`);
    }
    if (typeof runtime.wake_wait_ms === 'number') {
        details.push(`wake_wait_ms=${runtime.wake_wait_ms}`);
    }
    return details.join('|');
}
