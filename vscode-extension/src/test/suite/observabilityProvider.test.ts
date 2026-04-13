import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { State } from 'vscode-languageclient/node';
import { ObservabilityProvider } from '../../providers/observabilityProvider';
import * as clientModule from '../../lsp/client';
import * as customRequestsModule from '../../lsp/customRequests';
import {
    initializeSnapshotStatus,
    resetSnapshotStatusForTests,
} from '../../lsp/snapshotStatus';
import {
    handleServerStatus,
    initializeServerStatus,
    resetServerStatusForTests,
} from '../../lsp/serverStatus';

async function flushPromises(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
}

suite('Observability Provider Test Suite', () => {
    let outputChannelStub: any;
    let statusBarStub: any;
    let snapshotStatusBarStub: any;
    let getObservabilityMetricsStub: sinon.SinonStub;

    setup(() => {
        outputChannelStub = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub(),
        };
        statusBarStub = {
            text: '',
            tooltip: '',
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub(),
        };
        snapshotStatusBarStub = {
            text: '',
            tooltip: '',
            command: undefined,
            show: sinon.stub(),
            hide: sinon.stub(),
            dispose: sinon.stub(),
        };

        resetServerStatusForTests();
        resetSnapshotStatusForTests();
        initializeServerStatus(outputChannelStub, statusBarStub);

        sinon.stub(clientModule, 'getLanguageClient').returns({
            state: State.Running,
            isRunning: () => true,
        } as any);
        sinon
            .stub(vscode.window, 'onDidChangeActiveTextEditor')
            .callsFake(() => new vscode.Disposable(() => {}));
        sinon
            .stub(vscode.workspace, 'onDidCloseTextDocument')
            .callsFake(() => new vscode.Disposable(() => {}));

        getObservabilityMetricsStub = sinon
            .stub(customRequestsModule, 'getObservabilityMetricsWithRequest')
            .resolves({
                metrics: {
                    uptime_seconds: 42,
                },
            } as any);
    });

    teardown(() => {
        resetServerStatusForTests();
        resetSnapshotStatusForTests();
        sinon.restore();
    });

    function stubActiveBslEditor(uri = 'file:///snapshot-observability-test.bsl'): vscode.TextEditor {
        const editor = {
            document: {
                languageId: 'bsl',
                uri: vscode.Uri.parse(uri),
            },
        } as unknown as vscode.TextEditor;
        sinon.stub(vscode.window, 'activeTextEditor').get(() => editor);
        return editor;
    }

    test('loadMetrics should skip polling while server is loading', async () => {
        handleServerStatus({ loading: true, message: 'Loading types...' });
        const provider = new ObservabilityProvider(outputChannelStub);

        try {
            const result = await (provider as any).loadMetrics();
            assert.strictEqual(result, null);
            assert.strictEqual(getObservabilityMetricsStub.callCount, 0);
        } finally {
            provider.dispose();
        }
    });

    test('loadMetrics should resume polling after server becomes ready', async () => {
        handleServerStatus({ loading: true, message: 'Loading types...' });
        const provider = new ObservabilityProvider(outputChannelStub);

        try {
            let result = await (provider as any).loadMetrics();
            assert.strictEqual(result, null);
            assert.strictEqual(getObservabilityMetricsStub.callCount, 0);

            handleServerStatus({ loading: false });
            await flushPromises();

            result = await (provider as any).loadMetrics();
            assert.deepStrictEqual(result, { uptime_seconds: 42 });
            assert.strictEqual(getObservabilityMetricsStub.callCount, 1);
            assert.deepStrictEqual(getObservabilityMetricsStub.firstCall.args, [{ shape: 'sidebar' }]);
        } finally {
            provider.dispose();
        }
    });

    test('loadMetrics should cooldown null responses before retrying sidebar fetch', async () => {
        const clock = sinon.useFakeTimers();
        handleServerStatus({ loading: false });
        const provider = new ObservabilityProvider(outputChannelStub);
        (provider as any).stopAutoRefresh();
        (provider as any).autoRefreshIntervalMs = 5000;

        getObservabilityMetricsStub.resetBehavior();
        getObservabilityMetricsStub.onFirstCall().resolves(null);
        getObservabilityMetricsStub.onSecondCall().resolves({
            metrics: {
                uptime_seconds: 99,
            },
        } as any);
        const cooldownMs = (provider as any).autoRefreshIntervalMs;

        try {
            let result = await (provider as any).loadMetrics();
            assert.strictEqual(result, null);
            assert.strictEqual(getObservabilityMetricsStub.callCount, 1);

            result = await (provider as any).loadMetrics();
            assert.strictEqual(result, null);
            assert.strictEqual(
                getObservabilityMetricsStub.callCount,
                1,
                'null response should be negatively cached during cooldown'
            );

            await clock.tickAsync(cooldownMs - 1);
            result = await (provider as any).loadMetrics();
            assert.strictEqual(result, null);
            assert.strictEqual(
                getObservabilityMetricsStub.callCount,
                1,
                'sidebar cooldown should still suppress retry before the interval expires'
            );

            await clock.tickAsync(1);
            result = await (provider as any).loadMetrics();
            assert.deepStrictEqual(result, { uptime_seconds: 99 });
            assert.strictEqual(getObservabilityMetricsStub.callCount, 2);
        } finally {
            provider.dispose();
            clock.restore();
        }
    });

    test('actions should expose export incident bundle command', async () => {
        const provider = new ObservabilityProvider(outputChannelStub);

        try {
            const items = await (provider as any).getActionItems();
            const exportItem = items.find((item: any) =>
                item.command?.command === 'bslAnalyzer.exportObservabilityIncidentBundle'
            );
            assert.ok(exportItem, 'export bundle action should be present in the Observability actions section');
        } finally {
            provider.dispose();
        }
    });

    test('snapshot section should expose exact snapshot details from shared snapshot store', async () => {
        stubActiveBslEditor();
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 1,
                uri: 'file:///snapshot-observability-test.bsl',
                requestedVersion: 21,
                readyVersion: 21,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                phase: 'materializing',
                trigger: 'did_save',
                fallbackReason: 'shadow_state_reused',
                updatedAtMs: 600,
            },
        });

        const snapshotDisposable = initializeSnapshotStatus(outputChannelStub, snapshotStatusBarStub);
        const provider = new ObservabilityProvider(outputChannelStub);
        await flushPromises();

        try {
            const items = (provider as any).getSnapshotItems();
            const labels = items.map((item: any) => item.label);
            assert.deepStrictEqual(labels.slice(0, 7), [
                'State: ready (exact)',
                'Requested revision: 21',
                'Ready revision: 21',
                'Task state: ready_same_revision',
                'Phase: materializing',
                'Trigger: did_save',
                'Fallback: shadow_state_reused',
            ]);
            assert.ok(
                labels.some((label: string) => label.startsWith('Updated: ')),
                `expected Updated label, got ${labels.join(', ')}`
            );
        } finally {
            provider.dispose();
            snapshotDisposable.dispose();
        }
    });
});
