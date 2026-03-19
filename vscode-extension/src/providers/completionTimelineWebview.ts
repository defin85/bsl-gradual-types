import * as vscode from 'vscode';
import { BslAnalyzerConfig } from '../config/configHelper';
import { CompletionTimelineFetchResult, getCompletionTimeline } from '../lsp/customRequests';
import {
    CompletionTimelineClipboardMode,
    formatSelectedCompletionTraceForClipboard,
    formatVisibleCompletionTimelineForClipboard,
} from './completionTimelineClipboard';
import { mapCompletionTimelineFetchResultToPanelState } from './completionTimelineModel';
import { CompletionProbe } from './completionProbe';
import { getSharedCompletionProbeRecorder } from './completionProbeRecorder';

type CompletionTimelineWebviewMessage =
    | { type: 'ready' | 'refresh' }
    | { type: 'exportBundle' }
    | { type: 'copyVisible'; mode: CompletionTimelineClipboardMode }
    | { type: 'copyTrace'; trace_id: string };

function asCompletionTimelineWebviewMessage(
    value: unknown
): CompletionTimelineWebviewMessage | null {
    if (!value || typeof value !== 'object') {
        return null;
    }
    const record = value as Record<string, unknown>;
    if (
        record.type === 'ready' ||
        record.type === 'refresh' ||
        record.type === 'exportBundle'
    ) {
        return { type: record.type };
    }
    if (
        record.type === 'copyVisible' &&
        (record.mode === 'all' || record.mode === 'average')
    ) {
        return { type: 'copyVisible', mode: record.mode };
    }
    if (record.type === 'copyTrace' && typeof record.trace_id === 'string' && record.trace_id.length > 0) {
        return { type: 'copyTrace', trace_id: record.trace_id };
    }
    return null;
}

export class CompletionTimelineWebviewProvider implements vscode.WebviewViewProvider, vscode.Disposable {
    private view: vscode.WebviewView | undefined;
    private pollTimer: NodeJS.Timeout | null = null;
    private refreshInFlight = false;
    private readonly disposables: vscode.Disposable[] = [];
    private latestState:
        | ReturnType<typeof mapCompletionTimelineFetchResultToPanelState>
        | null = null;
    private latestExportCapture:
        | {
            capturedAtMs: number;
            completionTimeline: CompletionTimelineFetchResult;
            clientProbes: CompletionProbe[];
        }
        | null = null;

    constructor(
        private readonly outputChannel: vscode.OutputChannel,
        private readonly clipboardWriter: (text: string) => Thenable<void> = (text) => vscode.env.clipboard.writeText(text)
    ) {
        this.disposables.push(
            vscode.workspace.onDidChangeConfiguration((event) => {
                if (event.affectsConfiguration('bslAnalyzer.observabilityRefreshMs')) {
                    this.restartPolling();
                }
            })
        );
    }

    resolveWebviewView(webviewView: vscode.WebviewView): void {
        this.view = webviewView;
        webviewView.webview.options = {
            enableScripts: true,
        };
        webviewView.webview.html = this.getWebviewContent(webviewView.webview);

        const messageDisposable = webviewView.webview.onDidReceiveMessage((message) => {
            void this.handleWebviewMessage(message);
        });

        const visibilityDisposable = webviewView.onDidChangeVisibility(() => {
            this.restartPolling();
            if (webviewView.visible) {
                void this.refreshInternal();
            }
        });

        webviewView.onDidDispose(() => {
            messageDisposable.dispose();
            visibilityDisposable.dispose();
            this.view = undefined;
            this.stopPolling();
        });

        this.restartPolling();
        void this.refreshInternal();
    }

    refresh(): void {
        void this.refreshInternal();
    }

    dispose(): void {
        this.stopPolling();
        this.view = undefined;
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
    }

    private restartPolling(): void {
        this.stopPolling();
        if (!this.view?.visible) {
            return;
        }

        const intervalMs = Math.min(
            60_000,
            Math.max(1_000, BslAnalyzerConfig.observabilityRefreshMs)
        );
        this.pollTimer = setInterval(() => {
            void this.refreshInternal();
        }, intervalMs);
    }

    private stopPolling(): void {
        if (this.pollTimer) {
            clearInterval(this.pollTimer);
            this.pollTimer = null;
        }
    }

    private async refreshInternal(): Promise<void> {
        if (!this.view || this.refreshInFlight) {
            return;
        }

        this.refreshInFlight = true;
        const updatedAtMs = Date.now();
        const clientProbes = getSharedCompletionProbeRecorder().snapshot();
        try {
            const fetchResult = await getCompletionTimeline({ limit: 50 });
            const state = mapCompletionTimelineFetchResultToPanelState(
                fetchResult,
                clientProbes,
                updatedAtMs
            );
            this.latestState = state;
            this.latestExportCapture = {
                capturedAtMs: updatedAtMs,
                completionTimeline: fetchResult,
                clientProbes,
            };
            await this.view.webview.postMessage({
                type: 'timelineState',
                state,
            });
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.outputChannel.appendLine(`❌ Completion Timeline refresh failed: ${message}`);
            const state = mapCompletionTimelineFetchResultToPanelState(
                {
                    kind: 'error',
                    message,
                },
                clientProbes,
                updatedAtMs
            );
            this.latestState = state;
            this.latestExportCapture = {
                capturedAtMs: updatedAtMs,
                completionTimeline: {
                    kind: 'error',
                    message,
                },
                clientProbes,
            };
            await this.view.webview.postMessage({
                type: 'timelineState',
                state,
            });
        } finally {
            this.refreshInFlight = false;
        }
    }

    private async handleWebviewMessage(message: unknown): Promise<void> {
        const parsedMessage = asCompletionTimelineWebviewMessage(message);
        if (!parsedMessage) {
            return;
        }

        if (parsedMessage.type === 'ready' || parsedMessage.type === 'refresh') {
            await this.refreshInternal();
            return;
        }

        if (parsedMessage.type === 'exportBundle') {
            await vscode.commands.executeCommand(
                'bslAnalyzer.exportObservabilityIncidentBundle',
                this.latestExportCapture ?? undefined
            );
            return;
        }

        if (parsedMessage.type === 'copyVisible') {
            const text = this.latestState
                ? formatVisibleCompletionTimelineForClipboard(this.latestState, parsedMessage.mode)
                : null;
            await this.copyTextToClipboard(text, 'visible traces');
            return;
        }

        if (parsedMessage.type === 'copyTrace') {
            const text = this.latestState
                ? formatSelectedCompletionTraceForClipboard(this.latestState, parsedMessage.trace_id)
                : null;
            await this.copyTextToClipboard(text, `trace ${parsedMessage.trace_id}`);
        }
    }

    private async copyTextToClipboard(
        text: string | null,
        label: string
    ): Promise<void> {
        if (!text) {
            await this.postCopyResult(false, `Nothing to copy for ${label}.`);
            return;
        }

        try {
            await this.clipboardWriter(text);
            await this.postCopyResult(true, `${label} copied.`);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.outputChannel.appendLine(`❌ Completion Timeline copy failed: ${message}`);
            await this.postCopyResult(false, `Copy failed: ${message}`);
        }
    }

    private async postCopyResult(ok: boolean, message: string): Promise<void> {
        if (!this.view) {
            return;
        }
        await this.view.webview.postMessage({
            type: 'copyResult',
            ok,
            message,
        });
    }

    private getWebviewContent(webview: vscode.Webview): string {
        const nonce = getNonce();
        return `<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Content-Security-Policy"
          content="default-src 'none';
                   style-src ${webview.cspSource} 'unsafe-inline';
                   script-src 'nonce-${nonce}';">
    <title>Completion Timeline</title>
    <style>
        :root {
            color-scheme: light dark;
        }
        body {
            margin: 0;
            padding: 10px;
            color: var(--vscode-editor-foreground);
            background: var(--vscode-editor-background);
            font-family: var(--vscode-font-family, 'Segoe UI', sans-serif);
            font-size: 12px;
        }
        .toolbar {
            display: flex;
            align-items: center;
            gap: 8px;
            margin-bottom: 10px;
            flex-wrap: wrap;
        }
        .refresh {
            border: 1px solid var(--vscode-button-border, transparent);
            background: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
            border-radius: 6px;
            padding: 4px 10px;
            cursor: pointer;
        }
        .refresh:hover {
            background: var(--vscode-button-hoverBackground);
        }
        .mode-toggle {
            display: inline-flex;
            border: 1px solid var(--vscode-panel-border);
            border-radius: 6px;
            overflow: hidden;
        }
        .mode-button {
            border: 0;
            background: transparent;
            color: var(--vscode-editor-foreground);
            padding: 4px 8px;
            cursor: pointer;
        }
        .mode-button.active {
            background: color-mix(in srgb, var(--vscode-button-background) 45%, transparent);
            color: var(--vscode-button-foreground);
        }
        .meta {
            color: var(--vscode-descriptionForeground);
        }
        .toolbar-spacer {
            flex: 1 1 auto;
        }
        .copy {
            border: 1px solid var(--vscode-button-secondaryBorder, var(--vscode-panel-border));
            background: var(--vscode-button-secondaryBackground, transparent);
            color: var(--vscode-button-secondaryForeground, var(--vscode-editor-foreground));
            border-radius: 6px;
            padding: 4px 10px;
            cursor: pointer;
        }
        .copy:hover {
            background: var(--vscode-button-secondaryHoverBackground, color-mix(in srgb, var(--vscode-button-background) 20%, transparent));
        }
        .trace {
            border: 1px solid var(--vscode-panel-border);
            border-radius: 8px;
            padding: 8px;
            margin-bottom: 8px;
            background:
                linear-gradient(
                    180deg,
                    color-mix(in srgb, var(--vscode-editor-background) 92%, var(--vscode-editor-foreground)),
                    var(--vscode-editor-background)
                );
        }
        .trace-header {
            display: flex;
            justify-content: space-between;
            gap: 10px;
            margin-bottom: 6px;
            flex-wrap: wrap;
        }
        .trace-actions {
            display: inline-flex;
            align-items: center;
            gap: 8px;
        }
        .badge {
            border-radius: 999px;
            padding: 2px 8px;
            font-size: 11px;
            font-weight: 600;
        }
        .badge-ok {
            background: color-mix(in srgb, #2ea043 25%, transparent);
            color: #1f7a33;
        }
        .badge-cancelled {
            background: color-mix(in srgb, #d29922 30%, transparent);
            color: #9a6700;
        }
        .badge-error {
            background: color-mix(in srgb, #cf222e 26%, transparent);
            color: #a40e26;
        }
        .timeline-track {
            display: flex;
            height: 10px;
            border-radius: 999px;
            overflow: hidden;
            background: var(--vscode-editorWidget-border);
            margin: 8px 0;
        }
        .segment {
            min-width: 1px;
            opacity: 0.9;
        }
        .segment-completed { background: #2ea043; }
        .segment-cancelled { background: #d29922; }
        .segment-failed { background: #cf222e; }
        .segment-skipped { background: #6e7781; }
        .segment-dominant {
            outline: 2px solid var(--vscode-focusBorder);
            outline-offset: -2px;
            opacity: 1;
        }
        .stage-table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 6px;
        }
        .stage-table td {
            padding: 2px 0;
            vertical-align: top;
        }
        .stage-name {
            width: 44%;
            font-family: var(--vscode-editor-font-family, 'Cascadia Mono', monospace);
        }
        .stage-hint {
            margin-left: 6px;
            color: var(--vscode-descriptionForeground);
            cursor: help;
            font-size: 10px;
            vertical-align: middle;
        }
        .stage-status {
            width: 20%;
        }
        .stage-time {
            width: 36%;
            color: var(--vscode-descriptionForeground);
            text-align: right;
        }
        .overhead {
            margin-top: 4px;
            color: var(--vscode-descriptionForeground);
        }
        .dominant-chip {
            margin-left: 6px;
            border-radius: 999px;
            border: 1px solid var(--vscode-focusBorder);
            padding: 0 6px;
            font-size: 10px;
            color: var(--vscode-focusBorder);
        }
        .placeholder {
            border: 1px dashed var(--vscode-panel-border);
            border-radius: 8px;
            padding: 12px;
            color: var(--vscode-descriptionForeground);
        }
        .section {
            margin-bottom: 12px;
        }
        .section-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 8px;
            margin-bottom: 6px;
            flex-wrap: wrap;
        }
        .section-title {
            font-size: 13px;
            font-weight: 600;
        }
        .section-subtitle {
            margin: 2px 0 0;
            color: var(--vscode-descriptionForeground);
        }
        .section-pill {
            border-radius: 999px;
            padding: 2px 8px;
            font-size: 11px;
            font-weight: 600;
            background: color-mix(in srgb, var(--vscode-focusBorder) 18%, transparent);
            color: var(--vscode-focusBorder);
        }
        .probe-feed {
            display: flex;
            flex-direction: column;
            gap: 8px;
        }
        .probe {
            border: 1px solid var(--vscode-panel-border);
            border-radius: 8px;
            padding: 8px;
            background:
                linear-gradient(
                    180deg,
                    color-mix(in srgb, var(--vscode-editor-background) 95%, var(--vscode-editor-foreground)),
                    var(--vscode-editor-background)
                );
        }
        .probe-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 4px 10px;
            margin-top: 6px;
        }
        .probe-cell {
            color: var(--vscode-descriptionForeground);
        }
        .probe-cell strong {
            color: var(--vscode-editor-foreground);
        }
    </style>
</head>
<body>
    <div class="toolbar">
        <button class="refresh" id="refresh">Refresh</button>
        <div class="mode-toggle" role="tablist" aria-label="Timeline mode">
            <button class="mode-button active" id="modeAll" role="tab" aria-selected="true">All traces</button>
            <button class="mode-button" id="modeAverage" role="tab" aria-selected="false">Averaged</button>
        </div>
        <button class="copy" id="copyVisible">Copy visible</button>
        <button class="copy" id="exportBundle">Export bundle</button>
        <span class="toolbar-spacer"></span>
        <span class="meta" id="updatedAt">Waiting for data...</span>
        <span class="meta" id="copyStatus" aria-live="polite"></span>
    </div>
    <section class="section" aria-labelledby="serverTimelineTitle">
        <div class="section-header">
            <div>
                <div class="section-title" id="serverTimelineTitle">Server Timeline</div>
                <p class="section-subtitle">Authoritative server-driven completion timeline from <code>bsl.getCompletionTimeline</code>.</p>
            </div>
        </div>
        <div id="serverRoot" class="placeholder">Loading completion timeline...</div>
    </section>
    <section class="section" aria-labelledby="clientProbeTitle">
        <div class="section-header">
            <div>
                <div class="section-title" id="clientProbeTitle">Client Probe Feed</div>
                <p class="section-subtitle">Local-only extension debug data for the client-side edge of completion.</p>
            </div>
            <span class="section-pill">Local-only debug data</span>
        </div>
        <p class="section-subtitle">Client probes never replace server stages, routes, or outcomes and are not correlated to a specific server trace in this MVP.</p>
        <div id="clientRoot" class="placeholder">No client probes recorded yet.</div>
    </section>
    <script nonce="${nonce}">
        const vscode = acquireVsCodeApi();
        const serverRoot = document.getElementById('serverRoot');
        const clientRoot = document.getElementById('clientRoot');
        const updatedAtNode = document.getElementById('updatedAt');
        const copyStatusNode = document.getElementById('copyStatus');
        const refreshButton = document.getElementById('refresh');
        const copyVisibleButton = document.getElementById('copyVisible');
        const exportBundleButton = document.getElementById('exportBundle');
        const modeAllButton = document.getElementById('modeAll');
        const modeAverageButton = document.getElementById('modeAverage');
        let currentMode = 'all';
        let latestReadyState = null;
        let copyStatusTimer = null;

        refreshButton.addEventListener('click', () => {
            vscode.postMessage({ type: 'refresh' });
        });
        copyVisibleButton.addEventListener('click', () => {
            vscode.postMessage({ type: 'copyVisible', mode: currentMode });
        });
        exportBundleButton.addEventListener('click', () => {
            vscode.postMessage({ type: 'exportBundle' });
        });
        modeAllButton.addEventListener('click', () => {
            setMode('all');
        });
        modeAverageButton.addEventListener('click', () => {
            setMode('average');
        });
        serverRoot.addEventListener('click', (event) => {
            const target = event.target;
            if (!(target instanceof Element)) {
                return;
            }
            const button = target.closest('[data-copy-trace-id]');
            if (!button) {
                return;
            }
            const traceId = button.getAttribute('data-copy-trace-id');
            if (!traceId) {
                return;
            }
            vscode.postMessage({ type: 'copyTrace', trace_id: traceId });
        });

        function escapeHtml(value) {
            return String(value)
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;')
                .replace(/"/g, '&quot;')
                .replace(/'/g, '&#39;');
        }

        function outcomeBadgeClass(outcome) {
            if (outcome.startsWith('ok_')) {
                return 'badge-ok';
            }
            if (outcome === 'cancelled' || outcome.startsWith('superseded')) {
                return 'badge-cancelled';
            }
            return 'badge-error';
        }

        function stageDescription(stageName) {
            const dictionary = {
                turn_wait: 'Ожидание очереди запроса completion до получения активного turn.',
                sync_globals: 'Синхронизация глобального состояния и зависимостей перед анализом.',
                prepare_stateful: 'Подготовка stateful snapshot и runtime-контекста для completion.',
                query_bundle: 'Основной запросный блок: чтение snapshot/IR и сбор контекста.',
                snapshot_read: 'Чтение snapshot данных для completion pipeline.',
                collect: 'Сбор кандидатов completion из доступных источников.',
                rank: 'Ранжирование кандидатов по релевантности.',
                format: 'Финальное форматирование кандидатов для LSP response.',
                response_build: 'Агрегированная сборка LSP completion response.',
                response_build_other: 'Оставшееся время сборки response, не вошедшее в snapshot/collect/rank/format.',
                cache_store: 'Запись результата в fallback-кэш для устойчивости последующих запросов.',
                terminal: 'Финальный terminal-чекпоинт outcome (ok/cancelled/failed).',
            };
            return dictionary[stageName] || ('Pipeline stage: ' + stageName);
        }

        function renderStageRow(stage) {
            const dominant = stage.is_dominant
                ? '<span class="dominant-chip">dominant</span>'
                : '';
            const description = stageDescription(stage.name);
            return '<tr>' +
                '<td class="stage-name" title="' + escapeHtml(description) + '">' +
                    escapeHtml(stage.name) + dominant +
                    '<span class="stage-hint" title="' + escapeHtml(description) + '">?</span>' +
                '</td>' +
                '<td class="stage-status">' + escapeHtml(stage.status) + '</td>' +
                '<td class="stage-time">' +
                    escapeHtml(stage.started_offset_ms) + 'ms -> ' +
                    escapeHtml(stage.end_offset_ms) + 'ms (' +
                    escapeHtml(stage.duration_ms) + 'ms, ' +
                    escapeHtml(stage.duration_percent.toFixed(1)) + '%)' +
                '</td>' +
            '</tr>';
        }

        function renderTurnHolder(label, holder) {
            if (!holder) {
                return '';
            }
            const requestId = holder.request_id || 'n/a';
            const versionHint = typeof holder.version_hint === 'number'
                ? ' | version_hint=' + escapeHtml(holder.version_hint)
                : '';
            return '<div class="overhead">' +
                escapeHtml(label) + ': request=' + escapeHtml(requestId) +
                ' | file_seq=' + escapeHtml(holder.file_seq) +
                ' | epoch=' + escapeHtml(holder.request_epoch) +
                ' | trigger=' + escapeHtml(holder.trigger_mode) +
                versionHint +
                ' | age=' + escapeHtml(holder.age_ms) + 'ms' +
            '</div>';
        }

        function renderTurnAttribution(trace) {
            if (!trace.turn_attribution) {
                return '';
            }
            const turn = trace.turn_attribution;
            const dropped = Array.isArray(turn.dropped_completion_file_seq) && turn.dropped_completion_file_seq.length > 0
                ? ' | dropped=' + escapeHtml(turn.dropped_completion_file_seq.join(','))
                : '';
            const waitOutcome = turn.turn_wait_outcome
                ? ' | turn_wait=' + escapeHtml(turn.turn_wait_outcome)
                : '';
            return '<div class="overhead">' +
                'Turn attribution: file_seq=' + escapeHtml(turn.request_file_seq) +
                ' | epoch=' + escapeHtml(turn.request_epoch) +
                ' | queue_outcome=' + escapeHtml(turn.queue_outcome) +
                waitOutcome +
                ' | queue=' + escapeHtml(turn.queue_depth_before_enqueue) + '->' +
                escapeHtml(turn.queue_depth_after_enqueue) + '/' + escapeHtml(turn.queue_capacity) +
                ' | queued_completion_ahead=' + escapeHtml(turn.queued_completion_ahead_count) +
                ' | did_change_ahead=' + escapeHtml(turn.did_change_ahead_count) +
                ' | active=' + escapeHtml(turn.active_completion_count) +
                dropped +
            '</div>' +
            renderTurnHolder('Active holder', turn.active_holder) +
            renderTurnHolder('Queued ahead', turn.queued_completion_ahead);
        }

        function renderPrepareDetails(trace) {
            if (!trace.prepare_details) {
                return '';
            }
            const details = trace.prepare_details;
            const bits = [];
            if (typeof details.wait_budget_ms === 'number') {
                bits.push('prepare_wait_budget=' + escapeHtml(details.wait_budget_ms) + 'ms');
            }
            if (details.guard_outcome) {
                bits.push('prepare_guard=' + escapeHtml(details.guard_outcome));
            }
            if (details.outcome) {
                bits.push('prepare_outcome=' + escapeHtml(details.outcome));
            }
            if (details.route) {
                bits.push('completion_route=' + escapeHtml(details.route));
            }
            if (details.fail_closed_cause) {
                bits.push('fail_closed_cause=' + escapeHtml(details.fail_closed_cause));
            }
            if (typeof details.min_file_version === 'number') {
                bits.push('min_version=' + escapeHtml(details.min_file_version));
            }
            if (typeof details.shadow_version_at_start === 'number') {
                bits.push('shadow_version=' + escapeHtml(details.shadow_version_at_start));
            }
            if (typeof details.observed_file_version === 'number') {
                bits.push('observed_version=' + escapeHtml(details.observed_file_version));
            }
            if (typeof details.wait_elapsed_ms === 'number') {
                bits.push('wait_elapsed=' + escapeHtml(details.wait_elapsed_ms) + 'ms');
            }
            if (typeof details.snapshot_elapsed_ms === 'number') {
                bits.push('snapshot_elapsed=' + escapeHtml(details.snapshot_elapsed_ms) + 'ms');
            }
            if (typeof details.apply_age_at_start_ms === 'number') {
                bits.push('apply_age_start=' + escapeHtml(details.apply_age_at_start_ms) + 'ms');
            }
            if (typeof details.apply_age_at_terminal_ms === 'number') {
                bits.push('apply_age_terminal=' + escapeHtml(details.apply_age_at_terminal_ms) + 'ms');
            }
            if (bits.length === 0) {
                return '';
            }
            return '<div class="overhead">' + bits.join(' | ') + '</div>';
        }

        function renderServerEdgeDetails(trace) {
            if (!trace.server_edge_details) {
                return '';
            }
            const details = trace.server_edge_details;
            const bits = [
                'transport_received=' + escapeHtml(new Date(details.transport_received_at_ms).toLocaleTimeString()),
                'handler_entered=' + escapeHtml(new Date(details.handler_entered_at_ms).toLocaleTimeString()),
                'response_sent=' + escapeHtml(new Date(details.response_sent_at_ms).toLocaleTimeString()),
                'transport_to_handler_wait=' + escapeHtml(details.transport_to_handler_wait_ms) + 'ms',
                'server_handler_exec=' + escapeHtml(details.server_handler_exec_ms) + 'ms',
            ];
            if (typeof details.cancel_observed_at_ms === 'number') {
                bits.push('cancel_observed=' + escapeHtml(new Date(details.cancel_observed_at_ms).toLocaleTimeString()));
            }
            if (typeof details.cancel_observed_after_handler_enter_ms === 'number') {
                bits.push(
                    'cancel_after_handler_enter=' +
                    escapeHtml(details.cancel_observed_after_handler_enter_ms) +
                    'ms'
                );
            }
            return '<div class="overhead">' + bits.join(' | ') + '</div>';
        }

        function renderTrace(trace) {
            const outcomeClass = outcomeBadgeClass(trace.outcome);
            const stageSegments = trace.stages.map((stage) => {
                const classes = [
                    'segment',
                    'segment-' + stage.status,
                    stage.is_dominant ? 'segment-dominant' : '',
                ].join(' ').trim();
                const title = stage.name + ' (' + stage.duration_ms + 'ms, ' +
                    stage.duration_percent.toFixed(1) + '%, ' + stage.status + '). ' +
                    stageDescription(stage.name);
                return '<div class="' + classes + '" style="width:' + stage.width_percent.toFixed(2) + '%" title="' + escapeHtml(title) + '"></div>';
            }).join('');

            const stageRows = trace.stages.map(renderStageRow).join('');
            const requestId = trace.request_id || 'n/a';
            const startedAt = new Date(trace.started_at_ms).toLocaleTimeString();
            const overhead = trace.unattributed_overhead_ms > 0
                ? '<div class="overhead" title="Total duration minus max stage end offset">' +
                    'Unattributed overhead: ' + escapeHtml(trace.unattributed_overhead_ms) + 'ms ' +
                    '(total ' + escapeHtml(trace.total_duration_ms) + 'ms - max_stage_end ' +
                    escapeHtml(trace.max_stage_end_ms) + 'ms)' +
                '</div>'
                : '';
            const serverEdgeDetails = renderServerEdgeDetails(trace);
            const prepareDetails = renderPrepareDetails(trace);
            const turnAttribution = renderTurnAttribution(trace);

            return '<section class="trace">' +
                '<div class="trace-header">' +
                    '<div>' +
                        '<strong>' + escapeHtml(trace.trace_id) + '</strong>' +
                        ' <span class="meta">(' + escapeHtml(trace.trigger_mode) + ')</span>' +
                        (trace.sample_count ? ' <span class="meta">| sample=' + escapeHtml(trace.sample_count) + '</span>' : '') +
                    '</div>' +
                    '<div class="trace-actions">' +
                        '<button class="copy" data-copy-trace-id="' + escapeHtml(trace.trace_id) + '">Copy</button>' +
                        '<span class="badge ' + outcomeClass + '">' + escapeHtml(trace.outcome) + '</span>' +
                        ' <span class="meta">' + escapeHtml(trace.total_duration_ms) + 'ms</span>' +
                    '</div>' +
                '</div>' +
                '<div class="meta">request=' + escapeHtml(requestId) + ' | started=' + escapeHtml(startedAt) + '</div>' +
                '<div class="timeline-track">' + stageSegments + '</div>' +
                overhead +
                serverEdgeDetails +
                prepareDetails +
                turnAttribution +
                '<table class="stage-table"><tbody>' + stageRows + '</tbody></table>' +
            '</section>';
        }

        function renderClientProbe(probe) {
            const outcomeClass = outcomeBadgeClass(probe.client_terminal_state);
            const didChangeDelta = probe.time_since_last_did_change_sent_ms === 'unknown'
                ? 'unknown'
                : probe.time_since_last_did_change_sent_ms + 'ms';
            const triggerCharacter = probe.trigger_character
                ? ' | trigger_character=' + escapeHtml(probe.trigger_character)
                : '';
            const dispatchDeltaMs = Math.max(0, probe.lsp_request_started_at_ms - probe.request_started_at_ms);
            const lspRoundtripMs = Math.max(0, probe.lsp_response_received_at_ms - probe.lsp_request_started_at_ms);
            const postResponseMs = Math.max(0, probe.request_completed_at_ms - probe.lsp_response_received_at_ms);
            const incompleteSuffix = typeof probe.is_incomplete === 'boolean'
                ? ' | is_incomplete=' + escapeHtml(String(probe.is_incomplete))
                : '';
            const supersededSuffix = probe.superseded_by_probe_id
                ? ' | superseded_by=' + escapeHtml(probe.superseded_by_probe_id)
                : '';
            const supersededAfterSuffix = typeof probe.superseded_after_ms === 'number'
                ? ' | superseded_after=' + escapeHtml(probe.superseded_after_ms) + 'ms'
                : '';

            return '<section class="probe">' +
                '<div class="trace-header">' +
                    '<div>' +
                        '<strong>' + escapeHtml(probe.probe_id) + '</strong>' +
                        ' <span class="meta">(' + escapeHtml(probe.trigger_mode) + ')</span>' +
                    '</div>' +
                    '<div class="trace-actions">' +
                        '<span class="badge ' + outcomeClass + '">' + escapeHtml(probe.client_terminal_state) + '</span>' +
                        '<span class="meta">' + escapeHtml(probe.client_duration_ms) + 'ms</span>' +
                    '</div>' +
                '</div>' +
                '<div class="meta">started=' + escapeHtml(new Date(probe.request_started_at_ms).toLocaleTimeString()) +
                    ' | uri=' + escapeHtml(probe.uri) +
                    ' | version=' + escapeHtml(probe.document_version) +
                    ' | terminal_version=' + escapeHtml(probe.document_version_at_terminal) + '</div>' +
                '<div class="probe-grid">' +
                    '<div class="probe-cell"><strong>Local edit</strong><br>' + escapeHtml(probe.time_since_last_local_edit_ms) + 'ms</div>' +
                    '<div class="probe-cell"><strong>didChange sent</strong><br>' + escapeHtml(didChangeDelta) + '</div>' +
                    '<div class="probe-cell"><strong>Transport</strong><br>dispatch=' + escapeHtml(dispatchDeltaMs) + 'ms | wait=' + escapeHtml(lspRoundtripMs) + 'ms | post=' + escapeHtml(postResponseMs) + 'ms</div>' +
                    '<div class="probe-cell"><strong>Result</strong><br>' + escapeHtml(probe.result_kind) + ' | bucket=' + escapeHtml(probe.item_count_bucket) + incompleteSuffix + '</div>' +
                    '<div class="probe-cell"><strong>Cancel hint</strong><br>' + escapeHtml(probe.cancel_reason_hint) + supersededSuffix + supersededAfterSuffix + '</div>' +
                    '<div class="probe-cell"><strong>Drift</strong><br>did_change=' + escapeHtml(probe.did_change_count_during_probe) + ' | cursor_moved=' + escapeHtml(probe.cursor_moved_during_probe) + '</div>' +
                    '<div class="probe-cell"><strong>Overlap</strong><br>active=' + escapeHtml(probe.active_completion_count_at_start) + ' | same_uri=' + escapeHtml(probe.same_uri_probe_overlap_count) + ' | newer=' + escapeHtml(probe.newer_probe_started_before_terminal) + '</div>' +
                    '<div class="probe-cell"><strong>After dot</strong><br>' + escapeHtml(probe.is_after_dot) + '</div>' +
                    '<div class="probe-cell"><strong>Identifier tail</strong><br>' + escapeHtml(probe.identifier_tail_length) + '</div>' +
                    '<div class="probe-cell"><strong>Trigger</strong><br>' + escapeHtml(probe.trigger_mode) + triggerCharacter + '</div>' +
                '</div>' +
            '</section>';
        }

        function renderClientProbeFeed(feed) {
            if (!feed || !Array.isArray(feed.probes) || feed.probes.length === 0) {
                clientRoot.innerHTML = '<div class="placeholder">No client probes recorded yet. Trigger completion in a BSL document to populate the local feed.</div>';
                return;
            }

            clientRoot.innerHTML =
                '<div class="meta">Updated ' + escapeHtml(new Date(feed.updated_at_ms).toLocaleTimeString()) + '</div>' +
                '<div class="probe-feed">' + feed.probes.map(renderClientProbe).join('') + '</div>';
        }

        function applyModeUi() {
            const allActive = currentMode === 'all';
            modeAllButton.classList.toggle('active', allActive);
            modeAverageButton.classList.toggle('active', !allActive);
            modeAllButton.setAttribute('aria-selected', allActive ? 'true' : 'false');
            modeAverageButton.setAttribute('aria-selected', allActive ? 'false' : 'true');
        }

        function setMode(nextMode) {
            currentMode = nextMode;
            applyModeUi();
            if (latestReadyState) {
                renderReadyState(latestReadyState);
            }
        }

        function renderReadyState(state) {
            const traces = state.traces || [];
            if (currentMode === 'average') {
                if (!state.average_trace) {
                    serverRoot.innerHTML = '<div class="placeholder">No completion traces to average yet.</div>';
                } else {
                    serverRoot.innerHTML = renderTrace(state.average_trace);
                }
                return;
            }

            if (traces.length === 0) {
                serverRoot.innerHTML = '<div class="placeholder">No completion traces yet. Trigger completion to populate the server timeline.</div>';
            } else {
                serverRoot.innerHTML = traces.map(renderTrace).join('');
            }
        }

        function renderState(state) {
            if (!state || typeof state !== 'object') {
                return;
            }

            renderClientProbeFeed(state.client_probe_feed);

            if (state.kind === 'unsupported') {
                serverRoot.innerHTML = '<div class="placeholder">' + escapeHtml(state.message) + '</div>';
                updatedAtNode.textContent = 'Timeline unsupported by current LSP server';
                return;
            }

            if (state.kind === 'error') {
                serverRoot.innerHTML = '<div class="placeholder">Failed to load timeline: ' + escapeHtml(state.message) + '</div>';
                updatedAtNode.textContent = 'Last update failed';
                return;
            }

            if (state.kind === 'ready') {
                latestReadyState = state;
                renderReadyState(state);
                updatedAtNode.textContent = 'Updated ' + new Date(state.updated_at_ms).toLocaleTimeString() +
                    ' | contract v' + state.version;
            }
        }

        function showCopyStatus(message, ok) {
            if (!copyStatusNode) {
                return;
            }
            copyStatusNode.textContent = message;
            copyStatusNode.style.color = ok
                ? 'var(--vscode-descriptionForeground)'
                : 'var(--vscode-errorForeground)';
            if (copyStatusTimer) {
                clearTimeout(copyStatusTimer);
            }
            copyStatusTimer = setTimeout(() => {
                copyStatusNode.textContent = '';
            }, 3000);
        }

        window.addEventListener('message', (event) => {
            const payload = event.data;
            if (payload && payload.type === 'timelineState') {
                renderState(payload.state);
            }
            if (payload && payload.type === 'copyResult') {
                showCopyStatus(String(payload.message || ''), Boolean(payload.ok));
            }
        });

        applyModeUi();
        vscode.postMessage({ type: 'ready' });
    <\/script>
</body>
</html>`;
    }
}

function getNonce(): string {
    const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let value = '';
    for (let idx = 0; idx < 32; idx += 1) {
        value += alphabet.charAt(Math.floor(Math.random() * alphabet.length));
    }
    return value;
}
