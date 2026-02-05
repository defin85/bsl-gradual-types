import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { getLanguageClient } from '../lsp';
import { BslAnalyzerConfig } from '../config/configHelper';

let cacheSettingUpdateInProgress = false;

async function sendCacheEnabled(enabled: boolean, outputChannel: vscode.OutputChannel) {
    const client = getLanguageClient();
    if (!client) {
        vscode.window.showWarningMessage('BSL Analyzer: LSP client not ready');
        return;
    }

    try {
        const result: any = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.cache.setEnabled',
            arguments: [{ enabled }]
        });
        outputChannel.appendLine(`[Cache] setEnabled: ${JSON.stringify(result)}`);
        if (result && typeof result.effective === 'boolean' && result.effective !== enabled) {
            const reason =
                result && typeof result.env_disabled === 'boolean' && result.env_disabled
                    ? 'BSL_CACHE_DISABLE=true (env или bsl.envOverrides)'
                    : 'конфигурация/политика сервера';
            vscode.window.showWarningMessage(
                `BSL Analyzer: не удалось применить cacheEnabled=${enabled} (effective=${result.effective}). Причина: ${reason}.`
            );
        }
    } catch (error) {
        outputChannel.appendLine(`[Cache] setEnabled failed: ${error}`);
        vscode.window.showErrorMessage('BSL Analyzer: не удалось обновить режим кэша');
    }
}

async function updateCacheSetting(enabled: boolean) {
    const config = vscode.workspace.getConfiguration('bslAnalyzer');
    await config.update('cacheEnabled', enabled, vscode.ConfigurationTarget.Workspace);
}

function resolveConfigurationPath(): string | null {
    const configPath = BslAnalyzerConfig.configurationPath;
    if (!configPath) {
        return null;
    }
    return configPath;
}

async function requestCacheCommand(
    command: string,
    outputChannel: vscode.OutputChannel
) {
    const client = getLanguageClient();
    if (!client) {
        vscode.window.showWarningMessage('BSL Analyzer: LSP client not ready');
        return null;
    }

    const configPath = resolveConfigurationPath();
    if (!configPath) {
        vscode.window.showWarningMessage('BSL Analyzer: configurationPath not set');
        return null;
    }

    outputChannel.appendLine(`[Cache] Request: ${command}`);
    return await client.sendRequest('workspace/executeCommand', {
        command,
        arguments: [{ configurationPath: configPath }]
    });
}

export function registerCacheCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (
        commandId: string,
        callback: CommandHandler
    ) => Promise<vscode.Disposable | null>,
    outputChannel: vscode.OutputChannel
) {
    void safeRegisterCommand('bslAnalyzer.cacheStats', async () => {
        try {
            const result = await requestCacheCommand('bsl.cache.getStats', outputChannel);
            if (!result) {
                return;
            }
            outputChannel.appendLine(`[Cache] Stats: ${JSON.stringify(result, null, 2)}`);
            vscode.window.showInformationMessage('BSL Analyzer: cache stats получены (см. Output)');
        } catch (error) {
            outputChannel.appendLine(`[Cache] Stats failed: ${error}`);
            vscode.window.showErrorMessage('BSL Analyzer: не удалось получить cache stats');
        }
    });

    void safeRegisterCommand('bslAnalyzer.clearCache', async () => {
        try {
            const result: any = await requestCacheCommand('bsl.cache.clear', outputChannel);
            if (!result) {
                return;
            }
            outputChannel.appendLine(`[Cache] Clear: ${JSON.stringify(result, null, 2)}`);
            const freedBytes = result?.disk?.freed_bytes ?? result?.disk?.freedBytes ?? 0;
            vscode.window.showInformationMessage(
                `BSL Analyzer: cache очищен (freed ${freedBytes} bytes).`
            );
            vscode.commands.executeCommand('bslAnalyzer.refreshCacheDashboard');
        } catch (error) {
            outputChannel.appendLine(`[Cache] Clear failed: ${error}`);
            vscode.window.showErrorMessage('BSL Analyzer: не удалось очистить cache');
        }
    });

    void safeRegisterCommand('bslAnalyzer.toggleCache', async () => {
        const enabled = !BslAnalyzerConfig.cacheEnabled;
        cacheSettingUpdateInProgress = true;
        await updateCacheSetting(enabled);
        await sendCacheEnabled(enabled, outputChannel);
        cacheSettingUpdateInProgress = false;
        vscode.window.showInformationMessage(
            `BSL Analyzer: cache ${enabled ? 'enabled' : 'disabled'} (workspace)`
        );
        vscode.commands.executeCommand('bslAnalyzer.refreshCacheDashboard');
    });

    const disposable = vscode.workspace.onDidChangeConfiguration((e) => {
        if (!e.affectsConfiguration('bslAnalyzer.cacheEnabled')) {
            return;
        }
        if (cacheSettingUpdateInProgress) {
            return;
        }
        void sendCacheEnabled(BslAnalyzerConfig.cacheEnabled, outputChannel);
    });
    context.subscriptions.push(disposable);
}
