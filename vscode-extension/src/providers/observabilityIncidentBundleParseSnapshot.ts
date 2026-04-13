import { ObservabilityMetricsFetchResult } from '../lsp/customRequests';
import { ObservabilityIncidentDiagnosticsSaveSummary } from './observabilityIncidentBundleDiagnosticsSave';

export interface ObservabilityIncidentDidChangeParseSnapshotSummary {
    evidence_id: string;
    uri: string;
    requested_version: number;
    started_at_ms: number;
    parse_mode: string;
    base_text_source: string;
    change_shape: string;
    content_changes_count: number;
    replay_order: string;
    base_document_version?: number;
    changed_ranges_count: number;
    fallback_reason?: string;
    correlated_diagnostics_save_trace_ids: string[];
}

export interface ObservabilityIncidentDidChangeParseSnapshotSection {
    entryCount: number;
    entries: ObservabilityIncidentDidChangeParseSnapshotSummary[];
    gaps: string[];
}

export function buildObservabilityIncidentDidChangeParseSnapshotSection(
    observabilityMetrics: ObservabilityMetricsFetchResult,
    diagnosticsSaveRequests: ObservabilityIncidentDiagnosticsSaveSummary[]
): ObservabilityIncidentDidChangeParseSnapshotSection {
    if (observabilityMetrics.kind !== 'ok') {
        return {
            entryCount: 0,
            entries: [],
            gaps: [],
        };
    }

    const evidence = observabilityMetrics.response.didChangeParseSnapshotEvidence;
    if (!evidence) {
        return {
            entryCount: 0,
            entries: [],
            gaps: [
                'Observability metrics response does not expose version-bound didChange parse-snapshot evidence by design.',
            ],
        };
    }

    const entries = evidence.entries
        .filter((entry) => entry.parseMode === 'full' || typeof entry.fallbackReason === 'string')
        .map((entry) => ({
            evidence_id: entry.evidenceId,
            uri: entry.uri,
            requested_version: entry.requestedVersion,
            started_at_ms: entry.startedAtMs,
            parse_mode: entry.parseMode,
            base_text_source: entry.baseTextSource,
            change_shape: entry.changeShape,
            content_changes_count: entry.contentChangesCount,
            replay_order: entry.replayOrder,
            base_document_version: entry.baseDocumentVersion,
            changed_ranges_count: entry.changedRangesCount,
            fallback_reason: entry.fallbackReason,
            correlated_diagnostics_save_trace_ids: diagnosticsSaveRequests
                .filter(
                    (trace) =>
                        trace.uri === entry.uri
                        && trace.requested_version === entry.requestedVersion
                )
                .map((trace) => trace.trace_id),
        }));

    return {
        entryCount: entries.length,
        entries,
        gaps: [],
    };
}

export function renderDidChangeParseSnapshotSummaryLines(
    section: ObservabilityIncidentDidChangeParseSnapshotSection
): string[] {
    if (section.entries.length === 0) {
        return ['- No didChange parse-snapshot fallback evidence was recorded for this bundle.'];
    }

    return section.entries.map((entry) => {
        const parts = [
            `evidence=${entry.evidence_id}`,
            `uri=${entry.uri}`,
            `requested_version=${entry.requested_version}`,
            `parse_mode=${entry.parse_mode}`,
            `base_text_source=${entry.base_text_source}`,
            `change_shape=${entry.change_shape}`,
            `content_changes_count=${entry.content_changes_count}`,
            `replay_order=${entry.replay_order}`,
            `changed_ranges_count=${entry.changed_ranges_count}`,
        ];
        if (typeof entry.base_document_version === 'number') {
            parts.push(`base_document_version=${entry.base_document_version}`);
        }
        if (entry.fallback_reason) {
            parts.push(`fallback_reason=${entry.fallback_reason}`);
        }
        if (entry.correlated_diagnostics_save_trace_ids.length > 0) {
            parts.push(
                `correlated_diagnostics_save_traces=${entry.correlated_diagnostics_save_trace_ids.join(',')}`
            );
        }
        return `- ${parts.join(' | ')}`;
    });
}
