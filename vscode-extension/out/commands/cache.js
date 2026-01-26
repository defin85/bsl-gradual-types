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
exports.registerCacheCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const configHelper_1 = require("../config/configHelper");
let cacheSettingUpdateInProgress = false;
async function sendCacheEnabled(enabled, outputChannel) {
    const client = (0, lsp_1.getLanguageClient)();
    if (!client) {
        vscode.window.showWarningMessage('BSL Analyzer: LSP client not ready');
        return;
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.cache.setEnabled',
            arguments: [{ enabled }]
        });
        outputChannel.appendLine(`[Cache] setEnabled: ${JSON.stringify(result)}`);
        if (result && typeof result.effective === 'boolean' && result.effective !== enabled) {
            vscode.window.showWarningMessage('BSL Analyzer: кэш отключен через ENV (BSL_CACHE_DISABLE=1), настройка workspace проигнорирована.');
        }
    }
    catch (error) {
        outputChannel.appendLine(`[Cache] setEnabled failed: ${error}`);
        vscode.window.showErrorMessage('BSL Analyzer: не удалось обновить режим кэша');
    }
}
async function updateCacheSetting(enabled) {
    const config = vscode.workspace.getConfiguration('bslAnalyzer');
    await config.update('cacheEnabled', enabled, vscode.ConfigurationTarget.Workspace);
}
function resolveConfigurationPath() {
    const configPath = configHelper_1.BslAnalyzerConfig.configurationPath;
    if (!configPath) {
        return null;
    }
    return configPath;
}
async function requestCacheCommand(command, outputChannel) {
    const client = (0, lsp_1.getLanguageClient)();
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
function registerCacheCommands(context, safeRegisterCommand, outputChannel) {
    void safeRegisterCommand('bslAnalyzer.cacheStats', async () => {
        try {
            const result = await requestCacheCommand('bsl.cache.getStats', outputChannel);
            if (!result) {
                return;
            }
            outputChannel.appendLine(`[Cache] Stats: ${JSON.stringify(result, null, 2)}`);
            vscode.window.showInformationMessage('BSL Analyzer: cache stats получены (см. Output)');
        }
        catch (error) {
            outputChannel.appendLine(`[Cache] Stats failed: ${error}`);
            vscode.window.showErrorMessage('BSL Analyzer: не удалось получить cache stats');
        }
    });
    void safeRegisterCommand('bslAnalyzer.clearCache', async () => {
        try {
            const result = await requestCacheCommand('bsl.cache.clear', outputChannel);
            if (!result) {
                return;
            }
            outputChannel.appendLine(`[Cache] Clear: ${JSON.stringify(result, null, 2)}`);
            const freedBytes = result?.disk?.freed_bytes ?? result?.disk?.freedBytes ?? 0;
            vscode.window.showInformationMessage(`BSL Analyzer: cache очищен (freed ${freedBytes} bytes).`);
            vscode.commands.executeCommand('bslAnalyzer.refreshCacheDashboard');
        }
        catch (error) {
            outputChannel.appendLine(`[Cache] Clear failed: ${error}`);
            vscode.window.showErrorMessage('BSL Analyzer: не удалось очистить cache');
        }
    });
    void safeRegisterCommand('bslAnalyzer.toggleCache', async () => {
        const enabled = !configHelper_1.BslAnalyzerConfig.cacheEnabled;
        cacheSettingUpdateInProgress = true;
        await updateCacheSetting(enabled);
        await sendCacheEnabled(enabled, outputChannel);
        cacheSettingUpdateInProgress = false;
        vscode.window.showInformationMessage(`BSL Analyzer: cache ${enabled ? 'enabled' : 'disabled'} (workspace)`);
        vscode.commands.executeCommand('bslAnalyzer.refreshCacheDashboard');
    });
    const disposable = vscode.workspace.onDidChangeConfiguration((e) => {
        if (!e.affectsConfiguration('bslAnalyzer.cacheEnabled')) {
            return;
        }
        if (cacheSettingUpdateInProgress) {
            return;
        }
        void sendCacheEnabled(configHelper_1.BslAnalyzerConfig.cacheEnabled, outputChannel);
    });
    context.subscriptions.push(disposable);
}
exports.registerCacheCommands = registerCacheCommands;
//# sourceMappingURL=cache.js.map