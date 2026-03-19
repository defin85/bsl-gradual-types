import * as assert from 'assert';
import { buildCompletionTraceBottleneckVerdicts } from '../../providers/completionTimelineDrilldown';

suite('Completion Timeline Drilldown Test Suite', () => {
    test('buildCompletionTraceBottleneckVerdicts should distinguish handler prelude dominance', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
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
    });

    test('buildCompletionTraceBottleneckVerdicts should prefer timeout source when v6 attribution is available', () => {
        const verdicts = buildCompletionTraceBottleneckVerdicts({
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
});
