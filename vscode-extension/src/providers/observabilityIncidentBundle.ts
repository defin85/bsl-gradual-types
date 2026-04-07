import {
    CompletionTimelineFetchResult,
    DiagnosticsSaveTimelineFetchResult,
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
import {
    ObservabilityIncidentDiagnosticsSaveSummary,
    buildObservabilityIncidentDiagnosticsSaveSection,
    renderDiagnosticsSaveSummaryLines,
} from './observabilityIncidentBundleDiagnosticsSave';

const BUNDLE_FORMAT = 'bsl-observability-incident/v1';
const COMPLETION_TIMELINE_RAW_PATH = 'raw/completion_timeline.json';
const DIAGNOSTICS_SAVE_TIMELINE_RAW_PATH = 'raw/diagnostics_save_timeline.json';
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
    diagnosticsSaveTimeline?: DiagnosticsSaveTimelineFetchResult;
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
    diagnostics_save_window: {
        request_count: number;
    };
    diagnostics_save_requests: ObservabilityIncidentDiagnosticsSaveSummary[];
    sources: {
        completion_timeline: ObservabilityIncidentBundleSource;
        diagnostics_save_timeline: ObservabilityIncidentBundleSource;
        client_probes: ObservabilityIncidentBundleSource;
        observability_metrics: ObservabilityIncidentBundleSource;
    };
    findings: string[];
    gaps: string[];
    raw_attachments: Array<{
        path: string;
        section:
            | 'completion_timeline'
            | 'diagnostics_save_timeline'
            | 'client_probes'
            | 'observability_metrics';
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
    const diagnosticsSaveTimeline: DiagnosticsSaveTimelineFetchResult =
        input.diagnosticsSaveTimeline ?? { kind: 'unsupported' };
    const rawAttachments: ObservabilityIncidentBundleReport['raw_attachments'] = [];
    const files: ObservabilityIncidentBundleFile[] = [];
    const gaps: string[] = [];
    const requestSection = buildObservabilityIncidentRequestSection(
        input.completionTimeline,
        input.clientProbes
    );
    const diagnosticsSaveSection = buildObservabilityIncidentDiagnosticsSaveSection(
        diagnosticsSaveTimeline
    );
    const findings = deriveFindings(input, requestSection);

    const completionTimelineSource = buildCompletionTimelineSource(
        input.completionTimeline,
        rawAttachments,
        gaps,
        files
    );
    const diagnosticsSaveTimelineSource = buildDiagnosticsSaveTimelineSource(
        diagnosticsSaveTimeline,
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
        diagnostics_save_window: {
            request_count: diagnosticsSaveSection.requestCount,
        },
        diagnostics_save_requests: diagnosticsSaveSection.requests,
        sources: {
            completion_timeline: completionTimelineSource,
            diagnostics_save_timeline: diagnosticsSaveTimelineSource,
            client_probes: clientProbesSource,
            observability_metrics: observabilityMetricsSource,
        },
        findings: findings.length > 0 ? findings : ['No derived bottleneck heuristic matched this capture window.'],
        gaps: [...gaps, ...requestSection.gaps, ...diagnosticsSaveSection.gaps],
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
        if (completionTimeline.response.version < 24) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include truthful v24 pre-enqueue handoff split; handoff-start / handoff-enqueued / writer-selection separation is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 22) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include finer v22 output-egress split; enqueue/queue/encode/write+flush detail is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 23) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include truthful v23 output-egress boundary; encode-start vs literal write-start detail is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 21) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v21 flush-aware post-handler egress split; response_ready_to_flush_wait_ms is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 20) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include truthful v20 grouped query-body split; detailed query-body breakdown is unavailable by design.`
            );
        }
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
        if (completionTimeline.response.version < 13) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v13 first-poll contender snapshot; exact inflight contender list is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 14) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v14 executeCommand command detail inside first-poll contenders; exact executeCommand subcommand is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 15) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v15 completion phase detail inside first-poll contenders; exact inflight completion stage is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 16) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v16 turn-wait resolution detail; exact turn_wait entered/resolved/wake timestamps are unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 17) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v17 transport slot release detail; exact handoff boundary between ingress and off-transport wait is unavailable by design.`
            );
        }
        if (completionTimeline.response.version < 18) {
            gaps.push(
                `Completion timeline contract v${completionTimeline.response.version} does not include v18 request-bound client probe correlation detail; deterministic probe-to-trace matching is unavailable by design.`
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

function buildDiagnosticsSaveTimelineSource(
    diagnosticsSaveTimeline: DiagnosticsSaveTimelineFetchResult,
    rawAttachments: ObservabilityIncidentBundleReport['raw_attachments'],
    gaps: string[],
    files: ObservabilityIncidentBundleFile[]
): ObservabilityIncidentBundleSource {
    if (diagnosticsSaveTimeline.kind === 'ok') {
        rawAttachments.push({
            path: DIAGNOSTICS_SAVE_TIMELINE_RAW_PATH,
            section: 'diagnostics_save_timeline',
            classification: 'authoritative_server_trace',
        });
        files.push({
            relativePath: DIAGNOSTICS_SAVE_TIMELINE_RAW_PATH,
            contents: `${JSON.stringify(diagnosticsSaveTimeline.response, null, 2)}\n`,
        });
        return {
            classification: 'authoritative_server_trace',
            status: 'available',
            raw_attachment: DIAGNOSTICS_SAVE_TIMELINE_RAW_PATH,
            trace_count: diagnosticsSaveTimeline.response.traces.length,
            contract_version: diagnosticsSaveTimeline.response.version,
        };
    }

    if (diagnosticsSaveTimeline.kind === 'unsupported') {
        gaps.push('Diagnostics save timeline is unsupported by the connected server.');
        return {
            classification: 'authoritative_server_trace',
            status: 'unsupported',
            message: 'Connected server does not support bsl.getDiagnosticsSaveTimeline.',
        };
    }

    gaps.push(`Diagnostics save timeline is unavailable: ${diagnosticsSaveTimeline.message}`);
    return {
        classification: 'authoritative_server_trace',
        status: 'unavailable',
        message: diagnosticsSaveTimeline.message,
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
        if (input.completionTimeline.response.version < 24) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but truthful v24 pre-enqueue handoff split is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 22) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but finer v22 output-egress split is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 23) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but truthful v23 output-egress boundary is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 21) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v21 flush-aware post-handler egress split is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 20) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but truthful v20 grouped query-body split is unavailable by design.`
            );
        }
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
        if (input.completionTimeline.response.version < 13) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v13 first-poll contender snapshot is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 14) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v14 executeCommand command detail inside first-poll contenders is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 15) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v15 completion phase detail inside first-poll contenders is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 16) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v16 turn-wait resolution detail is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 17) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v17 transport slot release detail is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 18) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v18 request-bound client probe correlation detail is unavailable by design.`
            );
        }
        if (input.completionTimeline.response.version < 19) {
            findings.push(
                `Completion timeline contract v${input.completionTimeline.response.version} is available, but v19 adapter ingress pre-dispatch split is unavailable by design.`
            );
        }
        const queryBundleDominantCount = requestSection.requests.filter((request) =>
            request.bottleneck_verdicts.includes('query_bundle_dominant')
        ).length;
        if (queryBundleDominantCount > 0) {
            findings.push(
                `authoritative query-body dominance was observed in ${queryBundleDominantCount} completion trace(s).`
            );
        }
        const queryBundleLeafFindings: Array<[string, string]> = [
            ['query_bundle_pool_wait_dominant', 'bounded blocking pool wait'],
            ['query_bundle_deps_and_file_snapshot_dominant', 'query-body deps/file snapshot work'],
            ['query_bundle_owner_hint_dominant', 'query-body owner-hint work'],
            ['query_bundle_ir_query_dominant', 'query-body IR query work'],
            ['query_bundle_ir_retry_dominant', 'query-body IR retry work'],
            ['query_bundle_other_dominant', 'unclassified query-body remainder'],
        ];
        for (const [verdict, label] of queryBundleLeafFindings) {
            const count = requestSection.requests.filter((request) =>
                request.bottleneck_verdicts.includes(verdict)
            ).length;
            if (count > 0) {
                findings.push(`${label} dominated ${count} completion trace(s).`);
            }
        }
        const adapterBeforeDispatchCount = requestSection.requests.filter((request) =>
            request.bottleneck_verdicts.includes('adapter_before_dispatch_dominant')
        ).length;
        if (adapterBeforeDispatchCount > 0) {
            findings.push(
                `server-side pre-dispatch adapter backlog dominated ${adapterBeforeDispatchCount} completion trace(s).`
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

    const correlatedLegacyProbeCount = requestSection.requests.filter(
        (request) =>
            request.client_correlation.status === 'correlated'
            && request.client_correlation.raw_transport_receive_state === 'unavailable'
    ).length;
    if (correlatedLegacyProbeCount > 0) {
        findings.push(
            `raw transport receive boundary was unavailable for ${correlatedLegacyProbeCount} correlated completion trace(s); transport_to_client_receive_wait_ms and client_receive_to_resolve_wait_ms remain unavailable on legacy probe paths.`
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
        '## Diagnostics Save Summary',
        ...renderDiagnosticsSaveSummaryLines({
            requestCount: report.diagnostics_save_window.request_count,
            requests: report.diagnostics_save_requests,
            gaps: [],
        }),
        '',
        '## Source Status',
        renderSourceStatusLine('Completion timeline', report.sources.completion_timeline),
        renderSourceStatusLine('Diagnostics save timeline', report.sources.diagnostics_save_timeline),
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
        '- Completion timeline remains the authoritative completion trace in this bundle.',
        '- Diagnostics save timeline is an authoritative per-didSave server trace when supported.',
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
