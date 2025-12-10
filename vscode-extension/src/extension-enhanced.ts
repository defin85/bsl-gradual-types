/**
 * BSL Gradual Type System - Enhanced VSCode Extension
 *
 * Интегрирует VSCode с enhanced LSP сервером, предоставляя:
 * - Flow-sensitive type analysis
 * - Union types с инкрементальным парсингом
 * - Code actions и type hints
 * - Performance profiling integration
 */

import * as vscode from 'vscode';

// Enhanced imports из setup модулей
import { EnhancedLspClient } from './lsp/enhanced-client';
import { PerformanceMonitor } from './utils/performance-monitor';

import {
    loadConfiguration,
    initializeEnhancedLsp,
    registerEnhancedProviders,
    registerEnhancedCommands,
    createStatusBarItem,
    showWelcomeMessage
} from './setup';

// Re-export package contributions
export { getEnhancedPackageContributions } from './setup';

// Глобальные переменные
let languageClient: EnhancedLspClient | null = null;
let outputChannel: vscode.OutputChannel;
let statusBarItem: vscode.StatusBarItem;
let performanceMonitor: PerformanceMonitor;

/**
 * Активация расширения
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
    // Создаем output channel
    outputChannel = vscode.window.createOutputChannel('BSL Gradual Types');
    context.subscriptions.push(outputChannel);

    // Создаем status bar item
    statusBarItem = createStatusBarItem(context);

    // Инициализируем performance monitor
    performanceMonitor = new PerformanceMonitor(outputChannel);

    try {
        outputChannel.appendLine('Activating BSL Gradual Type System...');

        // Загружаем конфигурацию
        await loadConfiguration(outputChannel);

        // Инициализируем enhanced LSP клиент
        languageClient = await initializeEnhancedLsp(context, outputChannel, performanceMonitor);

        // Регистрируем providers
        await registerEnhancedProviders(context, languageClient, outputChannel);

        // Регистрируем команды
        registerEnhancedCommands(context, languageClient, outputChannel, statusBarItem);

        // Устанавливаем final status
        statusBarItem.text = "$(check) BSL: Ready";
        statusBarItem.tooltip = "BSL Gradual Type System active";

        outputChannel.appendLine('BSL Gradual Type System activated successfully!');

        // Показываем welcome message при первом запуске
        await showWelcomeMessage();

    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Activation failed: ${errorMessage}`);

        statusBarItem.text = "$(error) BSL: Error";
        statusBarItem.tooltip = `Error: ${errorMessage}`;

        vscode.window.showErrorMessage(
            `BSL Gradual Type System activation failed: ${errorMessage}`
        );
    }
}

/**
 * Деактивация расширения
 */
export async function deactivate(): Promise<void> {
    outputChannel.appendLine('Deactivating BSL Gradual Type System...');

    try {
        // Останавливаем LSP клиент
        if (languageClient) {
            await languageClient.stop();
            languageClient = null;
        }

        // Cleanup performance monitor
        performanceMonitor?.dispose();

        outputChannel.appendLine('BSL Gradual Type System deactivated successfully');

    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Deactivation warning: ${errorMessage}`);
    }
}
