import {
    CompletionTimelineExactArtifactPollTrace,
    CompletionTimelineExactWaitDetailsTrace,
    CompletionTimelineFetchResult,
    CompletionTimelinePrepareTimeoutAttributionTrace,
    CompletionTimelineTrace,
} from '../lsp/customRequests';
import { CompletionProbe, CompletionProbeTerminalState } from './completionProbe';
import { buildCompletionTraceBottleneckVerdicts } from './completionTimelineDrilldown';

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
    transport_to_handler_wait_ms?: number;
    transport_to_method_wait_ms?: number;
    method_prelude_exec_ms?: number;
    server_handler_exec_ms?: number;
    bottleneck_verdicts: string[];
    prepare_timeout?: CompletionTimelinePrepareTimeoutAttributionTrace;
    exact_deadline?: ObservabilityIncidentExactDeadlineSummary;
    client_correlation: ObservabilityIncidentClientCorrelation;
}

export interface ObservabilityIncidentRequestSection {
    captureScope: ObservabilityIncidentCaptureScope;
    requestCount: number;
    requests: ObservabilityIncidentRequestSummary[];
    gaps: string[];
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
            transport_to_handler_wait_ms: trace.server_edge_details?.transport_to_handler_wait_ms,
            transport_to_method_wait_ms: trace.server_edge_details?.transport_to_method_wait_ms,
            method_prelude_exec_ms: trace.server_edge_details?.method_prelude_exec_ms,
            server_handler_exec_ms: trace.server_edge_details?.server_handler_exec_ms,
            bottleneck_verdicts: buildCompletionTraceBottleneckVerdicts(trace),
            prepare_timeout: trace.prepare_details?.fail_closed_cause === 'prepare_timeout'
                ? trace.prepare_details.timeout_attribution
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
            typeof request.transport_to_handler_wait_ms === 'number'
                ? `transport_to_handler_wait_ms=${request.transport_to_handler_wait_ms}`
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
