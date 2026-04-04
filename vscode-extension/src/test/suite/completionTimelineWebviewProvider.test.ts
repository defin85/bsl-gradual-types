import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vm from 'vm';
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

class FakeWebviewElement {
    innerHTML = '';
    textContent = '';
    style: Record<string, string> = {};
    private readonly attrs = new Map<string, string>();
    readonly classList = {
        toggle: () => undefined,
    };

    addEventListener(): void {
        // No-op for inline render harness.
    }

    setAttribute(name: string, value: string): void {
        this.attrs.set(name, value);
    }

    getAttribute(name: string): string | null {
        return this.attrs.get(name) ?? null;
    }

    closest(): null {
        return null;
    }
}

function extractInlineWebviewScript(html: string): string {
    const scriptStart = html.indexOf('<script nonce=');
    assert.notStrictEqual(scriptStart, -1, 'expected webview html to include inline script');
    const contentStart = html.indexOf('>', scriptStart);
    assert.notStrictEqual(contentStart, -1, 'expected script start tag to terminate');
    const scriptEnd = html.indexOf('</script>', contentStart) !== -1
        ? html.indexOf('</script>', contentStart)
        : html.indexOf('<\\/script>', contentStart);
    assert.notStrictEqual(scriptEnd, -1, 'expected inline script terminator');
    return html.slice(contentStart + 1, scriptEnd);
}

function renderTimelineStateInInlineWebview(
    html: string,
    state: unknown
): {
    serverHtml: string;
    clientHtml: string;
    updatedText: string;
} {
    const elements = new Map(
        [
            'serverRoot',
            'clientRoot',
            'updatedAt',
            'copyStatus',
            'refresh',
            'copyVisible',
            'exportBundle',
            'modeAll',
            'modeAverage',
        ].map((id) => [id, new FakeWebviewElement()])
    );
    const messageHandlers: Array<(event: { data: unknown }) => void> = [];
    const context = {
        console,
        acquireVsCodeApi: () => ({
            postMessage: () => undefined,
        }),
        document: {
            getElementById: (id: string) => elements.get(id) ?? null,
        },
        window: {
            addEventListener: (type: string, handler: (event: { data: unknown }) => void) => {
                if (type === 'message') {
                    messageHandlers.push(handler);
                }
            },
        },
        Element: FakeWebviewElement,
        setTimeout,
        clearTimeout,
        Date,
        Math,
        String,
        Array,
        Object,
        JSON,
    };

    vm.createContext(context);
    vm.runInContext(extractInlineWebviewScript(html), context, {
        filename: 'completionTimelineWebview.inline.js',
    });

    const messageHandler = messageHandlers.at(-1);
    assert.ok(messageHandler, 'expected inline webview script to register a message handler');
    messageHandler?.({
        data: {
            type: 'timelineState',
            state,
        },
    });

    return {
        serverHtml: elements.get('serverRoot')?.innerHTML ?? '',
        clientHtml: elements.get('clientRoot')?.innerHTML ?? '',
        updatedText: elements.get('updatedAt')?.textContent ?? '',
    };
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
                        dominant_stage: 'query_bundle_ir_query',
                        stages: [
                            {
                                name: 'query_bundle_ir_query',
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

    test('visible webview stays quiet during active completion probes and quiet window', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 20,
                traces: [
                    {
                        trace_id: 'trace-quiet',
                        request_id: 'req-quiet',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle_ir_query',
                        stages: [
                            {
                                name: 'query_bundle_ir_query',
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

        const recorder = getSharedCompletionProbeRecorder();
        recorder.clear();
        const activeToken = new vscode.CancellationTokenSource();
        const document = {
            uri: vscode.Uri.parse('file:///tmp/test.bsl'),
            version: 4,
            lineAt: () => ({ text: 'Документы.' }),
        } as unknown as vscode.TextDocument;

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
        assert.strictEqual(getCompletionTimelineStub.callCount, 1);

        recorder.recordCompletionStarted({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: activeToken.token,
            requestStartedAtMs: clock.now,
        });

        clock.tick(3_100);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            1,
            'auto-polling must stay quiet while an active completion probe exists'
        );

        onDidReceiveMessageEmitter.fire({ type: 'refresh' });
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            2,
            'manual refresh must remain explicit even while quiet auto-polling is suspended'
        );

        recorder.recordCompletionOutcome({
            document,
            position: new vscode.Position(0, 'Документы.'.length),
            context: {
                triggerKind: vscode.CompletionTriggerKind.TriggerCharacter,
                triggerCharacter: '.',
            },
            token: activeToken.token,
            result: [{ label: 'Форма' }] as vscode.CompletionItem[],
            requestStartedAtMs: clock.now - 10,
            requestCompletedAtMs: clock.now,
            wasCancelled: false,
        });

        clock.tick(1_100);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            2,
            'quiet window must suppress the first post-probe auto refresh'
        );

        clock.tick(3_100);
        await flushPromises();
        assert.strictEqual(
            getCompletionTimelineStub.callCount,
            3,
            'auto-polling must resume shortly after quiet window expires'
        );

        activeToken.dispose();
        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('copyVisible message should write current visible traces to clipboard', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 20,
                traces: [
                    {
                        trace_id: 'trace-copy',
                        request_id: 'req-copy',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle_ir_query',
                        server_edge_details: {
                            adapter_read_at_ms: 1_699_999_999_996,
                            transport_received_at_ms: 1_700_000_000_000,
                            transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                            jsonrpc_dispatch_received_at_ms: 1_700_000_000_000,
                            transport_slot_released_at_ms: 1_700_000_000_002,
                            service_future_created_at_ms: 1_700_000_000_001,
                            service_future_first_poll_entered_at_ms: 1_700_000_000_003,
                            service_future_first_poll_outcome: 'pending',
                            service_future_first_wake_scheduled_at_ms: 1_700_000_000_007,
                            first_poll_contention_attribution: {
                                contender_class: 'document_sync',
                                uri_scope: 'same_uri',
                                inflight_count: 1,
                                oldest_inflight_age_ms: 2,
                                concurrency_level: 16,
                            },
                            pre_method_attribution_provenance: 'same_request_authoritative',
                            service_scope_entered_at_ms: 1_700_000_000_002,
                            method_entered_at_ms: 1_700_000_000_005,
                            handler_entered_at_ms: 1_700_000_000_009,
                            response_sent_at_ms: 1_700_000_000_016,
                            dispatch_to_request_context_wait_ms: 0,
                            adapter_to_dispatch_wait_ms: 4,
                            transport_to_slot_release_wait_ms: 2,
                            transport_to_service_future_wait_ms: 1,
                            service_future_to_scope_wait_ms: 1,
                            service_future_to_first_poll_wait_ms: 2,
                            first_poll_to_first_wake_wait_ms: 4,
                            transport_to_service_scope_wait_ms: 2,
                            service_scope_to_method_wait_ms: 3,
                            transport_to_method_wait_ms: 5,
                            method_prelude_exec_ms: 4,
                            slot_release_to_handler_wait_ms: 7,
                            slot_release_to_response_wait_ms: 14,
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
                            snapshot_with_deps_timeout_runtime: {
                                queue_wait_ms: 11,
                                exec_ms: 17,
                                wake_wait_ms: 2870,
                                resolution: 'wake_wait',
                            },
                        },
                        turn_attribution: {
                            request_file_seq: 1,
                            request_epoch: 1,
                            queue_outcome: 'enqueued',
                            dispatcher_resolution_latency_ms: 4,
                            turn_wait_entered_at_ms: 1_700_000_000_003,
                            turn_wait_resolved_at_ms: 1_700_000_000_006,
                            wake_after_turn_resolution_at_ms: 1_700_000_000_007,
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
                                name: 'query_bundle_ir_query',
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
        assert.ok(clipboardPayload.includes('contract=v20'));
        assert.ok(clipboardPayload.includes('trace-copy (invoked)'));
        assert.ok(clipboardPayload.includes('transport_received_at_ms_provenance=jsonrpc_dispatch_received'));
        assert.ok(clipboardPayload.includes('jsonrpc_dispatch_received_at_ms=1700000000000'));
        assert.ok(clipboardPayload.includes('transport_slot_released_at_ms=1700000000002'));
        assert.ok(clipboardPayload.includes('service_future_created_at_ms=1700000000001'));
        assert.ok(clipboardPayload.includes('service_future_first_poll_entered_at_ms=1700000000003'));
        assert.ok(clipboardPayload.includes('service_future_first_poll_outcome=pending'));
        assert.ok(clipboardPayload.includes('service_future_first_wake_scheduled_at_ms=1700000000007'));
        assert.ok(clipboardPayload.includes('first_poll_contention_contender_class=document_sync'));
        assert.ok(clipboardPayload.includes('first_poll_contention_uri_scope=same_uri'));
        assert.ok(clipboardPayload.includes('first_poll_contention_inflight_count=1'));
        assert.ok(clipboardPayload.includes('first_poll_contention_concurrency_level=16'));
        assert.ok(clipboardPayload.includes('pre_method_attribution_provenance=same_request_authoritative'));
        assert.ok(clipboardPayload.includes('service_scope_entered_at_ms=1700000000002'));
        assert.ok(clipboardPayload.includes('method_entered_at_ms=1700000000005'));
        assert.ok(clipboardPayload.includes('dispatch_to_request_context_wait_ms=0'));
        assert.ok(clipboardPayload.includes('transport_to_slot_release_wait_ms=2'));
        assert.ok(clipboardPayload.includes('transport_to_service_future_wait_ms=1'));
        assert.ok(clipboardPayload.includes('service_future_to_scope_wait_ms=1'));
        assert.ok(clipboardPayload.includes('service_future_to_first_poll_wait_ms=2'));
        assert.ok(clipboardPayload.includes('first_poll_to_first_wake_wait_ms=4'));
        assert.ok(clipboardPayload.includes('transport_to_service_scope_wait_ms=2'));
        assert.ok(clipboardPayload.includes('service_scope_to_method_wait_ms=3'));
        assert.ok(clipboardPayload.includes('transport_to_method_wait_ms=5'));
        assert.ok(clipboardPayload.includes('method_prelude_exec_ms=4'));
        assert.ok(clipboardPayload.includes('slot_release_to_handler_wait_ms=7'));
        assert.ok(clipboardPayload.includes('slot_release_to_response_wait_ms=14'));
        assert.ok(clipboardPayload.includes('transport_to_handler_wait_ms=9'));
        assert.ok(clipboardPayload.includes('server_handler_exec_ms=7'));
        assert.ok(clipboardPayload.includes('bottleneck_verdict=query_bundle_dominant'));
        assert.ok(clipboardPayload.includes('bottleneck_verdict=query_bundle_ir_query_dominant'));
        assert.ok(!clipboardPayload.includes('bottleneck_verdict=server_before_method_entry_dominant'));
        assert.ok(clipboardPayload.includes('bottleneck_verdict=prepare_timeout@prepare_guard'));
        assert.ok(
            clipboardPayload.includes(
                'timeout_attribution | source=prepare_guard | phase=wait_for_file_version | budget_ms=120 | elapsed_ms=2996 | overshoot_ms=2876'
            )
        );
        assert.ok(clipboardPayload.includes('dispatcher_resolution_latency_ms=4'));
        assert.ok(clipboardPayload.includes('turn_wait_entered_at_ms=1700000000003'));
        assert.ok(clipboardPayload.includes('turn_wait_resolved_at_ms=1700000000006'));
        assert.ok(clipboardPayload.includes('wake_after_turn_resolution_at_ms=1700000000007'));
        assert.ok(
            clipboardPayload.includes(
                'snapshot_with_deps_timeout_runtime | queue_wait_ms=11 | exec_ms=17 | wake_wait_ms=2870 | resolution=wake_wait'
            )
        );
        assert.ok(clipboardPayload.includes('Client Probe Feed | local-only debug data'));

        const copyAck = postMessageStub.lastCall.args[0];
        assert.strictEqual(copyAck.type, 'copyResult');
        assert.strictEqual(copyAck.ok, true);

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('copyVisible message should mark average mode traces as synthetic provenance', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 20,
                traces: [
                    {
                        trace_id: 'trace-copy',
                        request_id: 'req-copy',
                        uri: 'file:///tmp/test.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle_ir_query',
                        server_edge_details: {
                            adapter_read_at_ms: 1_699_999_999_996,
                            transport_received_at_ms: 1_700_000_000_000,
                            transport_received_at_ms_provenance: 'jsonrpc_dispatch_received',
                            jsonrpc_dispatch_received_at_ms: 1_700_000_000_000,
                            service_future_created_at_ms: 1_700_000_000_001,
                            pre_method_attribution_provenance: 'same_request_authoritative',
                            service_scope_entered_at_ms: 1_700_000_000_002,
                            method_entered_at_ms: 1_700_000_000_005,
                            handler_entered_at_ms: 1_700_000_000_009,
                            response_sent_at_ms: 1_700_000_000_016,
                            dispatch_to_request_context_wait_ms: 0,
                            adapter_to_dispatch_wait_ms: 4,
                            transport_to_service_future_wait_ms: 1,
                            service_future_to_scope_wait_ms: 1,
                            transport_to_service_scope_wait_ms: 2,
                            service_scope_to_method_wait_ms: 3,
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
                            snapshot_with_deps_timeout_runtime: {
                                queue_wait_ms: 11,
                                exec_ms: 17,
                                wake_wait_ms: 2870,
                                resolution: 'wake_wait',
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
                                name: 'query_bundle_ir_query',
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

        onDidReceiveMessageEmitter.fire({ type: 'copyVisible', mode: 'average' });
        await flushPromises();

        assert.strictEqual(clipboardStub.callCount, 1);
        const clipboardPayload = clipboardStub.firstCall.args[0];
        assert.ok(clipboardPayload.includes('Completion Timeline | mode=average'));
        assert.ok(
            clipboardPayload.includes(
                'Average trace is synthetic; v8 trustworthy pre-method attribution provenance, v9 pre-service-scope split, v10 dispatch split, and v11 first-poll / first-wake split are unavailable by design.'
                    .replace(
                        'and v11 first-poll / first-wake split are unavailable by design.',
                        'v11 first-poll / first-wake split, v12 first-poll contention attribution, v13 contender snapshot, v14 executeCommand command detail, v15 completion phase detail, v16 turn-wait resolution detail, v17 transport slot release detail, v18 request-bound client probe correlation detail, v19 adapter ingress pre-dispatch split, v21 flush-aware post-handler egress split, v22 shipped compatibility output-egress split, and v23 truthful encode-start/write-start boundary are unavailable by design.'
                    )
            )
        );
        assert.ok(!clipboardPayload.includes('bottleneck_verdict=server_before_method_entry_dominant'));

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
        assert.ok(webview.html.includes('service_scope_entered='));
        assert.ok(webview.html.includes('snapshot_with_deps_timeout_runtime'));
        assert.ok(
            webview.html.includes('v9 pre-service-scope split is unavailable by design on this payload.')
        );
        assert.ok(
            webview.html.includes('v10 dispatch split is unavailable by design on this payload.')
        );
        assert.ok(
            webview.html.includes('v11 first-poll / first-wake split is unavailable by design on this payload.')
        );
        assert.ok(
            webview.html.includes('v12 first-poll contention attribution is unavailable by design on this payload.')
        );

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('inline webview script renders non-empty server timeline state', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 12,
                traces: [
                    {
                        trace_id: 'trace-inline',
                        request_id: 'req-inline',
                        uri: 'file:///tmp/inline.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle_ir_query',
                        stages: [
                            {
                                name: 'query_bundle_ir_query',
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
        assert.strictEqual(timelineStateMessage.state.kind, 'ready');

        const rendered = renderTimelineStateInInlineWebview(
            webview.html,
            timelineStateMessage.state
        );

        assert.ok(rendered.serverHtml.includes('trace-inline'));
        assert.ok(rendered.updatedText.includes('contract v12'));
        assert.ok(
            rendered.clientHtml.includes('No client probes recorded yet'),
            'expected empty client feed placeholder to remain intact'
        );

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('inline webview script should mark v10 payload as missing v11 first-poll / first-wake split by design', () => {
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

        const rendered = renderTimelineStateInInlineWebview(webview.html, {
            kind: 'ready',
            version: 10,
            updated_at_ms: 1_700_000_000_100,
            traces: [
                {
                    trace_id: 'trace-v9',
                    request_id: 'req-v9',
                    uri: 'file:///tmp/v9.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_000,
                    total_duration_ms: 10,
                    max_stage_end_ms: 10,
                    unattributed_overhead_ms: 0,
                    dominant_stage: 'query_bundle_ir_query',
                    server_edge_details: {
                        transport_received_at_ms: 1_700_000_000_000,
                        service_future_created_at_ms: 1_700_000_000_001,
                        pre_method_attribution_provenance: 'same_request_authoritative',
                        service_scope_entered_at_ms: 1_700_000_000_002,
                        method_entered_at_ms: 1_700_000_000_005,
                        handler_entered_at_ms: 1_700_000_000_009,
                        response_sent_at_ms: 1_700_000_000_016,
                        transport_to_service_future_wait_ms: 1,
                        service_future_to_scope_wait_ms: 1,
                        transport_to_service_scope_wait_ms: 2,
                        service_scope_to_method_wait_ms: 3,
                        transport_to_method_wait_ms: 5,
                        method_prelude_exec_ms: 4,
                        transport_to_handler_wait_ms: 9,
                        server_handler_exec_ms: 7,
                    },
                    stages: [
                        {
                            name: 'query_bundle_ir_query',
                            status: 'completed',
                            started_offset_ms: 0,
                            end_offset_ms: 10,
                            duration_ms: 10,
                            width_percent: 100,
                            duration_percent: 100,
                            is_dominant: true,
                        },
                    ],
                },
            ],
            average_trace: null,
            client_probe_feed: {
                updated_at_ms: 1_700_000_000_100,
                probes: [],
            },
        });

        assert.ok(!rendered.serverHtml.includes('v10 dispatch split is unavailable by design on this payload.'));
        assert.ok(rendered.serverHtml.includes('v11 first-poll / first-wake split is unavailable by design on this payload.'));
        assert.ok(rendered.serverHtml.includes('v12 first-poll contention attribution is unavailable by design on this payload.'));
        assert.ok(rendered.serverHtml.includes('service_future_created='));
        assert.ok(!rendered.serverHtml.includes('transport_received_provenance='));
        assert.ok(!rendered.serverHtml.includes('jsonrpc_dispatch_received='));
        assert.ok(!rendered.serverHtml.includes('dispatch_to_request_context_wait='));

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('inline webview script should mark v19 payload as missing v20 query-body split by design', () => {
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

        const rendered = renderTimelineStateInInlineWebview(webview.html, {
            kind: 'ready',
            version: 19,
            updated_at_ms: 1_700_000_000_100,
            traces: [
                {
                    trace_id: 'trace-v19',
                    request_id: 'req-v19',
                    uri: 'file:///tmp/v19.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_000,
                    total_duration_ms: 10,
                    max_stage_end_ms: 10,
                    unattributed_overhead_ms: 0,
                    dominant_stage: 'query_bundle',
                    stages: [
                        {
                            name: 'query_bundle',
                            status: 'completed',
                            started_offset_ms: 0,
                            end_offset_ms: 10,
                            duration_ms: 10,
                            width_percent: 100,
                            duration_percent: 100,
                            is_dominant: true,
                        },
                    ],
                },
            ],
            average_trace: null,
            client_probe_feed: {
                updated_at_ms: 1_700_000_000_100,
                probes: [],
            },
        });

        assert.ok(
            rendered.serverHtml.includes(
                'v20 truthful grouped query-body split is unavailable by design on this payload.'
            )
        );
        assert.ok(rendered.serverHtml.includes('query_bundle'));
        assert.ok(!rendered.serverHtml.includes('query_bundle_ir_query_dominant'));

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('inline webview script should mark v22 payload as missing truthful v23 boundary by design', () => {
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

        const rendered = renderTimelineStateInInlineWebview(webview.html, {
            kind: 'ready',
            version: 22,
            updated_at_ms: 1_700_000_000_100,
            traces: [
                {
                    trace_id: 'trace-v22',
                    request_id: 'req-v22',
                    uri: 'file:///tmp/v22.bsl',
                    trigger_mode: 'invoked',
                    outcome: 'ok_non_empty',
                    started_at_ms: 1_700_000_000_000,
                    total_duration_ms: 12,
                    max_stage_end_ms: 12,
                    unattributed_overhead_ms: 0,
                    dominant_stage: 'query_bundle_ir_query',
                    server_edge_details: {
                        transport_received_at_ms: 1_700_000_000_000,
                        handler_entered_at_ms: 1_700_000_000_002,
                        response_sent_at_ms: 1_700_000_000_010,
                        response_output_enqueue_completed_at_ms: 1_700_000_000_011,
                        response_output_encode_started_at_ms: 1_700_000_000_012,
                        response_output_encode_completed_at_ms: 1_700_000_000_013,
                        response_output_write_started_at_ms: 1_700_000_000_013,
                        response_flush_completed_at_ms: 1_700_000_000_014,
                        transport_to_handler_wait_ms: 2,
                        server_handler_exec_ms: 8,
                        response_ready_to_output_enqueue_wait_ms: 1,
                        response_output_queue_wait_ms: 1,
                        response_output_encode_exec_ms: 1,
                        response_output_write_and_flush_exec_ms: 1,
                        response_ready_to_flush_wait_ms: 4,
                    },
                    bottleneck_verdicts: [],
                    stages: [
                        {
                            name: 'query_bundle_ir_query',
                            status: 'completed',
                            started_offset_ms: 0,
                            end_offset_ms: 12,
                            duration_ms: 12,
                            width_percent: 100,
                            duration_percent: 100,
                            is_dominant: true,
                        },
                    ],
                },
            ],
            average_trace: null,
            client_probe_feed: {
                updated_at_ms: 1_700_000_000_100,
                probes: [],
            },
        });

        assert.ok(
            rendered.serverHtml.includes(
                'v23 truthful encode-start / write-start boundary is unavailable by design on this payload.'
            )
        );
        assert.ok(!rendered.serverHtml.includes('response_output_encode_started='));

        onDidDisposeEmitter.dispose();
        onDidReceiveMessageEmitter.dispose();
        onDidChangeVisibilityEmitter.dispose();
    });

    test('exportBundle message should execute shared export command', async () => {
        const customRequestsModule = await import('../../lsp/customRequests');
        const timelinePayload: CompletionTimelineFetchResult = {
            kind: 'ok',
            response: {
                version: 12,
                traces: [
                    {
                        trace_id: 'trace-export',
                        request_id: 'req-export',
                        uri: 'file:///tmp/export.bsl',
                        trigger_mode: 'invoked',
                        outcome: 'ok_non_empty',
                        started_at_ms: 1_700_000_000_000,
                        total_duration_ms: 10,
                        dominant_stage: 'query_bundle_ir_query',
                        stages: [
                            {
                                name: 'query_bundle_ir_query',
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
