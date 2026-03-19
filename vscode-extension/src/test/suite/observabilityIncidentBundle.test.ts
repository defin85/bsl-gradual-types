import * as assert from 'assert';
import { CompletionTimelineFetchResult, ObservabilityMetricsResponse } from '../../lsp/customRequests';
import { CompletionProbe } from '../../providers/completionProbe';
import { buildObservabilityIncidentBundle } from '../../providers/observabilityIncidentBundle';

suite('Observability Incident Bundle Test Suite', () => {
    function sampleProbe(): CompletionProbe {
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
        };
    }

    function sampleTimeline(): CompletionTimelineFetchResult {
        return {
            kind: 'ok',
            response: {
                version: 4,
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
                        },
                        server_edge_details: {
                            transport_received_at_ms: 1_700_000_000_000,
                            handler_entered_at_ms: 1_700_000_003_000,
                            response_sent_at_ms: 1_700_000_003_172,
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

    function sampleMetrics(): ObservabilityMetricsResponse {
        return {
            metrics: {
                uptime_seconds: 184,
                histograms: {
                    intellisense_v2_semantic_diagnostics_query_ms: {
                        p95: 3374,
                    },
                },
            },
        };
    }

    test('happy path bundle should contain summary, incident and all raw attachments', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: sampleTimeline(),
            completionTraceLimit: 50,
            clientProbes: [sampleProbe()],
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
        assert.strictEqual(bundle.incidentReport.sources.completion_timeline.contract_version, 4);
        assert.strictEqual(bundle.incidentReport.sources.client_probes.probe_count, 1);
        assert.strictEqual(bundle.incidentReport.sources.observability_metrics.uptime_seconds, 184);
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('prepare_timeout')));
        assert.ok(bundle.incidentReport.findings.some((finding) => finding.includes('exact_deadline')));
        assert.ok(
            bundle.incidentReport.findings.some((finding) => finding.includes('semantic diagnostics p95=3374ms'))
        );
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

    test('missing metrics should keep available sections and mark metrics gap explicitly', () => {
        const bundle = buildObservabilityIncidentBundle({
            capturedAtMs: Date.parse('2026-03-19T10:23:21.000Z'),
            completionTimeline: sampleTimeline(),
            completionTraceLimit: 50,
            clientProbes: [],
            observabilityMetrics: null,
        });

        assert.strictEqual(bundle.incidentReport.sources.observability_metrics.status, 'unavailable');
        assert.ok(bundle.incidentReport.gaps.some((gap) => gap.includes('metrics snapshot')));
        assert.ok(
            !bundle.files.some((file) => file.relativePath === 'raw/observability_metrics.json'),
            'missing metrics must not reuse stale output dumps as raw evidence'
        );
        assert.ok(bundle.summaryMarkdown.includes('status=unavailable'));
    });
});
