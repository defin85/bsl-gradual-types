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

function createEditor(
    document: vscode.TextDocument,
    character: number
): vscode.TextEditor {
    return {
        document,
        selection: new vscode.Selection(
            new vscode.Position(0, character),
            new vscode.Position(0, character)
        ),
    } as unknown as vscode.TextEditor;
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

    test('records bounded supersession diagnostics for same-version cancellations', () => {
        let nowMs = 1_700_000_001_000;
        const recorder = new CompletionProbeRecorder({
            now: () => nowMs,
            store: new CompletionProbeStore(8),
        });
        const document = createDocument(7, 'Документы.');
        const firstToken = new vscode.CancellationTokenSource();
        const secondToken = new vscode.CancellationTokenSource();

        recorder.recordCompletionStarted({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.Invoke,
                triggerCharacter: undefined,
            },
            token: firstToken.token,
            requestStartedAtMs: nowMs,
        });

        nowMs += 12;
        recorder.recordCompletionStarted({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: secondToken.token,
            requestStartedAtMs: nowMs,
        });

        nowMs += 4;
        recorder.recordCompletionOutcome({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: secondToken.token,
            result: [{ label: 'Форма' }] as vscode.CompletionItem[],
            requestStartedAtMs: nowMs - 4,
            requestCompletedAtMs: nowMs,
            wasCancelled: false,
        });

        nowMs += 6;
        recorder.recordCompletionOutcome({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.Invoke,
                triggerCharacter: undefined,
            },
            token: firstToken.token,
            result: undefined,
            requestStartedAtMs: 1_700_000_001_000,
            requestCompletedAtMs: nowMs,
            wasCancelled: true,
        });

        const snapshot = recorder.snapshot();
        const cancelled = snapshot.find((probe) => probe.client_terminal_state === 'cancelled');
        const winner = snapshot.find((probe) => probe.client_terminal_state === 'ok_non_empty');

        assert.ok(cancelled);
        assert.ok(winner);
        assert.strictEqual(cancelled?.cancel_reason_hint, 'superseded_same_version');
        assert.strictEqual(cancelled?.superseded_by_probe_id, winner?.probe_id);
        assert.strictEqual(cancelled?.superseded_after_ms, 12);
        assert.strictEqual(cancelled?.newer_probe_started_before_terminal, true);
        assert.strictEqual(cancelled?.active_completion_count_at_start, 0);
        assert.strictEqual(winner?.active_completion_count_at_start, 1);
        assert.strictEqual(winner?.same_uri_probe_overlap_count, 1);

        firstToken.dispose();
        secondToken.dispose();
    });

    test('records transport, result-shape, drift and overlap diagnostics', () => {
        let nowMs = 1_700_000_002_000;
        const recorder = new CompletionProbeRecorder({
            now: () => nowMs,
            store: new CompletionProbeStore(8),
        });
        const documentV7 = createDocument(7, 'Документы.');
        const documentV8 = createDocument(8, 'Документы.');
        const token = new vscode.CancellationTokenSource();

        recorder.recordTextDocumentDidChange({
            document: documentV7,
            contentChanges: [{ text: '.', rangeLength: 0 }] as any,
        } as vscode.TextDocumentChangeEvent);
        recorder.recordTextEditorSelectionChanged(createEditor(documentV7, 'Документы.'.length));
        recorder.recordCompletionStarted({
            document: documentV7,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: token.token,
            requestStartedAtMs: nowMs,
        });

        nowMs += 3;
        recorder.recordCompletionLspRequestStarted(token.token, nowMs);

        nowMs += 10;
        recorder.recordTextDocumentDidChange({
            document: documentV8,
            contentChanges: [{ text: 'Н', rangeLength: 0 }] as any,
        } as vscode.TextDocumentChangeEvent);
        recorder.recordTextEditorSelectionChanged(createEditor(documentV8, 'Документы.Н'.length));

        nowMs += 17;
        recorder.recordCompletionRawTransportResponseReceived('probe-1', nowMs - 3);
        recorder.recordCompletionLspResponseResolved(token.token, nowMs);

        nowMs += 5;
        recorder.recordCompletionOutcome({
            document: documentV8,
            position: new vscode.Position(0, 'Документы.Н'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: token.token,
            result: {
                items: [],
                isIncomplete: true,
            } as vscode.CompletionList,
            requestStartedAtMs: 1_700_000_002_000,
            requestCompletedAtMs: nowMs,
            wasCancelled: false,
        });

        const snapshot = recorder.snapshot();
        assert.strictEqual(snapshot.length, 1);
        assert.strictEqual(snapshot[0].document_version, 7);
        assert.strictEqual(snapshot[0].document_version_at_terminal, 8);
        assert.strictEqual(snapshot[0].lsp_request_started_at_ms, 1_700_000_002_003);
        assert.strictEqual(snapshot[0].transport_response_receive_state, 'observed');
        assert.strictEqual(snapshot[0].transport_response_received_at_ms, 1_700_000_002_027);
        assert.strictEqual(snapshot[0].lsp_response_received_at_ms, 1_700_000_002_030);
        assert.strictEqual(snapshot[0].result_kind, 'empty_list');
        assert.strictEqual(snapshot[0].item_count_bucket, '0');
        assert.strictEqual(snapshot[0].is_incomplete, true);
        assert.strictEqual(snapshot[0].did_change_count_during_probe, 1);
        assert.strictEqual(snapshot[0].cursor_moved_during_probe, true);

        token.dispose();
    });
});
