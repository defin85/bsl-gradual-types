import {
    CompletionTimelineFetchResult,
    ObservabilityMetricsFetchResult,
} from '../lsp/customRequests';
import { CompletionProbe } from './completionProbe';

export interface CompletionTimelineExportCapture {
    capturedAtMs?: number;
    completionTimeline?: CompletionTimelineFetchResult;
    clientProbes?: CompletionProbe[];
    observabilityMetrics?: ObservabilityMetricsFetchResult;
}

let sharedCompletionTimelineExportCapture: CompletionTimelineExportCapture | undefined;

function cloneCapture(
    capture: CompletionTimelineExportCapture | undefined
): CompletionTimelineExportCapture | undefined {
    if (!capture) {
        return undefined;
    }
    return {
        capturedAtMs: capture.capturedAtMs,
        completionTimeline: capture.completionTimeline,
        clientProbes: capture.clientProbes ? [...capture.clientProbes] : undefined,
        observabilityMetrics: capture.observabilityMetrics,
    };
}

export function getSharedCompletionTimelineExportCapture():
    | CompletionTimelineExportCapture
    | undefined {
    return cloneCapture(sharedCompletionTimelineExportCapture);
}

export function setSharedCompletionTimelineExportCapture(
    capture: CompletionTimelineExportCapture
): void {
    sharedCompletionTimelineExportCapture = cloneCapture(capture);
}

export function setSharedCompletionTimelineExportCaptureForTests(
    capture: CompletionTimelineExportCapture
): void {
    setSharedCompletionTimelineExportCapture(capture);
}

export function clearSharedCompletionTimelineExportCaptureForTests(): void {
    sharedCompletionTimelineExportCapture = undefined;
}
