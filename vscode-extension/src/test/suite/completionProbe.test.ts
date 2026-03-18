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
            trigger_mode: 'trigger_character',
            trigger_character: '::very-long-trigger::',
            request_started_at_ms: 1_700_000_000_000,
            request_completed_at_ms: 1_700_000_000_125,
            client_terminal_state: 'ok_non_empty',
            time_since_last_local_edit_ms: 31,
            time_since_last_did_change_sent_ms: 'unknown',
            is_after_dot: true,
            identifier_tail_length: 19,
            raw_document_text: 'Секрет = Новый Массив;',
            line_prefix: 'Секрет.',
            free_form_label: 'user-generated',
        } as any);

        assert.deepStrictEqual(Object.keys(probe).sort(), [
            'client_duration_ms',
            'client_terminal_state',
            'document_version',
            'identifier_tail_length',
            'is_after_dot',
            'probe_id',
            'request_completed_at_ms',
            'request_started_at_ms',
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
            trigger_mode: 'invoked',
            request_started_at_ms: 1_700_000_000_250,
            request_completed_at_ms: 1_700_000_000_200,
            client_terminal_state: 'cancelled',
            time_since_last_local_edit_ms: -5,
            time_since_last_did_change_sent_ms: -3,
            is_after_dot: false,
            identifier_tail_length: COMPLETION_PROBE_MAX_IDENTIFIER_TAIL_LENGTH + 500,
        });

        assert.strictEqual(probe.document_version, 0);
        assert.strictEqual(probe.client_duration_ms, 0);
        assert.strictEqual(probe.time_since_last_local_edit_ms, 0);
        assert.strictEqual(probe.time_since_last_did_change_sent_ms, 0);
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
            trigger_mode: 'trigger_for_incomplete_completions',
            request_started_at_ms: 1_700_000_000_500,
            request_completed_at_ms: 1_700_000_000_580,
            client_terminal_state: 'ok_empty',
            time_since_last_local_edit_ms: 80,
            time_since_last_did_change_sent_ms: 12,
            is_after_dot: false,
            identifier_tail_length: 0,
        });

        assert.strictEqual(probe.client_terminal_state, 'ok_empty');
        assert.strictEqual(probe.client_duration_ms, 80);
        assert.strictEqual(probe.trigger_mode, 'trigger_for_incomplete_completions');
    });
});
