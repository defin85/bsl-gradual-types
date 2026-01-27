import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { getLanguageClient } from '../lsp';
import { getObservabilityMetrics } from '../lsp/customRequests';

function tryGet(obj: any, path: string): any {
    const parts = path.split('.');
    let cur: any = obj;
    for (const p of parts) {
        if (cur == null) return undefined;
        cur = cur[p];
    }
    return cur;
}

function fmtMs(value: any): string {
    if (typeof value !== 'number') return '-';
    return `${Math.round(value)}ms`;
}

/**
 * Register observability/diagnostic commands.
 */
export function registerObservabilityCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>,
    outputChannel: vscode.OutputChannel
) {
    safeRegisterCommand('bslAnalyzer.dumpLspMetrics', async () => {
        const client = getLanguageClient();
        if (!client || !client.isRunning()) {
            vscode.window.showErrorMessage('LSP server is not running. Please wait or restart the server.');
            return;
        }

        outputChannel.appendLine('');
        outputChannel.appendLine('===================================================');
        outputChannel.appendLine('BSL LSP OBSERVABILITY METRICS (snapshot)');
        outputChannel.appendLine('===================================================');

        const resp = await getObservabilityMetrics();
        if (!resp) {
            outputChannel.appendLine('No metrics (LSP unavailable or unsupported).');
            return;
        }

        const metrics = resp.metrics || {};
        const uptimeSeconds = tryGet(metrics, 'uptime_seconds');
        outputChannel.appendLine(`uptime_seconds: ${uptimeSeconds ?? 'unknown'}`);

        const hist = tryGet(metrics, 'histograms') || {};

        const rows: Array<[string, string, string, string]> = [
            ['intellisense_v2_wait_for_file_version_diagnostics', 'p50', 'p95', 'p99'],
            ['intellisense_v2_syntax_diagnostics_query', 'p50', 'p95', 'p99'],
            ['intellisense_v2_semantic_diagnostics_query', 'p50', 'p95', 'p99'],
            ['intellisense_v2_wait_for_file_version_completion', 'p50', 'p95', 'p99'],
            ['intellisense_v2_snapshot_completion', 'p50', 'p95', 'p99'],
            ['intellisense_v2_ir_query_completion', 'p50', 'p95', 'p99'],
            ['intellisense_v2_wait_for_file_version_hover', 'p50', 'p95', 'p99'],
            ['intellisense_v2_snapshot_hover', 'p50', 'p95', 'p99'],
            ['intellisense_v2_ir_query_hover', 'p50', 'p95', 'p99'],
        ];

        outputChannel.appendLine('');
        outputChannel.appendLine('Key latencies (ms):');
        for (const [prefix] of rows) {
            const entry = hist[`${prefix}_ms`];
            if (!entry) continue;
            outputChannel.appendLine(
                `${prefix}: p50=${fmtMs(entry.p50)} p95=${fmtMs(entry.p95)} p99=${fmtMs(entry.p99)} (n=${entry.count ?? '?'})`
            );
        }

        outputChannel.appendLine('');
        outputChannel.appendLine('Raw JSON (trimmed):');
        const json = JSON.stringify(metrics, null, 2);
        const MAX_CHARS = 12000;
        outputChannel.appendLine(json.length > MAX_CHARS ? json.slice(0, MAX_CHARS) + '\n... (truncated)' : json);

        outputChannel.appendLine('===================================================');
    });
}

