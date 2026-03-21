import * as assert from 'assert';
import {
    CompletionTimelineFetchResult,
    ObservabilityMetricsFetchResult,
    ObservabilityMetricsResponse,
} from '../../lsp/customRequests';
import { CompletionProbe } from '../../providers/completionProbe';
import { buildObservabilityIncidentBundle } from '../../providers/observabilityIncidentBundle';

suite('Observability Incident Bundle Test Suite', () => {
    function sampleProbe(overrides: Partial<CompletionProbe> = {}): CompletionProbe {
        return {
            probe_id: 'probe-1',
            uri: 'file:///tmp/test.bsl',
            document_version: 9,
            document_version_at_terminal: 9,
            trigger_mode: 'invoked',
            request_started_at_ms: 1_700_000_000_000,
            lsp_request_started_at_ms: 1_700_000_000_005,
            lsp_response_received_at_ms: 1_700_000_000_020,
            request_completed_at_ms: 1_700_000_000_021,
            client_duration_ms: 21,
            client_terminal_state: 'ok_non_empty',
            cancel_reason_hint: 'unknown',
            result_kind: 'non_empty',
            item_count_bucket: '21_plus',
            time_since_last_local_edit_ms: 25,
            time_since_last_did_change_sent_ms: 24,
            did_change_count_during_probe: 0,
            cursor_moved_during_probe: false,
            active_completion_count_at_start: 0,
            same_uri_probe_overlap_count: 0,
            newer_probe_started_before_terminal: false,
            is_after_dot: true,
            identifier_tail_length: 0,
            ...overrides,
        };
    }

    function sampleTimeline(): CompletionTimelineFetchResult {
        return {
            kind: 'ok',
            response: {
                version: 9,
                traces: [
                    {
                        trace_id: 'trace-1',
                        request_id: 'req-1',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'fail_closed',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 172,
                        dominant_stage: 'wait_exact_type_index',
                        prepare_details: {
                            fail_closed_cause: 'exact_deadline',
                            exact_wait: {
                                artifact_poll: {
                                    poll_count: 14,
                                    poll_elapsed_ms: 155,
                                    observed_file_version: 9,
                                    head_ready: false,
                                    exact_ready: false,
                                },
                                type_index_waiter_action: 'promoted',
                                matching_task_state: 'matching',
                                task_phase: 'waiting_cpu_permit',
                            },
                        },
                        server_edge_details: {
                            transport_received_at_ms: 1_700_000_000_000,
                            service_future_created_at_ms: 1_700_000_001_200,
                            pre_method_attribution_provenance: 'same_request_authoritative',
                            service_scope_entered_at_ms: 1_700_000_002_000,
                            method_entered_at_ms: 1_700_000_003_000,
                            handler_entered_at_ms: 1_700_000_003_000,
                            response_sent_at_ms: 1_700_000_003_172,
                            transport_to_service_future_wait_ms: 1200,
                            service_future_to_scope_wait_ms: 800,
                            transport_to_service_scope_wait_ms: 2000,
                            service_scope_to_method_wait_ms: 1000,
                            transport_to_method_wait_ms: 3000,
                            method_prelude_exec_ms: 0,
                            transport_to_handler_wait_ms: 3000,
                            server_handler_exec_ms: 172,
                        },
                        stages: [
                            {
                                name: 'wait_exact_type_index',
                                status: 'completed',
                                started_offset_ms: 0,
                                duration_ms: 125,
                            },
                        ],
                    },
                    {
                        trace_id: 'trace-2',
                        request_id: 'req-2',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'fail_closed',
                        started_at_ms: 1_700_000_010_000,
                        total_duration_ms: 2996,
                        dominant_stage: 'prepare_stateful',
                        prepare_details: {
                            fail_closed_cause: 'prepare_timeout',
                            timeout_attribution: {
                                source: 'prepare_guard',
                                phase: 'snapshot_with_deps',
                                budget_ms: 120,
                                elapsed_ms: 2996,
                                overshoot_ms: 2876,
                            },
                            progress: {
                                phase: 'snapshot_with_deps',
                                wait_completed_offset_ms: 19,
                            },
                            snapshot_with_deps_timeout_runtime: {
                                queue_wait_ms: 17,
                                exec_ms: 22,
                                wake_wait_ms: 2837,
                                resolution: 'wake_wait',
                            },
                        },
                        server_edge_details: {
                            transport_received_at_ms: 1_700_000_010_010,
                            pre_method_attribution_provenance: 'same_request_authoritative',
                            method_entered_at_ms: 1_700_000_010_015,
                            handler_entered_at_ms: 1_700_000_010_015,
                            response_sent_at_ms: 1_700_000_012_996,
                            transport_to_method_wait_ms: 5,
                            method_prelude_exec_ms: 0,
                            transport_to_handler_wait_ms: 5,
                            server_handler_exec_ms: 2981,
                        },
                        stages: [
                            {
                                name: 'prepare_stateful',
                                status: 'failed',
                                started_offset_ms: 0,
                                duration_ms: 2996,
                            },
                        ],
                    },
                ],
            },
        };
    }

    function sampleMetrics(): ObservabilityMetricsFetchResult {
        return {
            kind: 'ok',
            response: {
                metrics: {
                    uptime_seconds: 184,
                    histograms: {
                        intellisense_v2_semantic_diagnostics_query_ms: {
                            p95: 3374,
                        },
                    },
                },
            } as ObservabilityMetricsResponse,
        };
    }

    test('happy path bundle should contain request-centric incident report and all raw attachments', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: sampleTimeline(),
            completionTraceLimit: 50,
            clientProbes: [
                sampleProbe({
                    probe_id: 'probe-1',
                    trigger_mode: 'invoked',
                    request_started_at_ms: 1_700_000_000_000,
                    lsp_request_started_at_ms: 1_700_000_000_000,
                    lsp_response_received_at_ms: 1_700_000_003_173,
                    request_completed_at_ms: 1_700_000_003_174,
                    client_duration_ms: 3174,
                }),
                sampleProbe({
                    probe_id: 'probe-2',
                    trigger_mode: 'invoked',
                    request_started_at_ms: 1_700_000_010_010,
                    lsp_request_started_at_ms: 1_700_000_010_010,
                    lsp_response_received_at_ms: 1_700_000_012_997,
                    request_completed_at_ms: 1_700_000_012_999,
                    client_duration_ms: 2989,
                }),
            ],
            observabilityMetrics: sampleMetrics(),
        });

        assert.strictEqual(bundle.folderName, 'bsl-observability-incident-2026-03-19T10-23-21Z');
        assert.deepStrictEqual(
            bundle.files.map((file) => file.relativePath),
            [
                'summary.md',
                'incident.json',
                'raw/completion_timeline.json',
                'raw/client_probes.json',
                'raw/observability_metrics.json',
            ]
        );
        assert.strictEqual(bundle.incidentReport.sources.completion_timeline.status, 'available');
        assert.strictEqual(bundle.incidentReport.sources.completion_timeline.contract_version, 9);
        assert.strictEqual(bundle.incidentReport.sources.client_probes.probe_count, 2);
        assert.strictEqual(bundle.incidentReport.sources.observability_metrics.uptime_seconds, 184);
        assert.deepStrictEqual(bundle.incidentReport.capture_scope, {
            kind: 'single_uri',
            uri: 'file:///tmp/test.bsl',
            uri_count: 1,
        });
        assert.strictEqual(bundle.incidentReport.request_window.request_count, 2);
        assert.strictEqual(bundle.incidentReport.requests.length, 2);
        assert.ok(bundle.incidentReport.requests[0].bottleneck_verdicts.includes('exact_deadline@artifact_poll'));
        assert.ok(bundle.incidentReport.requests[0].bottleneck_verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(!bundle.incidentReport.requests[0].bottleneck_verdicts.includes('client_before_transport_dominant'));
        assert.strictEqual(bundle.incidentReport.requests[0].client_correlation?.status, 'correlated');
        assert.strictEqual(bundle.incidentReport.requests[0].client_correlation?.probe_id, 'probe-1');
        assert.strictEqual(bundle.incidentReport.requests[0].client_correlation?.client_to_transport_wait_ms, 0);
        assert.strictEqual(bundle.incidentReport.requests[0].service_future_created_at_ms, 1_700_000_001_200);
        assert.strictEqual(bundle.incidentReport.requests[0].transport_to_service_future_wait_ms, 1200);
        assert.strictEqual(bundle.incidentReport.requests[0].service_future_to_scope_wait_ms, 800);
        assert.strictEqual(bundle.incidentReport.requests[0].transport_to_service_scope_wait_ms, 2000);
        assert.strictEqual(bundle.incidentReport.requests[0].service_scope_to_method_wait_ms, 1000);
        assert.strictEqual(bundle.incidentReport.requests[1].prepare_timeout?.source, 'prepare_guard');
        assert.strictEqual(
            bundle.incidentReport.requests[1].snapshot_with_deps_timeout_runtime?.resolution,
            'wake_wait'
        );
        assert.strictEqual(bundle.incidentReport.requests[1].client_correlation?.status, 'correlated');
        assert.strictEqual(bundle.incidentReport.requests[1].client_correlation?.probe_id, 'probe-2');
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('prepare_timeout was observed in 1 completion trace(s): prepare_timeout@prepare_guard')));
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('exact_deadline was observed after prepare completed: exact_deadline@artifact_poll')));
        assert.ok(
            bundle.incidentReport.findings.some((finding) => finding.includes('semantic diagnostics p95=3374ms'))
        );
        assert.ok(bundle.summaryMarkdown.includes('## Request Scope'));
        assert.ok(bundle.summaryMarkdown.includes('scope=single_uri | uri=file:///tmp/test.bsl | request_count=2'));
        assert.ok(bundle.summaryMarkdown.includes('## Request Summary'));
        assert.ok(bundle.summaryMarkdown.includes('trace-1 | request=req-1'));
        assert.ok(bundle.summaryMarkdown.includes('pre_method_provenance=same_request_authoritative'));
        assert.ok(bundle.summaryMarkdown.includes('service_future_created_at_ms=1700000001200'));
        assert.ok(bundle.summaryMarkdown.includes('transport_to_service_future_wait_ms=1200'));
        assert.ok(bundle.summaryMarkdown.includes('service_future_to_scope_wait_ms=800'));
        assert.ok(bundle.summaryMarkdown.includes('transport_to_service_scope_wait_ms=2000'));
        assert.ok(bundle.summaryMarkdown.includes('service_scope_to_method_wait_ms=1000'));
        assert.ok(
            bundle.summaryMarkdown.includes(
                'snapshot_with_deps_timeout_runtime=wake_wait|queue_wait_ms=17|exec_ms=22|wake_wait_ms=2837'
            )
        );
        assert.ok(bundle.summaryMarkdown.includes('correlation=correlated:probe-1'));
        assert.ok(bundle.summaryMarkdown.includes('raw/completion_timeline.json'));
    });

    test('unsupported completion timeline should produce partial bundle without fabricated raw trace', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: { kind: 'unsupported' },
            completionTraceLimit: 50,
            clientProbes: [sampleProbe()],
            observabilityMetrics: sampleMetrics(),
        });

        assert.strictEqual(bundle.incidentReport.sources.completion_timeline.status, 'unsupported');
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('unsupported')));
        assert.ok(
            !bundle.files.some((file) => file.relativePath === 'raw/completion_timeline.json'),
            'unsupported server timeline must not create a fake raw attachment'
        );
        assert.ok(bundle.summaryMarkdown.includes('status=unsupported'));
    });

    test('v7 completion timeline should stay valid and mark v8 provenance details as unavailable', () => {
        const timeline = sampleTimeline();
        if (timeline.kind !== 'ok') {
            throw new Error('expected ok timeline fixture');
        }
        timeline.response.version = 7;
        timeline.response.traces[0].server_edge_details = {
            transport_received_at_ms: 1_700_000_000_000,
            method_entered_at_ms: 1_700_000_003_000,
            handler_entered_at_ms: 1_700_000_003_000,
            response_sent_at_ms: 1_700_000_003_172,
            transport_to_method_wait_ms: 3000,
            method_prelude_exec_ms: 0,
            transport_to_handler_wait_ms: 3000,
            server_handler_exec_ms: 172,
        };
        timeline.response.traces[1].prepare_details = {
            fail_closed_cause: 'prepare_timeout',
            timeout_attribution: {
                source: 'prepare_guard',
                phase: 'snapshot_with_deps',
                budget_ms: 120,
                elapsed_ms: 2996,
                overshoot_ms: 2876,
            },
            progress: {
                phase: 'snapshot_with_deps',
            },
        };

        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: timeline,
            completionTraceLimit: 50,
            clientProbes: [sampleProbe()],
            observabilityMetrics: sampleMetrics(),
        });

        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('contract v7')));
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('contract v7')));
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('prepare_timeout was observed in 1 completion trace(s): prepare_timeout@prepare_guard')));
        assert.strictEqual(bundle.incidentReport.request_window.request_count, 2);
        assert.strictEqual(bundle.incidentReport.requests[0].pre_method_attribution_provenance, undefined);
        assert.strictEqual(bundle.incidentReport.requests[1].snapshot_with_deps_timeout_runtime, undefined);
        assert.strictEqual(bundle.incidentReport.requests[0].client_correlation?.status, 'unavailable');
        assert.ok(!bundle.incidentReport.requests[0].bottleneck_verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(!bundle.incidentReport.requests[0].bottleneck_verdicts.includes('client_before_transport_dominant'));
    });

    test('v8 completion timeline should mark v9 pre-service-scope split as unavailable by design', () => {
        const timeline = sampleTimeline();
        if (timeline.kind !== 'ok') {
            throw new Error('expected ok timeline fixture');
        }
        timeline.response.version = 8;
        timeline.response.traces[0].server_edge_details = {
            ...timeline.response.traces[0].server_edge_details!,
            service_future_created_at_ms: undefined,
            transport_to_service_future_wait_ms: undefined,
            service_future_to_scope_wait_ms: undefined,
        };

        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: timeline,
            completionTraceLimit: 50,
            clientProbes: [sampleProbe()],
            observabilityMetrics: sampleMetrics(),
        });

        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('contract v8')));
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('pre-service-scope split')));
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('contract v8')));
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('pre-service-scope split')));
        assert.strictEqual(bundle.incidentReport.requests[0].service_future_created_at_ms, undefined);
        assert.strictEqual(bundle.incidentReport.requests[0].transport_to_service_future_wait_ms, undefined);
        assert.strictEqual(bundle.incidentReport.requests[0].service_future_to_scope_wait_ms, undefined);
        assert.ok(bundle.summaryMarkdown.includes('contract=v8'));
        assert.ok(bundle.summaryMarkdown.includes('pre-service-scope split is unavailable by design'));
        assert.ok(!bundle.summaryMarkdown.includes('No gaps were recorded for this bundle.'));
    });

    test('correlated request should expose client-before-transport verdict when client wait dominates', () => {
        const timeline = sampleTimeline();
        if (timeline.kind !== 'ok') {
            throw new Error('expected ok timeline fixture');
        }
        timeline.response.traces = [
            {
                ...timeline.response.traces[0],
                server_edge_details: {
                    transport_received_at_ms: 1_700_000_000_100,
                    pre_method_attribution_provenance: 'same_request_authoritative',
                    method_entered_at_ms: 1_700_000_000_140,
                    handler_entered_at_ms: 1_700_000_000_140,
                    response_sent_at_ms: 1_700_000_000_220,
                    transport_to_method_wait_ms: 40,
                    method_prelude_exec_ms: 0,
                    transport_to_handler_wait_ms: 40,
                    server_handler_exec_ms: 80,
                },
            },
        ];

        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: timeline,
            completionTraceLimit: 50,
            clientProbes: [
                sampleProbe({
                    probe_id: 'probe-client',
                    trigger_mode: 'invoked',
                    request_started_at_ms: 1_700_000_000_000,
                    lsp_request_started_at_ms: 1_700_000_000_000,
                    lsp_response_received_at_ms: 1_700_000_000_221,
                    request_completed_at_ms: 1_700_000_000_222,
                    client_duration_ms: 222,
                }),
            ],
            observabilityMetrics: sampleMetrics(),
        });

        assert.ok(bundle.incidentReport.requests[0].bottleneck_verdicts.includes('client_before_transport_dominant'));
        assert.ok(bundle.incidentReport.requests[0].bottleneck_verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(
            bundle.incidentReport.findings.some((finding) =>
                finding.includes('client-side ingress dominated 1 completion trace(s)')
            )
        );
        assert.ok(
            bundle.incidentReport.findings.some((finding) =>
                finding.includes('server-side ingress before method entry dominated 1 completion trace(s)')
            )
        );
    });

    test('best-effort pre-method provenance should stay visible but not aggregate as strong ingress finding', () => {
        const timeline = sampleTimeline();
        if (timeline.kind !== 'ok') {
            throw new Error('expected ok timeline fixture');
        }
        timeline.response.traces = [
            {
                ...timeline.response.traces[0],
                server_edge_details: {
                    transport_received_at_ms: 1_700_000_000_100,
                    pre_method_attribution_provenance: 'best_effort_fallback',
                    method_entered_at_ms: 1_700_000_000_140,
                    handler_entered_at_ms: 1_700_000_000_140,
                    response_sent_at_ms: 1_700_000_000_220,
                    transport_to_method_wait_ms: 40,
                    method_prelude_exec_ms: 0,
                    transport_to_handler_wait_ms: 40,
                    server_handler_exec_ms: 80,
                },
            },
        ];

        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: timeline,
            completionTraceLimit: 50,
            clientProbes: [
                sampleProbe({
                    probe_id: 'probe-best-effort',
                    trigger_mode: 'invoked',
                    request_started_at_ms: 1_700_000_000_000,
                    lsp_request_started_at_ms: 1_700_000_000_000,
                    lsp_response_received_at_ms: 1_700_000_000_221,
                    request_completed_at_ms: 1_700_000_000_222,
                    client_duration_ms: 222,
                }),
            ],
            observabilityMetrics: sampleMetrics(),
        });

        assert.strictEqual(
            bundle.incidentReport.requests[0].pre_method_attribution_provenance,
            'best_effort_fallback'
        );
        assert.ok(!bundle.incidentReport.requests[0].bottleneck_verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(!bundle.incidentReport.requests[0].bottleneck_verdicts.includes('client_before_transport_dominant'));
        assert.ok(
            !bundle.incidentReport.findings.some((finding) =>
                finding.includes('server-side ingress before method entry dominated')
            )
        );
        assert.ok(
            !bundle.incidentReport.findings.some((finding) =>
                finding.includes('client-side ingress dominated')
            )
        );
        assert.ok(bundle.summaryMarkdown.includes('pre_method_provenance=best_effort_fallback'));
    });

    test('missing metrics should keep available sections and mark metrics gap explicitly', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: sampleTimeline(),
            completionTraceLimit: 50,
            clientProbes: [],
            observabilityMetrics: {
                kind: 'error',
                message: 'Observability request timed out after 1500ms',
            },
        });

        assert.strictEqual(bundle.incidentReport.sources.observability_metrics.status, 'unavailable');
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('metrics snapshot')));
        assert.ok(
            !bundle.files.some((file) => file.relativePath === 'raw/observability_metrics.json'),
            'missing metrics must not reuse stale output dumps as raw evidence'
        );
        assert.ok(bundle.summaryMarkdown.includes('status=unavailable'));
    });

    test('unsupported metrics should stay partial and be marked as unsupported', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: sampleTimeline(),
            completionTraceLimit: 50,
            clientProbes: [sampleProbe()],
            observabilityMetrics: {
                kind: 'unsupported',
            },
        });

        assert.strictEqual(bundle.incidentReport.sources.observability_metrics.status, 'unsupported');
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('unsupported')));
        assert.ok(
            !bundle.files.some((file) => file.relativePath === 'raw/observability_metrics.json'),
            'unsupported metrics must not produce a fake raw attachment'
        );
        assert.ok(bundle.summaryMarkdown.includes('status=unsupported'));
    });

    test('completion timeline error should mark authoritative server trace as unavailable', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: {
                kind: 'error',
                message: 'LSP client not available',
            },
            completionTraceLimit: 50,
            clientProbes: [sampleProbe()],
            observabilityMetrics: sampleMetrics(),
        });

        assert.strictEqual(bundle.incidentReport.sources.completion_timeline.status, 'unavailable');
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('LSP client not available')));
        assert.ok(
            !bundle.files.some((file) => file.relativePath === 'raw/completion_timeline.json'),
            'unavailable server timeline must not create a fake raw attachment'
        );
        assert.deepStrictEqual(bundle.incidentReport.capture_scope, {
            kind: 'unavailable',
        });
        assert.strictEqual(bundle.incidentReport.request_window.request_count, 0);
        assert.deepStrictEqual(bundle.incidentReport.requests, []);
    });

    test('ambiguous correlation should keep request summary server-centric and record a gap', () => {
        const timeline = sampleTimeline();
        if (timeline.kind !== 'ok') {
            throw new Error('expected ok timeline fixture');
        }
        timeline.response.traces = [timeline.response.traces[0]];

        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: timeline,
            completionTraceLimit: 50,
            clientProbes: [
                sampleProbe({
                    probe_id: 'probe-1',
                    trigger_mode: 'invoked',
                    request_started_at_ms: 1_700_000_000_000,
                    lsp_request_started_at_ms: 1_700_000_000_000,
                    lsp_response_received_at_ms: 1_700_000_003_173,
                    request_completed_at_ms: 1_700_000_003_174,
                    client_duration_ms: 3174,
                }),
                sampleProbe({
                    probe_id: 'probe-2',
                    trigger_mode: 'invoked',
                    request_started_at_ms: 1_700_000_000_001,
                    lsp_request_started_at_ms: 1_700_000_000_001,
                    lsp_response_received_at_ms: 1_700_000_003_171,
                    request_completed_at_ms: 1_700_000_003_175,
                    client_duration_ms: 3174,
                }),
            ],
            observabilityMetrics: sampleMetrics(),
        });

        assert.strictEqual(bundle.incidentReport.requests.length, 1);
        assert.strictEqual(bundle.incidentReport.requests[0].client_correlation?.status, 'ambiguous');
        assert.strictEqual(
            bundle.incidentReport.requests[0].client_correlation?.reason,
            'multiple_probe_candidates'
        );
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('ambiguous')));
        assert.ok(bundle.summaryMarkdown.includes('correlation=ambiguous:multiple_probe_candidates'));
    });
});
