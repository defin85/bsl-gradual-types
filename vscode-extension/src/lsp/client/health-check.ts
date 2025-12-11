import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { updateStatusBar } from '../progress';

/** Интервал health check */
let healthCheckInterval: NodeJS.Timeout | null = null;

/**
 * Запускает периодическую проверку состояния LSP сервера
 * @param client LSP клиент для проверки
 * @param outputChannel Канал для логирования
 */
export function startHealthCheck(
    client: LanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    // Останавливаем предыдущий интервал, если он есть
    stopHealthCheck();

    // Проверяем состояние каждые 30 секунд
    healthCheckInterval = setInterval(() => {
        if (client) {
            const isRunning = client.isRunning();
            if (!isRunning) {
                outputChannel.appendLine('Health check: LSP client is not running');
                updateStatusBar('$(error) BSL Analyzer: Disconnected');
                vscode.commands.executeCommand('bslAnalyzer.refreshOverview');

                // Показываем уведомление только один раз
                stopHealthCheck();
                vscode.window.showWarningMessage(
                    'BSL Analyzer: Language server stopped unexpectedly',
                    'Restart Server',
                    'Dismiss'
                ).then(selection => {
                    if (selection === 'Restart Server') {
                        vscode.commands.executeCommand('bslAnalyzer.restartServer');
                    }
                });
            }
        }
    }, 30000); // 30 секунд
}

/**
 * Останавливает периодическую проверку состояния
 */
export function stopHealthCheck(): void {
    if (healthCheckInterval) {
        clearInterval(healthCheckInterval);
        healthCheckInterval = null;
    }
}
