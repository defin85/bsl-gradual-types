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
            version: 19,
            updated_at_ms: 1_700_000_000_100,
            client_probe_feed: {
                updated_at_ms: 1_700_000_000_100,
                probes: [
                    {
                        probe_id: 'probe-1',
                        uri: 'file:///tmp/test1.bsl',
                        document_version: 9,
                        document_version_at_terminal: 10,
                        trigger_mode: 'trigger_character',
                        trigger_character: '.',
                        request_started_at_ms: 1_700_000_000_090,
                        lsp_request_started_at_ms: 1_700_000_000_091,
                        lsp_response_received_at_ms: 1_700_000_000_098,
                        request_completed_at_ms: 1_700_000_000_100,
                        client_duration_ms: 10,
                        client_terminal_state: 'ok_non_empty',
                        cancel_reason_hint: 'superseded_newer_version',
                        result_kind: 'non_empty',
                        item_count_bucket: '1_5',
                        is_incomplete: false,
                        time_since_last_local_edit_ms: 21,
                        time_since_last_did_change_sent_ms: 8,
                        did_change_count_during_probe: 1,
                        cursor_moved_during_probe: true,
                        active_completion_count_at_start: 1,
                        same_uri_probe_overlap_count: 1,
                        newer_probe_started_before_terminal: true,
                        superseded_by_probe_id: 'probe-2',
                        superseded_after_ms: 6,
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
                    server_edge_details: {
                        adapter_read_at_ms: 1_699_999_999_956,
                        transport_received_at_ms: 1_699_999_999_960,
                        transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                        jsonrpc_dispatch_received_at_ms: 1_699_999_999_960,
                        transport_slot_released_at_ms: 1_699_999_999_989,
                        service_future_created_at_ms: 1_699_999_999_972,
                        service_future_first_poll_entered_at_ms: 1_699_999_999_989,
                        service_future_first_poll_outcome: 'pending',
                        service_future_first_wake_scheduled_at_ms: 1_699_999_999_995,
                        first_poll_contention_attribution: {
                            contender_class: 'document_sync',
                            uri_scope: 'same_uri',
                            inflight_count: 1,
                            oldest_inflight_age_ms: 17,
                            concurrency_level: 16,
                        },
                        pre_method_attribution_provenance: 'same_request_authoritative',
                        service_scope_entered_at_ms: 1_699_999_999_988,
                        method_entered_at_ms: 1_700_000_000_000,
                        handler_entered_at_ms: 1_700_000_000_002,
                        response_sent_at_ms: 1_700_000_000_030,
                        cancel_observed_at_ms: 1_700_000_000_021,
                        dispatch_to_request_context_wait_ms: 4,
                        adapter_to_dispatch_wait_ms: 4,
                        transport_to_slot_release_wait_ms: 29,
                        transport_to_service_future_wait_ms: 12,
                        service_future_to_scope_wait_ms: 16,
                        service_future_to_first_poll_wait_ms: 17,
                        first_poll_to_first_wake_wait_ms: 6,
                        transport_to_service_scope_wait_ms: 28,
                        service_scope_to_method_wait_ms: 12,
                        transport_to_method_wait_ms: 40,
                        method_prelude_exec_ms: 2,
                        slot_release_to_handler_wait_ms: 13,
                        slot_release_to_response_wait_ms: 41,
                        transport_to_handler_wait_ms: 42,
                        server_handler_exec_ms: 28,
                        cancel_observed_after_handler_enter_ms: 19,
                    },
                    prepare_details: {
                        wait_budget_ms: 120,
                        guard_outcome: 'timeout',
                        outcome: 'wait_not_ready',
                        route: 'exact_hit',
                        fail_closed_cause: 'prepare_timeout',
                        min_file_version: 9,
                        shadow_version_at_start: 9,
                        observed_file_version: 8,
                        apply_age_at_start_ms: 3001,
                        apply_age_at_terminal_ms: 3088,
                        timeout_attribution: {
                            source: 'prepare_guard',
                            phase: 'snapshot_with_deps',
                            budget_ms: 120,
                            elapsed_ms: 2996,
                            overshoot_ms: 2876,
                        },
                        progress: {
                            phase: 'snapshot_with_deps',
                            phase_started_offset_ms: 9,
                            wait_completed_offset_ms: 9,
                        },
                        wait_for_file_version_runtime: {
                            queue_wait_ms: 7,
                            exec_ms: 2,
                            wake_wait_ms: 90,
                            resolution: 'waiter',
                        },
                        snapshot_with_deps_runtime: {
                            queue_wait_ms: 3,
                            exec_ms: 5,
                        },
                        snapshot_with_deps_timeout_runtime: {
                            queue_wait_ms: 11,
                            exec_ms: 17,
                            wake_wait_ms: 2868,
                            resolution: 'wake_wait',
                        },
                    },
                    turn_attribution: {
                        request_file_seq: 17,
                        request_epoch: 9,
                        queue_outcome: 'enqueued',
                        turn_wait_outcome: 'ready',
                        dispatcher_resolution_latency_ms: 4,
                        turn_wait_entered_at_ms: 1_700_000_000_004,
                        turn_wait_resolved_at_ms: 1_700_000_000_006,
                        wake_after_turn_resolution_at_ms: 1_700_000_000_007,
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
        assert.ok(text!.includes('contract=v19'));
        assert.ok(text!.includes('Client Probe Feed | local-only debug data'));
        assert.ok(text!.includes('probe-1 (trigger_character)'));
        assert.ok(text!.includes('transport_received_at_ms=1699999999960'));
        assert.ok(text!.includes('transport_received_at_ms_provenance=jsonrpc_dispatch_received'));
        assert.ok(text!.includes('adapter_read_at_ms=1699999999956'));
        assert.ok(text!.includes('adapter_to_dispatch_wait_ms=4'));
        assert.ok(text!.includes('jsonrpc_dispatch_received_at_ms=1699999999960'));
        assert.ok(text!.includes('transport_slot_released_at_ms=1699999999989'));
        assert.ok(text!.includes('service_future_created_at_ms=1699999999972'));
        assert.ok(text!.includes('service_future_first_poll_entered_at_ms=1699999999989'));
        assert.ok(text!.includes('service_future_first_poll_outcome=pending'));
        assert.ok(text!.includes('service_future_first_wake_scheduled_at_ms=1699999999995'));
        assert.ok(text!.includes('first_poll_contention_contender_class=document_sync'));
        assert.ok(text!.includes('first_poll_contention_uri_scope=same_uri'));
        assert.ok(text!.includes('first_poll_contention_inflight_count=1'));
        assert.ok(text!.includes('first_poll_contention_oldest_inflight_age_ms=17'));
        assert.ok(text!.includes('first_poll_contention_concurrency_level=16'));
        assert.ok(text!.includes('pre_method_attribution_provenance=same_request_authoritative'));
        assert.ok(text!.includes('service_scope_entered_at_ms=1699999999988'));
        assert.ok(text!.includes('method_entered_at_ms=1700000000000'));
        assert.ok(text!.includes('handler_entered_at_ms=1700000000002'));
        assert.ok(text!.includes('response_sent_at_ms=1700000000030'));
        assert.ok(text!.includes('dispatch_to_request_context_wait_ms=4'));
        assert.ok(text!.includes('transport_to_slot_release_wait_ms=29'));
        assert.ok(text!.includes('transport_to_service_future_wait_ms=12'));
        assert.ok(text!.includes('service_future_to_scope_wait_ms=16'));
        assert.ok(text!.includes('service_future_to_first_poll_wait_ms=17'));
        assert.ok(text!.includes('first_poll_to_first_wake_wait_ms=6'));
        assert.ok(text!.includes('transport_to_service_scope_wait_ms=28'));
        assert.ok(text!.includes('service_scope_to_method_wait_ms=12'));
        assert.ok(text!.includes('transport_to_method_wait_ms=40'));
        assert.ok(text!.includes('method_prelude_exec_ms=2'));
        assert.ok(text!.includes('slot_release_to_handler_wait_ms=13'));
        assert.ok(text!.includes('slot_release_to_response_wait_ms=41'));
        assert.ok(text!.includes('transport_to_handler_wait_ms=42'));
        assert.ok(text!.includes('server_handler_exec_ms=28'));
        assert.ok(text!.includes('cancel_observed_after_handler_enter_ms=19'));
        assert.ok(text!.includes('document_version_at_terminal=10'));
        assert.ok(text!.includes('cancel_reason_hint=superseded_newer_version'));
        assert.ok(text!.includes('superseded_by_probe_id=probe-2'));
        assert.ok(text!.includes('transport_dispatch_delta_ms=1'));
        assert.ok(text!.includes('lsp_roundtrip_ms=7'));
        assert.ok(text!.includes('client_post_response_ms=2'));
        assert.ok(text!.includes('result_kind=non_empty | item_count_bucket=1_5 | is_incomplete=false'));
        assert.ok(text!.includes('did_change_count_during_probe=1'));
        assert.ok(text!.includes('cursor_moved_during_probe=true'));
        assert.ok(text!.includes('same_uri_probe_overlap_count=1'));
        assert.ok(text!.includes('prepare_wait_budget_ms=120'));
        assert.ok(text!.includes('prepare_guard_outcome=timeout'));
        assert.ok(text!.includes('prepare_outcome=wait_not_ready'));
        assert.ok(text!.includes('completion_route=exact_hit'));
        assert.ok(text!.includes('fail_closed_cause=prepare_timeout'));
        assert.ok(text!.includes('bottleneck_verdict=server_before_method_entry_dominant'));
        assert.ok(text!.includes('bottleneck_verdict=prepare_timeout@prepare_guard'));
        assert.ok(text!.includes('timeout_attribution | source=prepare_guard | phase=snapshot_with_deps | budget_ms=120 | elapsed_ms=2996 | overshoot_ms=2876'));
        assert.ok(text!.includes('prepare_progress | phase=snapshot_with_deps | phase_started_offset_ms=9 | wait_completed_offset_ms=9'));
        assert.ok(text!.includes('wait_for_file_version_runtime | queue_wait_ms=7 | exec_ms=2 | wake_wait_ms=90 | resolution=waiter'));
        assert.ok(text!.includes('snapshot_with_deps_runtime | queue_wait_ms=3 | exec_ms=5'));
        assert.ok(text!.includes('snapshot_with_deps_timeout_runtime | queue_wait_ms=11 | exec_ms=17 | wake_wait_ms=2868 | resolution=wake_wait'));
        assert.ok(text!.includes('turn_request_file_seq=17'));
        assert.ok(text!.includes('dispatcher_resolution_latency_ms=4'));
        assert.ok(text!.includes('turn_wait_entered_at_ms=1700000000004'));
        assert.ok(text!.includes('turn_wait_resolved_at_ms=1700000000006'));
        assert.ok(text!.includes('wake_after_turn_resolution_at_ms=1700000000007'));
        assert.ok(text!.includes('active_holder | request=req-0'));
        assert.ok(text!.includes('query_bundle | completed'));
        assert.ok(
            text!.includes(
                'Client probes are extension-local debug records and do not replace server timeline stages, routes, or outcomes.'
            )
        );
    });

    test('formatVisibleCompletionTimelineForClipboard should keep adapter bottleneck verdicts when prepare details are absent', () => {
        const state = buildReadyState();
        if (state.kind !== 'ready') {
            throw new Error('expected ready state fixture');
        }
        state.traces[0].server_edge_details = {
            ...state.traces[0].server_edge_details!,
            transport_to_method_wait_ms: undefined,
            method_prelude_exec_ms: undefined,
        };
        state.traces[0].prepare_details = undefined;

        const text = formatVisibleCompletionTimelineForClipboard(state, 'all');
        assert.ok(text);
        assert.ok(text!.includes('adapter_read_at_ms=1699999999956'));
        assert.ok(text!.includes('adapter_to_dispatch_wait_ms=4'));
        assert.ok(text!.includes('bottleneck_verdict=adapter_before_dispatch_dominant'));
        assert.ok(!text!.includes('prepare_wait_budget_ms='));
        assert.ok(!text!.includes('prepare_outcome='));
    });

    test('formatVisibleCompletionTimelineForClipboard should use average trace in average mode', () => {
        const text = formatVisibleCompletionTimelineForClipboard(buildReadyState(), 'average');
        assert.ok(text);
        assert.ok(text!.includes('Completion Timeline | mode=average'));
        assert.ok(text!.includes('average(1) (averaged) | sample=1'));
        assert.ok(
            text!.includes(
                        'Average trace is synthetic; v8 trustworthy pre-method attribution provenance, v9 pre-service-scope split, v10 dispatch split, and v11 first-poll / first-wake split are unavailable by design.'
                    .replace(
                        'and v11 first-poll / first-wake split are unavailable by design.',
                        'v11 first-poll / first-wake split, v12 first-poll contention attribution, v13 contender snapshot, v14 executeCommand command detail, v15 completion phase detail, v16 turn-wait resolution detail, v17 transport slot release detail, v18 request-bound client probe correlation detail, and v19 adapter ingress pre-dispatch split are unavailable by design.'
                    )
            )
        );
        assert.ok(!text!.includes('bottleneck_verdict=server_before_method_entry_dominant'));
        assert.ok(!text!.includes('trace-1 (invoked)'));
    });

    test('formatVisibleCompletionTimelineForClipboard should mark v11 payload as missing v12 contention attribution by design', () => {
        const state = buildReadyState();
        if (state.kind !== 'ready') {
            throw new Error('expected ready state fixture');
        }
        state.version = 11;
        state.traces[0].server_edge_details = {
            ...state.traces[0].server_edge_details!,
            first_poll_contention_attribution: undefined,
        };

        const text = formatVisibleCompletionTimelineForClipboard(state, 'all');
        assert.ok(text);
        assert.ok(text!.includes('contract=v11'));
        assert.ok(
            text!.includes(
                'v12 first-poll contention attribution is unavailable by design on this payload.'
            )
        );
        assert.ok(text!.includes('service_future_first_poll_entered_at_ms=1699999999989'));
        assert.ok(!text!.includes('first_poll_contention_contender_class='));
        assert.ok(!text!.includes('first_poll_contention_uri_scope='));
        assert.ok(!text!.includes('first_poll_contention_inflight_count='));
    });

    test('formatVisibleCompletionTimelineForClipboard should mark v10 payload as missing v11 first-poll / first-wake split by design', () => {
        const state = buildReadyState();
        if (state.kind !== 'ready') {
            throw new Error('expected ready state fixture');
        }
        state.version = 10;
        state.traces[0].server_edge_details = {
            ...state.traces[0].server_edge_details!,
            service_future_first_poll_entered_at_ms: undefined,
            service_future_first_poll_outcome: undefined,
            service_future_first_wake_scheduled_at_ms: undefined,
            service_future_to_first_poll_wait_ms: undefined,
            first_poll_to_first_wake_wait_ms: undefined,
        };

        const text = formatVisibleCompletionTimelineForClipboard(state, 'all');
        assert.ok(text);
        assert.ok(text!.includes('contract=v10'));
        assert.ok(
            text!.includes(
                'v11 first-poll / first-wake split is unavailable by design on this payload.'
            )
        );
        assert.ok(
            text!.includes(
                'v12 first-poll contention attribution is unavailable by design on this payload.'
            )
        );
        assert.ok(text!.includes('service_future_created_at_ms=1699999999972'));
        assert.ok(text!.includes('transport_to_service_future_wait_ms=12'));
        assert.ok(text!.includes('service_future_to_scope_wait_ms=16'));
        assert.ok(!text!.includes('service_future_first_poll_entered_at_ms='));
        assert.ok(!text!.includes('service_future_first_poll_outcome='));
        assert.ok(!text!.includes('service_future_first_wake_scheduled_at_ms='));
        assert.ok(!text!.includes('service_future_to_first_poll_wait_ms='));
        assert.ok(!text!.includes('first_poll_to_first_wake_wait_ms='));
        assert.ok(!text!.includes('first_poll_contention_contender_class='));
    });

    test('formatVisibleCompletionTimelineForClipboard should mark v7 payload as missing v8 provenance by design', () => {
        const state = buildReadyState();
        if (state.kind !== 'ready') {
            throw new Error('expected ready state fixture');
        }
        state.version = 7;
        state.traces[0].server_edge_details = {
            transport_received_at_ms: 1_699_999_999_960,
            transport_received_at_ms_provenance: undefined,
            method_entered_at_ms: 1_700_000_000_000,
            handler_entered_at_ms: 1_700_000_000_002,
            response_sent_at_ms: 1_700_000_000_030,
            cancel_observed_at_ms: 1_700_000_000_021,
            transport_to_method_wait_ms: 40,
            method_prelude_exec_ms: 2,
            transport_to_handler_wait_ms: 42,
            server_handler_exec_ms: 28,
            cancel_observed_after_handler_enter_ms: 19,
        };
        state.traces[0].prepare_details = {
            ...state.traces[0].prepare_details,
            snapshot_with_deps_timeout_runtime: undefined,
        };

        const text = formatVisibleCompletionTimelineForClipboard(state, 'all');
        assert.ok(text);
        assert.ok(text!.includes('contract=v7'));
        assert.ok(
            text!.includes(
                'v8 trustworthy pre-method attribution provenance is unavailable by design on this payload.'
            )
        );
        assert.ok(
            text!.includes(
                'v9 pre-service-scope split is unavailable by design on this payload.'
            )
        );
        assert.ok(
            text!.includes(
                'v10 dispatch split is unavailable by design on this payload.'
            )
        );
        assert.ok(
            text!.includes(
                'v11 first-poll / first-wake split is unavailable by design on this payload.'
            )
        );
        assert.ok(
            text!.includes(
                'v12 first-poll contention attribution is unavailable by design on this payload.'
            )
        );
        assert.ok(text!.includes('transport_to_method_wait_ms=40'));
        assert.ok(!text!.includes('pre_method_attribution_provenance='));
        assert.ok(!text!.includes('bottleneck_verdict=server_before_method_entry_dominant'));
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
        assert.ok(text!.includes('cancel_reason_hint=superseded_newer_version'));
    });
});
