import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import {
    instrumentCompletionProbeTransport,
    registerCompletionProbeSelectionObserver,
} from '../../lsp/client/completionProbeRuntime';
import { CompletionProbeRecorder } from '../../providers/completionProbeRecorder';
import { CompletionProbeStore } from '../../providers/completionProbeStore';

function createDocument(
    version: number,
    lineText: string,
    languageId: string = 'bsl'
): vscode.TextDocument {
    return {
        uri: vscode.Uri.parse('file:///tmp/runtime-probe.bsl'),
        version,
        languageId,
        lineAt: () => ({ text: lineText }),
    } as unknown as vscode.TextDocument;
}

suite('Completion Probe Runtime Test Suite', () => {
    test('transport observer records completion dispatch and response timestamps', async () => {
        const clock = sinon.useFakeTimers({ now: 1_700_000_100_000 });
        const recorder = new CompletionProbeRecorder({
            store: new CompletionProbeStore(4),
            now: () => Date.now(),
        });
        const tokenSource = new vscode.CancellationTokenSource();
        const document = createDocument(7, 'Документы.');
        const client = {
            sendRequest: sinon.stub().callsFake(async () => {
                await clock.tickAsync(20);
                return { items: [{ label: 'Форма' }] };
            }),
        };

        recorder.recordCompletionStarted({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: tokenSource.token,
            requestStartedAtMs: Date.now(),
        });
        instrumentCompletionProbeTransport(client, recorder, () => Date.now());

        try {
            await client.sendRequest(
                { method: 'textDocument/completion' },
                {},
                tokenSource.token
            );

            recorder.recordCompletionOutcome({
                document,
                position: new vscode.Position(0, 'Документы.'.length),
                context: {
                    triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                    triggerCharacter: '.',
                },
                token: tokenSource.token,
                result: [{ label: 'Форма' }] as vscode.CompletionItem[],
                requestStartedAtMs: 1_700_000_100_000,
                requestCompletedAtMs: Date.now(),
                wasCancelled: false,
            });

            const snapshot = recorder.snapshot();
            assert.strictEqual(snapshot.length, 1);
            assert.strictEqual(snapshot[0].lsp_request_started_at_ms, 1_700_000_100_000);
            assert.strictEqual(snapshot[0].lsp_response_received_at_ms, 1_700_000_100_020);
        } finally {
            clock.restore();
            tokenSource.dispose();
        }
    });

    test('selection observer forwards only bsl editor cursor moves to recorder', () => {
        const recorder = new CompletionProbeRecorder({
            store: new CompletionProbeStore(4),
        });
        const onDidChangeTextEditorSelection = (
            handler: (event: vscode.TextEditorSelectionChangeEvent) => void
        ): vscode.Disposable => {
            handler({
                textEditor: {
                    document: createDocument(3, 'Документы.', 'plaintext'),
                    selection: new vscode.Selection(
                        new vscode.Position(0, 0),
                        new vscode.Position(0, 0)
                    ),
                } as vscode.TextEditor,
                kind: undefined,
                selections: [],
            });
            handler({
                textEditor: {
                    document: createDocument(4, 'Документы.', 'bsl'),
                    selection: new vscode.Selection(
                        new vscode.Position(0, 1),
                        new vscode.Position(0, 1)
                    ),
                } as vscode.TextEditor,
                kind: undefined,
                selections: [],
            });
            return new vscode.Disposable(() => undefined);
        };

        const disposable = registerCompletionProbeSelectionObserver(recorder, {
            onDidChangeTextEditorSelection,
        });

        try {
            recorder.recordCompletionStarted({
                document: createDocument(4, 'Документы.', 'bsl'),
                position: new vscode.Position(0, 'Документы.'.length),
                context: {
                    triggerKind: vscode.CompletionTriggerKind.Invoke,
                    triggerCharacter: undefined,
                },
                token: new vscode.CancellationTokenSource().token,
                requestStartedAtMs: Date.now(),
            });

            const probe = recorder.recordCompletionOutcome({
                document: createDocument(4, 'Документы.', 'bsl'),
                position: new vscode.Position(0, 'Документы.'.length),
                context: {
                    triggerKind: vscode.CompletionTriggerKind.Invoke,
                    triggerCharacter: undefined,
                },
                result: undefined,
                requestStartedAtMs: Date.now(),
                requestCompletedAtMs: Date.now(),
                wasCancelled: true,
            });

            assert.strictEqual(probe.cursor_moved_during_probe, false);
        } finally {
            disposable.dispose();
        }
    });
});
