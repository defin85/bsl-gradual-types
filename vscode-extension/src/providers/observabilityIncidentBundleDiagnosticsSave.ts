import {
    DiagnosticsSaveTimelineFetchResult,
    DiagnosticsSaveTimelinePublishTrace,
} from '../lsp/customRequests';

export interface ObservabilityIncidentDiagnosticsSaveSummary {
    trace_id: string;
    uri: string;
    requested_version: number;
    save_cycle_sequence: number;
    diagnostics_generation: number;
    trigger: string;
    first_publish?: DiagnosticsSaveTimelinePublishTrace;
    followup_publish?: DiagnosticsSaveTimelinePublishTrace;
    save_fastlane_outcome?: string;
    idle_heavy_outcome?: string;
    followup_syntax_work_mode?: string;
    followup_wait_reason?: string;
    followup_runtime_queue_wait_ms?: number;
    followup_apply_lag_ms?: number;
    followup_wait_for_file_version_ms?: number;
    followup_snapshot_with_deps_ms?: number;
    terminal_outcome?: string;
}

export interface ObservabilityIncidentDiagnosticsSaveSection {
    requestCount: number;
    requests: ObservabilityIncidentDiagnosticsSaveSummary[];
    gaps: string[];
}

export function buildObservabilityIncidentDiagnosticsSaveSection(
    diagnosticsSaveTimeline: DiagnosticsSaveTimelineFetchResult
): ObservabilityIncidentDiagnosticsSaveSection {
    if (diagnosticsSaveTimeline.kind !== 'ok') {
        return {
            requestCount: 0,
            requests: [],
            gaps: [],
        };
    }

    return {
        requestCount: diagnosticsSaveTimeline.response.traces.length,
        requests: diagnosticsSaveTimeline.response.traces.map((trace) => ({
            trace_id: trace.trace_id,
            uri: trace.uri,
            requested_version: trace.requested_version,
            save_cycle_sequence: trace.save_cycle_sequence,
            diagnostics_generation: trace.diagnostics_generation,
            trigger: trace.trigger,
            first_publish: trace.first_publish,
            followup_publish: trace.followup_publish,
            save_fastlane_outcome: trace.save_fastlane_outcome,
            idle_heavy_outcome: trace.idle_heavy_outcome,
            followup_syntax_work_mode: trace.followup_syntax_work_mode,
            followup_wait_reason: trace.followup_wait_reason,
            followup_runtime_queue_wait_ms: trace.followup_runtime_queue_wait_ms,
            followup_apply_lag_ms: trace.followup_apply_lag_ms,
            followup_wait_for_file_version_ms: trace.followup_wait_for_file_version_ms,
            followup_snapshot_with_deps_ms: trace.followup_snapshot_with_deps_ms,
            terminal_outcome: trace.terminal_outcome,
        })),
        gaps: [],
    };
}

function isPositiveTimingValue(value: number | undefined): value is number {
    return typeof value === 'number' && value > 0;
}

function formatPublish(label: string, publish: DiagnosticsSaveTimelinePublishTrace | undefined): string {
    if (!publish) {
        return `${label}=none`;
    }

    const parts = [
        `${label}=${publish.profile}:${publish.publish_kind}:${publish.outcome}@${publish.elapsed_ms}ms`,
    ];
    if (publish.syntax_work_mode) {
        parts.push(`syntax_work_mode=${publish.syntax_work_mode}`);
    }
    if (isPositiveTimingValue(publish.runtime_queue_wait_ms)) {
        parts.push(`runtime_queue_wait_ms=${publish.runtime_queue_wait_ms}`);
    }
    if (isPositiveTimingValue(publish.apply_lag_ms)) {
        parts.push(`apply_lag_ms=${publish.apply_lag_ms}`);
    }
    if (isPositiveTimingValue(publish.blocking_queue_wait_ms)) {
        parts.push(`blocking_queue_wait_ms=${publish.blocking_queue_wait_ms}`);
    }
    if (isPositiveTimingValue(publish.wait_for_file_version_ms)) {
        parts.push(`wait_for_file_version_ms=${publish.wait_for_file_version_ms}`);
    }
    if (isPositiveTimingValue(publish.snapshot_with_deps_ms)) {
        parts.push(`snapshot_with_deps_ms=${publish.snapshot_with_deps_ms}`);
    }
    if (isPositiveTimingValue(publish.syntax_diagnostics_query_ms)) {
        parts.push(`syntax_diagnostics_query_ms=${publish.syntax_diagnostics_query_ms}`);
    }
    if (isPositiveTimingValue(publish.semantic_diagnostics_query_ms)) {
        parts.push(`semantic_diagnostics_query_ms=${publish.semantic_diagnostics_query_ms}`);
    }
    if (isPositiveTimingValue(publish.publish_wait_ms)) {
        parts.push(`publish_wait_ms=${publish.publish_wait_ms}`);
    }
    return parts.join(' | ');
}

function renderProfileOutcome(
    outcome: string | undefined,
    terminalOutcome: string | undefined
): string {
    if (outcome) {
        return outcome;
    }
    if (!terminalOutcome) {
        return 'pending';
    }
    return 'unknown';
}

function renderTerminalOutcome(terminalOutcome: string | undefined): string {
    return terminalOutcome ?? 'in_flight';
}

function formatPublishWithLifecycle(
    label: string,
    publish: DiagnosticsSaveTimelinePublishTrace | undefined,
    terminalOutcome: string | undefined
): string {
    if (publish) {
        return formatPublish(label, publish);
    }
    return `${label}=${terminalOutcome ? 'none' : 'pending'}`;
}

function formatFollowupWait(
    syntaxWorkMode: string | undefined,
    reason: string | undefined,
    runtimeQueueWaitMs: number | undefined,
    applyLagMs: number | undefined,
    waitForFileVersionMs: number | undefined,
    snapshotWithDepsMs: number | undefined
): string | undefined {
    if (
        !reason
        && !syntaxWorkMode
        && !isPositiveTimingValue(runtimeQueueWaitMs)
        && !isPositiveTimingValue(applyLagMs)
        && !isPositiveTimingValue(waitForFileVersionMs)
        && !isPositiveTimingValue(snapshotWithDepsMs)
    ) {
        return undefined;
    }

    const parts: string[] = [];
    if (syntaxWorkMode) {
        parts.push(`followup_syntax_work_mode=${syntaxWorkMode}`);
    }
    if (reason) {
        parts.push(`followup_wait=${reason}`);
    }
    if (isPositiveTimingValue(runtimeQueueWaitMs)) {
        parts.push(`followup_runtime_queue_wait_ms=${runtimeQueueWaitMs}`);
    }
    if (isPositiveTimingValue(applyLagMs)) {
        parts.push(`followup_apply_lag_ms=${applyLagMs}`);
    }
    if (isPositiveTimingValue(waitForFileVersionMs)) {
        parts.push(`followup_wait_for_file_version_ms=${waitForFileVersionMs}`);
    }
    if (isPositiveTimingValue(snapshotWithDepsMs)) {
        parts.push(`followup_snapshot_with_deps_ms=${snapshotWithDepsMs}`);
    }
    return parts.join(' | ');
}

export function renderDiagnosticsSaveSummaryLines(
    section: ObservabilityIncidentDiagnosticsSaveSection
): string[] {
    if (section.requests.length === 0) {
        return ['No diagnostics save traces captured in this bundle.'];
    }

    return section.requests.flatMap((request) => [
        `trace=${request.trace_id} | uri=${request.uri} | requested_version=${request.requested_version} | save_cycle_sequence=${request.save_cycle_sequence} | diagnostics_generation=${request.diagnostics_generation} | trigger=${request.trigger} | save_fastlane_outcome=${renderProfileOutcome(request.save_fastlane_outcome, request.terminal_outcome)} | idle_heavy_outcome=${renderProfileOutcome(request.idle_heavy_outcome, request.terminal_outcome)} | terminal=${renderTerminalOutcome(request.terminal_outcome)}`,
        formatPublishWithLifecycle('first_publish', request.first_publish, request.terminal_outcome),
        formatPublishWithLifecycle('followup_publish', request.followup_publish, request.terminal_outcome),
        formatFollowupWait(
            request.followup_syntax_work_mode,
            request.followup_wait_reason,
            request.followup_runtime_queue_wait_ms,
            request.followup_apply_lag_ms,
            request.followup_wait_for_file_version_ms,
            request.followup_snapshot_with_deps_ms
        ),
    ].filter((line): line is string => Boolean(line)));
}
