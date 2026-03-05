import * as vscode from 'vscode';
import { BslAnalyzerConfig } from '../config/configHelper';
import { getCompletionTimeline } from '../lsp/customRequests';
import { mapCompletionTimelineFetchResultToPanelState } from './completionTimelineModel';

type CompletionTimelineWebviewMessage = {
    type: 'ready' | 'refresh';
};

function asCompletionTimelineWebviewMessage(
    value: unknown
): CompletionTimelineWebviewMessage | null {
    if (!value || typeof value !== 'object') {
        return null;
    }
    const record = value as Record<string, unknown>;
    if (record.type === 'ready' || record.type === 'refresh') {
        return { type: record.type };
    }
    return null;
}

export class CompletionTimelineWebviewProvider implements vscode.WebviewViewProvider, vscode.Disposable {
    private view: vscode.WebviewView | undefined;
    private pollTimer: NodeJS.Timeout | null = null;
    private refreshInFlight = false;
    private readonly disposables: vscode.Disposable[] = [];

    constructor(private readonly outputChannel: vscode.OutputChannel) {
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
            const parsedMessage = asCompletionTimelineWebviewMessage(message);
            if (!parsedMessage) {
                return;
            }
            if (parsedMessage.type === 'ready' || parsedMessage.type === 'refresh') {
                void this.refreshInternal();
            }
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
        try {
            const fetchResult = await getCompletionTimeline({ limit: 50 });
            const state = mapCompletionTimelineFetchResultToPanelState(fetchResult);
            await this.view.webview.postMessage({
                type: 'timelineState',
                state,
            });
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            this.outputChannel.appendLine(`❌ Completion Timeline refresh failed: ${message}`);
            await this.view.webview.postMessage({
                type: 'timelineState',
                state: {
                    kind: 'error',
                    message,
                },
            });
        } finally {
            this.refreshInFlight = false;
        }
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
        .meta {
            color: var(--vscode-descriptionForeground);
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
    </style>
</head>
<body>
    <div class="toolbar">
        <button class="refresh" id="refresh">Refresh</button>
        <span class="meta" id="updatedAt">Waiting for data...</span>
    </div>
    <div id="root" class="placeholder">Loading completion timeline...</div>
    <script nonce="${nonce}">
        const vscode = acquireVsCodeApi();
        const root = document.getElementById('root');
        const updatedAtNode = document.getElementById('updatedAt');
        const refreshButton = document.getElementById('refresh');

        refreshButton.addEventListener('click', () => {
            vscode.postMessage({ type: 'refresh' });
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

            return '<section class="trace">' +
                '<div class="trace-header">' +
                    '<div>' +
                        '<strong>' + escapeHtml(trace.trace_id) + '</strong>' +
                        ' <span class="meta">(' + escapeHtml(trace.trigger_mode) + ')</span>' +
                    '</div>' +
                    '<div>' +
                        '<span class="badge ' + outcomeClass + '">' + escapeHtml(trace.outcome) + '</span>' +
                        ' <span class="meta">' + escapeHtml(trace.total_duration_ms) + 'ms</span>' +
                    '</div>' +
                '</div>' +
                '<div class="meta">request=' + escapeHtml(requestId) + ' | started=' + escapeHtml(startedAt) + '</div>' +
                '<div class="timeline-track">' + stageSegments + '</div>' +
                overhead +
                '<table class="stage-table"><tbody>' + stageRows + '</tbody></table>' +
            '</section>';
        }

        function renderState(state) {
            if (!state || typeof state !== 'object') {
                return;
            }

            if (state.kind === 'unsupported') {
                root.innerHTML = '<div class="placeholder">' + escapeHtml(state.message) + '</div>';
                updatedAtNode.textContent = 'Timeline unsupported by current LSP server';
                return;
            }

            if (state.kind === 'error') {
                root.innerHTML = '<div class="placeholder">Failed to load timeline: ' + escapeHtml(state.message) + '</div>';
                updatedAtNode.textContent = 'Last update failed';
                return;
            }

            if (state.kind === 'ready') {
                const traces = state.traces || [];
                if (traces.length === 0) {
                    root.innerHTML = '<div class="placeholder">No completion traces yet. Trigger completion to populate timeline.</div>';
                } else {
                    root.innerHTML = traces.map(renderTrace).join('');
                }
                updatedAtNode.textContent = 'Updated ' + new Date(state.updated_at_ms).toLocaleTimeString() +
                    ' | contract v' + state.version;
            }
        }

        window.addEventListener('message', (event) => {
            const payload = event.data;
            if (payload && payload.type === 'timelineState') {
                renderState(payload.state);
            }
        });

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
