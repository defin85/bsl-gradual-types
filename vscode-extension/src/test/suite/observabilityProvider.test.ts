import * as assert from 'assert';
import * as sinon from 'sinon';
import { State } from 'vscode-languageclient/node';
import { ObservabilityProvider } from '../../providers/observabilityProvider';
import * as clientModule from '../../lsp/client';
import * as customRequestsModule from '../../lsp/customRequests';
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

        resetServerStatusForTests();
        initializeServerStatus(outputChannelStub, statusBarStub);

        sinon.stub(clientModule, 'getLanguageClient').returns({
            state: State.Running,
            isRunning: () => true,
        } as any);

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
        sinon.restore();
    });

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
});
