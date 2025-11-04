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
exports.registerParseConfigurationCommand = void 0;
const vscode = __importStar(require("vscode"));
function registerParseConfigurationCommand(context, client) {
    return vscode.commands.registerCommand('bslAnalyzer.parseConfiguration', async () => {
        // Открываем диалог выбора папки
        const folderUri = await vscode.window.showOpenDialog({
            canSelectFiles: false,
            canSelectFolders: true,
            canSelectMany: false,
            openLabel: 'Выбрать папку конфигурации',
            title: 'Выберите папку конфигурации (содержащую Configuration.xml)'
        });
        if (!folderUri || folderUri.length === 0) {
            return; // Пользователь отменил
        }
        const configPath = folderUri[0].fsPath;
        // Сохраняем путь в настройках
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        await config.update('configurationPath', configPath, vscode.ConfigurationTarget.Workspace);
        try {
            // ✅ ИСПРАВЛЕНО (2025-01-18): Правильный вызов LSP команды
            // Используем ExecuteCommandRequest с правильными параметрами
            const result = await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Парсинг конфигурации',
                cancellable: false
            }, async () => {
                // workspace/executeCommand требует ExecuteCommandParams
                // arguments: any[] - массив аргументов команды
                // LSP Server ожидает ParseConfigurationParams { config_path: string }
                return await client.sendRequest('workspace/executeCommand', {
                    command: 'bsl.parseConfiguration',
                    arguments: [{ configPath: configPath }] // ✅ Соответствует camelCase в LSP
                });
            });
            const response = result;
            if (response.success) {
                vscode.window.showInformationMessage(`✅ Типы конфигурации успешно загружены: ${response.loadedTypes} типов`);
            }
            else {
                vscode.window.showErrorMessage(`❌ Ошибка загрузки конфигурации: ${response.message || 'Unknown error'}`);
            }
        }
        catch (error) {
            vscode.window.showErrorMessage(`❌ Ошибка парсинга конфигурации: ${error}`);
        }
    });
}
exports.registerParseConfigurationCommand = registerParseConfigurationCommand;
//# sourceMappingURL=parseConfiguration.js.map