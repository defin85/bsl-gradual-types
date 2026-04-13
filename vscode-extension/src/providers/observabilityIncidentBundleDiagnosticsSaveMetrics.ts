import { ObservabilityMetricsFetchResult } from '../lsp/customRequests';

type DiagnosticsSaveMetricsSectionStatus = 'available' | 'unsupported' | 'unavailable';

export interface ObservabilityIncidentDiagnosticsSaveMaterializationSummary {
    source: string;
    count: number;
    p50_ms?: number;
    p95_ms?: number;
    p99_ms?: number;
}

export interface ObservabilityIncidentDiagnosticsSaveProbeOutcomeSummary {
    outcome: string;
    count: number;
    p50_ms?: number;
    p95_ms?: number;
    p99_ms?: number;
}

export interface ObservabilityIncidentDiagnosticsSaveProbeSummary {
    slot: string;
    outcomes: ObservabilityIncidentDiagnosticsSaveProbeOutcomeSummary[];
}

export interface ObservabilityIncidentDiagnosticsSaveStartedSummary {
    source: string;
    count: number;
}

export interface ObservabilityIncidentDiagnosticsSaveWorkerTerminationReasonSummary {
    reason: string;
    count: number;
    p50_ms?: number;
    p95_ms?: number;
    p99_ms?: number;
}

export interface ObservabilityIncidentDiagnosticsSaveWorkerTerminationSummary {
    source: string;
    reasons: ObservabilityIncidentDiagnosticsSaveWorkerTerminationReasonSummary[];
}

export interface ObservabilityIncidentDiagnosticsSaveCountSummary {
    name: string;
    count: number;
}

export interface ObservabilityIncidentDiagnosticsSaveMetricsSection {
    status: DiagnosticsSaveMetricsSectionStatus;
    ready_snapshot_worker_started: ObservabilityIncidentDiagnosticsSaveStartedSummary[];
    ready_snapshot_worker_terminated_without_materialization: ObservabilityIncidentDiagnosticsSaveWorkerTerminationSummary[];
    ready_snapshot_worker_in_flight_estimate: ObservabilityIncidentDiagnosticsSaveStartedSummary[];
    materialization: ObservabilityIncidentDiagnosticsSaveMaterializationSummary[];
    ready_snapshot_probe: ObservabilityIncidentDiagnosticsSaveProbeSummary[];
    followup_wait_state: ObservabilityIncidentDiagnosticsSaveCountSummary[];
    followup_semantic_path: ObservabilityIncidentDiagnosticsSaveCountSummary[];
    message?: string;
    gaps: string[];
}

const READY_SNAPSHOT_WORKER_STARTED_COUNTER_PREFIX =
    'intellisense_v2_ready_parse_snapshot_worker_started_total_origin_lsp_source_';
const READY_SNAPSHOT_WORKER_TERMINATION_COUNTER_PREFIX =
    'intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_total_origin_lsp_source_';
const READY_SNAPSHOT_WORKER_TERMINATION_HISTOGRAM_PREFIX =
    'intellisense_v2_ready_parse_snapshot_worker_terminated_without_materialization_ms_origin_lsp_source_';
const READY_SNAPSHOT_MATERIALIZATION_COUNTER_PREFIX =
    'intellisense_v2_ready_parse_snapshot_materialization_total_origin_lsp_source_';
const READY_SNAPSHOT_MATERIALIZATION_HISTOGRAM_PREFIX =
    'intellisense_v2_ready_parse_snapshot_materialization_ms_origin_lsp_source_';
const READY_SNAPSHOT_PROBE_COUNTER_PREFIX =
    'intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_total_slot_';
const READY_SNAPSHOT_PROBE_HISTOGRAM_PREFIX =
    'intellisense_v2_diagnostics_save_followup_ready_snapshot_probe_ms_slot_';
const FOLLOWUP_WAIT_STATE_COUNTER_PREFIX =
    'intellisense_v2_diagnostics_save_followup_wait_state_total_reason_';
const FOLLOWUP_SEMANTIC_PATH_COUNTER_PREFIX =
    'intellisense_v2_diagnostics_save_followup_semantic_path_total_path_';

const WORKER_SOURCE_ORDER = ['did_change', 'did_save', 'did_open', 'other'];
const WORKER_TERMINATION_REASON_ORDER = [
    'aborted',
    'superseded',
    'latest_version_mismatch',
    'build_snapshot_aborted',
    'other',
];
const MATERIALIZATION_SOURCE_ORDER = ['did_change', 'did_save', 'did_open', 'other'];
const PROBE_SLOT_ORDER = ['bounded_wait', 'zero_budget', 'other'];
const PROBE_OUTCOME_ORDER = [
    'timeout',
    'not_ready',
    'ready',
    'generation_mismatch',
    'version_mismatch',
    'cancelled',
    'superseded',
    'other',
];
const FOLLOWUP_WAIT_STATE_ORDER = [
    'apply_lag',
    'runtime_queue_wait',
    'semantic_work',
    'pending_publish',
    'other',
];
const FOLLOWUP_SEMANTIC_PATH_ORDER = [
    'ready_artifacts',
    'shadow_state',
    'generic_pipeline',
    'other',
];

export function buildObservabilityIncidentDiagnosticsSaveMetricsSection(
    observabilityMetrics: ObservabilityMetricsFetchResult
): ObservabilityIncidentDiagnosticsSaveMetricsSection {
    if (observabilityMetrics.kind === 'unsupported') {
        return {
            status: 'unsupported',
            ready_snapshot_worker_started: [],
            ready_snapshot_worker_terminated_without_materialization: [],
            ready_snapshot_worker_in_flight_estimate: [],
            materialization: [],
            ready_snapshot_probe: [],
            followup_wait_state: [],
            followup_semantic_path: [],
            message: 'Connected server does not support bsl.getObservabilityMetrics.',
            gaps: [],
        };
    }

    if (observabilityMetrics.kind === 'error') {
        return {
            status: 'unavailable',
            ready_snapshot_worker_started: [],
            ready_snapshot_worker_terminated_without_materialization: [],
            ready_snapshot_worker_in_flight_estimate: [],
            materialization: [],
            ready_snapshot_probe: [],
            followup_wait_state: [],
            followup_semantic_path: [],
            message: observabilityMetrics.message,
            gaps: [],
        };
    }

    const metrics = asRecord(observabilityMetrics.response.metrics);
    const counters = asRecord(metrics?.counters);
    const histograms = asRecord(metrics?.histograms);
    const readySnapshotWorkerStarted = collectStartedSummaries(
        counters,
        READY_SNAPSHOT_WORKER_STARTED_COUNTER_PREFIX,
        WORKER_SOURCE_ORDER
    );
    const readySnapshotWorkerTerminatedWithoutMaterialization =
        collectWorkerTerminationSummaries(counters, histograms);
    const materialization = collectMaterializationSummaries(counters, histograms);

    return {
        status: 'available',
        ready_snapshot_worker_started: readySnapshotWorkerStarted,
        ready_snapshot_worker_terminated_without_materialization:
            readySnapshotWorkerTerminatedWithoutMaterialization,
        ready_snapshot_worker_in_flight_estimate: collectInFlightEstimateSummaries(
            readySnapshotWorkerStarted,
            materialization,
            readySnapshotWorkerTerminatedWithoutMaterialization
        ),
        materialization,
        ready_snapshot_probe: collectReadySnapshotProbeSummaries(counters, histograms),
        followup_wait_state: collectCountSummaries(
            counters,
            FOLLOWUP_WAIT_STATE_COUNTER_PREFIX,
            FOLLOWUP_WAIT_STATE_ORDER
        ),
        followup_semantic_path: collectCountSummaries(
            counters,
            FOLLOWUP_SEMANTIC_PATH_COUNTER_PREFIX,
            FOLLOWUP_SEMANTIC_PATH_ORDER
        ),
        gaps: [],
    };
}

export function renderDiagnosticsSaveMetricsSummaryLines(
    section: ObservabilityIncidentDiagnosticsSaveMetricsSection
): string[] {
    if (section.status === 'unsupported') {
        return [
            `- Diagnostics-save aggregate metrics are unsupported by the connected server${section.message ? `: ${section.message}` : '.'}`,
        ];
    }
    if (section.status === 'unavailable') {
        return [
            `- Diagnostics-save aggregate metrics are unavailable${section.message ? `: ${section.message}` : '.'}`,
        ];
    }

    if (
        section.ready_snapshot_worker_started.length === 0
        && section.ready_snapshot_worker_terminated_without_materialization.length === 0
        && section.ready_snapshot_worker_in_flight_estimate.length === 0
        && section.materialization.length === 0
        && section.ready_snapshot_probe.length === 0
        && section.followup_wait_state.length === 0
        && section.followup_semantic_path.length === 0
    ) {
        return ['- No diagnostics-save aggregate metrics were recorded in this bundle.'];
    }

    const lines: string[] = [];
    if (section.followup_wait_state.length > 0) {
        lines.push(
            `- followup_wait_state | ${section.followup_wait_state
                .map((entry) => `${entry.name}=${entry.count}`)
                .join(' | ')}`
        );
    }
    if (section.ready_snapshot_probe.length > 0) {
        lines.push(
            ...section.ready_snapshot_probe.map((slot) => {
                const outcomeParts = slot.outcomes.map((outcome) => {
                    const parts = [`${outcome.outcome}=${outcome.count}`];
                    if (typeof outcome.p95_ms === 'number') {
                        parts.push(`p95_ms=${Math.round(outcome.p95_ms)}`);
                    }
                    return parts.join(' ');
                });
                return `- ready_snapshot_probe | slot=${slot.slot} | ${outcomeParts.join(' | ')}`;
            })
        );
    }
    if (section.followup_semantic_path.length > 0) {
        lines.push(
            `- followup_semantic_path | ${section.followup_semantic_path
                .map((entry) => `${entry.name}=${entry.count}`)
                .join(' | ')}`
        );
    }
    if (section.ready_snapshot_worker_started.length > 0) {
        lines.push(
            `- ready_snapshot_worker_started | ${section.ready_snapshot_worker_started
                .map((entry) => `${entry.source}=${entry.count}`)
                .join(' | ')}`
        );
    }
    if (section.ready_snapshot_worker_terminated_without_materialization.length > 0) {
        lines.push(
            ...section.ready_snapshot_worker_terminated_without_materialization.map((entry) => {
                const reasonParts = entry.reasons.map((reason) => {
                    const parts = [`${reason.reason}=${reason.count}`];
                    if (typeof reason.p95_ms === 'number') {
                        parts.push(`p95_ms=${Math.round(reason.p95_ms)}`);
                    }
                    return parts.join(' ');
                });
                return `- ready_snapshot_worker_terminated_without_materialization | source=${entry.source} | ${reasonParts.join(' | ')}`;
            })
        );
    }
    if (section.ready_snapshot_worker_in_flight_estimate.length > 0) {
        lines.push(
            `- ready_snapshot_worker_in_flight_estimate | ${section.ready_snapshot_worker_in_flight_estimate
                .map((entry) => `${entry.source}=${entry.count}`)
                .join(' | ')}`
        );
    }
    if (section.materialization.length > 0) {
        lines.push(
            ...section.materialization.map((entry) => {
                const parts = [
                    '- ready_snapshot_materialization',
                    `source=${entry.source}`,
                    `count=${entry.count}`,
                ];
                if (typeof entry.p50_ms === 'number') {
                    parts.push(`p50_ms=${Math.round(entry.p50_ms)}`);
                }
                if (typeof entry.p95_ms === 'number') {
                    parts.push(`p95_ms=${Math.round(entry.p95_ms)}`);
                }
                if (typeof entry.p99_ms === 'number') {
                    parts.push(`p99_ms=${Math.round(entry.p99_ms)}`);
                }
                return parts.join(' | ');
            })
        );
    }
    return lines;
}

function collectStartedSummaries(
    counters: Record<string, unknown> | null,
    prefix: string,
    preferredOrder: string[]
): ObservabilityIncidentDiagnosticsSaveStartedSummary[] {
    return Object.entries(counters ?? {})
        .flatMap(([key, value]) => {
            if (!key.startsWith(prefix)) {
                return [];
            }
            const count = asNumber(value);
            if (count === null || count <= 0) {
                return [];
            }
            return [
                {
                    source: key.slice(prefix.length),
                    count,
                },
            ];
        })
        .sort((left, right) => compareOrderedNames(left.source, right.source, preferredOrder));
}

function collectWorkerTerminationSummaries(
    counters: Record<string, unknown> | null,
    histograms: Record<string, unknown> | null
): ObservabilityIncidentDiagnosticsSaveWorkerTerminationSummary[] {
    const sources = new Map<
        string,
        Map<string, ObservabilityIncidentDiagnosticsSaveWorkerTerminationReasonSummary>
    >();
    for (const [key, value] of Object.entries(counters ?? {})) {
        if (!key.startsWith(READY_SNAPSHOT_WORKER_TERMINATION_COUNTER_PREFIX)) {
            continue;
        }
        const suffix = key.slice(READY_SNAPSHOT_WORKER_TERMINATION_COUNTER_PREFIX.length);
        const splitIndex = suffix.indexOf('_reason_');
        if (splitIndex === -1) {
            continue;
        }
        const source = suffix.slice(0, splitIndex);
        const reason = suffix.slice(splitIndex + '_reason_'.length);
        const count = asNumber(value);
        if (count === null || count <= 0) {
            continue;
        }
        const reasonMap = sources.get(source) ?? new Map();
        reasonMap.set(reason, {
            reason,
            count,
        });
        sources.set(source, reasonMap);
    }
    for (const [key, value] of Object.entries(histograms ?? {})) {
        if (!key.startsWith(READY_SNAPSHOT_WORKER_TERMINATION_HISTOGRAM_PREFIX)) {
            continue;
        }
        const suffix = key.slice(READY_SNAPSHOT_WORKER_TERMINATION_HISTOGRAM_PREFIX.length);
        const splitIndex = suffix.indexOf('_reason_');
        if (splitIndex === -1) {
            continue;
        }
        const source = suffix.slice(0, splitIndex);
        const reason = suffix.slice(splitIndex + '_reason_'.length);
        const histogram = asRecord(value);
        const reasonMap = sources.get(source) ?? new Map();
        const existing = reasonMap.get(reason) ?? {
            reason,
            count: asNumber(histogram?.count) ?? 0,
        };
        if (typeof histogram?.count === 'number') {
            existing.count = histogram.count;
        }
        existing.p50_ms = asNumber(histogram?.p50) ?? existing.p50_ms;
        existing.p95_ms = asNumber(histogram?.p95) ?? existing.p95_ms;
        existing.p99_ms = asNumber(histogram?.p99) ?? existing.p99_ms;
        if (
            hasPositiveCountOrLatency(
                existing.count,
                existing.p50_ms,
                existing.p95_ms,
                existing.p99_ms
            )
        ) {
            reasonMap.set(reason, existing);
            sources.set(source, reasonMap);
        }
    }
    return Array.from(sources.entries())
        .map(([source, reasons]) => ({
            source,
            reasons: Array.from(reasons.values()).sort((left, right) =>
                compareOrderedNames(left.reason, right.reason, WORKER_TERMINATION_REASON_ORDER)
            ),
        }))
        .filter((entry) => entry.reasons.length > 0)
        .sort((left, right) => compareOrderedNames(left.source, right.source, WORKER_SOURCE_ORDER));
}

function collectMaterializationSummaries(
    counters: Record<string, unknown> | null,
    histograms: Record<string, unknown> | null
): ObservabilityIncidentDiagnosticsSaveMaterializationSummary[] {
    const entries = new Map<string, ObservabilityIncidentDiagnosticsSaveMaterializationSummary>();
    for (const [key, value] of Object.entries(counters ?? {})) {
        if (!key.startsWith(READY_SNAPSHOT_MATERIALIZATION_COUNTER_PREFIX)) {
            continue;
        }
        const source = key.slice(READY_SNAPSHOT_MATERIALIZATION_COUNTER_PREFIX.length);
        const count = asNumber(value);
        if (count === null || count <= 0) {
            continue;
        }
        entries.set(source, {
            source,
            count,
        });
    }
    for (const [key, value] of Object.entries(histograms ?? {})) {
        if (!key.startsWith(READY_SNAPSHOT_MATERIALIZATION_HISTOGRAM_PREFIX)) {
            continue;
        }
        const source = key.slice(READY_SNAPSHOT_MATERIALIZATION_HISTOGRAM_PREFIX.length);
        const histogram = asRecord(value);
        const existing = entries.get(source) ?? {
            source,
            count: asNumber(histogram?.count) ?? 0,
        };
        existing.p50_ms = asNumber(histogram?.p50) ?? existing.p50_ms;
        existing.p95_ms = asNumber(histogram?.p95) ?? existing.p95_ms;
        existing.p99_ms = asNumber(histogram?.p99) ?? existing.p99_ms;
        if (typeof histogram?.count === 'number') {
            existing.count = histogram.count;
        }
        if (
            hasPositiveCountOrLatency(
                existing.count,
                existing.p50_ms,
                existing.p95_ms,
                existing.p99_ms
            )
        ) {
            entries.set(source, existing);
        }
    }
    return Array.from(entries.values())
        .filter((entry) =>
            hasPositiveCountOrLatency(entry.count, entry.p50_ms, entry.p95_ms, entry.p99_ms)
        )
        .sort((left, right) => compareOrderedNames(left.source, right.source, MATERIALIZATION_SOURCE_ORDER));
}

function collectReadySnapshotProbeSummaries(
    counters: Record<string, unknown> | null,
    histograms: Record<string, unknown> | null
): ObservabilityIncidentDiagnosticsSaveProbeSummary[] {
    const slots = new Map<
        string,
        Map<string, ObservabilityIncidentDiagnosticsSaveProbeOutcomeSummary>
    >();
    for (const [key, value] of Object.entries(counters ?? {})) {
        if (!key.startsWith(READY_SNAPSHOT_PROBE_COUNTER_PREFIX)) {
            continue;
        }
        const suffix = key.slice(READY_SNAPSHOT_PROBE_COUNTER_PREFIX.length);
        const splitIndex = suffix.indexOf('_outcome_');
        if (splitIndex === -1) {
            continue;
        }
        const slot = suffix.slice(0, splitIndex);
        const outcome = suffix.slice(splitIndex + '_outcome_'.length);
        const count = asNumber(value);
        if (count === null || count <= 0) {
            continue;
        }
        const outcomeMap = slots.get(slot) ?? new Map();
        outcomeMap.set(outcome, {
            outcome,
            count,
        });
        slots.set(slot, outcomeMap);
    }
    for (const [key, value] of Object.entries(histograms ?? {})) {
        if (!key.startsWith(READY_SNAPSHOT_PROBE_HISTOGRAM_PREFIX)) {
            continue;
        }
        const suffix = key.slice(READY_SNAPSHOT_PROBE_HISTOGRAM_PREFIX.length);
        const splitIndex = suffix.indexOf('_outcome_');
        if (splitIndex === -1) {
            continue;
        }
        const slot = suffix.slice(0, splitIndex);
        const outcome = suffix.slice(splitIndex + '_outcome_'.length);
        const histogram = asRecord(value);
        const outcomeMap = slots.get(slot) ?? new Map();
        const existing = outcomeMap.get(outcome) ?? {
            outcome,
            count: asNumber(histogram?.count) ?? 0,
        };
        if (typeof histogram?.count === 'number') {
            existing.count = histogram.count;
        }
        existing.p50_ms = asNumber(histogram?.p50) ?? existing.p50_ms;
        existing.p95_ms = asNumber(histogram?.p95) ?? existing.p95_ms;
        existing.p99_ms = asNumber(histogram?.p99) ?? existing.p99_ms;
        if (
            hasPositiveCountOrLatency(
                existing.count,
                existing.p50_ms,
                existing.p95_ms,
                existing.p99_ms
            )
        ) {
            outcomeMap.set(outcome, existing);
            slots.set(slot, outcomeMap);
        }
    }
    return Array.from(slots.entries())
        .map(([slot, outcomes]) => ({
            slot,
            outcomes: Array.from(outcomes.values()).sort((left, right) =>
                compareOrderedNames(left.outcome, right.outcome, PROBE_OUTCOME_ORDER)
            ),
        }))
        .filter((entry) => entry.outcomes.length > 0)
        .sort((left, right) => compareOrderedNames(left.slot, right.slot, PROBE_SLOT_ORDER));
}

function collectInFlightEstimateSummaries(
    started: ObservabilityIncidentDiagnosticsSaveStartedSummary[],
    materialization: ObservabilityIncidentDiagnosticsSaveMaterializationSummary[],
    terminated: ObservabilityIncidentDiagnosticsSaveWorkerTerminationSummary[]
): ObservabilityIncidentDiagnosticsSaveStartedSummary[] {
    const materializedBySource = new Map(
        materialization.map((entry) => [entry.source, entry.count])
    );
    const terminatedBySource = new Map(
        terminated.map((entry) => [
            entry.source,
            entry.reasons.reduce((sum, reason) => sum + reason.count, 0),
        ])
    );
    return started
        .map((entry) => ({
            source: entry.source,
            count: Math.max(
                0,
                entry.count
                    - (materializedBySource.get(entry.source) ?? 0)
                    - (terminatedBySource.get(entry.source) ?? 0)
            ),
        }))
        .filter((entry) => entry.count > 0)
        .sort((left, right) => compareOrderedNames(left.source, right.source, WORKER_SOURCE_ORDER));
}

function collectCountSummaries(
    counters: Record<string, unknown> | null,
    prefix: string,
    preferredOrder: string[]
): ObservabilityIncidentDiagnosticsSaveCountSummary[] {
    return Object.entries(counters ?? {})
        .flatMap(([key, value]) => {
            if (!key.startsWith(prefix)) {
                return [];
            }
            const count = asNumber(value);
            if (count === null || count <= 0) {
                return [];
            }
            return [
                {
                    name: key.slice(prefix.length),
                    count,
                },
            ];
        })
        .sort((left, right) => compareOrderedNames(left.name, right.name, preferredOrder));
}

function compareOrderedNames(left: string, right: string, preferredOrder: string[]): number {
    const leftIndex = preferredOrder.indexOf(left);
    const rightIndex = preferredOrder.indexOf(right);
    if (leftIndex !== -1 || rightIndex !== -1) {
        if (leftIndex === -1) {
            return 1;
        }
        if (rightIndex === -1) {
            return -1;
        }
        return leftIndex - rightIndex;
    }
    return left.localeCompare(right);
}

function asRecord(value: unknown): Record<string, any> | null {
    if (!value || typeof value !== 'object') {
        return null;
    }
    return value as Record<string, any>;
}

function asNumber(value: unknown): number | null {
    return typeof value === 'number' ? value : null;
}

function hasPositiveCountOrLatency(
    count: number,
    p50_ms?: number,
    p95_ms?: number,
    p99_ms?: number
): boolean {
    return count > 0 || (p50_ms ?? 0) > 0 || (p95_ms ?? 0) > 0 || (p99_ms ?? 0) > 0;
}
