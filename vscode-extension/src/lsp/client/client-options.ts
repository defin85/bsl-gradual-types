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
        platformVersion: BslAnalyzerConfig.platformVersion,
        cacheEnabled: BslAnalyzerConfig.cacheEnabled,
        enableTypeHints: typeHintsEnabled,
        enableCodeActions: codeActionsEnabled
    };

    outputChannel.appendLine(`Sending initializationOptions to LSP:`);
    outputChannel.appendLine(`   platformDocsArchive: ${initializationOptions.platformDocsArchive || 'NOT SET'}`);
    outputChannel.appendLine(`   configurationPath: ${initializationOptions.configurationPath || 'NOT SET'}`);
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

                try {
                    const result = await next(document, position, context, token);
                    completionProbeRecorder.recordCompletionOutcome({
                        document,
                        position,
                        context,
                        result,
                        requestStartedAtMs,
                        requestCompletedAtMs: Date.now(),
                        wasCancelled: false,
                    });
                    return result;
                } catch (error) {
                    completionProbeRecorder.recordCompletionOutcome({
                        document,
                        position,
                        context,
                        result: undefined,
                        requestStartedAtMs,
                        requestCompletedAtMs: Date.now(),
                        wasCancelled: token.isCancellationRequested,
                    });
                    throw error;
                }
            },
        }
    };

    // ПРИНУДИТЕЛЬНО устанавливаем VERBOSE tracing для отладки Work Done Progress
    (clientOptions as any).trace = Trace.Verbose;
    outputChannel.appendLine('TRACE: Verbose logging enabled');

    return clientOptions;
}
