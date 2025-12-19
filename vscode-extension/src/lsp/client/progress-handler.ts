import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

/** Состояние активного прогресса */
interface ProgressState {
    resolve: ((value: void) => void) | null;
    reporter: vscode.Progress<{ message?: string; increment?: number }> | null;
    lastReportedPercentage: number;
}

function tokenKey(token: any): string {
    // LSP ProgressToken может быть string | number
    if (typeof token === 'string') return token;
    if (typeof token === 'number') return String(token);
    try {
        return JSON.stringify(token);
    } catch {
        return String(token);
    }
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

    // Multi-token: несколько параллельных прогрессов не должны "перетирать" друг друга.
    // Храним состояние по token.
    const states = new Map<string, ProgressState>();

    client.onNotification('$/progress', (params: any) => {
        const key = tokenKey(params.token);
        const value = params.value;

        if (value.kind === 'begin') {
            handleProgressBegin(key, value, states, outputChannel);
        }

        if (value.kind === 'report') {
            handleProgressReport(key, value, states, outputChannel);
        }

        if (value.kind === 'end') {
            handleProgressEnd(key, value, states, outputChannel);
        }
    });

    outputChannel.appendLine('$/progress handler registered');
}

/**
 * Обрабатывает начало прогресса
 */
function handleProgressBegin(
    key: string,
    value: any,
    states: Map<string, ProgressState>,
    outputChannel: vscode.OutputChannel
): void {
    outputChannel.appendLine(`[Progress] BEGIN: ${key} | ${value.title}`);

    const existing = states.get(key);
    // Завершаем старый прогресс с тем же token (из-за crash/restart LSP)
    if (existing?.resolve) {
        outputChannel.appendLine(`[Progress] Clearing previous progress for token: ${key}`);
        existing.resolve();
    }

    const state: ProgressState = {
        resolve: null,
        reporter: null,
        lastReportedPercentage: 0
    };
    states.set(key, state);

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
    key: string,
    value: any,
    states: Map<string, ProgressState>,
    outputChannel: vscode.OutputChannel
): void {
    const percentage = value.percentage || 0;
    const message = value.message || '';

    outputChannel.appendLine(`[Progress] REPORT: ${key} | ${message} (${percentage}%)`);

    // Обновить Progress Window
    const state = states.get(key);
    if (state?.reporter) {
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
    key: string,
    value: any,
    states: Map<string, ProgressState>,
    outputChannel: vscode.OutputChannel
): void {
    const message = value.message || 'Завершено';

    outputChannel.appendLine(`[Progress] END: ${key} | ${message}`);

    // Закрыть Progress Window
    const state = states.get(key);
    if (state?.resolve) {
        state.resolve();
    }

    states.delete(key);
}
