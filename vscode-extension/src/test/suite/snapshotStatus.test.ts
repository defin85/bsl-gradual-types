import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';

import * as customRequestsModule from '../../lsp/customRequests';
import {
    formatSnapshotStatusLogLine,
    getActiveSnapshotStatusSnapshot,
    getSnapshotStatusHistoryForUri,
    handleSnapshotStatusNotification,
    initializeSnapshotStatus,
    resetSnapshotStatusForTests,
    sanitizeSnapshotDetail,
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
                schemaVersion: 2,
                uri: 'file:///snapshot-status-building.bsl',
                requestedVersion: 9,
                state: 'building',
                exact: false,
                taskState: 'in_flight_same_revision',
                phase: 'parsing',
                reason: {
                    code: 'building',
                    message: 'A matching snapshot worker is building the requested revision',
                },
                worker: {
                    targetVersion: 9,
                    phase: 'parsing',
                    trigger: 'did_change',
                    ageMs: 42,
                },
                artifacts: {
                    readyParseSnapshot: { state: 'building', version: 9 },
                    exactTypeIndex: { state: 'building', version: 9 },
                },
                recommendation: 'wait',
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
            assert.match(statusBarStub.tooltip, /reason=building/);
            assert.match(statusBarStub.tooltip, /worker target=v9 phase=parsing/);
        } finally {
            disposable.dispose();
        }
    });

    test('shows unavailable immediately while active BSL snapshot hydration is pending', async () => {
        stubActiveBslEditor('file:///snapshot-status-pending.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').returns(
            new Promise(() => {})
        );

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            const snapshot = getActiveSnapshotStatusSnapshot();
            assert.strictEqual(snapshot.kind, 'unavailable');
            assert.match(statusBarStub.text, /unavailable/i);
            assert.strictEqual(statusBarStub.show.callCount > 0, true);
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

    test('hydrates failed status for active BSL document', async () => {
        stubActiveBslEditor('file:///snapshot-status-failed.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 1,
                uri: 'file:///snapshot-status-failed.bsl',
                requestedVersion: 18,
                state: 'failed',
                exact: false,
                taskState: 'absent',
                fallbackReason: 'build_snapshot_aborted',
                updatedAtMs: 450,
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
            assert.strictEqual(snapshot.status.state, 'failed');
            assert.match(statusBarStub.text, /failed v18/i);
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
            assert.match(statusBarStub.text, /unsupported/i);
            assert.strictEqual(statusBarStub.show.callCount > 0, true);
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
            const history = getSnapshotStatusHistoryForUri('file:///snapshot-status-stale.bsl');
            assert.strictEqual(history.length, 1);
            assert.strictEqual(history[0].updatedAtMs, 500);
        } finally {
            disposable.dispose();
        }
    });

    test('transition history keeps bounded accepted updates', async () => {
        const uri = 'file:///snapshot-status-history.bsl';
        stubActiveBslEditor(uri);
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 2,
                uri,
                requestedVersion: 1,
                readyVersion: 1,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                updatedAtMs: 1000,
            },
        });

        const disposable = initializeSnapshotStatus(outputChannelStub, statusBarStub);
        await flushPromises();

        try {
            for (let index = 0; index < 25; index += 1) {
                handleSnapshotStatusNotification({
                    schemaVersion: 2,
                    uri,
                    requestedVersion: index + 2,
                    state: index % 2 === 0 ? 'building' : 'shadow_only',
                    exact: false,
                    taskState: 'in_flight_same_revision',
                    updatedAtMs: 1001 + index,
                });
            }

            const history = getSnapshotStatusHistoryForUri(uri);
            assert.strictEqual(history.length, 20);
            assert.strictEqual(history[0].updatedAtMs, 1006);
            assert.strictEqual(history[19].updatedAtMs, 1025);
        } finally {
            disposable.dispose();
        }
    });

    test('snapshot status log line includes bounded readiness details', () => {
        const line = formatSnapshotStatusLogLine({
            schemaVersion: 2,
            uri: 'file:///snapshot-status-log.bsl',
            requestedVersion: 17,
            readyVersion: 16,
            state: 'building',
            exact: false,
            taskState: 'in_flight_same_revision',
            phase: 'parsing',
            trigger: 'did_change',
            fallbackReason: 'input_edit_conversion_failed',
            reason: {
                code: 'shadow_only_exact_missing',
                message: 'Only the editor shadow is current',
            },
            artifacts: {
                readyParseSnapshot: {
                    state: 'stale',
                    version: 16,
                    detail: 'fallback\nreason\twith whitespace',
                },
                exactTypeIndex: { state: 'missing', version: 17 },
            },
            lastFailure: {
                stage: 'snapshot_build',
                reason: 'build_snapshot_aborted',
                message: `line1\n${'x'.repeat(220)}`,
                requestedVersion: 17,
            },
            recommendation: 'prime_exact_index',
            updatedAtMs: 777,
        });

        assert.ok(line.includes('state=building'));
        assert.ok(line.includes('requested=v17'));
        assert.ok(line.includes('ready=v16'));
        assert.ok(line.includes('task=in_flight_same_revision'));
        assert.ok(line.includes('phase=parsing'));
        assert.ok(line.includes('trigger=did_change'));
        assert.ok(line.includes('fallback=input_edit_conversion_failed'));
        assert.ok(line.includes('reason=shadow_only_exact_missing'));
        assert.ok(line.includes('readyParse=stale version=v16 detail=fallback reason with whitespace'));
        assert.ok(line.includes('lastFailure=snapshot_build'));
        assert.ok(line.includes('recommendation=prime_exact_index'));
        assert.ok(!line.includes('\n'));
        assert.ok(line.includes('updatedAtMs=777'));
    });

    test('sanitizeSnapshotDetail removes control whitespace and caps free text', () => {
        const sanitized = sanitizeSnapshotDetail(`line1\nline2\t${'x'.repeat(220)}`);
        assert.ok(sanitized);
        assert.ok(!sanitized.includes('\n'));
        assert.ok(!sanitized.includes('\t'));
        assert.strictEqual(sanitized.length, 160);
    });
});
