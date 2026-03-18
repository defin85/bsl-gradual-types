import * as assert from 'assert';
import {
    formatSelectedCompletionTraceForClipboard,
    formatVisibleCompletionTimelineForClipboard,
} from '../../providers/completionTimelineClipboard';
import { CompletionTimelinePanelState } from '../../providers/completionTimelineModel';

suite('Completion Timeline Clipboard Test Suite', () => {
    function buildReadyState(): CompletionTimelinePanelState {
        return {
            kind: 'ready',
            version: 2,
            updated_at_ms: 1_700_000_000_100,
            client_probe_feed: {
                updated_at_ms: 1_700_000_000_100,
                probes: [
                    {
                        probe_id: 'probe-1',
                        uri: 'file:///tmp/test1.bsl',
                        document_version: 9,
                        trigger_mode: 'trigger_character',
                        trigger_character: '.',
                        request_started_at_ms: 1_700_000_000_090,
                        request_completed_at_ms: 1_700_000_000_100,
                        client_duration_ms: 10,
                        client_terminal_state: 'ok_non_empty',
                        time_since_last_local_edit_ms: 21,
                        time_since_last_did_change_sent_ms: 8,
                        is_after_dot: true,
                        identifier_tail_length: 0,
                    },
                ],
            },
            traces: [
                {
                    trace_id: 'trace-1',
                    request_id: 'req-1',
                    uri: 'file:///tmp/test1.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_001,
                    total_duration_ms: 30,
                    max_stage_end_ms: 28,
                    unattributed_overhead_ms: 2,
                    dominant_stage: 'query_bundle',
                    prepare_details: {
                        wait_budget_ms: 120,
                        guard_outcome: 'timeout',
                        outcome: 'wait_not_ready',
                        route: 'exact_hit',
                        fail_closed_cause: 'exact_deadline',
                        min_file_version: 9,
                        shadow_version_at_start: 9,
                        observed_file_version: 8,
                        apply_age_at_start_ms: 3001,
                        apply_age_at_terminal_ms: 3088,
                    },
                    turn_attribution: {
                        request_file_seq: 17,
                        request_epoch: 9,
                        queue_outcome: 'enqueued',
                        turn_wait_outcome: 'ready',
                        queue_capacity: 256,
                        queue_depth_before_enqueue: 1,
                        queue_depth_after_enqueue: 2,
                        queued_completion_ahead_count: 1,
                        did_change_ahead_count: 0,
                        active_completion_count: 1,
                        dropped_completion_file_seq: [12],
                        active_holder: {
                            request_id: 'req-0',
                            file_seq: 16,
                            request_epoch: 8,
                            trigger_mode: 'trigger_character',
                            version_hint: 1,
                            age_ms: 88,
                        },
                    },
                    stages: [
                        {
                            name: 'prepare_stateful',
                            status: 'completed',
                            started_offset_ms: 0,
                            end_offset_ms: 10,
                            duration_ms: 10,
                            width_percent: 33.3,
                            duration_percent: 33.3,
                            is_dominant: false,
                        },
                        {
                            name: 'query_bundle',
                            status: 'completed',
                            started_offset_ms: 10,
                            end_offset_ms: 28,
                            duration_ms: 18,
                            width_percent: 60,
                            duration_percent: 60,
                            is_dominant: true,
                        },
                    ],
                },
            ],
            average_trace: {
                trace_id: 'average(1)',
                sample_count: 1,
                request_id: undefined,
                uri: 'average://completion-timeline',
                trigger_mode: 'averaged',
                outcome: 'ok_non_empty',
                started_at_ms: 1_700_000_000_001,
                total_duration_ms: 30,
                max_stage_end_ms: 30,
                unattributed_overhead_ms: 0,
                dominant_stage: 'query_bundle',
                stages: [
                    {
                        name: 'query_bundle',
                        status: 'completed',
                        started_offset_ms: 0,
                        end_offset_ms: 30,
                        duration_ms: 30,
                        width_percent: 100,
                        duration_percent: 100,
                        is_dominant: true,
                    },
                ],
            },
        };
    }

    test('formatVisibleCompletionTimelineForClipboard should include header and visible traces', () => {
        const text = formatVisibleCompletionTimelineForClipboard(buildReadyState(), 'all');
        assert.ok(text);
        assert.ok(text!.includes('Completion Timeline | mode=all'));
        assert.ok(text!.includes('Server Timeline'));
        assert.ok(text!.includes('trace-1 (invoked)'));
        assert.ok(text!.includes('Client Probe Feed | local-only debug data'));
        assert.ok(text!.includes('probe-1 (trigger_character)'));
        assert.ok(text!.includes('prepare_wait_budget_ms=120'));
        assert.ok(text!.includes('prepare_guard_outcome=timeout'));
        assert.ok(text!.includes('prepare_outcome=wait_not_ready'));
        assert.ok(text!.includes('completion_route=exact_hit'));
        assert.ok(text!.includes('fail_closed_cause=exact_deadline'));
        assert.ok(text!.includes('turn_request_file_seq=17'));
        assert.ok(text!.includes('active_holder | request=req-0'));
        assert.ok(text!.includes('query_bundle | completed'));
        assert.ok(
            text!.includes(
                'Client probes are extension-local debug records and do not replace server timeline stages, routes, or outcomes.'
            )
        );
    });

    test('formatVisibleCompletionTimelineForClipboard should use average trace in average mode', () => {
        const text = formatVisibleCompletionTimelineForClipboard(buildReadyState(), 'average');
        assert.ok(text);
        assert.ok(text!.includes('Completion Timeline | mode=average'));
        assert.ok(text!.includes('average(1) (averaged) | sample=1'));
        assert.ok(!text!.includes('trace-1 (invoked)'));
    });

    test('formatSelectedCompletionTraceForClipboard should return single requested trace', () => {
        const text = formatSelectedCompletionTraceForClipboard(buildReadyState(), 'trace-1');
        assert.ok(text);
        assert.ok(text!.startsWith('trace-1 (invoked)'));
        assert.ok(!text!.includes('Completion Timeline | mode='));
    });

    test('formatVisibleCompletionTimelineForClipboard should keep client probes visible when server timeline is unsupported', () => {
        const text = formatVisibleCompletionTimelineForClipboard(
            {
                kind: 'unsupported',
                message: 'Timeline unsupported',
                client_probe_feed: buildReadyState().client_probe_feed,
            },
            'all'
        );

        assert.ok(text);
        assert.ok(text!.includes('Server Timeline'));
        assert.ok(text!.includes('Timeline unsupported'));
        assert.ok(text!.includes('Client Probe Feed | local-only debug data'));
        assert.ok(text!.includes('probe-1 (trigger_character)'));
    });
});
