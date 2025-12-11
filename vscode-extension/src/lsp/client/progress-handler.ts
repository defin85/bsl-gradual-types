import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

/** Состояние активного прогресса */
interface ProgressState {
    resolve: ((value: void) => void) | null;
    reporter: vscode.Progress<{ message?: string; increment?: number }> | null;
    lastReportedPercentage: number;
}

/**
 * Настраивает обработчик $/progress для Work Done Progress
 * @param client LSP клиент
 * @param outputChannel Канал для логирования
 */
export function setupProgressHandler(
    client: LanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    // MILESTONE 2.20.4: Обработчик Work Done Progress (для Progress Window)
    // vscode-languageclient НЕ показывает Progress Window автоматически для server-initiated progress
    // (progress tokens, созданные LSP Server через window/workDoneProgress/create)
    // Нужна явная регистрация обработчика для показа Progress Window

    const state: ProgressState = {
        resolve: null,
        reporter: null,
        lastReportedPercentage: 0
    };

    client.onNotification('$/progress', (params: any) => {
        const value = params.value;

        if (value.kind === 'begin') {
            handleProgressBegin(value, state, outputChannel);
        }

        if (value.kind === 'report') {
            handleProgressReport(value, state, outputChannel);
        }

        if (value.kind === 'end') {
            handleProgressEnd(value, state, outputChannel);
        }
    });

    outputChannel.appendLine('$/progress handler registered');
}

/**
 * Обрабатывает начало прогресса
 */
function handleProgressBegin(
    value: any,
    state: ProgressState,
    outputChannel: vscode.OutputChannel
): void {
    outputChannel.appendLine(`[Progress] BEGIN: ${value.title}`);

    // Завершаем старый прогресс если он ещё активен (из-за crash/restart LSP)
    if (state.resolve) {
        outputChannel.appendLine('Clearing previous progress before starting new one');
        state.resolve();
        state.resolve = null;
        state.reporter = null;
    }

    // Сброс состояния
    state.lastReportedPercentage = 0;

    // Показать Progress в Status Bar (всегда видно)
    vscode.window.withProgress({
        location: vscode.ProgressLocation.Window,
        title: value.title,
        cancellable: false
    }, async (progress) => {
        // Сохранить reporter для использования в REPORT
        state.reporter = progress;

        // Начальное сообщение
        progress.report({
            message: value.message || 'Инициализация...',
            increment: 0
        });

        // Ждём завершения прогресса
        return new Promise<void>((resolve) => {
            state.resolve = resolve;
        });
    });
}

/**
 * Обрабатывает отчёт о прогрессе
 */
function handleProgressReport(
    value: any,
    state: ProgressState,
    outputChannel: vscode.OutputChannel
): void {
    const percentage = value.percentage || 0;
    const message = value.message || '';

    outputChannel.appendLine(`[Progress] REPORT: ${message} (${percentage}%)`);

    // Обновить Progress Window
    if (state.reporter) {
        const increment = percentage - state.lastReportedPercentage;
        state.lastReportedPercentage = percentage;

        state.reporter.report({
            message: message,
            increment: Math.max(0, increment)  // Не допускать отрицательных значений
        });
    }
}

/**
 * Обрабатывает завершение прогресса
 */
function handleProgressEnd(
    value: any,
    state: ProgressState,
    outputChannel: vscode.OutputChannel
): void {
    const message = value.message || 'Завершено';

    outputChannel.appendLine(`[Progress] END: ${message}`);

    // Закрыть Progress Window
    if (state.resolve) {
        state.resolve();
        state.resolve = null;
    }

    // Очистить состояние
    state.reporter = null;
    state.lastReportedPercentage = 0;
}
