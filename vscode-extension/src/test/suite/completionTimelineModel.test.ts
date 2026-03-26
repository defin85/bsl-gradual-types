import * as assert from 'assert';
import {
    getAverageTraceProvenanceNotice,
    mapCompletionTimelineFetchResultToPanelState,
    mapCompletionTimelineResponseToPanelState,
} from '../../providers/completionTimelineModel';
import { CompletionTimelineResponse } from '../../lsp/customRequests';
import { CompletionProbe } from '../../providers/completionProbe';

function buildClientProbe(probeId: string, version: number, startedAtMs: number): CompletionProbe {
    return {
        probe_id: probeId,
        uri: 'file:///tmp/test.bsl',
        document_version: version,
        document_version_at_terminal: version + 1,
        trigger_mode: 'trigger_character',
        trigger_character: '.',
        request_started_at_ms: startedAtMs,
        lsp_request_started_at_ms: startedAtMs + 1,
        lsp_response_received_at_ms: startedAtMs + 4,
        request_completed_at_ms: startedAtMs + 5,
        client_duration_ms: 5,
        client_terminal_state: 'ok_non_empty',
        cancel_reason_hint: 'superseded_same_version',
        result_kind: 'non_empty',
        item_count_bucket: '1_5',
        is_incomplete: false,
        time_since_last_local_edit_ms: 11,
        time_since_last_did_change_sent_ms: 7,
        did_change_count_during_probe: 1,
        cursor_moved_during_probe: true,
        active_completion_count_at_start: 1,
        same_uri_probe_overlap_count: 1,
        newer_probe_started_before_terminal: true,
        superseded_by_probe_id: `next-${probeId}`,
        superseded_after_ms: 4,
        is_after_dot: true,
        identifier_tail_length: 0,
    };
}

suite('Completion Timeline Model Test Suite', () => {
    test('Mapping LSP timeline payload -> UI model', () => {
        const payload: CompletionTimelineResponse = {
            version: 15,
            traces: [
                {
                    trace_id: 'trace-42',
                    request_id: 'req-42',
                    uri: 'file:///tmp/test.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_042,
                    total_duration_ms: 48,
                    dominant_stage: 'query_bundle',
                    server_edge_details: {
                        transport_received_at_ms: 1_700_000_000_040,
                        transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                        jsonrpc_dispatch_received_at_ms: 1_700_000_000_040,
                        service_future_created_at_ms: 1_700_000_000_040,
                        service_future_first_poll_entered_at_ms: 1_700_000_000_041,
                        service_future_first_poll_outcome: 'pending',
                        service_future_first_wake_scheduled_at_ms: 1_700_000_000_046,
                        first_poll_contention_attribution: {
                            contender_class: 'document_sync',
                            uri_scope: 'same_uri',
                            inflight_count: 1,
                            oldest_inflight_age_ms: 1,
                            concurrency_level: 16,
                        },
                        first_poll_contention_contenders: [
                            {
                                request_class: 'document_sync',
                                method: 'textDocument/didChange',
                                uri: 'file:///tmp/test.bsl',
                                age_ms: 1,
                            },
                        ],
                        pre_method_attribution_provenance: 'same_request_authoritative',
                        service_scope_entered_at_ms: 1_700_000_000_041,
                        method_entered_at_ms: 1_700_000_000_042,
                        handler_entered_at_ms: 1_700_000_000_042,
                        response_sent_at_ms: 1_700_000_000_090,
                        dispatch_to_request_context_wait_ms: 0,
                        transport_to_service_future_wait_ms: 0,
                        service_future_to_scope_wait_ms: 1,
                        service_future_to_first_poll_wait_ms: 1,
                        first_poll_to_first_wake_wait_ms: 5,
                        transport_to_service_scope_wait_ms: 1,
                        service_scope_to_method_wait_ms: 1,
                        transport_to_handler_wait_ms: 2,
                        server_handler_exec_ms: 48,
                    },
                    prepare_details: {
                        wait_budget_ms: 120,
                        route: 'head_hit',
                        outcome: 'ready',
                        min_file_version: 7,
                        shadow_version_at_start: 7,
                        observed_file_version: 7,
                        wait_elapsed_ms: 12,
                        snapshot_elapsed_ms: 4,
                        apply_age_at_start_ms: 9,
                        apply_age_at_terminal_ms: 13,
                        progress: {
                            phase: 'ready',
                            snapshot_completed_offset_ms: 16,
                        },
                        wait_for_file_version_runtime: {
                            queue_wait_ms: 1,
                            exec_ms: 2,
                            resolution: 'immediate',
                        },
                        snapshot_with_deps_runtime: {
                            queue_wait_ms: 3,
                            exec_ms: 4,
                        },
                        snapshot_with_deps_timeout_runtime: {
                            queue_wait_ms: 6,
                            exec_ms: 7,
                            wake_wait_ms: 8,
                            resolution: 'wake_wait',
                        },
                        exact_wait: {
                            type_index_waiter_action: 'joined',
                            matching_task_state: 'matching',
                            task_phase: 'computing',
                        },
                    },
                    turn_attribution: {
                        request_file_seq: 42,
                        request_epoch: 7,
                        queue_outcome: 'enqueued',
                        turn_wait_outcome: 'ready',
                        dispatcher_resolution_latency_ms: 5,
                        queue_capacity: 256,
                        queue_depth_before_enqueue: 1,
                        queue_depth_after_enqueue: 2,
                        queued_completion_ahead_count: 1,
                        did_change_ahead_count: 0,
                        active_completion_count: 1,
                        dropped_completion_file_seq: [],
                        active_holder: {
                            request_id: 'req-41',
                            file_seq: 41,
                            request_epoch: 6,
                            trigger_mode: 'trigger_character',
                            version_hint: 2,
                            age_ms: 55,
                        },
                    },
                    stages: [
                        { name: 'prepare_stateful', status: 'completed', started_offset_ms: 0, duration_ms: 12 },
                        { name: 'query_bundle', status: 'completed', started_offset_ms: 12, duration_ms: 30 },
                        { name: 'response_build', status: 'completed', started_offset_ms: 42, duration_ms: 6 },
                    ],
                },
            ],
        };
        const clientProbes = [
            buildClientProbe('probe-1', 7, 1_700_000_000_090),
            buildClientProbe('probe-2', 8, 1_700_000_000_120),
        ];

        const state = mapCompletionTimelineResponseToPanelState(
            payload,
            clientProbes,
            1_700_000_000_100
        );
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.strictEqual(state.version, 15);
        assert.strictEqual(state.traces.length, 1);
        assert.strictEqual(state.traces[0].trace_id, 'trace-42');
        assert.strictEqual(
            state.traces[0].server_edge_details?.transport_received_at_ms_provenance,
            'jsonrpc_dispatch_received'
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.jsonrpc_dispatch_received_at_ms,
            1_700_000_000_040
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_future_created_at_ms,
            1_700_000_000_040
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_future_first_poll_entered_at_ms,
            1_700_000_000_041
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_future_first_poll_outcome,
            'pending'
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_future_first_wake_scheduled_at_ms,
            1_700_000_000_046
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_attribution?.contender_class,
            'document_sync'
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_attribution?.uri_scope,
            'same_uri'
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_attribution?.inflight_count,
            1
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_attribution?.concurrency_level,
            16
        );
        assert.deepStrictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_contenders,
            [
                {
                    request_class: 'document_sync',
                    method: 'textDocument/didChange',
                    uri: 'file:///tmp/test.bsl',
                    age_ms: 1,
                },
            ]
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.pre_method_attribution_provenance,
            'same_request_authoritative'
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.transport_to_service_future_wait_ms,
            0
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_future_to_scope_wait_ms,
            1
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_future_to_first_poll_wait_ms,
            1
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.first_poll_to_first_wake_wait_ms,
            5
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.dispatch_to_request_context_wait_ms,
            0
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.transport_to_service_scope_wait_ms,
            1
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.service_scope_to_method_wait_ms,
            1
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.transport_to_handler_wait_ms,
            2
        );
        assert.strictEqual(
            state.traces[0].server_edge_details?.server_handler_exec_ms,
            48
        );
        assert.strictEqual(state.traces[0].prepare_details?.wait_budget_ms, 120);
        assert.strictEqual(state.traces[0].prepare_details?.route, 'head_hit');
        assert.strictEqual(
            state.traces[0].prepare_details?.wait_for_file_version_runtime?.resolution,
            'immediate'
        );
        assert.strictEqual(
            state.traces[0].prepare_details?.snapshot_with_deps_timeout_runtime?.resolution,
            'wake_wait'
        );
        assert.strictEqual(
            state.traces[0].prepare_details?.exact_wait?.task_phase,
            'computing'
        );
        assert.strictEqual(state.traces[0].stages.length, 3);
        assert.strictEqual(state.traces[0].turn_attribution?.queue_outcome, 'enqueued');
        assert.strictEqual(
            state.traces[0].turn_attribution?.dispatcher_resolution_latency_ms,
            5
        );
        assert.strictEqual(state.traces[0].turn_attribution?.active_holder?.file_seq, 41);
        assert.strictEqual(state.client_probe_feed.updated_at_ms, 1_700_000_000_100);
        assert.deepStrictEqual(
            state.client_probe_feed.probes.map((probe) => probe.probe_id),
            ['probe-2', 'probe-1']
        );
        assert.strictEqual(state.client_probe_feed.probes[0].cancel_reason_hint, 'superseded_same_version');
        assert.strictEqual(state.client_probe_feed.probes[0].document_version_at_terminal, 9);
        assert.strictEqual(state.client_probe_feed.probes[0].superseded_by_probe_id, 'next-probe-2');
        assert.ok(state.traces[0].stages.every((stage) => stage.width_percent >= 0));
        assert.ok(state.traces[0].stages.every((stage) => stage.duration_percent >= 0));
        assert.ok(state.average_trace, 'average trace should be available for non-empty payload');
    });

    test('Older contract payloads should not surface v13 contender snapshot', () => {
        const payload: CompletionTimelineResponse = {
            version: 12,
            traces: [
                {
                    trace_id: 'trace-v12',
                    request_id: 'req-v12',
                    uri: 'file:///tmp/v12.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_010,
                    total_duration_ms: 14,
                    dominant_stage: 'query_bundle',
                    server_edge_details: {
                        transport_received_at_ms: 1_700_000_000_000,
                        service_future_created_at_ms: 1_700_000_000_001,
                        service_future_first_poll_entered_at_ms: 1_700_000_000_002,
                        service_future_first_poll_outcome: 'pending',
                        service_future_first_wake_scheduled_at_ms: 1_700_000_000_003,
                        first_poll_contention_attribution: {
                            contender_class: 'document_sync',
                            uri_scope: 'same_uri',
                            inflight_count: 1,
                            oldest_inflight_age_ms: 1,
                            concurrency_level: 16,
                        },
                        first_poll_contention_contenders: [
                            {
                                request_class: 'document_sync',
                                method: 'textDocument/didChange',
                                uri: 'file:///tmp/v12.bsl',
                                age_ms: 1,
                            },
                        ],
                        handler_entered_at_ms: 1_700_000_000_004,
                        response_sent_at_ms: 1_700_000_000_014,
                        transport_to_handler_wait_ms: 4,
                        server_handler_exec_ms: 10,
                    },
                    stages: [
                        {
                            name: 'query_bundle',
                            status: 'completed',
                            started_offset_ms: 0,
                            duration_ms: 14,
                        },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.strictEqual(state.version, 12);
        assert.strictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_contenders,
            undefined
        );
    });

    test('v13 payload should not surface v14 executeCommand command detail inside contenders', () => {
        const payload: CompletionTimelineResponse = {
            version: 13,
            traces: [
                {
                    trace_id: 'trace-v13',
                    request_id: 'req-v13',
                    uri: 'file:///tmp/v13.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_010,
                    total_duration_ms: 14,
                    dominant_stage: 'query_bundle',
                    server_edge_details: {
                        transport_received_at_ms: 1_700_000_000_000,
                        service_future_created_at_ms: 1_700_000_000_001,
                        service_future_first_poll_entered_at_ms: 1_700_000_000_002,
                        service_future_first_poll_outcome: 'pending',
                        service_future_first_wake_scheduled_at_ms: 1_700_000_000_003,
                        first_poll_contention_attribution: {
                            contender_class: 'other_request',
                            uri_scope: 'unavailable',
                            inflight_count: 1,
                            oldest_inflight_age_ms: 1,
                            concurrency_level: 16,
                        },
                        first_poll_contention_contenders: [
                            {
                                request_class: 'other_request',
                                method: 'workspace/executeCommand',
                                command: 'bsl.getCompletionTimeline',
                                phase: 'query_bundle',
                                age_ms: 1,
                            },
                        ],
                        handler_entered_at_ms: 1_700_000_000_004,
                        response_sent_at_ms: 1_700_000_000_014,
                        transport_to_handler_wait_ms: 4,
                        server_handler_exec_ms: 10,
                    },
                    stages: [
                        {
                            name: 'query_bundle',
                            status: 'completed',
                            started_offset_ms: 0,
                            duration_ms: 14,
                        },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.deepStrictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_contenders,
            [
                {
                    request_class: 'other_request',
                    method: 'workspace/executeCommand',
                    age_ms: 1,
                },
            ]
        );
    });

    test('v14 payload should not surface v15 completion phase detail inside contenders', () => {
        const payload: CompletionTimelineResponse = {
            version: 14,
            traces: [
                {
                    trace_id: 'trace-v14',
                    request_id: 'req-v14',
                    uri: 'file:///tmp/v14.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_010,
                    total_duration_ms: 14,
                    dominant_stage: 'query_bundle',
                    server_edge_details: {
                        transport_received_at_ms: 1_700_000_000_000,
                        service_future_created_at_ms: 1_700_000_000_001,
                        service_future_first_poll_entered_at_ms: 1_700_000_000_002,
                        service_future_first_poll_outcome: 'pending',
                        service_future_first_wake_scheduled_at_ms: 1_700_000_000_003,
                        first_poll_contention_attribution: {
                            contender_class: 'completion',
                            uri_scope: 'same_uri',
                            inflight_count: 1,
                            oldest_inflight_age_ms: 1,
                            concurrency_level: 16,
                        },
                        first_poll_contention_contenders: [
                            {
                                request_class: 'completion',
                                method: 'textDocument/completion',
                                phase: 'query_bundle',
                                uri: 'file:///tmp/v14.bsl',
                                age_ms: 1,
                            },
                        ],
                        handler_entered_at_ms: 1_700_000_000_004,
                        response_sent_at_ms: 1_700_000_000_014,
                        transport_to_handler_wait_ms: 4,
                        server_handler_exec_ms: 10,
                    },
                    stages: [
                        {
                            name: 'query_bundle',
                            status: 'completed',
                            started_offset_ms: 0,
                            duration_ms: 14,
                        },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.deepStrictEqual(
            state.traces[0].server_edge_details?.first_poll_contention_contenders,
            [
                {
                    request_class: 'completion',
                    method: 'textDocument/completion',
                    uri: 'file:///tmp/v14.bsl',
                    age_ms: 1,
                },
            ]
        );
    });

    test('Legacy v2 payload without server edge details remains readable', () => {
        const payload = {
            version: 2,
            traces: [
                {
                    trace_id: 'trace-legacy',
                    request_id: 'req-legacy',
                    uri: 'file:///tmp/legacy.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_empty',
                    started_at_ms: 1_700_000_000_010,
                    total_duration_ms: 14,
                    dominant_stage: 'query_bundle',
                    stages: [
                        {
                            name: 'query_bundle',
                            status: 'completed',
                            started_offset_ms: 0,
                            duration_ms: 14,
                        },
                    ],
                },
            ],
        } as CompletionTimelineResponse;

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.strictEqual(state.version, 2);
        assert.strictEqual(state.traces[0].trace_id, 'trace-legacy');
        assert.strictEqual(state.traces[0].server_edge_details, undefined);
    });

    test('Dominant stage highlight should fallback to max duration stage', () => {
        const payload: CompletionTimelineResponse = {
            version: 1,
            traces: [
                {
                    trace_id: 'trace-dominant',
                    request_id: 'req-dominant',
                    uri: 'file:///tmp/test.bsl',
                    trigger_mode: 'trigger_character',
                    outcome: 'ok_empty',
                    started_at_ms: 1_700_000_000_001,
                    total_duration_ms: 25,
                    dominant_stage: 'missing_stage',
                    prepare_details: {
                        wait_budget_ms: 120,
                    },
                    stages: [
                        { name: 'sync_globals', status: 'completed', started_offset_ms: 0, duration_ms: 5 },
                        { name: 'query_bundle', status: 'completed', started_offset_ms: 5, duration_ms: 15 },
                        { name: 'response_build', status: 'completed', started_offset_ms: 20, duration_ms: 5 },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        const dominant = state.traces[0].stages.filter((stage) => stage.is_dominant);
        assert.strictEqual(dominant.length, 1);
        assert.strictEqual(dominant[0].name, 'query_bundle');
    });

    test('Overhead and stage percent should be derived from total duration', () => {
        const payload: CompletionTimelineResponse = {
            version: 1,
            traces: [
                {
                    trace_id: 'trace-overhead',
                    request_id: 'req-overhead',
                    uri: 'file:///tmp/test.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_500,
                    total_duration_ms: 50,
                    dominant_stage: 'query_bundle',
                    prepare_details: {
                        wait_budget_ms: 120,
                        outcome: 'ready',
                    },
                    stages: [
                        { name: 'prepare_stateful', status: 'completed', started_offset_ms: 0, duration_ms: 10 },
                        { name: 'query_bundle', status: 'completed', started_offset_ms: 10, duration_ms: 30 },
                        { name: 'response_build_other', status: 'completed', started_offset_ms: 40, duration_ms: 5 },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        const trace = state.traces[0];
        assert.strictEqual(trace.max_stage_end_ms, 45);
        assert.strictEqual(trace.unattributed_overhead_ms, 5);

        const queryBundle = trace.stages.find((stage) => stage.name === 'query_bundle');
        assert.ok(queryBundle, 'query_bundle stage should exist');
        assert.ok(queryBundle!.duration_percent > 59.9 && queryBundle!.duration_percent < 60.1);
    });

    test('Average trace should aggregate durations and expose sample count', () => {
        const payload: CompletionTimelineResponse = {
            version: 1,
            traces: [
                {
                    trace_id: 'trace-1',
                    request_id: 'req-1',
                    uri: 'file:///tmp/test1.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_001,
                    total_duration_ms: 30,
                    dominant_stage: 'query_bundle',
                    stages: [
                        { name: 'prepare_stateful', status: 'completed', started_offset_ms: 0, duration_ms: 10 },
                        { name: 'query_bundle', status: 'completed', started_offset_ms: 10, duration_ms: 20 },
                    ],
                },
                {
                    trace_id: 'trace-2',
                    request_id: 'req-2',
                    uri: 'file:///tmp/test2.bsl',
                    trigger_mode: 'trigger_character',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_002,
                    total_duration_ms: 50,
                    dominant_stage: 'query_bundle',
                    stages: [
                        { name: 'prepare_stateful', status: 'completed', started_offset_ms: 0, duration_ms: 20 },
                        { name: 'query_bundle', status: 'completed', started_offset_ms: 20, duration_ms: 30 },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.ok(state.average_trace, 'average trace should exist');
        const average = state.average_trace!;
        assert.strictEqual(average.sample_count, 2);
        assert.strictEqual(average.trace_id, 'average(2)');

        const prepare = average.stages.find((stage) => stage.name === 'prepare_stateful');
        const query = average.stages.find((stage) => stage.name === 'query_bundle');
        assert.ok(prepare);
        assert.ok(query);
        assert.strictEqual(prepare!.duration_ms, 15);
        assert.strictEqual(query!.duration_ms, 25);
    });

    test('Average trace provenance notice should mark averaged traces as synthetic', () => {
        assert.strictEqual(
            getAverageTraceProvenanceNotice({
                trace_id: 'average(2)',
                trigger_mode: 'averaged',
            } as never),
            'Average trace is synthetic; v8 trustworthy pre-method attribution provenance, v9 pre-service-scope split, v10 dispatch split, v11 first-poll / first-wake split, v12 first-poll contention attribution, v13 contender snapshot, v14 executeCommand command detail, and v15 completion phase detail are unavailable by design.'
        );
        assert.strictEqual(
            getAverageTraceProvenanceNotice({
                trace_id: 'trace-1',
                trigger_mode: 'invoked',
            } as never),
            null
        );
    });

    test('Legacy unsupported path should map to explicit unsupported state', () => {
        const state = mapCompletionTimelineFetchResultToPanelState(
            { kind: 'unsupported' },
            [buildClientProbe('probe-legacy', 3, 1_700_000_000_100)],
            1_700_000_000_120
        );
        assert.strictEqual(state.kind, 'unsupported');
        if (state.kind !== 'unsupported') {
            return;
        }
        assert.ok(state.message.includes('bsl.getCompletionTimeline'));
        assert.strictEqual(state.client_probe_feed.probes.length, 1);
        assert.strictEqual(state.client_probe_feed.probes[0].probe_id, 'probe-legacy');
        assert.strictEqual(state.client_probe_feed.probes[0].result_kind, 'non_empty');
    });
});
