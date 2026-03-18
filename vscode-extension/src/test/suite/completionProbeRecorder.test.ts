import * as assert from 'assert';
import * as vscode from 'vscode';
import { CompletionProbeRecorder } from '../../providers/completionProbeRecorder';
import { CompletionProbeStore } from '../../providers/completionProbeStore';

function createDocument(version: number, lineText: string): vscode.TextDocument {
    return {
        uri: vscode.Uri.parse('file:///tmp/module.bsl'),
        version,
        lineAt: () => ({ text: lineText }),
    } as unknown as vscode.TextDocument;
}

suite('Completion Probe Recorder Test Suite', () => {
    test('records same-version didChange timings and derived completion context', () => {
        let nowMs = 1_700_000_000_000;
        const recorder = new CompletionProbeRecorder({
            now: () => nowMs,
            store: new CompletionProbeStore(4),
        });
        const document = createDocument(7, 'Справочники.');

        recorder.recordTextDocumentDidChange({
            document,
            contentChanges: [{ text: '.', rangeLength: 0 }] as any,
        } as vscode.TextDocumentChangeEvent);

        nowMs += 10;
        recorder.recordTextDocumentDidChangeSent(document);

        nowMs += 15;
        recorder.recordCompletionOutcome({
            document,
            position: new vscode.Position(0, 'Справочники.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            result: [{ label: 'Номенклатура' }] as vscode.CompletionItem[],
            requestStartedAtMs: nowMs,
            requestCompletedAtMs: nowMs + 5,
            wasCancelled: false,
        });

        const snapshot = recorder.snapshot();
        assert.strictEqual(snapshot.length, 1);
        assert.strictEqual(snapshot[0].document_version, 7);
        assert.strictEqual(snapshot[0].trigger_mode, 'trigger_character');
        assert.strictEqual(snapshot[0].trigger_character, '.');
        assert.strictEqual(snapshot[0].time_since_last_local_edit_ms, 25);
        assert.strictEqual(snapshot[0].time_since_last_did_change_sent_ms, 15);
        assert.strictEqual(snapshot[0].is_after_dot, true);
        assert.strictEqual(snapshot[0].identifier_tail_length, 0);
        assert.strictEqual(snapshot[0].client_terminal_state, 'ok_non_empty');
    });

    test('records unknown didChange timing and cancelled terminal state when exact version is unavailable', () => {
        let nowMs = 1_700_000_000_100;
        const recorder = new CompletionProbeRecorder({
            now: () => nowMs,
            store: new CompletionProbeStore(4),
        });
        const document = createDocument(11, 'Переменная');

        recorder.recordCompletionOutcome({
            document,
            position: new vscode.Position(0, 'Переменная'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.Invoke,
                triggerCharacter: undefined,
            },
            result: undefined,
            requestStartedAtMs: nowMs,
            requestCompletedAtMs: nowMs + 3,
            wasCancelled: true,
        });

        const snapshot = recorder.snapshot();
        assert.strictEqual(snapshot.length, 1);
        assert.strictEqual(snapshot[0].time_since_last_local_edit_ms, 0);
        assert.strictEqual(snapshot[0].time_since_last_did_change_sent_ms, 'unknown');
        assert.strictEqual(snapshot[0].client_terminal_state, 'cancelled');
        assert.strictEqual(snapshot[0].trigger_mode, 'invoked');
    });

    test('records error terminal state for non-cancelled completion failures', () => {
        let nowMs = 1_700_000_000_200;
        const recorder = new CompletionProbeRecorder({
            now: () => nowMs,
            store: new CompletionProbeStore(4),
        });
        const document = createDocument(13, 'Справочники.Номенклатура');

        recorder.recordCompletionOutcome({
            document,
            position: new vscode.Position(0, 'Справочники.Номенклатура'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.Invoke,
                triggerCharacter: undefined,
            },
            result: undefined,
            requestStartedAtMs: nowMs,
            requestCompletedAtMs: nowMs + 7,
            wasCancelled: false,
            error: new Error('completion failed'),
        });

        const snapshot = recorder.snapshot();
        assert.strictEqual(snapshot.length, 1);
        assert.strictEqual(snapshot[0].client_terminal_state, 'error');
        assert.strictEqual(snapshot[0].client_duration_ms, 7);
    });
});
