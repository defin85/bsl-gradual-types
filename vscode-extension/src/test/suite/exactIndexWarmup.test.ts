import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';

import * as customRequestsModule from '../../lsp/customRequests';
import * as snapshotStatusModule from '../../lsp/snapshotStatus';
import {
    initializeExactIndexWarmup,
    resetExactIndexWarmupForTests,
} from '../../lsp/exactIndexWarmup';

async function flushPromises(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
}

suite('Exact Index Warmup Test Suite', () => {
    let onSnapshotStatusChangeListener: (() => void) | undefined;

    setup(() => {
        sinon
            .stub(vscode.window, 'onDidChangeActiveTextEditor')
            .callsFake(() => new vscode.Disposable(() => {}));
        sinon
            .stub(vscode.workspace, 'onDidCloseTextDocument')
            .callsFake(() => new vscode.Disposable(() => {}));
        sinon
            .stub(snapshotStatusModule, 'onSnapshotStatusChange')
            .callsFake((listener: () => void) => {
                onSnapshotStatusChangeListener = listener;
                return new vscode.Disposable(() => {
                    onSnapshotStatusChangeListener = undefined;
                });
            });
    });

    teardown(() => {
        resetExactIndexWarmupForTests();
        customRequestsModule.resetPrimeExactTypeIndexCapabilityCacheForTests();
        sinon.restore();
    });

    test('primes active ready snapshot once per requested version', async () => {
        const clock = sinon.useFakeTimers();
        const uri = 'file:///exact-warmup-test.bsl';
        const editor = {
            document: {
                languageId: 'bsl',
                uri: vscode.Uri.parse(uri),
            },
        } as unknown as vscode.TextEditor;
        sinon.stub(vscode.window, 'activeTextEditor').get(() => editor);

        let requestedVersion = 7;
        sinon.stub(snapshotStatusModule, 'getActiveSnapshotStatusSnapshot').callsFake(() => ({
            kind: 'ok',
            status: {
                schemaVersion: 1,
                uri,
                requestedVersion,
                readyVersion: requestedVersion,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                updatedAtMs: 1_000 + requestedVersion,
            },
        }));

        const primeExactTypeIndexStub = sinon
            .stub(customRequestsModule, 'primeExactTypeIndex')
            .resolves({
                accepted: true,
                alreadyReady: false,
                observedVersion: requestedVersion,
                action: 'promoted',
            });

        const disposable = initializeExactIndexWarmup();
        try {
            clock.tick(60);
            await flushPromises();

            assert.strictEqual(primeExactTypeIndexStub.callCount, 1);
            assert.deepStrictEqual(primeExactTypeIndexStub.firstCall.args[0], {
                uri,
                requestedVersion: 7,
                reason: 'active_editor_cold_hover_warmup',
            });

            onSnapshotStatusChangeListener?.();
            clock.tick(60);
            await flushPromises();
            assert.strictEqual(
                primeExactTypeIndexStub.callCount,
                1,
                'same-version snapshot updates must not re-prime exact index'
            );

            requestedVersion = 8;
            onSnapshotStatusChangeListener?.();
            clock.tick(60);
            await flushPromises();
            assert.strictEqual(primeExactTypeIndexStub.callCount, 2);
            assert.deepStrictEqual(primeExactTypeIndexStub.secondCall.args[0], {
                uri,
                requestedVersion: 8,
                reason: 'active_editor_cold_hover_warmup',
            });
        } finally {
            disposable.dispose();
            clock.restore();
        }
    });
});
