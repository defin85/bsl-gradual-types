import {
    CompletionTimelineFetchResult,
    CompletionTimelineResponse,
    CompletionTimelineStageTrace,
    CompletionTimelineTrace,
} from '../lsp/customRequests';

export const COMPLETION_TIMELINE_UNSUPPORTED_MESSAGE =
    'Connected LSP server does not support completion timeline (`bsl.getCompletionTimeline`). Update backend binary.';

export interface CompletionTimelineStageViewModel extends CompletionTimelineStageTrace {
    end_offset_ms: number;
    width_percent: number;
    is_dominant: boolean;
}

export interface CompletionTimelineTraceViewModel {
    trace_id: string;
    request_id?: string;
    uri: string;
    trigger_mode: string;
    outcome: string;
    started_at_ms: number;
    total_duration_ms: number;
    dominant_stage?: string;
    stages: CompletionTimelineStageViewModel[];
}

export type CompletionTimelinePanelState =
    | {
        kind: 'ready';
        version: number;
        updated_at_ms: number;
        traces: CompletionTimelineTraceViewModel[];
    }
    | { kind: 'unsupported'; message: string }
    | { kind: 'error'; message: string };

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

function mapTrace(trace: CompletionTimelineTrace): CompletionTimelineTraceViewModel {
    const dominantStage = resolveDominantStageName(trace);
    const maxStageEnd = trace.stages.reduce(
        (max, stage) => Math.max(max, stage.started_offset_ms + stage.duration_ms),
        0
    );
    const totalDurationMs = Math.max(trace.total_duration_ms, maxStageEnd, 1);

    const stages = trace.stages.map((stage) => {
        const endOffsetMs = stage.started_offset_ms + stage.duration_ms;
        return {
            ...stage,
            end_offset_ms: endOffsetMs,
            width_percent: stageWidthPercent(stage.duration_ms, totalDurationMs),
            is_dominant: stage.name === dominantStage,
        };
    });

    return {
        ...trace,
        dominant_stage: dominantStage,
        total_duration_ms: trace.total_duration_ms,
        stages,
    };
}

export function mapCompletionTimelineResponseToPanelState(
    response: CompletionTimelineResponse,
    updatedAtMs: number = Date.now()
): CompletionTimelinePanelState {
    const traces = [...response.traces]
        .sort((left, right) => right.started_at_ms - left.started_at_ms)
        .map(mapTrace);

    return {
        kind: 'ready',
        version: response.version,
        updated_at_ms: updatedAtMs,
        traces,
    };
}

export function mapCompletionTimelineFetchResultToPanelState(
    result: CompletionTimelineFetchResult,
    updatedAtMs: number = Date.now()
): CompletionTimelinePanelState {
    if (result.kind === 'unsupported') {
        return {
            kind: 'unsupported',
            message: COMPLETION_TIMELINE_UNSUPPORTED_MESSAGE,
        };
    }

    if (result.kind === 'error') {
        return {
            kind: 'error',
            message: result.message,
        };
    }

    return mapCompletionTimelineResponseToPanelState(result.response, updatedAtMs);
}

