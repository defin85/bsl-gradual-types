import {
    CompletionTimelineFetchResult,
    ObservabilityMetricsFetchResult,
} from '../lsp/customRequests';
import { CompletionProbe } from './completionProbe';
import { buildCompletionTraceBottleneckVerdicts } from './completionTimelineDrilldown';
import {
    ObservabilityIncidentCaptureScope,
    ObservabilityIncidentRequestSection,
    ObservabilityIncidentRequestSummary,
    buildObservabilityIncidentRequestSection,
    renderRequestScopeLine,
    renderRequestSummaryLines,
} from './observabilityIncidentBundleRequests';

const BUNDLE_FORMAT = 'bsl-observability-incident/v1';
const COMPLETION_TIMELINE_RAW_PATH = 'raw/completion_timeline.json';
const CLIENT_PROBES_RAW_PATH = 'raw/client_probes.json';
const OBSERVABILITY_METRICS_RAW_PATH = 'raw/observability_metrics.json';

type IncidentSourceStatus = 'available' | 'unsupported' | 'unavailable';

type IncidentSourceClassification =
    | 'authoritative_server_trace'
    | 'local_only_client_probes'
    | 'cumulative_metrics_snapshot';

export interface ObservabilityIncidentBundleInput {
    capturedAtMs: number;
    completionTimeline: CompletionTimelineFetchResult;
    completionTraceLimit: number;
    clientProbes: CompletionProbe[];
    observabilityMetrics: ObservabilityMetricsFetchResult;
}

export interface ObservabilityIncidentBundleFile {
    relativePath: string;
    contents: string;
}

export interface ObservabilityIncidentBundleSource {
    classification: IncidentSourceClassification;
    status: IncidentSourceStatus;
    raw_attachment?: string;
    trace_count?: number;
    probe_count?: number;
    contract_version?: number;
    uptime_seconds?: number;
    message?: string;
}

export interface ObservabilityIncidentBundleReport {
    bundle_format: string;
    captured_at: string;
    capture_scope: ObservabilityIncidentCaptureScope;
    request_window: {
        completion_trace_limit: number;
        request_count: number;
    };
    requests: ObservabilityIncidentRequestSummary[];
    sources: {
        completion_timeline: ObservabilityIncidentBundleSource;
        client_probes: ObservabilityIncidentBundleSource;
        observability_metrics: ObservabilityIncidentBundleSource;
    };
    findings: string[];
    gaps: string[];
    raw_attachments: Array<{
        path: string;
        section: 'completion_timeline' | 'client_probes' | 'observability_metrics';
        classification: IncidentSourceClassification;
    }>;
}

export interface ObservabilityIncidentBundle {
    folderName: string;
    files: ObservabilityIncidentBundleFile[];
    incidentReport: ObservabilityIncidentBundleReport;
    summaryMarkdown: string;
}

export function buildObservabilityIncidentBundle(
    input: ObservabilityIncidentBundleInput
): ObservabilityIncidentBundle {
    const capturedAtIso = new Date(input.capturedAtMs).toISOString();
    const rawAttachments: ObservabilityIncidentBundleReport['raw_attachments'] = [];
    const files: ObservabilityIncidentBundleFile[] = [];
    const gaps: string[] = [];
    const requestSection = buildObservabilityIncidentRequestSection(
        input.completionTimeline,
        input.clientProbes
    );
    const findings = deriveFindings(input, requestSection);

    const completionTimelineSource = buildCompletionTimelineSource(
        input.completionTimeline,
        rawAttachments,
        gaps,
        files
    );
    const clientProbesSource = buildClientProbeSource(input.clientProbes, rawAttachments, files);
    const observabilityMetricsSource = buildObservabilityMetricsSource(
        input.observabilityMetrics,
        rawAttachments,
        gaps,
        files
    );

    const incidentReport: ObservabilityIncidentBundleReport = {
        bundle_format: BUNDLE_FORMAT,
        captured_at: capturedAtIso,
        capture_scope: requestSection.captureScope,
        request_window: {
            completion_trace_limit: input.completionTraceLimit,
            request_count: requestSection.requestCount,
        },
        requests: requestSection.requests,
        sources: {
            completion_timeline: completionTimelineSource,
            client_probes: clientProbesSource,
            observability_metrics: observabilityMetricsSource,
        },
        findings: findings.length > 0 ? findings : ['No derived bottleneck heuristic matched this capture window.'],
        gaps: [...gaps, ...requestSection.gaps],
        raw_attachments: rawAttachments,
    };

    const summaryMarkdown = renderSummaryMarkdown(incidentReport);
    files.unshift(
        {
            relativePath: 'summary.md',
            contents: summaryMarkdown,
        },
        {
            relativePath: 'incident.json',
            contents: `${JSON.stringify(incidentReport, null, 2)}\n`,
        }
    );

    return {
        folderName: buildBundleFolderName(input.capturedAtMs),
        files,
        incidentReport,
        summaryMarkdown,
    };
}

function buildCompletionTimelineSource(
    completionTimeline: CompletionTimelineFetchResult,
    rawAttachments: ObservabilityIncidentBundleReport['raw_attachments'],
    gaps: string[],
    files: ObservabilityIncidentBundleFile[]
): ObservabilityIncidentBundleSource {
    if (completionTimeline.kind === 'ok') {
        if (completionTimeline.response.version < 7) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include all v7 pre-method and snapshot overshoot attribution fields; those facts are unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 8) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include trustworthy v8 pre-method attribution provenance; strong ingress findings are unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 9) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include bounded v9 pre-service-scope split fields; pre-service-scope split is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 10) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include bounded v10 dispatch split fields; dispatch split is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 11) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include bounded v11 first-poll / first-wake split fields; first-poll / first-wake split is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 12) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include bounded v12 first-poll contention attribution; contention attribution is unavailable by design.`
            );
        }
        rawAttachments.push({
            path: COMPLETION_TIMELINE_RAW_PATH,
            section: 'completion_timeline',
            classification: 'authoritative_server_trace',
        });
        files.push({
            relativePath: COMPLETION_TIMELINE_RAW_PATH,
            contents: `${JSON.stringify(completionTimeline.response, null, 2)}\n`,
        });
        return {
            classification: 'authoritative_server_trace',
            status: 'available',
            raw_attachment: COMPLETION_TIMELINE_RAW_PATH,
            trace_count: completionTimeline.response.traces.length,
            contract_version: completionTimeline.response.version,
        };
    }

    if (completionTimeline.kind === 'unsupported') {
        gaps.push('Completion timeline is unsupported by the connected server.');
        return {
            classification: 'authoritative_server_trace',
            status: 'unsupported',
            message: 'Connected server does not support bsl.getCompletionTimeline.',
        };
    }

    gaps.push(`Completion timeline is unavailable: ${completionTimeline.message}`);
    return {
        classification: 'authoritative_server_trace',
        status: 'unavailable',
        message: completionTimeline.message,
    };
}

function buildClientProbeSource(
    clientProbes: CompletionProbe[],
    rawAttachments: ObservabilityIncidentBundleReport['raw_attachments'],
    files: ObservabilityIncidentBundleFile[]
): ObservabilityIncidentBundleSource {
    rawAttachments.push({
        path: CLIENT_PROBES_RAW_PATH,
        section: 'client_probes',
        classification: 'local_only_client_probes',
    });
    files.push({
        relativePath: CLIENT_PROBES_RAW_PATH,
        contents: `${JSON.stringify(clientProbes, null, 2)}\n`,
    });
    return {
        classification: 'local_only_client_probes',
        status: 'available',
        raw_attachment: CLIENT_PROBES_RAW_PATH,
        probe_count: clientProbes.length,
    };
}

function buildObservabilityMetricsSource(
    observabilityMetrics: ObservabilityMetricsFetchResult,
    rawAttachments: ObservabilityIncidentBundleReport['raw_attachments'],
    gaps: string[],
    files: ObservabilityIncidentBundleFile[]
): ObservabilityIncidentBundleSource {
    if (observabilityMetrics.kind === 'unsupported') {
        gaps.push('Observability metrics snapshot is unsupported by the connected server.');
        return {
            classification: 'cumulative_metrics_snapshot',
            status: 'unsupported',
            message: 'Connected server does not support bsl.getObservabilityMetrics.',
        };
    }

    if (observabilityMetrics.kind === 'error') {
        gaps.push(`Observability metrics snapshot is unavailable for this bundle: ${observabilityMetrics.message}`);
        return {
            classification: 'cumulative_metrics_snapshot',
            status: 'unavailable',
            message: observabilityMetrics.message,
        };
    }

    const metrics = asRecord(observabilityMetrics.response.metrics);
    rawAttachments.push({
        path: OBSERVABILITY_METRICS_RAW_PATH,
        section: 'observability_metrics',
        classification: 'cumulative_metrics_snapshot',
    });
    files.push({
        relativePath: OBSERVABILITY_METRICS_RAW_PATH,
        contents: `${JSON.stringify(observabilityMetrics.response, null, 2)}\n`,
    });
    return {
        classification: 'cumulative_metrics_snapshot',
        status: 'available',
        raw_attachment: OBSERVABILITY_METRICS_RAW_PATH,
        uptime_seconds: typeof metrics?.uptime_seconds === 'number' ? metrics.uptime_seconds : undefined,
    };
}

function deriveFindings(
    input: ObservabilityIncidentBundleInput,
    requestSection: ObservabilityIncidentRequestSection
): string[] {
    const findings: string[] = [];
    if (input.completionTimeline.kind === 'ok') {
        const traces = input.completionTimeline.response.traces;
        if (input.completionTimeline.response.version < 7) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v7 pre-method and snapshot overshoot attribution details are unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 8) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but trustworthy v8 pre-method attribution provenance is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 9) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but bounded v9 pre-service-scope split is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 10) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but bounded v10 dispatch split is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 11) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but bounded v11 first-poll / first-wake split is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 12) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but bounded v12 first-poll contention attribution is unavailable by design.`
            );
        }
        const serverBeforeMethodCount = requestSection.requests.filter((request) =>
            request.bottleneck_verdicts.includes('server_before_method_entry_dominant')
        ).length;
        if (serverBeforeMethodCount > 0) {
            findings.push(
                `server-side ingress before method entry dominated ${serverBeforeMethodCount} completion trace(s).`
            );
        }

        const clientBeforeTransportCount = requestSection.requests.filter((request) =>
            request.bottleneck_verdicts.includes('client_before_transport_dominant')
        ).length;
        if (clientBeforeTransportCount > 0) {
            findings.push(
                `client-side ingress dominated ${clientBeforeTransportCount} completion trace(s).`
            );
        }

        const handlerPreludeDominantCount = requestSection.requests.filter((request) =>
            request.bottleneck_verdicts.includes('handler_prelude_dominant')
        ).length;
        if (handlerPreludeDominantCount > 0) {
            findings.push(
                `${handlerPreludeDominantCount} completion trace(s) were bottlenecked inside handler prelude before completion stages started.`
            );
        }

        const prepareTimeoutTraces = traces.filter(
            (trace) => trace.prepare_details?.fail_closed_cause === 'prepare_timeout'
        );
        if (prepareTimeoutTraces.length > 0) {
            const verdicts = new Set(
                prepareTimeoutTraces
                    .map((trace) => buildCompletionTraceBottleneckVerdicts(trace))
                    .flat()
                    .filter((verdict) => verdict.startsWith('prepare_timeout@'))
            );
            findings.push(
                `prepare_timeout was observed in ${prepareTimeoutTraces.length} completion trace(s): ${[...verdicts].join(', ')}.`
            );
        }

        const exactDeadlineFindings = traces
            .filter((trace) => trace.prepare_details?.fail_closed_cause === 'exact_deadline')
            .map((trace) => buildCompletionTraceBottleneckVerdicts(trace))
            .flat()
            .filter((verdict) => verdict.startsWith('exact_deadline'));
        if (exactDeadlineFindings.length > 0) {
            findings.push(
                `exact_deadline was observed after prepare completed: ${[...new Set(exactDeadlineFindings)].join('; ')}.`
            );
        }

        if (findings.length === 0) {
            findings.push(`${traces.length} completion trace(s) were captured from the authoritative server timeline.`);
        }
    } else if (input.clientProbes.length > 0) {
        findings.push('Bundle contains local client probes but no authoritative server trace for the current capture window.');
    }

    const ambiguousCorrelationCount = requestSection.requests.filter(
        (request) => request.client_correlation.status === 'ambiguous'
    ).length;
    if (ambiguousCorrelationCount > 0) {
        findings.push(
            `client/server correlation was ambiguous for ${ambiguousCorrelationCount} completion trace(s) in this capture window.`
        );
    }

    const semanticDiagnosticsP95 = getHistogramPercentile(
        input.observabilityMetrics,
        'intellisense_v2_semantic_diagnostics_query_ms',
        'p95'
    );
    if (semanticDiagnosticsP95 !== null) {
        findings.push(`semantic diagnostics p95=${Math.round(semanticDiagnosticsP95)}ms in the captured metrics snapshot.`);
    }

    return findings;
}

function renderSummaryMarkdown(report: ObservabilityIncidentBundleReport): string {
    const lines: string[] = [
        '# Observability Incident Bundle',
        '',
        `Captured at: ${report.captured_at}`,
        `Bundle format: ${report.bundle_format}`,
        `Completion trace limit: ${report.request_window.completion_trace_limit}`,
        '',
        '## Request Scope',
        renderRequestScopeLine({
            captureScope: report.capture_scope,
            requestCount: report.request_window.request_count,
            requests: report.requests,
            gaps: [],
        }),
        '',
        '## Request Summary',
        ...renderRequestSummaryLines({
            captureScope: report.capture_scope,
            requestCount: report.request_window.request_count,
            requests: report.requests,
            gaps: [],
        }),
        '',
        '## Source Status',
        renderSourceStatusLine('Completion timeline', report.sources.completion_timeline),
        renderSourceStatusLine('Client probes', report.sources.client_probes),
        renderSourceStatusLine('Observability metrics', report.sources.observability_metrics),
        '',
        '## Findings',
        ...renderBulletSection(report.findings),
        '',
        '## Gaps',
        ...renderBulletSection(
            report.gaps.length > 0 ? report.gaps : ['No gaps were recorded for this bundle.']
        ),
        '',
        '## Raw Attachments',
        ...renderBulletSection(report.raw_attachments.map((attachment) => attachment.path)),
        '',
        '## Notes',
        '- Completion timeline is the only authoritative server trace in this bundle.',
        '- Client probes are local-only extension data and never substitute server stages, routes, or outcomes.',
        '- Observability metrics are cumulative process snapshots, not per-request traces.',
        '',
    ];
    return lines.join('\n');
}

function renderSourceStatusLine(
    label: string,
    source: ObservabilityIncidentBundleSource
): string {
    const details: string[] = [
        `status=${source.status}`,
        `classification=${source.classification}`,
    ];
    if (typeof source.trace_count === 'number') {
        details.push(`trace_count=${source.trace_count}`);
    }
    if (typeof source.probe_count === 'number') {
        details.push(`probe_count=${source.probe_count}`);
    }
    if (typeof source.contract_version === 'number') {
        details.push(`contract=v${source.contract_version}`);
    }
    if (typeof source.uptime_seconds === 'number') {
        details.push(`uptime_seconds=${source.uptime_seconds}`);
    }
    if (source.raw_attachment) {
        details.push(`raw=${source.raw_attachment}`);
    }
    if (source.message) {
        details.push(`message=${source.message}`);
    }
    return `- ${label}: ${details.join(' | ')}`;
}

function renderBulletSection(values: string[]): string[] {
    return values.map((value) => `- ${value}`);
}

function getHistogramPercentile(
    observabilityMetrics: ObservabilityMetricsFetchResult,
    histogramName: string,
    percentile: 'p50' | 'p95' | 'p99'
): number | null {
    if (observabilityMetrics.kind !== 'ok') {
        return null;
    }
    const metrics = asRecord(observabilityMetrics.response.metrics);
    const histograms = asRecord(metrics?.histograms);
    const histogram = asRecord(histograms?.[histogramName]);
    const value = histogram?.[percentile];
    return typeof value === 'number' ? value : null;
}

function asRecord(value: unknown): Record<string, any> | null {
    if (!value || typeof value !== 'object') {
        return null;
    }
    return value as Record<string, any>;
}

function buildBundleFolderName(capturedAtMs: number): string {
    const suffix = new Date(capturedAtMs)
        .toISOString()
        .replace(/:/g, '-')
        .replace(/\.\d{3}Z$/, 'Z');
    return `bsl-observability-incident-${suffix}`;
}
