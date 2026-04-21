import {
    CompletionTimelineExactArtifactPollTrace,
    CompletionTimelineExactWaitDetailsTrace,
    CompletionTimelinePreMethodAttributionProvenance,
    CompletionTimelinePrepareDetailsTrace,
    CompletionTimelinePrepareProgressTrace,
    CompletionTimelinePrepareRuntimeTrace,
    CompletionTimelinePrepareTimeoutAttributionTrace,
    CompletionTimelineStageTrace,
    CompletionTimelineTrace,
    CompletionTimelineTurnAttributionTrace,
} from '../lsp/customRequests';
import { CompletionProbe } from './completionProbe';

export interface CompletionTraceClientIngressSupplement {
    correlation_status: 'correlated' | 'unavailable' | 'ambiguous';
    client_to_transport_wait_ms?: number;
}

export interface CompletionTracePostResponseSplit {
    client_before_transport_write_wait_ms?: number;
    client_to_transport_wait_ms?: number;
    response_ready_to_output_handoff_wait_ms?: number;
    response_output_handoff_send_wait_ms?: number;
    response_output_handoff_to_writer_wait_ms?: number;
    response_ready_to_output_enqueue_wait_ms?: number;
    response_output_queue_wait_ms?: number;
    response_output_encode_exec_ms?: number;
    response_output_write_and_flush_exec_ms?: number;
    response_ready_to_flush_wait_ms?: number;
    transport_to_client_receive_wait_ms?: number;
    client_receive_to_resolve_wait_ms?: number;
    client_post_response_ms?: number;
    server_to_client_post_response_ms?: number;
    raw_transport_receive_state?: 'observed' | 'unavailable';
}

const QUERY_BUNDLE_STAGE_TO_VERDICT: Record<string, string> = {
    query_bundle_pool_wait: 'query_bundle_pool_wait_dominant',
    query_bundle_deps_and_file_snapshot: 'query_bundle_deps_and_file_snapshot_dominant',
    query_bundle_owner_hint: 'query_bundle_owner_hint_dominant',
    query_bundle_ir_query: 'query_bundle_ir_query_dominant',
    query_bundle_ir_retry: 'query_bundle_ir_retry_dominant',
    query_bundle_other: 'query_bundle_other_dominant',
};

function isCanonicalQueryBundleStageName(stageName: string | undefined): stageName is string {
    return Boolean(stageName && QUERY_BUNDLE_STAGE_TO_VERDICT[stageName]);
}

function pickDominantStage(
    stages: CompletionTimelineStageTrace[] | undefined
): CompletionTimelineStageTrace | undefined {
    let bestStage: CompletionTimelineStageTrace | undefined;
    for (const stage of stages ?? []) {
        if (!bestStage || stage.duration_ms > bestStage.duration_ms) {
            bestStage = stage;
        }
    }
    return bestStage;
}

function resolveQueryBundleDominantStage(
    trace: Pick<CompletionTimelineTrace, 'dominant_stage' | 'stages'>
): string | undefined {
    if (isCanonicalQueryBundleStageName(trace.dominant_stage)) {
        return trace.dominant_stage;
    }
    const dominantStage = pickDominantStage(trace.stages);
    if (dominantStage && isCanonicalQueryBundleStageName(dominantStage.name)) {
        return dominantStage.name;
    }
    return undefined;
}

export function getPreMethodAttributionProvenance(
    trace: Pick<CompletionTimelineTrace, 'server_edge_details'>
): CompletionTimelinePreMethodAttributionProvenance | undefined {
    return trace.server_edge_details?.pre_method_attribution_provenance;
}

export function hasStrongPreMethodAttribution(
    trace: Pick<CompletionTimelineTrace, 'server_edge_details'>
): boolean {
    return getPreMethodAttributionProvenance(trace) === 'same_request_authoritative';
}

interface CompletionIngressCandidate {
    verdict: string;
    waitMs: number;
}

function strongestPositiveIngressCandidate(
    candidates: Array<CompletionIngressCandidate | undefined>
): CompletionIngressCandidate | undefined {
    let strongest: CompletionIngressCandidate | undefined;
    for (const candidate of candidates) {
        if (!candidate || candidate.waitMs <= 0) {
            continue;
        }
        if (!strongest || candidate.waitMs > strongest.waitMs) {
            strongest = candidate;
        }
    }
    return strongest;
}

export function derivePrepareTimeoutSubphase(
    details?: CompletionTimelinePrepareDetailsTrace
): 'wait_for_file_version' | 'snapshot_with_deps' | 'unavailable' | null {
    if (details?.fail_closed_cause !== 'prepare_timeout') {
        return null;
    }

    const phase = details.progress?.phase;
    if (phase === 'wait_for_file_version') {
        return 'wait_for_file_version';
    }
    if (
        phase === 'snapshot_with_deps' ||
        phase === 'deps_guard' ||
        (
            typeof details.progress?.wait_completed_offset_ms === 'number' &&
            typeof details.progress?.snapshot_completed_offset_ms !== 'number'
        )
    ) {
        return 'snapshot_with_deps';
    }

    return 'unavailable';
}

export function buildCompletionTraceBottleneckVerdicts(
    trace: Pick<
        CompletionTimelineTrace,
        'dominant_stage' | 'stages' | 'prepare_details' | 'server_edge_details' | 'turn_attribution'
    >,
    clientIngress?: CompletionTraceClientIngressSupplement
): string[] {
    const verdicts: string[] = [];
    const queryBundleDominantStage = resolveQueryBundleDominantStage(trace);
    if (queryBundleDominantStage) {
        verdicts.push('query_bundle_dominant');
        verdicts.push(QUERY_BUNDLE_STAGE_TO_VERDICT[queryBundleDominantStage]);
    }
    const serverEdgeDetails = trace.server_edge_details;
    const readLoopWait = serverEdgeDetails?.read_loop_wait_ms;
    const admissionQueueWait = serverEdgeDetails?.admission_queue_wait_ms;
    const schedulerPollReadyWait = serverEdgeDetails?.scheduler_poll_ready_wait_ms;
    const completionBarrierWait = serverEdgeDetails?.completion_barrier_wait_ms;
    const sameFileIngressTokenWait = serverEdgeDetails?.same_file_ingress_token_wait_ms;
    const adapterToDispatchWait = serverEdgeDetails?.adapter_to_dispatch_wait_ms;
    const transportToMethodWait = serverEdgeDetails?.transport_to_method_wait_ms;
    const methodPreludeExec = serverEdgeDetails?.method_prelude_exec_ms;
    const strongPreMethodAttribution = hasStrongPreMethodAttribution(trace);
    const hasPositiveServerAdmissionSplit = [
        readLoopWait,
        admissionQueueWait,
        schedulerPollReadyWait,
        completionBarrierWait,
        sameFileIngressTokenWait,
    ].some((waitMs) => typeof waitMs === 'number' && waitMs > 0);
    const fineGrainedIngressCandidate = !queryBundleDominantStage
        ? strongestPositiveIngressCandidate([
            typeof readLoopWait === 'number'
                ? { verdict: 'reader_backpressure_dominant', waitMs: readLoopWait }
                : undefined,
            typeof admissionQueueWait === 'number'
                ? { verdict: 'admission_queue_dominant', waitMs: admissionQueueWait }
                : undefined,
            typeof schedulerPollReadyWait === 'number'
                ? { verdict: 'scheduler_poll_ready_dominant', waitMs: schedulerPollReadyWait }
                : undefined,
            typeof completionBarrierWait === 'number'
                ? { verdict: 'completion_barrier_dominant', waitMs: completionBarrierWait }
                : undefined,
            typeof sameFileIngressTokenWait === 'number'
                ? {
                    verdict: 'same_file_ingress_token_dominant',
                    waitMs: sameFileIngressTokenWait,
                }
                : undefined,
        ])
        : undefined;
    const ingressDominantCandidate = fineGrainedIngressCandidate
        ?? (
            !queryBundleDominantStage
            && typeof adapterToDispatchWait === 'number'
            && adapterToDispatchWait > 0
                ? { verdict: 'adapter_before_dispatch_dominant', waitMs: adapterToDispatchWait }
                : undefined
        );
    if (ingressDominantCandidate) {
        verdicts.push(ingressDominantCandidate.verdict);
    }
    const preMethodDominantCandidate = !queryBundleDominantStage
        ? strongestPositiveIngressCandidate([
            strongPreMethodAttribution && typeof transportToMethodWait === 'number'
                ? {
                    verdict: 'server_before_method_entry_dominant',
                    waitMs: transportToMethodWait,
                }
                : undefined,
            typeof methodPreludeExec === 'number'
                ? { verdict: 'handler_prelude_dominant', waitMs: methodPreludeExec }
                : undefined,
        ])
        : undefined;
    if (
        preMethodDominantCandidate &&
        (
            !ingressDominantCandidate
            || preMethodDominantCandidate.waitMs > ingressDominantCandidate.waitMs
        )
    ) {
        verdicts.push(preMethodDominantCandidate.verdict);
    }

    if (
        !queryBundleDominantStage &&
        clientIngress?.correlation_status === 'correlated' &&
        !hasPositiveServerAdmissionSplit &&
        typeof clientIngress.client_to_transport_wait_ms === 'number' &&
        clientIngress.client_to_transport_wait_ms > 0 &&
        (
            !ingressDominantCandidate
            || clientIngress.client_to_transport_wait_ms > ingressDominantCandidate.waitMs
        ) &&
        (
            !preMethodDominantCandidate
            || clientIngress.client_to_transport_wait_ms > preMethodDominantCandidate.waitMs
        )
    ) {
        verdicts.push('client_before_transport_dominant');
    }

    if (trace.prepare_details?.fail_closed_cause === 'prepare_timeout') {
        const timeoutSource = trace.prepare_details.timeout_attribution?.source;
        if (timeoutSource) {
            verdicts.push(`prepare_timeout@${timeoutSource}`);
        } else {
            const prepareTimeoutSubphase = derivePrepareTimeoutSubphase(trace.prepare_details);
            if (prepareTimeoutSubphase && prepareTimeoutSubphase !== 'unavailable') {
                verdicts.push(`prepare_timeout@${prepareTimeoutSubphase}`);
            } else {
                verdicts.push('prepare_timeout@wait_for_file_version');
            }
        }
    }

    if (trace.prepare_details?.fail_closed_cause === 'exact_deadline') {
        if (trace.prepare_details.exact_wait?.artifact_poll) {
            verdicts.push('exact_deadline@artifact_poll');
        } else {
            const stateSummary = formatExactWaitStateSummary(trace.prepare_details.exact_wait);
            verdicts.push(stateSummary ? `exact_deadline | ${stateSummary}` : 'exact_deadline');
        }
    }

    return verdicts;
}

export function deriveCompletionTracePostResponseSplit(
    trace: Pick<CompletionTimelineTrace, 'server_edge_details'>,
    probe?: Pick<
        CompletionProbe,
        | 'request_started_at_ms'
        | 'lsp_request_started_at_ms'
        | 'transport_request_written_at_ms'
        | 'transport_response_receive_state'
        | 'transport_response_received_at_ms'
        | 'lsp_response_received_at_ms'
        | 'request_completed_at_ms'
    >,
): CompletionTracePostResponseSplit {
    const serverEdgeDetails = trace.server_edge_details;
    const serverIngressAtMs = typeof serverEdgeDetails?.adapter_read_at_ms === 'number'
        ? serverEdgeDetails.adapter_read_at_ms
        : serverEdgeDetails?.transport_received_at_ms;
    const transportWriteAtMs = typeof probe?.transport_request_written_at_ms === 'number'
        ? probe.transport_request_written_at_ms
        : probe?.lsp_request_started_at_ms;
    const split: CompletionTracePostResponseSplit = {
        client_before_transport_write_wait_ms:
            probe && typeof probe.transport_request_written_at_ms === 'number'
                ? Math.max(0, probe.transport_request_written_at_ms - probe.request_started_at_ms)
                : undefined,
        client_to_transport_wait_ms:
            typeof transportWriteAtMs === 'number' && typeof serverIngressAtMs === 'number'
                ? Math.max(0, serverIngressAtMs - transportWriteAtMs)
                : undefined,
        response_ready_to_output_handoff_wait_ms:
            serverEdgeDetails?.response_ready_to_output_handoff_wait_ms,
        response_output_handoff_send_wait_ms:
            serverEdgeDetails?.response_output_handoff_send_wait_ms,
        response_output_handoff_to_writer_wait_ms:
            serverEdgeDetails?.response_output_handoff_to_writer_wait_ms,
        response_ready_to_output_enqueue_wait_ms:
            serverEdgeDetails?.response_ready_to_output_enqueue_wait_ms,
        response_output_queue_wait_ms: serverEdgeDetails?.response_output_queue_wait_ms,
        response_output_encode_exec_ms: serverEdgeDetails?.response_output_encode_exec_ms,
        response_output_write_and_flush_exec_ms:
            serverEdgeDetails?.response_output_write_and_flush_exec_ms,
        response_ready_to_flush_wait_ms: serverEdgeDetails?.response_ready_to_flush_wait_ms,
        client_post_response_ms: probe
            ? Math.max(0, probe.request_completed_at_ms - probe.lsp_response_received_at_ms)
            : undefined,
        server_to_client_post_response_ms:
            probe && serverEdgeDetails
                ? Math.max(
                    0,
                    probe.request_completed_at_ms - serverEdgeDetails.response_sent_at_ms,
                )
                : undefined,
        raw_transport_receive_state: probe?.transport_response_receive_state,
    };

    if (
        probe?.transport_response_receive_state === 'observed'
        && typeof probe.transport_response_received_at_ms === 'number'
    ) {
        split.client_receive_to_resolve_wait_ms = Math.max(
            0,
            probe.lsp_response_received_at_ms - probe.transport_response_received_at_ms,
        );
        if (typeof serverEdgeDetails?.response_flush_completed_at_ms === 'number') {
            split.transport_to_client_receive_wait_ms = Math.max(
                0,
                probe.transport_response_received_at_ms
                    - serverEdgeDetails.response_flush_completed_at_ms,
            );
        }
    }

    return split;
}

export function formatPrepareProgressTrace(
    progress?: CompletionTimelinePrepareProgressTrace
): string | null {
    if (!progress) {
        return null;
    }
    const bits = ['prepare_progress'];
    pushNumberFact(bits, 'phase_started_offset_ms', progress.phase_started_offset_ms);
    pushNumberFact(bits, 'wait_completed_offset_ms', progress.wait_completed_offset_ms);
    pushNumberFact(bits, 'snapshot_completed_offset_ms', progress.snapshot_completed_offset_ms);
    if (progress.phase) {
        bits.splice(1, 0, `phase=${progress.phase}`);
    }
    return bits.length > 1 ? bits.join(' | ') : null;
}

export function formatPrepareRuntimeTrace(
    label:
        | 'wait_for_file_version_runtime'
        | 'snapshot_with_deps_runtime'
        | 'snapshot_with_deps_timeout_runtime',
    trace?: CompletionTimelinePrepareRuntimeTrace
): string | null {
    if (!trace) {
        return null;
    }
    const bits: string[] = [label];
    pushNumberFact(bits, 'queue_wait_ms', trace.queue_wait_ms);
    pushNumberFact(bits, 'exec_ms', trace.exec_ms);
    pushNumberFact(bits, 'wake_wait_ms', trace.wake_wait_ms);
    if (trace.resolution) {
        bits.push(`resolution=${trace.resolution}`);
    }
    return bits.length > 1 ? bits.join(' | ') : null;
}

export function formatPrepareTimeoutAttributionTrace(
    trace?: CompletionTimelinePrepareTimeoutAttributionTrace
): string | null {
    if (!trace) {
        return null;
    }
    return [
        'timeout_attribution',
        `source=${trace.source}`,
        `phase=${trace.phase}`,
        `budget_ms=${trace.budget_ms}`,
        `elapsed_ms=${trace.elapsed_ms}`,
        `overshoot_ms=${trace.overshoot_ms}`,
    ].join(' | ');
}

export function formatExactWaitTrace(
    exactWait?: CompletionTimelineExactWaitDetailsTrace
): string | null {
    if (!exactWait) {
        return null;
    }
    const bits = ['exact_wait'];
    pushBooleanFact(bits, 'head_ready_before_wait', exactWait.head_ready_before_wait);
    pushBooleanFact(bits, 'exact_ready_before_wait', exactWait.exact_ready_before_wait);
    pushBooleanFact(
        bits,
        'current_revision_head_owner_hints_ready',
        exactWait.current_revision_head_owner_hints_ready
    );
    pushStringFact(bits, 'artifact_wait_outcome', exactWait.artifact_wait_outcome);
    pushStringFact(bits, 'type_index_wait_outcome', exactWait.type_index_wait_outcome);
    pushStringFact(bits, 'type_index_waiter_action', exactWait.type_index_waiter_action);
    pushStringFact(bits, 'matching_task_state', exactWait.matching_task_state);
    pushStringFact(bits, 'task_phase', exactWait.task_phase);
    return bits.length > 1 ? bits.join(' | ') : null;
}

export function formatExactArtifactPollTrace(
    trace?: CompletionTimelineExactArtifactPollTrace
): string | null {
    if (!trace) {
        return null;
    }
    const bits = [
        'artifact_poll',
        `poll_count=${trace.poll_count}`,
        `poll_elapsed_ms=${trace.poll_elapsed_ms}`,
    ];
    pushNumberFact(bits, 'observed_file_version', trace.observed_file_version);
    pushBooleanFact(bits, 'head_ready', trace.head_ready);
    pushBooleanFact(bits, 'exact_ready', trace.exact_ready);
    return bits.join(' | ');
}

export function formatDispatcherAttributionTrace(
    turnAttribution?: CompletionTimelineTurnAttributionTrace
): string | null {
    if (typeof turnAttribution?.dispatcher_resolution_latency_ms !== 'number') {
        return null;
    }
    return `dispatcher_resolution_latency_ms=${turnAttribution.dispatcher_resolution_latency_ms}`;
}

function formatExactWaitStateSummary(
    exactWait?: CompletionTimelineExactWaitDetailsTrace
): string | null {
    if (!exactWait) {
        return 'task_state=unavailable';
    }
    const bits: string[] = [];
    if (exactWait.type_index_waiter_action) {
        bits.push(`waiter_action=${exactWait.type_index_waiter_action}`);
    }
    if (exactWait.matching_task_state && exactWait.task_phase) {
        bits.push(`task_state=${exactWait.matching_task_state}:${exactWait.task_phase}`);
    } else if (exactWait.matching_task_state) {
        bits.push(`task_state=${exactWait.matching_task_state}`);
    } else {
        bits.push('task_state=unavailable');
    }
    return bits.join(' | ');
}

function pushNumberFact(bits: string[], key: string, value: number | undefined): void {
    if (typeof value === 'number') {
        bits.push(`${key}=${value}`);
    }
}

function pushBooleanFact(bits: string[], key: string, value: boolean | undefined): void {
    if (typeof value === 'boolean') {
        bits.push(`${key}=${value}`);
    }
}

function pushStringFact(bits: string[], key: string, value: string | undefined): void {
    if (value) {
        bits.push(`${key}=${value}`);
    }
}
