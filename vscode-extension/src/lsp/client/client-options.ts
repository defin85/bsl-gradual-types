import * as vscode from 'vscode';
import {
    LanguageClientOptions,
    RevealOutputChannelOn,
    Trace
} from 'vscode-languageclient/node';
import { BslAnalyzerConfig } from '../../config/configHelper';

/**
 * Строит LanguageClientOptions для LSP клиента
 * @param outputChannel Канал для логирования
 */
export function buildClientOptions(
    outputChannel: vscode.OutputChannel
): LanguageClientOptions {
    // MILESTONE 2.10: Подготавливаем initializationOptions для передачи в LSP
    const initializationOptions = {
        platformDocsArchive: BslAnalyzerConfig.platformDocsArchive,
        configurationPath: BslAnalyzerConfig.configurationPath,
        platformVersion: BslAnalyzerConfig.platformVersion
    };

    outputChannel.appendLine(`Sending initializationOptions to LSP:`);
    outputChannel.appendLine(`   platformDocsArchive: ${initializationOptions.platformDocsArchive || 'NOT SET'}`);
    outputChannel.appendLine(`   configurationPath: ${initializationOptions.configurationPath || 'NOT SET'}`);
    outputChannel.appendLine(`   platformVersion: ${initializationOptions.platformVersion || 'NOT SET'}`);

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
            }
        }
    };

    // ПРИНУДИТЕЛЬНО устанавливаем VERBOSE tracing для отладки Work Done Progress
    (clientOptions as any).trace = Trace.Verbose;
    outputChannel.appendLine('TRACE: Verbose logging enabled');

    return clientOptions;
}
