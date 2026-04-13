import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';

import * as customRequestsModule from '../../lsp/customRequests';
import {
    getActiveSnapshotStatusSnapshot,
    handleSnapshotStatusNotification,
    initializeSnapshotStatus,
    resetSnapshotStatusForTests,
} from '../../lsp/snapshotStatus';

async function flushPromises(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
}

suite('Snapshot Status Test Suite', () => {
    let outputChannelStub: any;
    let statusBarStub: any;

    setup(() => {
        outputChannelStub = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub(),
        };
        statusBarStub = {
            text: '',
            tooltip: '',
            command: undefined,
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub(),
        };

        sinon
            .stub(vscode.window, 'onDidChangeActiveTextEditor')
            .callsFake(() => new vscode.Disposable(() => {}));
        sinon
            .stub(vscode.workspace, 'onDidCloseTextDocument')
            .callsFake(() => new vscode.Disposable(() => {}));
    });

    teardown(() => {
        resetSnapshotStatusForTests();
        sinon.restore();
    });

    function stubActiveBslEditor(uri = 'file:///snapshot-status-test.bsl'): vscode.TextEditor {
        const editor = {
            document: {
                languageId: 'bsl',
                uri: vscode.Uri.parse(uri),
            },
        } as unknown as vscode.TextEditor;
        sinon.stub(vscode.window, 'activeTextEditor').get(() => editor);
        return editor;
    }

    test('hydrates exact-ready status for active BSL document', async () => {
        stubActiveBslEditor();
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 1,
                uri: 'file:///snapshot-status-test.bsl',
                requestedVersion: 15,
                readyVersion: 15,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                updatedAtMs: 200,
            },
        });

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            const snapshot = getActiveSnapshotStatusSnapshot();
            assert.strictEqual(snapshot.kind, 'ok');
            if (snapshot.kind !== 'ok') {
                return;
            }
            assert.strictEqual(snapshot.status.state, 'ready');
            assert.strictEqual(snapshot.status.exact, true);
            assert.match(statusBarStub.text, /ready v15/i);
        } finally {
            disposable.dispose();
        }
    });

    test('hydrates building status for active BSL document', async () => {
        stubActiveBslEditor('file:///snapshot-status-building.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 1,
                uri: 'file:///snapshot-status-building.bsl',
                requestedVersion: 9,
                state: 'building',
                exact: false,
                taskState: 'in_flight_same_revision',
                phase: 'parsing',
                updatedAtMs: 300,
            },
        });

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            const snapshot = getActiveSnapshotStatusSnapshot();
            assert.strictEqual(snapshot.kind, 'ok');
            if (snapshot.kind !== 'ok') {
                return;
            }
            assert.strictEqual(snapshot.status.state, 'building');
            assert.strictEqual(snapshot.status.phase, 'parsing');
            assert.match(statusBarStub.text, /building v9/i);
        } finally {
            disposable.dispose();
        }
    });

    test('hydrates shadow-only status for active BSL document', async () => {
        stubActiveBslEditor('file:///snapshot-status-shadow.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 1,
                uri: 'file:///snapshot-status-shadow.bsl',
                requestedVersion: 5,
                state: 'shadow_only',
                exact: false,
                taskState: 'absent',
                updatedAtMs: 400,
            },
        });

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            const snapshot = getActiveSnapshotStatusSnapshot();
            assert.strictEqual(snapshot.kind, 'ok');
            if (snapshot.kind !== 'ok') {
                return;
            }
            assert.strictEqual(snapshot.status.state, 'shadow_only');
            assert.match(statusBarStub.text, /shadow-only v5/i);
        } finally {
            disposable.dispose();
        }
    });

    test('unsupported server stays fail-closed', async () => {
        stubActiveBslEditor('file:///snapshot-status-unsupported.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'unsupported',
        });

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            const snapshot = getActiveSnapshotStatusSnapshot();
            assert.strictEqual(snapshot.kind, 'unsupported');
            assert.strictEqual(statusBarStub.hide.callCount > 0, true);
        } finally {
            disposable.dispose();
        }
    });

    test('older notification does not overwrite newer cached state', async () => {
        stubActiveBslEditor('file:///snapshot-status-stale.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 1,
                uri: 'file:///snapshot-status-stale.bsl',
                requestedVersion: 12,
                readyVersion: 12,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                updatedAtMs: 500,
            },
        });

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            handleSnapshotStatusNotification({
                schemaVersion: 1,
                uri: 'file:///snapshot-status-stale.bsl',
                requestedVersion: 12,
                state: 'shadow_only',
                exact: false,
                taskState: 'absent',
                updatedAtMs: 499,
            });

            const snapshot = getActiveSnapshotStatusSnapshot();
            assert.strictEqual(snapshot.kind, 'ok');
            if (snapshot.kind !== 'ok') {
                return;
            }
            assert.strictEqual(snapshot.status.state, 'ready');
            assert.strictEqual(snapshot.status.updatedAtMs, 500);
            assert.match(statusBarStub.text, /ready v12/i);
        } finally {
            disposable.dispose();
        }
    });
});
