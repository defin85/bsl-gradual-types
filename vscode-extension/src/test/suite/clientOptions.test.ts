import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { buildClientOptions } from '../../lsp/client/client-options';
import { CompletionProbeRecorder } from '../../providers/completionProbeRecorder';
import { CompletionProbeStore } from '../../providers/completionProbeStore';

function createDocument(version: number, lineText: string): vscode.TextDocument {
    return {
        uri: vscode.Uri.parse('file:///tmp/default-path.bsl'),
        version,
        lineAt: () => ({ text: lineText }),
    } as unknown as vscode.TextDocument;
}

suite('Client Options Test Suite', () => {
    test('default LanguageClient path wires didChange and completion middleware into probe recorder', async () => {
        const clock = sinon.useFakeTimers({ now: 1_700_000_010_000 });
        const recorder = new CompletionProbeRecorder({
            store: new CompletionProbeStore(4),
        });
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        const document = createDocument(9, 'Документы.');
        const options = buildClientOptions(outputChannel, recorder);
        const didChange = options.middleware?.didChange;
        const provideCompletionItem = options.middleware?.provideCompletionItem;

        assert.ok(didChange, 'default client path must expose didChange middleware');
        assert.ok(
            provideCompletionItem,
            'default client path must expose provideCompletionItem middleware'
        );

        try {
            await didChange!(
                {
                    document,
                    contentChanges: [{ text: '.', rangeLength: 0 }] as any,
                } as vscode.TextDocumentChangeEvent,
                async () => {
                    await clock.tickAsync(20);
                }
            );

            await clock.tickAsync(15);
            const completionItems = await provideCompletionItem!(
                document,
                new vscode.Position(0, 'Документы.'.length),
                {
                    triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                    triggerCharacter: '.',
                },
                new vscode.CancellationTokenSource().token,
                async () => {
                    await clock.tickAsync(5);
                    return [{ label: 'Форма' }] as vscode.CompletionItem[];
                }
            );

            assert.strictEqual(Array.isArray(completionItems), true);

            const snapshot = recorder.snapshot();
            assert.strictEqual(snapshot.length, 1);
            assert.strictEqual(snapshot[0].document_version, 9);
            assert.strictEqual(snapshot[0].time_since_last_did_change_sent_ms, 15);
            assert.strictEqual(snapshot[0].client_terminal_state, 'ok_non_empty');
        } finally {
            clock.restore();
        }
    });

    test('default LanguageClient path records error terminal state for non-cancelled completion failures', async () => {
        const clock = sinon.useFakeTimers({ now: 1_700_000_020_000 });
        const recorder = new CompletionProbeRecorder({
            store: new CompletionProbeStore(4),
        });
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        const document = createDocument(10, 'Документы.');
        const options = buildClientOptions(outputChannel, recorder);
        const provideCompletionItem = options.middleware?.provideCompletionItem;

        assert.ok(
            provideCompletionItem,
            'default client path must expose provideCompletionItem middleware'
        );

        try {
            await assert.rejects(
                async () =>
                    provideCompletionItem!(
                        document,
                        new vscode.Position(0, 'Документы.'.length),
                        {
                            triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                            triggerCharacter: '.',
                        },
                        new vscode.CancellationTokenSource().token,
                        async () => {
                            await clock.tickAsync(5);
                            throw new Error('server exploded');
                        }
                    ),
                /server exploded/
            );

            const snapshot = recorder.snapshot();
            assert.strictEqual(snapshot.length, 1);
            assert.strictEqual(snapshot[0].client_terminal_state, 'error');
            assert.strictEqual(snapshot[0].client_duration_ms, 5);
        } finally {
            clock.restore();
        }
    });
});
