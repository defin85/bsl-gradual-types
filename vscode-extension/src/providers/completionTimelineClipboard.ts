import {
    CompletionTimelinePanelState,
    CompletionTimelineTraceViewModel,
} from './completionTimelineModel';

export type CompletionTimelineClipboardMode = 'all' | 'average';

export function formatVisibleCompletionTimelineForClipboard(
    state: CompletionTimelinePanelState,
    mode: CompletionTimelineClipboardMode
): string | null {
    if (state.kind !== 'ready') {
        return null;
    }

    const traces = mode === 'average'
        ? (state.average_trace ? [state.average_trace] : [])
        : state.traces;
    if (traces.length === 0) {
        return null;
    }

    const header = [
        'Completion Timeline',
        `mode=${mode}`,
        `updated=${new Date(state.updated_at_ms).toLocaleString()}`,
        `contract=v${state.version}`,
    ].join(' | ');

    return [header, ...traces.map(formatCompletionTimelineTraceForClipboard)]
        .join('\n\n');
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

    for (const stage of trace.stages) {
        const dominant = stage.is_dominant ? ' | dominant' : '';
        lines.push(
            `${stage.name} | ${stage.status} | ${stage.started_offset_ms}ms -> ${stage.end_offset_ms}ms (${stage.duration_ms}ms, ${stage.duration_percent.toFixed(1)}%)${dominant}`
        );
    }

    return lines.join('\n');
}
