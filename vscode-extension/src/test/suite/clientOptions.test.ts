import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { buildClientOptions } from '../../lsp/client/client-options';
import * as customRequestsModule from '../../lsp/customRequests';
import type { SnapshotStatusResponse } from '../../lsp/customRequests';
import { CompletionProbeRecorder } from '../../providers/completionProbeRecorder';
import { CompletionProbeStore } from '../../providers/completionProbeStore';
import * as snapshotStatusModule from '../../lsp/snapshotStatus';

function createDocument(version: number, lineText: string): vscode.TextDocument {
    return {
        uri: vscode.Uri.parse('file:///tmp/default-path.bsl'),
        version,
        lineAt: () => ({ text: lineText }),
    } as unknown as vscode.TextDocument;
}

suite('Client Options Test Suite', () => {
    let sandbox: sinon.SinonSandbox;

    setup(() => {
        sandbox = sinon.createSandbox();
    });

    teardown(() => {
        sandbox.restore();
    });

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

    test('default LanguageClient path records cancelled terminal state when token is cancelled before empty completion resolves', async () => {
        const clock = sinon.useFakeTimers({ now: 1_700_000_030_000 });
        const recorder = new CompletionProbeRecorder({
            store: new CompletionProbeStore(4),
        });
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        const document = createDocument(11, 'Документы.');
        const options = buildClientOptions(outputChannel, recorder);
        const provideCompletionItem = options.middleware?.provideCompletionItem;
        const cancellationSource = new vscode.CancellationTokenSource();

        assert.ok(
            provideCompletionItem,
            'default client path must expose provideCompletionItem middleware'
        );

        try {
            const result = await provideCompletionItem!(
                document,
                new vscode.Position(0, 'Документы.'.length),
                {
                    triggerKind: vscode.CompletionTriggerKind.Invoke,
                    triggerCharacter: undefined,
                },
                cancellationSource.token,
                async () => {
                    await clock.tickAsync(5);
                    cancellationSource.cancel();
                    await clock.tickAsync(2);
                    return [] as vscode.CompletionItem[];
                }
            );

            assert.deepStrictEqual(result, []);

            const snapshot = recorder.snapshot();
            assert.strictEqual(snapshot.length, 1);
            assert.strictEqual(snapshot[0].client_terminal_state, 'cancelled');
            assert.strictEqual(snapshot[0].client_duration_ms, 7);
        } finally {
            clock.restore();
            cancellationSource.dispose();
        }
    });

    test('default LanguageClient path retries empty hover once when snapshot status reaches ready_same_revision', async () => {
        const clock = sinon.useFakeTimers({
            now: 1_700_000_040_000,
            shouldClearNativeTimers: true,
        });
        const recorder = new CompletionProbeRecorder({
            store: new CompletionProbeStore(4),
        });
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        const document = createDocument(12, 'ТаблЗнач1 = Новый ТаблицаЗначений;');
        const options = buildClientOptions(outputChannel, recorder);
        const provideHover = options.middleware?.provideHover;

        assert.ok(provideHover, 'default client path must expose provideHover middleware');

        let snapshotStatus: SnapshotStatusResponse = {
            schemaVersion: 1,
            uri: document.uri.toString(),
            requestedVersion: 12,
            state: 'building',
            exact: false,
            taskState: 'in_flight_same_revision',
            updatedAtMs: 1,
        };
        let snapshotListener: (() => void) | undefined;

        sandbox
            .stub(snapshotStatusModule, 'getSnapshotStatusForUri')
            .callsFake((uri: string) => (uri === document.uri.toString() ? snapshotStatus : undefined));
        sandbox.stub(snapshotStatusModule, 'onSnapshotStatusChange').callsFake((listener: () => void) => {
            snapshotListener = listener;
            return new vscode.Disposable(() => {
                if (snapshotListener === listener) {
                    snapshotListener = undefined;
                }
            });
        });
        const primeStub = sandbox
            .stub(customRequestsModule, 'primeExactTypeIndex')
            .resolves({
                accepted: true,
                alreadyReady: false,
                observedVersion: 12,
                action: 'promoted',
            });
        const next = sandbox.stub();
        next.onFirstCall().resolves(null);
        next.onSecondCall().resolves(
            new vscode.Hover(new vscode.MarkdownString('`ТаблицаЗначений`'))
        );

        try {
            const hoverPromise = provideHover!(
                document,
                new vscode.Position(0, 5),
                new vscode.CancellationTokenSource().token,
                next
            );

            await clock.tickAsync(5);
            assert.strictEqual(next.callCount, 1, 'first hover request must be attempted immediately');

            snapshotStatus = {
                ...snapshotStatus,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                readyVersion: 12,
                updatedAtMs: 2,
            };
            snapshotListener?.();

            const hover = await hoverPromise;
            assert.ok(hover instanceof vscode.Hover, 'retry must resolve to a hover');
            assert.strictEqual(next.callCount, 2, 'hover middleware must retry once after ready snapshot');
            assert.strictEqual(primeStub.callCount, 1, 'ready snapshot retry must prime exact index once');
            assert.deepStrictEqual(primeStub.firstCall.args[0], {
                uri: document.uri.toString(),
                requestedVersion: 12,
                reason: 'hover_cold_snapshot_retry',
            });
        } finally {
            clock.restore();
        }
    });
});
