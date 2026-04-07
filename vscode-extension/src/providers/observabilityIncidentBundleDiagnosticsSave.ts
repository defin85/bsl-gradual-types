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
    followup_wait_reason?: string;
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
            followup_wait_reason: trace.followup_wait_reason,
            followup_wait_for_file_version_ms: trace.followup_wait_for_file_version_ms,
            followup_snapshot_with_deps_ms: trace.followup_snapshot_with_deps_ms,
            terminal_outcome: trace.terminal_outcome,
        })),
        gaps: [],
    };
}

function formatPublish(label: string, publish: DiagnosticsSaveTimelinePublishTrace | undefined): string {
    if (!publish) {
        return `${label}=none`;
    }

    const parts = [
        `${label}=${publish.profile}:${publish.publish_kind}:${publish.outcome}@${publish.elapsed_ms}ms`,
    ];
    if (typeof publish.blocking_queue_wait_ms === 'number') {
        parts.push(`blocking_queue_wait_ms=${publish.blocking_queue_wait_ms}`);
    }
    if (typeof publish.wait_for_file_version_ms === 'number') {
        parts.push(`wait_for_file_version_ms=${publish.wait_for_file_version_ms}`);
    }
    if (typeof publish.snapshot_with_deps_ms === 'number') {
        parts.push(`snapshot_with_deps_ms=${publish.snapshot_with_deps_ms}`);
    }
    if (typeof publish.syntax_diagnostics_query_ms === 'number') {
        parts.push(`syntax_diagnostics_query_ms=${publish.syntax_diagnostics_query_ms}`);
    }
    if (typeof publish.semantic_diagnostics_query_ms === 'number') {
        parts.push(`semantic_diagnostics_query_ms=${publish.semantic_diagnostics_query_ms}`);
    }
    if (typeof publish.publish_wait_ms === 'number') {
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
    reason: string | undefined,
    waitForFileVersionMs: number | undefined,
    snapshotWithDepsMs: number | undefined
): string | undefined {
    if (!reason) {
        return undefined;
    }

    const parts = [`followup_wait=${reason}`];
    if (typeof waitForFileVersionMs === 'number') {
        parts.push(`followup_wait_for_file_version_ms=${waitForFileVersionMs}`);
    }
    if (typeof snapshotWithDepsMs === 'number') {
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
            request.followup_wait_reason,
            request.followup_wait_for_file_version_ms,
            request.followup_snapshot_with_deps_ms
        ),
    ].filter((line): line is string => Boolean(line)));
}
