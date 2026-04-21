import * as assert from 'assert';
import {
    buildCompletionTraceBottleneckVerdicts,
    deriveCompletionTracePostResponseSplit,
} from '../../providers/completionTimelineDrilldown';

suite('Completion Timeline Drilldown Test Suite', () => {
    test('buildCompletionTraceBottleneckVerdicts should not mark hot path with zero ingress or prelude wait', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                transport_received_at_ms: 1,
                method_entered_at_ms: 1,
                handler_entered_at_ms: 1,
                response_sent_at_ms: 5,
                transport_to_method_wait_ms: 0,
                method_prelude_exec_ms: 0,
                transport_to_handler_wait_ms: 0,
                server_handler_exec_ms: 4,
            },
        });

        assert.ok(!verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(!verdicts.includes('handler_prelude_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should distinguish server wait before method entry dominance', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                transport_received_at_ms: 1,
                pre_method_attribution_provenance: 'same_request_authoritative',
                method_entered_at_ms: 41,
                handler_entered_at_ms: 43,
                response_sent_at_ms: 61,
                transport_to_method_wait_ms: 40,
                method_prelude_exec_ms: 2,
                transport_to_handler_wait_ms: 42,
                server_handler_exec_ms: 18,
            },
        });

        assert.ok(verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(!verdicts.includes('handler_prelude_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should distinguish adapter pre-dispatch dominance', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                adapter_read_at_ms: 1,
                transport_received_at_ms: 6,
                transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                jsonrpc_dispatch_received_at_ms: 6,
                adapter_to_dispatch_wait_ms: 5,
                method_entered_at_ms: 10,
                handler_entered_at_ms: 11,
                response_sent_at_ms: 31,
                transport_to_method_wait_ms: 4,
                method_prelude_exec_ms: 1,
                transport_to_handler_wait_ms: 10,
                server_handler_exec_ms: 20,
            },
        });

        assert.ok(verdicts.includes('adapter_before_dispatch_dominant'));
        assert.ok(!verdicts.includes('server_before_method_entry_dominant'));
        assert.ok(!verdicts.includes('handler_prelude_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should distinguish reader backpressure before adapter_read', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                adapter_read_at_ms: 20,
                transport_received_at_ms: 60,
                transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                jsonrpc_dispatch_received_at_ms: 60,
                read_loop_wait_reason: 'completion_lane_space',
                read_loop_wait_ms: 40,
                adapter_to_dispatch_wait_ms: 40,
                admission_queue_wait_ms: 10,
                scheduler_poll_ready_wait_ms: 5,
                method_entered_at_ms: 70,
                handler_entered_at_ms: 72,
                response_sent_at_ms: 100,
                transport_to_method_wait_ms: 10,
                method_prelude_exec_ms: 2,
                transport_to_handler_wait_ms: 12,
                server_handler_exec_ms: 28,
            },
        }, {
            correlation_status: 'correlated',
            client_to_transport_wait_ms: 90,
        });

        assert.ok(verdicts.includes('reader_backpressure_dominant'));
        assert.ok(!verdicts.includes('client_before_transport_dominant'));
        assert.ok(!verdicts.includes('adapter_before_dispatch_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should distinguish completion barrier dominance', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                adapter_read_at_ms: 100,
                transport_received_at_ms: 180,
                transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                jsonrpc_dispatch_received_at_ms: 180,
                adapter_to_dispatch_wait_ms: 80,
                admission_queue_wait_ms: 10,
                scheduler_poll_ready_wait_ms: 5,
                completion_barrier_wait_ms: 50,
                completion_barrier_owner_method: 'textDocument/didOpen',
                method_entered_at_ms: 190,
                handler_entered_at_ms: 190,
                response_sent_at_ms: 240,
                transport_to_method_wait_ms: 10,
                method_prelude_exec_ms: 0,
                transport_to_handler_wait_ms: 10,
                server_handler_exec_ms: 50,
            },
        });

        assert.ok(verdicts.includes('completion_barrier_dominant'));
        assert.ok(!verdicts.includes('adapter_before_dispatch_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should fail-closed for weak pre-method provenance', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                transport_received_at_ms: 1,
                pre_method_attribution_provenance: 'best_effort_fallback',
                method_entered_at_ms: 41,
                handler_entered_at_ms: 43,
                response_sent_at_ms: 61,
                transport_to_method_wait_ms: 40,
                method_prelude_exec_ms: 2,
                transport_to_handler_wait_ms: 42,
                server_handler_exec_ms: 18,
            },
        });

        assert.ok(!verdicts.includes('server_before_method_entry_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should distinguish handler prelude dominance', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                transport_received_at_ms: 1,
                method_entered_at_ms: 11,
                handler_entered_at_ms: 41,
                response_sent_at_ms: 61,
                transport_to_method_wait_ms: 10,
                method_prelude_exec_ms: 30,
                transport_to_handler_wait_ms: 40,
                server_handler_exec_ms: 20,
            },
        });

        assert.ok(verdicts.includes('handler_prelude_dominant'));
        assert.ok(!verdicts.includes('server_before_method_entry_dominant'));
    });

    test('buildCompletionTraceBottleneckVerdicts should prefer timeout source when v6 attribution is available', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            prepare_details: {
                fail_closed_cause: 'prepare_timeout',
                timeout_attribution: {
                    source: 'prepare_guard',
                    phase: 'wait_for_file_version',
                    budget_ms: 120,
                    elapsed_ms: 2996,
                    overshoot_ms: 2876,
                },
                progress: {
                    phase: 'wait_for_file_version',
                },
            },
        });

        assert.ok(verdicts.includes('prepare_timeout@prepare_guard'));
    });

    test('buildCompletionTraceBottleneckVerdicts should distinguish artifact polling exact deadline', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
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
                },
            },
        });

        assert.ok(verdicts.includes('exact_deadline@artifact_poll'));
    });

    test('buildCompletionTraceBottleneckVerdicts should fail-closed for client ingress when adapter backlog already dominates', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
            stages: [],
            server_edge_details: {
                adapter_read_at_ms: 100,
                transport_received_at_ms: 100,
                pre_method_attribution_provenance: 'best_effort_fallback',
                adapter_to_dispatch_wait_ms: 60,
                method_entered_at_ms: 140,
                handler_entered_at_ms: 140,
                response_sent_at_ms: 220,
                transport_to_method_wait_ms: 40,
                method_prelude_exec_ms: 0,
                transport_to_handler_wait_ms: 40,
                server_handler_exec_ms: 80,
            },
        }, {
            correlation_status: 'correlated',
            client_to_transport_wait_ms: 50,
        });

        assert.ok(!verdicts.includes('client_before_transport_dominant'));
    });

    test('deriveCompletionTracePostResponseSplit should separate client pre-write wait from transport ingress', () => {
        const split = deriveCompletionTracePostResponseSplit({
            server_edge_details: {
                adapter_read_at_ms: 200,
                transport_received_at_ms: 230,
                handler_entered_at_ms: 240,
                response_sent_at_ms: 260,
                transport_to_handler_wait_ms: 0,
                server_handler_exec_ms: 60,
            },
        }, {
            request_started_at_ms: 100,
            lsp_request_started_at_ms: 110,
            transport_request_written_at_ms: 150,
            transport_response_receive_state: 'unavailable',
            lsp_response_received_at_ms: 270,
            request_completed_at_ms: 275,
        });

        assert.strictEqual(split.client_before_transport_write_wait_ms, 50);
        assert.strictEqual(split.client_to_transport_wait_ms, 50);
    });
});
