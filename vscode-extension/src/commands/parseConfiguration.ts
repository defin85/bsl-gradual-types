import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

export function registerParseConfigurationCommand(
    context: vscode.ExtensionContext,
    client: LanguageClient
): vscode.Disposable {
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
            const result = await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Парсинг конфигурации',
                    cancellable: false
                },
                async () => {
                    // workspace/executeCommand требует ExecuteCommandParams
                    // arguments: any[] - массив аргументов команды
                    // LSP Server ожидает ParseConfigurationParams { config_path: string }
                    return await client.sendRequest('workspace/executeCommand', {
                        command: 'bsl.parseConfiguration',
                        arguments: [{ configPath: configPath }] // ✅ Соответствует camelCase в LSP
                    });
                }
            );

            // Проверяем результат (типизация для безопасности)
            interface ParseConfigurationResponse {
                success: boolean;
                loadedTypes: number;
                message?: string;
            }
            const response = result as ParseConfigurationResponse;

            if (response.success) {
                vscode.window.showInformationMessage(
                    `✅ Типы конфигурации успешно загружены: ${response.loadedTypes} типов`
                );
            } else {
                vscode.window.showErrorMessage(
                    `❌ Ошибка загрузки конфигурации: ${response.message || 'Unknown error'}`
                );
            }
        } catch (error) {
            vscode.window.showErrorMessage(
                `❌ Ошибка парсинга конфигурации: ${error}`
            );
        }
    });
}
