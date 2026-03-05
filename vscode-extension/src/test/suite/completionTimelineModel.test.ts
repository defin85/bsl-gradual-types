import * as assert from 'assert';
import {
    mapCompletionTimelineFetchResultToPanelState,
    mapCompletionTimelineResponseToPanelState,
} from '../../providers/completionTimelineModel';
import { CompletionTimelineResponse } from '../../lsp/customRequests';

suite('Completion Timeline Model Test Suite', () => {
    test('Mapping LSP timeline payload -> UI model', () => {
        const payload: CompletionTimelineResponse = {
            version: 1,
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
                    stages: [
                        { name: 'prepare_stateful', status: 'completed', started_offset_ms: 0, duration_ms: 12 },
                        { name: 'query_bundle', status: 'completed', started_offset_ms: 12, duration_ms: 30 },
                        { name: 'response_build', status: 'completed', started_offset_ms: 42, duration_ms: 6 },
                    ],
                },
            ],
        };

        const state = mapCompletionTimelineResponseToPanelState(payload, 1_700_000_000_100);
        assert.strictEqual(state.kind, 'ready');
        if (state.kind !== 'ready') {
            return;
        }

        assert.strictEqual(state.version, 1);
        assert.strictEqual(state.traces.length, 1);
        assert.strictEqual(state.traces[0].trace_id, 'trace-42');
        assert.strictEqual(state.traces[0].stages.length, 3);
        assert.ok(state.traces[0].stages.every((stage) => stage.width_percent >= 0));
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

    test('Legacy unsupported path should map to explicit unsupported state', () => {
        const state = mapCompletionTimelineFetchResultToPanelState({ kind: 'unsupported' });
        assert.strictEqual(state.kind, 'unsupported');
        if (state.kind !== 'unsupported') {
            return;
        }
        assert.ok(state.message.includes('bsl.getCompletionTimeline'));
    });
});

