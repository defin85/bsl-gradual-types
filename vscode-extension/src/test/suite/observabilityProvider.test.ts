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

    test('snapshot actions should refresh live snapshot status', async () => {
        const provider = new ObservabilityProvider(outputChannelStub);

        try {
            const items = (provider as any).getSnapshotActionItems();
            const refreshItem = items.find((item: any) =>
                item.command?.title === 'Refresh Snapshot Status'
            );
            assert.ok(refreshItem, 'snapshot refresh action should be present');
            assert.strictEqual(refreshItem.command.command, 'bslAnalyzer.refreshSnapshotStatus');
        } finally {
            provider.dispose();
        }
    });

    test('focusSnapshotReadiness should focus Observability and reveal Snapshot Readiness', async () => {
        const provider = new ObservabilityProvider(outputChannelStub);
        const executeCommandStub = sinon
            .stub(vscode.commands, 'executeCommand')
            .resolves(undefined);
        const revealStub = sinon.stub().resolves(undefined);
        const treeView = {
            reveal: revealStub,
        } as unknown as vscode.TreeView<any>;

        try {
            await provider.focusSnapshotReadiness(treeView);

            assert.ok(
                executeCommandStub.calledWith('bslAnalyzer.observability.focus'),
                'Observability view should be focused before reveal'
            );
            assert.strictEqual(revealStub.callCount, 1);
            assert.strictEqual(revealStub.firstCall.args[0].label, 'Snapshot Readiness');
            assert.deepStrictEqual(revealStub.firstCall.args[1], {
                expand: true,
                focus: true,
                select: true,
            });
        } finally {
            provider.dispose();
        }
    });

    test('snapshot section should expose exact snapshot details from shared snapshot store', async () => {
        stubActiveBslEditor();
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'ok',
            response: {
                schemaVersion: 2,
                uri: 'file:///snapshot-observability-test.bsl',
                requestedVersion: 21,
                readyVersion: 21,
                state: 'ready',
                exact: true,
                taskState: 'ready_same_revision',
                phase: 'materializing',
                trigger: 'did_save',
                fallbackReason: 'shadow_state_reused',
                reason: {
                    code: 'ready',
                    message: 'Requested revision has canonical snapshot artifacts',
                },
                artifacts: {
                    shadowState: { state: 'ready', version: 21 },
                    readyParseSnapshot: { state: 'ready', version: 21 },
                    exactTypeIndex: { state: 'ready', version: 21 },
                    completionHead: { state: 'missing', version: 21 },
                },
                recommendation: 'open_timeline',
                updatedAtMs: 600,
            },
        });

        const snapshotDisposable = initializeSnapshotStatus(outputChannelStub, snapshotStatusBarStub);
        const provider = new ObservabilityProvider(outputChannelStub);
        await flushPromises();

        try {
            const items = (provider as any).getSnapshotItems();
            assert.deepStrictEqual(items.map((item: any) => item.label), [
                'Summary',
                'Why',
                'Artifacts',
                'Worker',
                'Last Failure',
                'Recent Transitions',
                'Actions',
            ]);

            const summaryLabels = (provider as any).getSnapshotSummaryItems().map((item: any) => item.label);
            assert.ok(summaryLabels.includes('State: ready (exact)'));
            assert.ok(summaryLabels.includes('Requested revision: 21'));
            assert.ok(summaryLabels.includes('Ready revision: 21'));
            assert.ok(summaryLabels.includes('Task state: ready_same_revision'));

            const whyLabels = (provider as any).getSnapshotWhyItems().map((item: any) => item.label);
            assert.ok(whyLabels.includes('Code: ready'));
            assert.ok(whyLabels.includes('Fallback: shadow_state_reused'));
            assert.ok(whyLabels.includes('Recommendation: open_timeline'));

            const artifactLabels = (provider as any).getSnapshotArtifactItems().map((item: any) => item.label);
            assert.ok(artifactLabels.includes('Shadow state: ready | v21'));
            assert.ok(artifactLabels.includes('Ready parse snapshot: ready | v21'));
            assert.ok(artifactLabels.includes('Exact type index: ready | v21'));
            assert.ok(artifactLabels.includes('Completion head: missing | v21'));

            const historyLabels = (provider as any).getSnapshotHistoryItems().map((item: any) => item.label);
            assert.ok(
                historyLabels.some((label: string) => label.includes('ready requested=21 ready=21')),
                `expected recent transition label, got ${historyLabels.join(', ')}`
            );
        } finally {
            provider.dispose();
            snapshotDisposable.dispose();
        }
    });

    test('snapshot unavailable tree item sanitizes error detail', async () => {
        stubActiveBslEditor('file:///snapshot-observability-error.bsl');
        sinon.stub(customRequestsModule, 'getSnapshotStatusFetchResult').resolves({
            kind: 'error',
            message: `first line\nsecond line\t${'x'.repeat(220)}`,
        });

        const snapshotDisposable = initializeSnapshotStatus(outputChannelStub, snapshotStatusBarStub);
        const provider = new ObservabilityProvider(outputChannelStub);
        await flushPromises();

        try {
            const items = (provider as any).getSnapshotItems();
            assert.strictEqual(items.length, 1);
            const label = String(items[0].label);
            assert.ok(label.startsWith('Snapshot: unavailable ('));
            assert.ok(!label.includes('\n'));
            assert.ok(!label.includes('\t'));
            assert.ok(!label.includes('x'.repeat(180)));
        } finally {
            provider.dispose();
            snapshotDisposable.dispose();
        }
    });
});
