import * as vscode from 'vscode';
import {
    LanguageClientOptions,
    RevealOutputChannelOn,
    Trace
} from 'vscode-languageclient/node';
import { BslAnalyzerConfig } from '../../config/configHelper';
import {
    CompletionProbeRecorder,
    getSharedCompletionProbeRecorder,
} from '../../providers/completionProbeRecorder';
import { primeExactTypeIndex, SnapshotStatusResponse } from '../customRequests';
import { getSnapshotStatusForUri, onSnapshotStatusChange } from '../snapshotStatus';

const HOVER_COLD_RETRY_WAIT_MS = 12_000;

export function resolveRulesConfigForInitialization(): string {
    const configuredPath = BslAnalyzerConfig.rulesConfig.trim();
    if (configuredPath) {
        return configuredPath;
    }

    const firstFolder = vscode.workspace.workspaceFolders?.[0];
    if (!firstFolder) {
        return '';
    }

    const rulesUri = vscode.Uri.joinPath(firstFolder.uri, 'bsl-rules.toml');
    return rulesUri.scheme === 'file' ? rulesUri.fsPath : rulesUri.toString();
}

function hoverHasVisibleContent(result: vscode.Hover | null | undefined): boolean {
    if (!result) {
        return false;
    }
    const contents = Array.isArray(result.contents) ? result.contents : [result.contents];
    return contents.some((content) => {
        if (typeof content === 'string') {
            return content.trim().length > 0;
        }
        if (content instanceof vscode.MarkdownString) {
            return content.value.trim().length > 0;
        }
        if ('value' in content && typeof content.value === 'string') {
            return content.value.trim().length > 0;
        }
        return true;
    });
}

function snapshotStatusNeedsColdHoverRetry(
    status: SnapshotStatusResponse | undefined,
    documentVersion: number
): boolean {
    if (!status) {
        return false;
    }
    if (
        typeof status.requestedVersion === 'number' &&
        status.requestedVersion !== documentVersion
    ) {
        return false;
    }
    return (
        status.taskState === 'in_flight_same_revision' ||
        status.state === 'building' ||
        status.state === 'shadow_only' ||
        status.state === 'stale'
    );
}

function snapshotStatusReadyForRetry(
    status: SnapshotStatusResponse | undefined,
    documentVersion: number
): boolean {
    if (!status) {
        return false;
    }
    return (
        status.state === 'ready' &&
        status.taskState === 'ready_same_revision' &&
        (typeof status.requestedVersion !== 'number' ||
            status.requestedVersion === documentVersion)
    );
}

async function waitForColdHoverRetrySnapshot(
    document: vscode.TextDocument,
    token: vscode.CancellationToken
): Promise<SnapshotStatusResponse | null> {
    const uri = document.uri.toString();
    const initialStatus = getSnapshotStatusForUri(uri);
    if (!snapshotStatusNeedsColdHoverRetry(initialStatus, document.version)) {
        return null;
    }

    return new Promise((resolve) => {
        let settled = false;
        let timeoutHandle: NodeJS.Timeout | undefined;
        let snapshotSubscription: vscode.Disposable | undefined;
        let cancellationSubscription: vscode.Disposable | undefined;

        const finish = (status: SnapshotStatusResponse | null): void => {
            if (settled) {
                return;
            }
            settled = true;
            if (timeoutHandle) {
                clearTimeout(timeoutHandle);
            }
            snapshotSubscription?.dispose();
            cancellationSubscription?.dispose();
            resolve(status);
        };

        const pollStatus = (): void => {
            const currentStatus = getSnapshotStatusForUri(uri);
            if (snapshotStatusReadyForRetry(currentStatus, document.version)) {
                finish(currentStatus ?? null);
                return;
            }
            if (!snapshotStatusNeedsColdHoverRetry(currentStatus, document.version)) {
                finish(null);
            }
        };

        snapshotSubscription = onSnapshotStatusChange(() => {
            pollStatus();
        });
        cancellationSubscription = token.onCancellationRequested(() => {
            finish(null);
        });
        timeoutHandle = setTimeout(() => {
            finish(null);
        }, HOVER_COLD_RETRY_WAIT_MS);
        pollStatus();
    });
}

async function maybePrimeExactIndexForHoverRetry(
    document: vscode.TextDocument,
    status: SnapshotStatusResponse
): Promise<void> {
    if (typeof status.requestedVersion !== 'number') {
        return;
    }
    try {
        await primeExactTypeIndex({
            uri: document.uri.toString(),
            requestedVersion: status.requestedVersion,
            reason: 'hover_cold_snapshot_retry',
        });
    } catch {
        // Best-effort only. The second hover must still run even if warmup fails.
    }
}

/**
 * Строит LanguageClientOptions для LSP клиента
 * @param outputChannel Канал для логирования
 */
export function buildClientOptions(
    outputChannel: vscode.OutputChannel,
    completionProbeRecorder: CompletionProbeRecorder = getSharedCompletionProbeRecorder()
): LanguageClientOptions {
    const typeHintsEnabled = vscode.workspace
        .getConfiguration('bsl.typeHints')
        .get<boolean>('enabled', false);
    const codeActionsEnabled = vscode.workspace
        .getConfiguration('bsl.codeActions')
        .get<boolean>('enabled', false);

    // MILESTONE 2.10: Подготавливаем initializationOptions для передачи в LSP
    const initializationOptions = {
        platformDocsArchive: BslAnalyzerConfig.platformDocsArchive,
        configurationPath: BslAnalyzerConfig.configurationPath,
        rulesConfig: resolveRulesConfigForInitialization(),
        platformVersion: BslAnalyzerConfig.platformVersion,
        cacheEnabled: BslAnalyzerConfig.cacheEnabled,
        enableTypeHints: typeHintsEnabled,
        enableCodeActions: codeActionsEnabled
    };

    outputChannel.appendLine(`Sending initializationOptions to LSP:`);
    outputChannel.appendLine(`   platformDocsArchive: ${initializationOptions.platformDocsArchive || 'NOT SET'}`);
    outputChannel.appendLine(`   configurationPath: ${initializationOptions.configurationPath || 'NOT SET'}`);
    outputChannel.appendLine(`   rulesConfig: ${initializationOptions.rulesConfig || 'NOT SET'}`);
    outputChannel.appendLine(`   platformVersion: ${initializationOptions.platformVersion || 'NOT SET'}`);
    outputChannel.appendLine(`   cacheEnabled: ${initializationOptions.cacheEnabled}`);
    outputChannel.appendLine(`   enableTypeHints: ${initializationOptions.enableTypeHints}`);
    outputChannel.appendLine(`   enableCodeActions: ${initializationOptions.enableCodeActions}`);

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'bsl' },
            { scheme: 'untitled', language: 'bsl' }
        ],
        synchronize: {
            fileEvents: [
                vscode.workspace.createFileSystemWatcher('**/*.bsl'),
                vscode.workspace.createFileSystemWatcher('**/*.os'),
                vscode.workspace.createFileSystemWatcher('**/Configuration.xml')
            ],
            // MILESTONE 3.6: Синхронизируем ОБЕ секции настроек (bslAnalyzer + bsl)
            configurationSection: ['bslAnalyzer', 'bsl']
        },
        // MILESTONE 2.10: Передаём initializationOptions в LSP
        initializationOptions: initializationOptions,
        outputChannel: outputChannel,
        revealOutputChannelOn: RevealOutputChannelOn.Never,
        traceOutputChannel: outputChannel,
        middleware: {
            // Перехватываем workspace-related notifications
            workspace: {
                configuration: (params, token, next) => {
                    outputChannel.appendLine(`Configuration request: ${JSON.stringify(params)}`);
                    return next(params, token);
                }
            },
            didChange: async (event, next) => {
                completionProbeRecorder.recordTextDocumentDidChange(event);
                await next(event);
                completionProbeRecorder.recordTextDocumentDidChangeSent(event.document);
            },
            didClose: async (document, next) => {
                await next(document);
                completionProbeRecorder.recordTextDocumentDidClose(document);
            },
            provideCompletionItem: async (document, position, context, token, next) => {
                const requestStartedAtMs = Date.now();
                completionProbeRecorder.recordCompletionStarted({
                    document,
                    position,
                    context,
                    token,
                    requestStartedAtMs,
                });

                try {
                    const result = await next(document, position, context, token);
                    completionProbeRecorder.recordCompletionOutcome({
                        document,
                        position,
                        context,
                        token,
                        result,
                        requestStartedAtMs,
                        requestCompletedAtMs: Date.now(),
                        wasCancelled: token.isCancellationRequested,
                    });
                    return result;
                } catch (error) {
                    completionProbeRecorder.recordCompletionOutcome({
                        document,
                        position,
                        context,
                        token,
                        result: undefined,
                        requestStartedAtMs,
                        requestCompletedAtMs: Date.now(),
                        wasCancelled: token.isCancellationRequested,
                        error,
                    });
                    throw error;
                }
            },
            provideHover: async (document, position, token, next) => {
                const firstResult = await next(document, position, token);
                if (hoverHasVisibleContent(firstResult) || token.isCancellationRequested) {
                    return firstResult;
                }

                const readyStatus = await waitForColdHoverRetrySnapshot(document, token);
                if (!readyStatus || token.isCancellationRequested) {
                    return firstResult;
                }

                await maybePrimeExactIndexForHoverRetry(document, readyStatus);
                return next(document, position, token);
            },
        }
    };

    // ПРИНУДИТЕЛЬНО устанавливаем VERBOSE tracing для отладки Work Done Progress
    (clientOptions as any).trace = Trace.Verbose;
    outputChannel.appendLine('TRACE: Verbose logging enabled');

    return clientOptions;
}
