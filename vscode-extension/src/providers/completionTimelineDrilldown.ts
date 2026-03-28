import {
    CompletionTimelineExactArtifactPollTrace,
    CompletionTimelineExactWaitDetailsTrace,
    CompletionTimelinePreMethodAttributionProvenance,
    CompletionTimelinePrepareDetailsTrace,
    CompletionTimelinePrepareProgressTrace,
    CompletionTimelinePrepareRuntimeTrace,
    CompletionTimelinePrepareTimeoutAttributionTrace,
    CompletionTimelineTrace,
    CompletionTimelineTurnAttributionTrace,
} from '../lsp/customRequests';

export interface CompletionTraceClientIngressSupplement {
    correlation_status: 'correlated' | 'unavailable' | 'ambiguous';
    client_to_transport_wait_ms?: number;
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
        'prepare_details' | 'server_edge_details' | 'turn_attribution'
    >,
    clientIngress?: CompletionTraceClientIngressSupplement
): string[] {
    const verdicts: string[] = [];
    const adapterToDispatchWait = trace.server_edge_details?.adapter_to_dispatch_wait_ms;
    const transportToMethodWait = trace.server_edge_details?.transport_to_method_wait_ms;
    const methodPreludeExec = trace.server_edge_details?.method_prelude_exec_ms;
    const strongPreMethodAttribution = hasStrongPreMethodAttribution(trace);
    if (
        typeof adapterToDispatchWait === 'number' &&
        adapterToDispatchWait > 0 &&
        (
            typeof transportToMethodWait !== 'number' ||
            adapterToDispatchWait > transportToMethodWait
        ) &&
        (
            typeof methodPreludeExec !== 'number' ||
            adapterToDispatchWait > methodPreludeExec
        )
    ) {
        verdicts.push('adapter_before_dispatch_dominant');
    }
    if (
        typeof transportToMethodWait === 'number' &&
        typeof methodPreludeExec === 'number'
    ) {
        if (
            strongPreMethodAttribution &&
            transportToMethodWait > 0 &&
            transportToMethodWait > methodPreludeExec &&
            (
                typeof adapterToDispatchWait !== 'number' ||
                transportToMethodWait > adapterToDispatchWait
            )
        ) {
            verdicts.push('server_before_method_entry_dominant');
        } else if (
            methodPreludeExec > 0 &&
            methodPreludeExec > transportToMethodWait &&
            (
                typeof adapterToDispatchWait !== 'number' ||
                methodPreludeExec > adapterToDispatchWait
            )
        ) {
            verdicts.push('handler_prelude_dominant');
        }
    }

    if (
        clientIngress?.correlation_status === 'correlated' &&
        typeof clientIngress.client_to_transport_wait_ms === 'number' &&
        clientIngress.client_to_transport_wait_ms > 0 &&
        (
            typeof adapterToDispatchWait !== 'number' ||
            clientIngress.client_to_transport_wait_ms > adapterToDispatchWait
        ) &&
        (
            typeof transportToMethodWait !== 'number' ||
            clientIngress.client_to_transport_wait_ms > transportToMethodWait
        ) &&
        (
            typeof methodPreludeExec !== 'number' ||
            clientIngress.client_to_transport_wait_ms > methodPreludeExec
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
