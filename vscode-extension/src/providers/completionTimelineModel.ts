import {
    CompletionTimelineFetchResult,
    CompletionTimelinePrepareDetailsTrace,
    CompletionTimelineResponse,
    CompletionTimelineServerEdgeDetailsTrace,
    CompletionTimelineStageStatus,
    CompletionTimelineStageTrace,
    CompletionTimelineTurnAttributionTrace,
    CompletionTimelineTrace,
} from '../lsp/customRequests';
import { CompletionProbe } from './completionProbe';

export const COMPLETION_TIMELINE_UNSUPPORTED_MESSAGE =
    'Connected LSP server does not support completion timeline (`bsl.getCompletionTimeline`). Update backend binary.';

export interface CompletionTimelineStageViewModel extends CompletionTimelineStageTrace {
    end_offset_ms: number;
    width_percent: number;
    duration_percent: number;
    is_dominant: boolean;
}

export interface CompletionTimelineTraceViewModel {
    trace_id: string;
    sample_count?: number;
    request_id?: string;
    uri: string;
    trigger_mode: string;
    outcome: string;
    started_at_ms: number;
    total_duration_ms: number;
    max_stage_end_ms: number;
    unattributed_overhead_ms: number;
    dominant_stage?: string;
    prepare_details?: CompletionTimelinePrepareDetailsTrace;
    server_edge_details?: CompletionTimelineServerEdgeDetailsTrace;
    turn_attribution?: CompletionTimelineTurnAttributionTrace;
    stages: CompletionTimelineStageViewModel[];
}

export interface CompletionProbeViewModel extends CompletionProbe {}

export interface CompletionProbeFeedViewModel {
    updated_at_ms: number;
    probes: CompletionProbeViewModel[];
}

export type CompletionTimelinePanelState =
    | ({
        kind: 'ready';
        version: number;
        updated_at_ms: number;
        traces: CompletionTimelineTraceViewModel[];
        average_trace: CompletionTimelineTraceViewModel | null;
    } & { client_probe_feed: CompletionProbeFeedViewModel })
    | ({ kind: 'unsupported'; message: string } & { client_probe_feed: CompletionProbeFeedViewModel })
    | ({ kind: 'error'; message: string } & { client_probe_feed: CompletionProbeFeedViewModel });


export const AVERAGE_TRACE_PROVENANCE_NOTICE =
    'Average trace is synthetic; v8 trustworthy pre-method attribution provenance and v9 pre-service-scope split are unavailable by design.';

export function getAverageTraceProvenanceNotice(
    trace: Pick<CompletionTimelineTraceViewModel, 'trigger_mode'> | null | undefined
): string | null {
    if (trace?.trigger_mode !== 'averaged') {
        return null;
    }

    return AVERAGE_TRACE_PROVENANCE_NOTICE;
}

export function resolveDominantStageName(trace: CompletionTimelineTrace): string | undefined {
    if (trace.dominant_stage && trace.stages.some((stage) => stage.name === trace.dominant_stage)) {
        return trace.dominant_stage;
    }

    let bestStage: CompletionTimelineStageTrace | undefined;
    for (const stage of trace.stages) {
        if (!bestStage || stage.duration_ms > bestStage.duration_ms) {
            bestStage = stage;
        }
    }
    return bestStage?.name;
}

function stageWidthPercent(
    durationMs: number,
    totalDurationMs: number
): number {
    if (durationMs <= 0 || totalDurationMs <= 0) {
        return 0;
    }
    const raw = (durationMs / totalDurationMs) * 100;
    return Math.min(100, Math.max(raw, 1));
}

function stageDurationPercent(
    durationMs: number,
    totalDurationMs: number
): number {
    if (durationMs <= 0 || totalDurationMs <= 0) {
        return 0;
    }
    const raw = (durationMs / totalDurationMs) * 100;
    return Math.min(100, Math.max(raw, 0));
}

function mapTrace(trace: CompletionTimelineTrace): CompletionTimelineTraceViewModel {
    const dominantStage = resolveDominantStageName(trace);
    const maxStageEnd = trace.stages.reduce(
        (max, stage) => Math.max(max, stage.started_offset_ms + stage.duration_ms),
        0
    );
    const totalDurationMs = Math.max(trace.total_duration_ms, maxStageEnd, 1);
    const unattributedOverheadMs = Math.max(trace.total_duration_ms - maxStageEnd, 0);

    const stages = trace.stages.map((stage) => {
        const endOffsetMs = stage.started_offset_ms + stage.duration_ms;
        return {
            ...stage,
            end_offset_ms: endOffsetMs,
            width_percent: stageWidthPercent(stage.duration_ms, totalDurationMs),
            duration_percent: stageDurationPercent(stage.duration_ms, totalDurationMs),
            is_dominant: stage.name === dominantStage,
        };
    });

    return {
        ...trace,
        max_stage_end_ms: maxStageEnd,
        unattributed_overhead_ms: unattributedOverheadMs,
        dominant_stage: dominantStage,
        total_duration_ms: trace.total_duration_ms,
        stages,
    };
}

function mostFrequentValue<T extends string>(
    values: T[],
    tieBreakerOrder: T[] = []
): T {
    const counts = new Map<string, number>();
    for (const value of values) {
        counts.set(value, (counts.get(value) ?? 0) + 1);
    }
    let bestValue = values[0] ?? tieBreakerOrder[0];
    let bestCount = -1;
    for (const [value, count] of counts.entries()) {
        if (count > bestCount) {
            bestValue = value as T;
            bestCount = count;
            continue;
        }
        if (count === bestCount) {
            const currentIndex = tieBreakerOrder.indexOf(bestValue as T);
            const candidateIndex = tieBreakerOrder.indexOf(value as T);
            if (candidateIndex !== -1 && (currentIndex === -1 || candidateIndex < currentIndex)) {
                bestValue = value as T;
            }
        }
    }
    return bestValue as T;
}

function buildAverageTrace(
    traces: CompletionTimelineTraceViewModel[]
): CompletionTimelineTraceViewModel | null {
    if (traces.length === 0) {
        return null;
    }

    type StageAccumulator = {
        name: string;
        position_sum: number;
        position_count: number;
        duration_sum_ms: number;
        duration_count: number;
        status_values: string[];
    };
    const stageMap = new Map<string, StageAccumulator>();
    for (const trace of traces) {
        trace.stages.forEach((stage, index) => {
            const existing = stageMap.get(stage.name);
            if (existing) {
                existing.position_sum += index;
                existing.position_count += 1;
                existing.duration_sum_ms += stage.duration_ms;
                existing.duration_count += 1;
                existing.status_values.push(stage.status);
                return;
            }
            stageMap.set(stage.name, {
                name: stage.name,
                position_sum: index,
                position_count: 1,
                duration_sum_ms: stage.duration_ms,
                duration_count: 1,
                status_values: [stage.status],
            });
        });
    }

    const stageAccumulators = [...stageMap.values()].sort((left, right) => {
        const leftAvgPos = left.position_sum / left.position_count;
        const rightAvgPos = right.position_sum / right.position_count;
        return leftAvgPos - rightAvgPos;
    });
    const stageStatusPriority: CompletionTimelineStageStatus[] = [
        'failed',
        'cancelled',
        'skipped',
        'completed',
    ];

    let cursorMs = 0;
    const averageStages = stageAccumulators.map((accumulator) => {
        const durationMs = Math.max(
            0,
            Math.round(accumulator.duration_sum_ms / accumulator.duration_count)
        );
        const status = mostFrequentValue(
            accumulator.status_values as CompletionTimelineStageStatus[],
            stageStatusPriority
        );
        const stage = {
            name: accumulator.name,
            status,
            started_offset_ms: cursorMs,
            duration_ms: durationMs,
        };
        cursorMs += durationMs;
        return stage;
    });

    const averageTotalDurationMs = Math.round(
        traces.reduce((sum, trace) => sum + trace.total_duration_ms, 0) / traces.length
    );
    const normalizedTotalDurationMs = Math.max(averageTotalDurationMs, cursorMs, 1);
    const averageOutcome = mostFrequentValue(traces.map((trace) => trace.outcome), [
        'handler_error',
        'cancelled',
        'superseded',
        'ok_non_empty',
        'ok_empty',
    ]);
    const averageTraceRaw: CompletionTimelineTrace = {
        trace_id: `average(${traces.length})`,
        request_id: undefined,
        uri: 'average://completion-timeline',
        trigger_mode: 'averaged',
        outcome: averageOutcome,
        started_at_ms: traces[0].started_at_ms,
        total_duration_ms: normalizedTotalDurationMs,
        dominant_stage: undefined,
        stages: averageStages,
    };
    const averageTrace = mapTrace(averageTraceRaw);
    return {
        ...averageTrace,
        sample_count: traces.length,
    };
}

function mapClientProbeFeed(
    probes: CompletionProbe[],
    updatedAtMs: number
): CompletionProbeFeedViewModel {
    return {
        updated_at_ms: updatedAtMs,
        probes: [...probes]
            .sort((left, right) => {
                if (right.request_started_at_ms !== left.request_started_at_ms) {
                    return right.request_started_at_ms - left.request_started_at_ms;
                }
                return right.request_completed_at_ms - left.request_completed_at_ms;
            })
            .map((probe) => ({ ...probe })),
    };
}

export function mapCompletionTimelineResponseToPanelState(
    response: CompletionTimelineResponse,
    clientProbes: CompletionProbe[] = [],
    updatedAtMs: number = Date.now()
): CompletionTimelinePanelState {
    const traces = [...response.traces]
        .sort((left, right) => right.started_at_ms - left.started_at_ms)
        .map(mapTrace);
    const averageTrace = buildAverageTrace(traces);

    return {
        kind: 'ready',
        version: response.version,
        updated_at_ms: updatedAtMs,
        traces,
        average_trace: averageTrace,
        client_probe_feed: mapClientProbeFeed(clientProbes, updatedAtMs),
    };
}

export function mapCompletionTimelineFetchResultToPanelState(
    result: CompletionTimelineFetchResult,
    clientProbes: CompletionProbe[] = [],
    updatedAtMs: number = Date.now()
): CompletionTimelinePanelState {
    if (result.kind === 'unsupported') {
        return {
            kind: 'unsupported',
            message: COMPLETION_TIMELINE_UNSUPPORTED_MESSAGE,
            client_probe_feed: mapClientProbeFeed(clientProbes, updatedAtMs),
        };
    }

    if (result.kind === 'error') {
        return {
            kind: 'error',
            message: result.message,
            client_probe_feed: mapClientProbeFeed(clientProbes, updatedAtMs),
        };
    }

    return mapCompletionTimelineResponseToPanelState(result.response, clientProbes, updatedAtMs);
}
