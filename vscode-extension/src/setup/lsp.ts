/**
 * LSP Client Setup Module
 *
 * Инициализация и настройка Enhanced LSP клиента
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
import { BslAnalyzerConfig, migrateLegacySettings } from '../config/configHelper';
import { getBinaryPath } from '../utils/binaryPath';
import { PerformanceMonitor } from '../utils/performance-monitor';

/**
 * Загрузка и валидация конфигурации
 */
export async function loadConfiguration(outputChannel: vscode.OutputChannel): Promise<void> {
    outputChannel.appendLine('Loading configuration...');

    // Миграция legacy настроек если нужно
    await migrateLegacySettings();

    // Валидация конфигурации
    const config = BslAnalyzerConfig;
    if (!config.isValid()) {
        throw new Error('Invalid configuration detected');
    }

    outputChannel.appendLine(`Configuration loaded: ${JSON.stringify(config.summary(), null, 2)}`);
}

/**
 * Инициализация Enhanced LSP клиента
 */
export async function initializeEnhancedLsp(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
    performanceMonitor: PerformanceMonitor
): Promise<EnhancedLspClient> {
    outputChannel.appendLine('Initializing Enhanced LSP client...');

    // Получаем путь к enhanced LSP серверу
    const serverPath = getBinaryPath('lsp-server', context);

    if (!serverPath) {
        throw new Error('Enhanced LSP server binary not found');
    }

    outputChannel.appendLine(`Enhanced LSP server path: ${serverPath}`);

    // Создаем enhanced LSP клиент
    const languageClient = new EnhancedLspClient(
        serverPath,
        outputChannel,
        performanceMonitor
    );

    // Запускаем клиент
    await languageClient.start();

    outputChannel.appendLine('Enhanced LSP client started successfully');

    return languageClient;
}
