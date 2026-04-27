import * as fs from 'fs/promises';
import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { getActiveServerLaunchInfo, getLanguageClient } from '../lsp';
import {
    getCompletionTimeline,
    getCurrentContextTimeline,
    getDiagnosticsSaveTimeline,
    getObservabilityMetrics,
    getObservabilityMetricsFetchResult,
} from '../lsp/customRequests';
import { getSharedCompletionProbeRecorder } from '../providers/completionProbeRecorder';
import {
    CompletionTimelineExportCapture,
    getSharedCompletionTimelineExportCapture,
} from '../providers/completionTimelineExportCapture';
import {
    buildObservabilityIncidentBundle,
    ObservabilityIncidentBundleBuildIdentity,
} from '../providers/observabilityIncidentBundle';
import { CompletionProbe } from '../providers/completionProbe';

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

const COMPLETION_TIMELINE_EXPORT_LIMIT = 50;
const textEncoder = new TextEncoder();

function asExportCapture(value: unknown): CompletionTimelineExportCapture | undefined {
    if (!value || typeof value !== 'object') {
        return undefined;
    }
    return value as CompletionTimelineExportCapture;
}

function mergeExportCaptures(
    sharedCapture: CompletionTimelineExportCapture | undefined,
    explicitCapture: CompletionTimelineExportCapture
): CompletionTimelineExportCapture {
    return {
        capturedAtMs: explicitCapture.capturedAtMs ?? sharedCapture?.capturedAtMs,
        completionTimeline:
            explicitCapture.completionTimeline ?? sharedCapture?.completionTimeline,
        currentContextTimeline:
            explicitCapture.currentContextTimeline ?? sharedCapture?.currentContextTimeline,
        diagnosticsSaveTimeline:
            explicitCapture.diagnosticsSaveTimeline ?? sharedCapture?.diagnosticsSaveTimeline,
        clientProbes: explicitCapture.clientProbes ?? sharedCapture?.clientProbes,
        observabilityMetrics:
            explicitCapture.observabilityMetrics ?? sharedCapture?.observabilityMetrics,
    };
}

async function exportObservabilityIncidentBundleToFolder(
    targetFolder: vscode.Uri,
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
    capture: CompletionTimelineExportCapture = {}
): Promise<void> {
    const resolvedCapture = mergeExportCaptures(
        getSharedCompletionTimelineExportCapture(),
        capture
    );
    const capturedAtMs = resolvedCapture.capturedAtMs ?? Date.now();
    const clientProbes =
        resolvedCapture.clientProbes ?? getSharedCompletionProbeRecorder().snapshot();
    const completionTimelinePromise = resolvedCapture.completionTimeline
        ? Promise.resolve(resolvedCapture.completionTimeline)
        : getCompletionTimeline({ limit: COMPLETION_TIMELINE_EXPORT_LIMIT });
    const currentContextTimelinePromise = resolvedCapture.currentContextTimeline
        ? Promise.resolve(resolvedCapture.currentContextTimeline)
        : getCurrentContextTimeline({ limit: COMPLETION_TIMELINE_EXPORT_LIMIT });
    const diagnosticsSaveTimelinePromise = resolvedCapture.diagnosticsSaveTimeline
        ? Promise.resolve(resolvedCapture.diagnosticsSaveTimeline)
        : getDiagnosticsSaveTimeline({ limit: COMPLETION_TIMELINE_EXPORT_LIMIT });
    const observabilityMetricsPromise = resolvedCapture.observabilityMetrics
        ? Promise.resolve(resolvedCapture.observabilityMetrics)
        : getObservabilityMetricsFetchResult({ shape: 'full' });
    const [
        completionTimeline,
        currentContextTimeline,
        diagnosticsSaveTimeline,
        observabilityMetrics,
    ] = await Promise.all([
        completionTimelinePromise,
        currentContextTimelinePromise,
        diagnosticsSaveTimelinePromise,
        observabilityMetricsPromise,
    ]);
    const buildIdentity = await buildCurrentIncidentBundleBuildIdentity(context);

    const bundle = buildObservabilityIncidentBundle({
        capturedAtMs,
        completionTimeline,
        currentContextTimeline,
        diagnosticsSaveTimeline,
        completionTraceLimit: COMPLETION_TIMELINE_EXPORT_LIMIT,
        clientProbes,
        observabilityMetrics,
        buildIdentity,
    });

    const bundleRoot = vscode.Uri.joinPath(targetFolder, bundle.folderName);
    await vscode.workspace.fs.createDirectory(bundleRoot);
    for (const file of bundle.files) {
        const segments = file.relativePath.split('/');
        const fileUri = vscode.Uri.joinPath(bundleRoot, ...segments);
        if (segments.length > 1) {
            await vscode.workspace.fs.createDirectory(
                vscode.Uri.joinPath(bundleRoot, ...segments.slice(0, -1))
            );
        }
        await vscode.workspace.fs.writeFile(fileUri, textEncoder.encode(file.contents));
    }

    outputChannel.appendLine(`[Observability] Incident bundle exported to ${bundleRoot.fsPath}`);
    void vscode.window.showInformationMessage(
        `Observability incident bundle exported to ${bundleRoot.fsPath}`
    );
}

async function buildCurrentIncidentBundleBuildIdentity(
    context: vscode.ExtensionContext
): Promise<ObservabilityIncidentBundleBuildIdentity | undefined> {
    const extensionHost = (context as { extension?: { packageJSON?: any; id?: string } }).extension;
    const extensionIdentity =
        extensionHost?.packageJSON || context.extensionPath
            ? {
                  display_name:
                      typeof extensionHost?.packageJSON?.displayName === 'string'
                          ? extensionHost.packageJSON.displayName
                          : undefined,
                  version:
                      typeof extensionHost?.packageJSON?.version === 'string'
                          ? extensionHost.packageJSON.version
                          : undefined,
                  id: typeof extensionHost?.id === 'string' ? extensionHost.id : undefined,
                  extension_path: context.extensionPath || undefined,
              }
            : undefined;

    const client = getLanguageClient();
    const serverInfo = (client as any)?.initializeResult?.serverInfo;
    const launchInfo = getActiveServerLaunchInfo();
    let binaryMtimeIso: string | undefined;
    let binarySizeBytes: number | undefined;
    if (launchInfo?.serverPath) {
        try {
            const stat = await fs.stat(launchInfo.serverPath);
            binaryMtimeIso = stat.mtime.toISOString();
            binarySizeBytes = stat.size;
        } catch {
            // Ignore missing or transient binary paths; serverInfo.version still identifies the build.
        }
    }
    const lspServerIdentity =
        serverInfo || launchInfo?.serverPath || launchInfo?.serverMode
            ? {
                  name: typeof serverInfo?.name === 'string' ? serverInfo.name : undefined,
                  version: typeof serverInfo?.version === 'string' ? serverInfo.version : undefined,
                  server_mode: launchInfo?.serverMode,
                  binary_path: launchInfo?.serverPath,
                  binary_mtime_iso: binaryMtimeIso,
                  binary_size_bytes: binarySizeBytes,
              }
            : undefined;

    if (!extensionIdentity && !lspServerIdentity) {
        return undefined;
    }

    return {
        extension: extensionIdentity,
        lsp_server: lspServerIdentity,
    };
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

    safeRegisterCommand('bslAnalyzer.exportObservabilityIncidentBundle', async (...args: unknown[]) => {
        const targetFolder = await vscode.window.showOpenDialog({
            canSelectFiles: false,
            canSelectFolders: true,
            canSelectMany: false,
            openLabel: 'Export incident bundle here',
        });
        if (!targetFolder || targetFolder.length === 0) {
            return;
        }

        try {
            await exportObservabilityIncidentBundleToFolder(
                targetFolder[0],
                context,
                outputChannel,
                asExportCapture(args[0]) ?? {}
            );
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            outputChannel.appendLine(`[Observability] Incident bundle export failed: ${message}`);
            void vscode.window.showErrorMessage(`Failed to export observability incident bundle: ${message}`);
        }
    });
}
