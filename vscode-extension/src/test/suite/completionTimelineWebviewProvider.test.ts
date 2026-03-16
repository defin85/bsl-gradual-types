import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { CompletionTimelineWebviewProvider } from '../../providers/completionTimelineWebview';
import { CompletionTimelineFetchResult } from '../../lsp/customRequests';

async function flushPromises(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
}

suite('Completion Timeline Webview Provider Test Suite', () => {
    let clock: sinon.SinonFakeTimers;
    let provider: CompletionTimelineWebviewProvider | null;

    setup(() => {
        clock = sinon.useFakeTimers();
        provider = null;
    });

    teardown(() => {
        provider?.dispose();
        provider = null;
        clock.restore();
        sinon.restore();
    });

    test('polling runs only while webview is visible', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 1,
                traces: [
                    {
                        trace_id: 'trace-1',
                        request_id: 'req-1',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle',
                        stages: [
                            {
                                name: 'query_bundle',
                                status: 'completed',
                                started_offset_ms: 0,
                                duration_ms: 10,
                            },
                        ],
                    },
                ],
            },
        };
        const getCompletionTimelineStub = sinon
            .stub(customRequestsModule, 'getCompletionTimeline')
            .resolves(timelinePayload);

        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        provider = new CompletionTimelineWebviewProvider(outputChannel);

        const onDidReceiveMessageEmitter = new vscode.EventEmitter<unknown>();
        const onDidChangeVisibilityEmitter = new vscode.EventEmitter<void>();
        const onDidDisposeEmitter = new vscode.EventEmitter<void>();
        const postMessageStub = sinon.stub().resolves(true);
        const webview = {
            options: {},
            html: '',
            cspSource: 'vscode-webview://test',
            onDidReceiveMessage: onDidReceiveMessageEmitter.event,
            postMessage: postMessageStub,
        } as unknown as vscode.Webview;
        const webviewView = {
            webview,
            visible: false,
            onDidChangeVisibility: onDidChangeVisibilityEmitter.event,
            onDidDispose: onDidDisposeEmitter.event,
        } as unknown as vscode.WebviewView;

        provider.resolveWebviewView(webviewView);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            1,
            'resolveWebviewView should trigger one immediate refresh'
        );

        clock.tick(9_000);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            1,
            'hidden view must not poll automatically'
        );

        (webviewView as unknown as { visible: boolean }).visible = true;
        onDidChangeVisibilityEmitter.fire();
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            2,
            'becoming visible should trigger immediate refresh'
        );

        clock.tick(3_100);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            3,
            'visible view should poll on interval'
        );

        (webviewView as unknown as { visible: boolean }).visible = false;
        onDidChangeVisibilityEmitter.fire();
        await flushPromises();
        const beforeHiddenTick = getCompletionTimelineStub.callCount;
        clock.tick(6_500);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            beforeHiddenTick,
            'polling must stop after view is hidden'
        );

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('copyVisible message should write current visible traces to clipboard', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 1,
                traces: [
                    {
                        trace_id: 'trace-copy',
                        request_id: 'req-copy',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle',
                        stages: [
                            {
                                name: 'query_bundle',
                                status: 'completed',
                                started_offset_ms: 0,
                                duration_ms: 10,
                            },
                        ],
                    },
                ],
            },
        };
        sinon.stub(customRequestsModule, 'getCompletionTimeline').resolves(timelinePayload);
        const clipboardStub = sinon.stub().resolves();

        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        provider = new CompletionTimelineWebviewProvider(outputChannel, clipboardStub);

        const onDidReceiveMessageEmitter = new vscode.EventEmitter<unknown>();
        const onDidChangeVisibilityEmitter = new vscode.EventEmitter<void>();
        const onDidDisposeEmitter = new vscode.EventEmitter<void>();
        const postMessageStub = sinon.stub().resolves(true);
        const webview = {
            options: {},
            html: '',
            cspSource: 'vscode-webview://test',
            onDidReceiveMessage: onDidReceiveMessageEmitter.event,
            postMessage: postMessageStub,
        } as unknown as vscode.Webview;
        const webviewView = {
            webview,
            visible: true,
            onDidChangeVisibility: onDidChangeVisibilityEmitter.event,
            onDidDispose: onDidDisposeEmitter.event,
        } as unknown as vscode.WebviewView;

        provider.resolveWebviewView(webviewView);
        await flushPromises();

        onDidReceiveMessageEmitter.fire({ type: 'copyVisible', mode: 'all' });
        await flushPromises();

        assert.strictEqual(clipboardStub.callCount, 1);
        const clipboardPayload = clipboardStub.firstCall.args[0];
        assert.ok(clipboardPayload.includes('Completion Timeline | mode=all'));
        assert.ok(clipboardPayload.includes('trace-copy (invoked)'));

        const copyAck = postMessageStub.lastCall.args[0];
        assert.strictEqual(copyAck.type, 'copyResult');
        assert.strictEqual(copyAck.ok, true);

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });
});
