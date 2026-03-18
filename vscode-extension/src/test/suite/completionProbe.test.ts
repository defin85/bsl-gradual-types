import * as assert from 'assert';
import {
    buildCompletionProbe,
    COMPLETION_PROBE_MAX_IDENTIFIER_TAIL_LENGTH,
    COMPLETION_PROBE_MAX_TRIGGER_CHARACTER_LENGTH,
} from '../../providers/completionProbe';

suite('Completion Probe Schema Test Suite', () => {
    test('buildCompletionProbe stores only bounded redacted fields', () => {
        const probe = buildCompletionProbe({
            probe_id: 'probe-42',
            uri: 'file:///tmp/example.bsl',
            document_version: 7,
            document_version_at_terminal: 8,
            trigger_mode: 'trigger_character',
            trigger_character: '::very-long-trigger::',
            request_started_at_ms: 1_700_000_000_000,
            lsp_request_started_at_ms: 1_700_000_000_003,
            lsp_response_received_at_ms: 1_700_000_000_120,
            request_completed_at_ms: 1_700_000_000_125,
            client_terminal_state: 'ok_non_empty',
            cancel_reason_hint: 'unknown',
            result_kind: 'non_empty',
            item_count_bucket: '21_plus',
            is_incomplete: true,
            time_since_last_local_edit_ms: 31,
            time_since_last_did_change_sent_ms: 'unknown',
            did_change_count_during_probe: 2,
            cursor_moved_during_probe: true,
            active_completion_count_at_start: 1,
            same_uri_probe_overlap_count: 1,
            newer_probe_started_before_terminal: true,
            superseded_by_probe_id: 'probe-99',
            superseded_after_ms: 15,
            is_after_dot: true,
            identifier_tail_length: 19,
            raw_document_text: 'Секрет = Новый Массив;',
            line_prefix: 'Секрет.',
            free_form_label: 'user-generated',
        } as any);

        assert.deepStrictEqual(Object.keys(probe).sort(), [
            'active_completion_count_at_start',
            'cancel_reason_hint',
            'client_duration_ms',
            'client_terminal_state',
            'cursor_moved_during_probe',
            'did_change_count_during_probe',
            'document_version',
            'document_version_at_terminal',
            'identifier_tail_length',
            'is_after_dot',
            'is_incomplete',
            'item_count_bucket',
            'lsp_request_started_at_ms',
            'lsp_response_received_at_ms',
            'newer_probe_started_before_terminal',
            'probe_id',
            'request_completed_at_ms',
            'request_started_at_ms',
            'result_kind',
            'same_uri_probe_overlap_count',
            'superseded_after_ms',
            'superseded_by_probe_id',
            'time_since_last_did_change_sent_ms',
            'time_since_last_local_edit_ms',
            'trigger_character',
            'trigger_mode',
            'uri',
        ]);
        assert.strictEqual(probe.trigger_character?.length, COMPLETION_PROBE_MAX_TRIGGER_CHARACTER_LENGTH);
        assert.strictEqual(probe.time_since_last_did_change_sent_ms, 'unknown');
        assert.ok(!('raw_document_text' in probe));
        assert.ok(!('line_prefix' in probe));
        assert.ok(!('free_form_label' in probe));
    });

    test('buildCompletionProbe clamps numeric fields and derives non-negative duration', () => {
        const probe = buildCompletionProbe({
            probe_id: 'probe-43',
            uri: 'file:///tmp/example.bsl',
            document_version: -9,
            document_version_at_terminal: -10,
            trigger_mode: 'invoked',
            request_started_at_ms: 1_700_000_000_250,
            lsp_request_started_at_ms: 1_700_000_000_240,
            lsp_response_received_at_ms: Number.NaN,
            request_completed_at_ms: 1_700_000_000_200,
            client_terminal_state: 'cancelled',
            cancel_reason_hint: 'superseded_newer_version',
            result_kind: 'nullish',
            item_count_bucket: '0',
            time_since_last_local_edit_ms: -5,
            time_since_last_did_change_sent_ms: -3,
            did_change_count_during_probe: -7,
            cursor_moved_during_probe: false,
            active_completion_count_at_start: -2,
            same_uri_probe_overlap_count: -1,
            newer_probe_started_before_terminal: false,
            superseded_after_ms: -15,
            is_after_dot: false,
            identifier_tail_length: COMPLETION_PROBE_MAX_IDENTIFIER_TAIL_LENGTH + 500,
        });

        assert.strictEqual(probe.document_version, 0);
        assert.strictEqual(probe.document_version_at_terminal, 0);
        assert.strictEqual(probe.client_duration_ms, 0);
        assert.strictEqual(probe.lsp_request_started_at_ms, 1_700_000_000_240);
        assert.strictEqual(probe.lsp_response_received_at_ms, 0);
        assert.strictEqual(probe.time_since_last_local_edit_ms, 0);
        assert.strictEqual(probe.time_since_last_did_change_sent_ms, 0);
        assert.strictEqual(probe.did_change_count_during_probe, 0);
        assert.strictEqual(probe.active_completion_count_at_start, 0);
        assert.strictEqual(probe.same_uri_probe_overlap_count, 0);
        assert.strictEqual(probe.superseded_after_ms, 0);
        assert.strictEqual(
            probe.identifier_tail_length,
            COMPLETION_PROBE_MAX_IDENTIFIER_TAIL_LENGTH
        );
    });

    test('buildCompletionProbe preserves bounded terminal summary fields', () => {
        const probe = buildCompletionProbe({
            probe_id: 'probe-44',
            uri: 'file:///tmp/example.bsl',
            document_version: 15,
            document_version_at_terminal: 15,
            trigger_mode: 'trigger_for_incomplete_completions',
            request_started_at_ms: 1_700_000_000_500,
            lsp_request_started_at_ms: 1_700_000_000_510,
            lsp_response_received_at_ms: 1_700_000_000_560,
            request_completed_at_ms: 1_700_000_000_580,
            client_terminal_state: 'ok_empty',
            cancel_reason_hint: 'editor_state_changed',
            result_kind: 'empty_list',
            item_count_bucket: '0',
            is_incomplete: false,
            time_since_last_local_edit_ms: 80,
            time_since_last_did_change_sent_ms: 12,
            did_change_count_during_probe: 1,
            cursor_moved_during_probe: true,
            active_completion_count_at_start: 2,
            same_uri_probe_overlap_count: 1,
            newer_probe_started_before_terminal: false,
            is_after_dot: false,
            identifier_tail_length: 0,
        });

        assert.strictEqual(probe.document_version_at_terminal, 15);
        assert.strictEqual(probe.client_terminal_state, 'ok_empty');
        assert.strictEqual(probe.client_duration_ms, 80);
        assert.strictEqual(probe.trigger_mode, 'trigger_for_incomplete_completions');
        assert.strictEqual(probe.cancel_reason_hint, 'editor_state_changed');
        assert.strictEqual(probe.result_kind, 'empty_list');
        assert.strictEqual(probe.item_count_bucket, '0');
        assert.strictEqual(probe.is_incomplete, false);
    });
});
