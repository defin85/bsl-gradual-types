"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerObservabilityCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const customRequests_1 = require("../lsp/customRequests");
function tryGet(obj, path) {
    const parts = path.split('.');
    let cur = obj;
    for (const p of parts) {
        if (cur == null)
            return undefined;
        cur = cur[p];
    }
    return cur;
}
function fmtMs(value) {
    if (typeof value !== 'number')
        return '-';
    return `${Math.round(value)}ms`;
}
/**
 * Register observability/diagnostic commands.
 */
function registerObservabilityCommands(context, safeRegisterCommand, outputChannel) {
    safeRegisterCommand('bslAnalyzer.dumpLspMetrics', async () => {
        const client = (0, lsp_1.getLanguageClient)();
        if (!client || !client.isRunning()) {
            vscode.window.showErrorMessage('LSP server is not running. Please wait or restart the server.');
            return;
        }
        outputChannel.appendLine('');
        outputChannel.appendLine('===================================================');
        outputChannel.appendLine('BSL LSP OBSERVABILITY METRICS (snapshot)');
        outputChannel.appendLine('===================================================');
        const resp = await (0, customRequests_1.getObservabilityMetrics)();
        if (!resp) {
            outputChannel.appendLine('No metrics (LSP unavailable or unsupported).');
            return;
        }
        const metrics = resp.metrics || {};
        const uptimeSeconds = tryGet(metrics, 'uptime_seconds');
        outputChannel.appendLine(`uptime_seconds: ${uptimeSeconds ?? 'unknown'}`);
        const hist = tryGet(metrics, 'histograms') || {};
        const rows = [
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
            if (!entry)
                continue;
            outputChannel.appendLine(`${prefix}: p50=${fmtMs(entry.p50)} p95=${fmtMs(entry.p95)} p99=${fmtMs(entry.p99)} (n=${entry.count ?? '?'})`);
        }
        outputChannel.appendLine('');
        outputChannel.appendLine('Raw JSON (trimmed):');
        const json = JSON.stringify(metrics, null, 2);
        const MAX_CHARS = 12000;
        outputChannel.appendLine(json.length > MAX_CHARS ? json.slice(0, MAX_CHARS) + '\n... (truncated)' : json);
        outputChannel.appendLine('===================================================');
    });
}
exports.registerObservabilityCommands = registerObservabilityCommands;
//# sourceMappingURL=observability.js.map
