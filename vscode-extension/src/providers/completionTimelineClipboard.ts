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
    if (trace.prepare_details) {
        const detailsBits: string[] = [];
        if (typeof trace.prepare_details.wait_budget_ms === 'number') {
            detailsBits.push(`prepare_wait_budget_ms=${trace.prepare_details.wait_budget_ms}`);
        }
        if (trace.prepare_details.outcome) {
            detailsBits.push(`prepare_outcome=${trace.prepare_details.outcome}`);
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
