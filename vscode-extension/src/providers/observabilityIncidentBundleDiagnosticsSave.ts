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
    followup_semantic_path?: string;
    followup_semantic_parse_source?: string;
    followup_semantic_ir_source?: string;
    followup_semantic_attribution_note?: string;
    followup_ready_snapshot_zero_probe?: string;
    followup_ready_snapshot_wait_probe?: string;
    followup_ready_snapshot_task_state?: string;
    followup_ready_snapshot_timeout_phase?: string;
    followup_ready_snapshot_timeout_phase_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_ms?: number;
    followup_ready_snapshot_post_parse_pre_materialization_ms?: number;
    followup_ready_snapshot_ready_install_ms?: number;
    followup_ready_snapshot_document_symbol_side_work_ms?: number;
    followup_ready_snapshot_dominant_phase?: string;
    followup_ready_snapshot_dominant_phase_ms?: number;
    followup_ready_snapshot_relief_valve_outcome?: string;
    followup_ready_snapshot_relief_valve_budget_ms?: number;
    followup_ready_snapshot_relief_valve_elapsed_ms?: number;
    followup_shadow_state_available?: boolean;
    followup_ready_snapshot_attribution_note?: string;
    followup_ready_snapshot_phase_attribution_note?: string;
    followup_ready_snapshot_relief_valve_note?: string;
    followup_wait_reason?: string;
    followup_blocker_reason?: string;
    followup_blocker_note?: string;
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

    const semanticAttributionNote =
        diagnosticsSaveTimeline.response.version < 8
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const readySnapshotAttributionNote =
        diagnosticsSaveTimeline.response.version < 9
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const readySnapshotPhaseAttributionNote =
        diagnosticsSaveTimeline.response.version < 10
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const readySnapshotReliefValveNote =
        diagnosticsSaveTimeline.response.version < 11
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const blockerNote =
        diagnosticsSaveTimeline.response.version < 12
            ? `unavailable_by_design(version=${diagnosticsSaveTimeline.response.version})`
            : undefined;
    const gaps: string[] = [];
    if (diagnosticsSaveTimeline.response.version < 8) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose semantic path/source attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 9) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose ready-snapshot miss attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 10) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose ready-snapshot phase attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 11) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose ready-snapshot relief-valve attribution by design.`
        );
    }
    if (diagnosticsSaveTimeline.response.version < 12) {
        gaps.push(
            `Diagnostics save timeline v${diagnosticsSaveTimeline.response.version} does not expose follow-up blocker attribution by design.`
        );
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
            followup_semantic_path: trace.followup_semantic_path,
            followup_semantic_parse_source: trace.followup_semantic_parse_source,
            followup_semantic_ir_source: trace.followup_semantic_ir_source,
            followup_semantic_attribution_note: semanticAttributionNote,
            followup_ready_snapshot_zero_probe: trace.followup_ready_snapshot_zero_probe,
            followup_ready_snapshot_wait_probe: trace.followup_ready_snapshot_wait_probe,
            followup_ready_snapshot_task_state: trace.followup_ready_snapshot_task_state,
            followup_ready_snapshot_timeout_phase: trace.followup_ready_snapshot_timeout_phase,
            followup_ready_snapshot_timeout_phase_elapsed_ms:
                trace.followup_ready_snapshot_timeout_phase_elapsed_ms,
            followup_ready_snapshot_parse_exec_ms:
                trace.followup_ready_snapshot_parse_exec_ms,
            followup_ready_snapshot_post_parse_pre_materialization_ms:
                trace.followup_ready_snapshot_post_parse_pre_materialization_ms,
            followup_ready_snapshot_ready_install_ms:
                trace.followup_ready_snapshot_ready_install_ms,
            followup_ready_snapshot_document_symbol_side_work_ms:
                trace.followup_ready_snapshot_document_symbol_side_work_ms,
            followup_ready_snapshot_dominant_phase:
                trace.followup_ready_snapshot_dominant_phase,
            followup_ready_snapshot_dominant_phase_ms:
                trace.followup_ready_snapshot_dominant_phase_ms,
            followup_ready_snapshot_relief_valve_outcome:
                trace.followup_ready_snapshot_relief_valve_outcome,
            followup_ready_snapshot_relief_valve_budget_ms:
                trace.followup_ready_snapshot_relief_valve_budget_ms,
            followup_ready_snapshot_relief_valve_elapsed_ms:
                trace.followup_ready_snapshot_relief_valve_elapsed_ms,
            followup_shadow_state_available: trace.followup_shadow_state_available,
            followup_ready_snapshot_attribution_note: readySnapshotAttributionNote,
            followup_ready_snapshot_phase_attribution_note:
                readySnapshotPhaseAttributionNote,
            followup_ready_snapshot_relief_valve_note:
                readySnapshotReliefValveNote,
            followup_wait_reason: trace.followup_wait_reason,
            followup_blocker_reason: trace.followup_blocker_reason,
            followup_blocker_note: blockerNote,
            followup_runtime_queue_wait_ms: trace.followup_runtime_queue_wait_ms,
            followup_apply_lag_ms: trace.followup_apply_lag_ms,
            followup_wait_for_file_version_ms: trace.followup_wait_for_file_version_ms,
            followup_snapshot_with_deps_ms: trace.followup_snapshot_with_deps_ms,
            terminal_outcome: trace.terminal_outcome,
        })),
        gaps,
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
    blockerReason: string | undefined,
    blockerNote: string | undefined,
    runtimeQueueWaitMs: number | undefined,
    applyLagMs: number | undefined,
    waitForFileVersionMs: number | undefined,
    snapshotWithDepsMs: number | undefined
): string | undefined {
    if (
        !reason
        && !blockerReason
        && !blockerNote
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
    if (blockerNote) {
        parts.push(`followup_blocker=${blockerNote}`);
    }
    if (blockerReason) {
        parts.push(`followup_blocker=${blockerReason}`);
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

function formatFollowupSemanticAttribution(
    semanticAttributionNote: string | undefined,
    semanticPath: string | undefined,
    semanticParseSource: string | undefined,
    semanticIrSource: string | undefined
): string | undefined {
    if (semanticAttributionNote) {
        return `followup_semantic_attribution=${semanticAttributionNote}`;
    }
    if (!semanticPath && !semanticParseSource && !semanticIrSource) {
        return undefined;
    }

    const parts: string[] = [];
    if (semanticPath) {
        parts.push(`followup_semantic_path=${semanticPath}`);
    }
    if (semanticParseSource) {
        parts.push(`followup_semantic_parse_source=${semanticParseSource}`);
    }
    if (semanticIrSource) {
        parts.push(`followup_semantic_ir_source=${semanticIrSource}`);
    }
    return parts.join(' | ');
}

function formatFollowupReadySnapshotAttribution(
    attributionNote: string | undefined,
    zeroProbe: string | undefined,
    waitProbe: string | undefined,
    taskState: string | undefined,
    shadowStateAvailable: boolean | undefined
): string | undefined {
    if (attributionNote) {
        return `followup_ready_snapshot_miss_attribution=${attributionNote}`;
    }
    if (
        !zeroProbe
        && !waitProbe
        && !taskState
        && typeof shadowStateAvailable !== 'boolean'
    ) {
        return undefined;
    }

    const parts: string[] = [];
    if (zeroProbe) {
        parts.push(`followup_ready_snapshot_zero_probe=${zeroProbe}`);
    }
    if (waitProbe) {
        parts.push(`followup_ready_snapshot_wait_probe=${waitProbe}`);
    }
    if (taskState) {
        parts.push(`followup_ready_snapshot_task_state=${taskState}`);
    }
    if (typeof shadowStateAvailable === 'boolean') {
        parts.push(`followup_shadow_state_available=${shadowStateAvailable}`);
    }
    return parts.join(' | ');
}

function formatFollowupReadySnapshotPhases(
    attributionNote: string | undefined,
    timeoutPhase: string | undefined,
    timeoutPhaseElapsedMs: number | undefined,
    parseExecMs: number | undefined,
    postParsePreMaterializationMs: number | undefined,
    readyInstallMs: number | undefined,
    documentSymbolSideWorkMs: number | undefined,
    dominantPhase: string | undefined,
    dominantPhaseMs: number | undefined
): string | undefined {
    if (attributionNote) {
        return `followup_ready_snapshot_phase_attribution=${attributionNote}`;
    }
    if (
        !timeoutPhase
        && !isPositiveTimingValue(timeoutPhaseElapsedMs)
        && !isPositiveTimingValue(parseExecMs)
        && !isPositiveTimingValue(postParsePreMaterializationMs)
        && !isPositiveTimingValue(readyInstallMs)
        && !isPositiveTimingValue(documentSymbolSideWorkMs)
        && !dominantPhase
        && !isPositiveTimingValue(dominantPhaseMs)
    ) {
        return undefined;
    }

    const parts: string[] = [];
    if (timeoutPhase) {
        parts.push(`followup_ready_snapshot_timeout_phase=${timeoutPhase}`);
    }
    if (isPositiveTimingValue(timeoutPhaseElapsedMs)) {
        parts.push(
            `followup_ready_snapshot_timeout_phase_elapsed_ms=${timeoutPhaseElapsedMs}`
        );
    }
    if (isPositiveTimingValue(parseExecMs)) {
        parts.push(`followup_ready_snapshot_parse_exec_ms=${parseExecMs}`);
    }
    if (isPositiveTimingValue(postParsePreMaterializationMs)) {
        parts.push(
            `followup_ready_snapshot_post_parse_pre_materialization_ms=${postParsePreMaterializationMs}`
        );
    }
    if (isPositiveTimingValue(readyInstallMs)) {
        parts.push(`followup_ready_snapshot_ready_install_ms=${readyInstallMs}`);
    }
    if (isPositiveTimingValue(documentSymbolSideWorkMs)) {
        parts.push(
            `followup_ready_snapshot_document_symbol_side_work_ms=${documentSymbolSideWorkMs}`
        );
    }
    if (dominantPhase) {
        parts.push(`followup_ready_snapshot_dominant_phase=${dominantPhase}`);
    }
    if (isPositiveTimingValue(dominantPhaseMs)) {
        parts.push(`followup_ready_snapshot_dominant_phase_ms=${dominantPhaseMs}`);
    }
    return parts.join(' | ');
}

function formatFollowupReadySnapshotReliefValve(
    note: string | undefined,
    outcome: string | undefined,
    budgetMs: number | undefined,
    elapsedMs: number | undefined
): string | undefined {
    if (note) {
        return `followup_ready_snapshot_relief_valve=${note}`;
    }
    if (!outcome && !isPositiveTimingValue(budgetMs) && !isPositiveTimingValue(elapsedMs)) {
        return undefined;
    }

    const parts: string[] = [];
    if (outcome) {
        parts.push(`followup_ready_snapshot_relief_valve_outcome=${outcome}`);
    }
    if (isPositiveTimingValue(budgetMs)) {
        parts.push(`followup_ready_snapshot_relief_valve_budget_ms=${budgetMs}`);
    }
    if (isPositiveTimingValue(elapsedMs)) {
        parts.push(`followup_ready_snapshot_relief_valve_elapsed_ms=${elapsedMs}`);
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
        formatFollowupSemanticAttribution(
            request.followup_semantic_attribution_note,
            request.followup_semantic_path,
            request.followup_semantic_parse_source,
            request.followup_semantic_ir_source
        ),
        formatFollowupReadySnapshotAttribution(
            request.followup_ready_snapshot_attribution_note,
            request.followup_ready_snapshot_zero_probe,
            request.followup_ready_snapshot_wait_probe,
            request.followup_ready_snapshot_task_state,
            request.followup_shadow_state_available
        ),
        formatFollowupReadySnapshotPhases(
            request.followup_ready_snapshot_phase_attribution_note,
            request.followup_ready_snapshot_timeout_phase,
            request.followup_ready_snapshot_timeout_phase_elapsed_ms,
            request.followup_ready_snapshot_parse_exec_ms,
            request.followup_ready_snapshot_post_parse_pre_materialization_ms,
            request.followup_ready_snapshot_ready_install_ms,
            request.followup_ready_snapshot_document_symbol_side_work_ms,
            request.followup_ready_snapshot_dominant_phase,
            request.followup_ready_snapshot_dominant_phase_ms
        ),
        formatFollowupReadySnapshotReliefValve(
            request.followup_ready_snapshot_relief_valve_note,
            request.followup_ready_snapshot_relief_valve_outcome,
            request.followup_ready_snapshot_relief_valve_budget_ms,
            request.followup_ready_snapshot_relief_valve_elapsed_ms
        ),
        formatFollowupWait(
            request.followup_syntax_work_mode,
            request.followup_wait_reason,
            request.followup_blocker_reason,
            request.followup_blocker_note,
            request.followup_runtime_queue_wait_ms,
            request.followup_apply_lag_ms,
            request.followup_wait_for_file_version_ms,
            request.followup_snapshot_with_deps_ms
        ),
    ].filter((line): line is string => Boolean(line)));
}
