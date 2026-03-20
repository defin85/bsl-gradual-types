import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';
import { CompletionTimelineWebviewProvider } from '../../providers/completionTimelineWebview';
import { CompletionTimelineFetchResult } from '../../lsp/customRequests';
import {
    getSharedCompletionProbeRecorder,
    resetSharedCompletionProbeRecorderForTests,
} from '../../providers/completionProbeRecorder';

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
        resetSharedCompletionProbeRecorderForTests();
        getSharedCompletionProbeRecorder().clear();
    });

    teardown(() => {
        provider?.dispose();
        provider = null;
        resetSharedCompletionProbeRecorderForTests();
        clock.restore();
        sinon.restore();
    });

    test('polling runs only while webview is visible', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 2,
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
                version: 6,
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
                        server_edge_details: {
                            transport_received_at_ms: 1_700_000_000_000,
                            method_entered_at_ms: 1_700_000_000_005,
                            handler_entered_at_ms: 1_700_000_000_009,
                            response_sent_at_ms: 1_700_000_000_016,
                            transport_to_method_wait_ms: 5,
                            method_prelude_exec_ms: 4,
                            transport_to_handler_wait_ms: 9,
                            server_handler_exec_ms: 7,
                        },
                        prepare_details: {
                            fail_closed_cause: 'prepare_timeout',
                            timeout_attribution: {
                                source: 'prepare_guard',
                                phase: 'wait_for_file_version',
                                budget_ms: 120,
                                elapsed_ms: 2996,
                                overshoot_ms: 2876,
                            },
                            progress: {
                                phase: 'wait_for_file_version',
                            },
                            wait_for_file_version_runtime: {
                                queue_wait_ms: 3,
                                exec_ms: 1,
                                wake_wait_ms: 40,
                                resolution: 'waiter',
                            },
                        },
                        turn_attribution: {
                            request_file_seq: 1,
                            request_epoch: 1,
                            queue_outcome: 'enqueued',
                            dispatcher_resolution_latency_ms: 4,
                            queue_capacity: 256,
                            queue_depth_before_enqueue: 0,
                            queue_depth_after_enqueue: 1,
                            queued_completion_ahead_count: 0,
                            did_change_ahead_count: 0,
                            active_completion_count: 0,
                            dropped_completion_file_seq: [],
                        },
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
        assert.ok(clipboardPayload.includes('Server Timeline'));
        assert.ok(clipboardPayload.includes('contract=v6'));
        assert.ok(clipboardPayload.includes('trace-copy (invoked)'));
        assert.ok(clipboardPayload.includes('method_entered_at_ms=1700000000005'));
        assert.ok(clipboardPayload.includes('transport_to_method_wait_ms=5'));
        assert.ok(clipboardPayload.includes('method_prelude_exec_ms=4'));
        assert.ok(clipboardPayload.includes('transport_to_handler_wait_ms=9'));
        assert.ok(clipboardPayload.includes('server_handler_exec_ms=7'));
        assert.ok(clipboardPayload.includes('bottleneck_verdict=server_before_method_entry_dominant'));
        assert.ok(clipboardPayload.includes('bottleneck_verdict=prepare_timeout@prepare_guard'));
        assert.ok(
            clipboardPayload.includes(
                'timeout_attribution | source=prepare_guard | phase=wait_for_file_version | budget_ms=120 | elapsed_ms=2996 | overshoot_ms=2876'
            )
        );
        assert.ok(clipboardPayload.includes('dispatcher_resolution_latency_ms=4'));
        assert.ok(clipboardPayload.includes('Client Probe Feed | local-only debug data'));

        const copyAck = postMessageStub.lastCall.args[0];
        assert.strictEqual(copyAck.type, 'copyResult');
        assert.strictEqual(copyAck.ok, true);

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('webview content declares separate server and client sections', () => {
        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        provider = new CompletionTimelineWebviewProvider(outputChannel);

        const onDidReceiveMessageEmitter = new vscode.EventEmitter<unknown>();
        const onDidChangeVisibilityEmitter = new vscode.EventEmitter<void>();
        const onDidDisposeEmitter = new vscode.EventEmitter<void>();
        const webview = {
            options: {},
            html: '',
            cspSource: 'vscode-webview://test',
            onDidReceiveMessage: onDidReceiveMessageEmitter.event,
            postMessage: sinon.stub().resolves(true),
        } as unknown as vscode.Webview;
        const webviewView = {
            webview,
            visible: false,
            onDidChangeVisibility: onDidChangeVisibilityEmitter.event,
            onDidDispose: onDidDisposeEmitter.event,
        } as unknown as vscode.WebviewView;

        provider.resolveWebviewView(webviewView);

        assert.ok(webview.html.includes('Server Timeline'));
        assert.ok(webview.html.includes('Client Probe Feed'));
        assert.ok(webview.html.includes('Local-only debug data'));
        assert.ok(webview.html.includes('Export bundle'));

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('exportBundle message should execute shared export command', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 6,
                traces: [
                    {
                        trace_id: 'trace-export',
                        request_id: 'req-export',
                        uri: 'file:///tmp/export.bsl',
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
        const executeCommandStub = sinon.stub(vscode.commands, 'executeCommand').resolves(undefined);

        const outputChannel = {
            appendLine: sinon.stub(),
        } as unknown as vscode.OutputChannel;
        provider = new CompletionTimelineWebviewProvider(outputChannel);

        const onDidReceiveMessageEmitter = new vscode.EventEmitter<unknown>();
        const onDidChangeVisibilityEmitter = new vscode.EventEmitter<void>();
        const onDidDisposeEmitter = new vscode.EventEmitter<void>();
        const webview = {
            options: {},
            html: '',
            cspSource: 'vscode-webview://test',
            onDidReceiveMessage: onDidReceiveMessageEmitter.event,
            postMessage: sinon.stub().resolves(true),
        } as unknown as vscode.Webview;
        const webviewView = {
            webview,
            visible: true,
            onDidChangeVisibility: onDidChangeVisibilityEmitter.event,
            onDidDispose: onDidDisposeEmitter.event,
        } as unknown as vscode.WebviewView;

        provider.resolveWebviewView(webviewView);
        await flushPromises();

        onDidReceiveMessageEmitter.fire({ type: 'exportBundle' });
        await flushPromises();

        assert.strictEqual(executeCommandStub.callCount, 1);
        assert.strictEqual(
            executeCommandStub.firstCall.args[0],
            'bslAnalyzer.exportObservabilityIncidentBundle'
        );
        assert.deepStrictEqual(executeCommandStub.firstCall.args[1], {
            capturedAtMs: clock.now,
            completionTimeline: timelinePayload,
            clientProbes: [],
        });

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('refresh merges server trace and shared client probes without correlation', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        sinon.stub(customRequestsModule, 'getCompletionTimeline').resolves({
            kind: 'unsupported',
        } as CompletionTimelineFetchResult);

        const recorder = getSharedCompletionProbeRecorder();
        recorder.clear();
        recorder['recordCompletionOutcome']({
            document: {
                uri: vscode.Uri.parse('file:///tmp/probe.bsl'),
                version: 4,
                lineAt: () => ({ text: 'Документы.' }),
            } as unknown as vscode.TextDocument,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            result: [{ label: 'Форма' }] as vscode.CompletionItem[],
            requestStartedAtMs: 1_700_000_000_010,
            requestCompletedAtMs: 1_700_000_000_020,
            wasCancelled: false,
        });

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
            visible: true,
            onDidChangeVisibility: onDidChangeVisibilityEmitter.event,
            onDidDispose: onDidDisposeEmitter.event,
        } as unknown as vscode.WebviewView;

        provider.resolveWebviewView(webviewView);
        await flushPromises();

        const timelineStateMessage = postMessageStub.firstCall.args[0];
        assert.strictEqual(timelineStateMessage.type, 'timelineState');
        assert.strictEqual(timelineStateMessage.state.kind, 'unsupported');
        assert.strictEqual(timelineStateMessage.state.client_probe_feed.probes.length, 1);
        assert.strictEqual(
            timelineStateMessage.state.client_probe_feed.probes[0].client_terminal_state,
            'ok_non_empty'
        );
        assert.strictEqual(
            timelineStateMessage.state.client_probe_feed.probes[0].result_kind,
            'non_empty'
        );
        assert.strictEqual(
            timelineStateMessage.state.client_probe_feed.probes[0].cancel_reason_hint,
            'unknown'
        );

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });
});
