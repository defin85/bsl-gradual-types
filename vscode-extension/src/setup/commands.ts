/**
 * Commands Setup Module
 *
 * Регистрация команд расширения
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
import {
    generateTypeInfoHtml,
    showFeaturesOverview,
    showProjectAnalysisResults
} from './ui';

/**
 * Регистрация всех enhanced команд
 */
export function registerEnhancedCommands(
    context: vscode.ExtensionContext,
    languageClient: EnhancedLspClient,
    outputChannel: vscode.OutputChannel,
    statusBarItem: vscode.StatusBarItem
): void {
    outputChannel.appendLine('Registering enhanced commands...');

    // Команда для показа type информации
    context.subscriptions.push(
        vscode.commands.registerCommand('bsl.showTypeInfo', async () => {
            await showTypeInfoAtCursor(languageClient, outputChannel);
        })
    );

    // Команда для запуска performance profiling
    context.subscriptions.push(
        vscode.commands.registerCommand('bsl.runPerformanceProfiling', async () => {
            await runPerformanceProfiling(languageClient, outputChannel, statusBarItem);
        })
    );

    // Команда для анализа проекта
    context.subscriptions.push(
        vscode.commands.registerCommand('bsl.analyzeProject', async () => {
            await analyzeCurrentProject(languageClient, outputChannel, statusBarItem);
        })
    );

    // Команда для показа type hints настроек
    context.subscriptions.push(
        vscode.commands.registerCommand('bsl.configureTypeHints', async () => {
            await configureTypeHints();
        })
    );

    // Команда для cache управления
    context.subscriptions.push(
        vscode.commands.registerCommand('bsl.clearCache', async () => {
            await clearCache(languageClient, outputChannel);
        })
    );

    outputChannel.appendLine('Enhanced commands registered');
}

/**
 * Показать type информацию под курсором
 */
async function showTypeInfoAtCursor(
    languageClient: EnhancedLspClient,
    outputChannel: vscode.OutputChannel
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || !languageClient) {
        return;
    }

    const position = editor.selection.active;
    const document = editor.document;

    try {
        // Запрашиваем enhanced hover информацию
        const hover = await languageClient.getEnhancedHover(document.uri.toString(), position);

        if (hover) {
            // Показываем в webview panel для rich content
            const panel = vscode.window.createWebviewPanel(
                'bslTypeInfo',
                'BSL Type Information',
                vscode.ViewColumn.Beside,
                { enableScripts: true }
            );

            panel.webview.html = generateTypeInfoHtml(hover);
        } else {
            vscode.window.showInformationMessage('No type information available at cursor');
        }
    } catch (error) {
        outputChannel.appendLine(`Error getting type info: ${error}`);
        vscode.window.showErrorMessage('Failed to get type information');
    }
}

/**
 * Запуск performance profiling текущего файла
 */
async function runPerformanceProfiling(
    languageClient: EnhancedLspClient,
    outputChannel: vscode.OutputChannel,
    statusBarItem: vscode.StatusBarItem
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No active BSL file');
        return;
    }

    const filePath = editor.document.uri.fsPath;

    try {
        statusBarItem.text = "$(loading~spin) BSL: Profiling...";

        // Запускаем profiler через наш LSP сервер
        const result = await languageClient?.requestPerformanceProfiling(filePath);

        if (result) {
            // Показываем результаты в output channel
            outputChannel.show();
            outputChannel.appendLine('Performance Profiling Results:');
            outputChannel.appendLine(result.humanReadableReport);

            vscode.window.showInformationMessage(
                `Performance profiling completed. Check output for details.`,
                'Show Output'
            ).then(selection => {
                if (selection === 'Show Output') {
                    outputChannel.show();
                }
            });
        }

        statusBarItem.text = "$(check) BSL: Ready";

    } catch (error) {
        statusBarItem.text = "$(error) BSL: Error";
        outputChannel.appendLine(`Profiling error: ${error}`);
        vscode.window.showErrorMessage('Performance profiling failed');
    }
}

/**
 * Анализ текущего проекта
 */
async function analyzeCurrentProject(
    languageClient: EnhancedLspClient,
    outputChannel: vscode.OutputChannel,
    statusBarItem: vscode.StatusBarItem
): Promise<void> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showWarningMessage('No workspace folder open');
        return;
    }

    const projectPath = workspaceFolder.uri.fsPath;

    try {
        statusBarItem.text = "$(loading~spin) BSL: Analyzing project...";

        // Показываем progress notification
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "Analyzing BSL project",
            cancellable: true
        }, async (progress, _token) => {
            progress.report({ increment: 0, message: "Starting analysis..." });

            const result = await languageClient?.requestProjectAnalysis(projectPath, {
                useParallelAnalysis: true,
                enableCaching: true,
                showProgress: true
            });

            if (result) {
                progress.report({ increment: 100, message: "Analysis completed" });

                // Показываем результаты
                const message = `Analysis completed!\n` +
                    `Files: ${result.stats.totalFiles}\n` +
                    `Functions: ${result.stats.totalFunctions}\n` +
                    `Diagnostics: ${result.stats.totalDiagnostics}`;

                vscode.window.showInformationMessage(message, 'Show Details').then(selection => {
                    if (selection === 'Show Details') {
                        showProjectAnalysisResults(result);
                    }
                });
            }
        });

        statusBarItem.text = "$(check) BSL: Ready";

    } catch (error) {
        statusBarItem.text = "$(error) BSL: Error";
        outputChannel.appendLine(`Project analysis error: ${error}`);
        vscode.window.showErrorMessage('Project analysis failed');
    }
}

/**
 * Настройка type hints
 */
async function configureTypeHints(): Promise<void> {
    const config = vscode.workspace.getConfiguration('bsl.typeHints');

    const options = [
        { label: 'Show variable types', setting: 'showVariableTypes' },
        { label: 'Show return types', setting: 'showReturnTypes' },
        { label: 'Show union details', setting: 'showUnionDetails' },
        { label: 'Show parameter types', setting: 'showParameterTypes' }
    ];

    const quickPick = vscode.window.createQuickPick();
    quickPick.items = options.map(opt => ({
        label: opt.label,
        description: config.get(opt.setting) ? 'Enabled' : 'Disabled',
        detail: opt.setting
    }));
    quickPick.canSelectMany = true;
    quickPick.title = 'Configure Type Hints';

    quickPick.onDidAccept(() => {
        const selected = quickPick.selectedItems;

        options.forEach(opt => {
            const isSelected = selected.some(item => item.detail === opt.setting);
            config.update(opt.setting, isSelected, vscode.ConfigurationTarget.Global);
        });

        vscode.window.showInformationMessage('Type hints configuration updated');
        quickPick.hide();
    });

    quickPick.show();
}

/**
 * Очистка cache анализа
 */
async function clearCache(
    languageClient: EnhancedLspClient,
    outputChannel: vscode.OutputChannel
): Promise<void> {
    try {
        const result = await languageClient?.requestCacheClear();

        if (result?.success) {
            vscode.window.showInformationMessage(
                `Cache cleared. Freed ${result.freedBytes} bytes.`
            );
        } else {
            vscode.window.showWarningMessage('Failed to clear cache');
        }

    } catch (error) {
        outputChannel.appendLine(`Cache clear error: ${error}`);
        vscode.window.showErrorMessage('Failed to clear cache');
    }
}

/**
 * Конфигурация для package.json contributions
 */
export function getEnhancedPackageContributions() {
    return {
        commands: [
            {
                command: "bsl.showTypeInfo",
                title: "Show Type Information",
                category: "BSL"
            },
            {
                command: "bsl.runPerformanceProfiling",
                title: "Run Performance Profiling",
                category: "BSL"
            },
            {
                command: "bsl.analyzeProject",
                title: "Analyze Project",
                category: "BSL"
            },
            {
                command: "bsl.configureTypeHints",
                title: "Configure Type Hints",
                category: "BSL"
            },
            {
                command: "bsl.clearCache",
                title: "Clear Cache",
                category: "BSL"
            }
        ],
        configuration: {
            type: "object",
            title: "BSL Gradual Type System",
            properties: {
                "bsl.typeHints.showVariableTypes": {
                    type: "boolean",
                    default: true,
                    description: "Show type hints for variables"
                },
                "bsl.typeHints.showReturnTypes": {
                    type: "boolean",
                    default: true,
                    description: "Show type hints for function return types"
                },
                "bsl.typeHints.showUnionDetails": {
                    type: "boolean",
                    default: true,
                    description: "Show detailed information for Union types"
                },
                "bsl.typeHints.minCertainty": {
                    type: "number",
                    default: 0.7,
                    minimum: 0.0,
                    maximum: 1.0,
                    description: "Minimum certainty level to show type hints"
                },
                "bsl.performance.enableProfiling": {
                    type: "boolean",
                    default: false,
                    description: "Enable automatic performance profiling"
                },
                "bsl.analysis.useParallelProcessing": {
                    type: "boolean",
                    default: true,
                    description: "Use parallel processing for project analysis"
                },
                "bsl.analysis.enableCaching": {
                    type: "boolean",
                    default: true,
                    description: "Enable caching of analysis results"
                }
            }
        }
    };
}
