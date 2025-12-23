import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

/** Состояние активного прогресса */
interface ProgressState {
    resolve: ((value: void) => void) | null;
    reporter: vscode.Progress<{ message?: string; increment?: number }> | null;
    lastReportedPercentage: number;
    pending: PendingReport | null;
    startedAtMs: number;
    pendingEndMessage: string | null;
    endTimer: NodeJS.Timeout | null;
}

interface PendingReport {
    message: string;
    percentage: number;
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

const MIN_PROGRESS_VISIBLE_MS = 800;

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
        if (existing.endTimer) {
            clearTimeout(existing.endTimer);
            existing.endTimer = null;
        }
        existing.resolve();
    }

    const state = existing && !existing.resolve ? existing : {
        resolve: null,
        reporter: null,
        lastReportedPercentage: 0,
        pending: null,
        startedAtMs: Date.now(),
        pendingEndMessage: null,
        endTimer: null
    };
    state.resolve = null;
    state.reporter = null;
    state.lastReportedPercentage = 0;
    state.startedAtMs = Date.now();
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

        flushPending(state);
        if (state.pendingEndMessage) {
            scheduleEnd(key, state, state.pendingEndMessage, states, outputChannel);
            state.pendingEndMessage = null;
        }

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
    let state = states.get(key);
    if (!state) {
        state = {
            resolve: null,
            reporter: null,
            lastReportedPercentage: 0,
            pending: null,
            startedAtMs: Date.now(),
            pendingEndMessage: null,
            endTimer: null
        };
        states.set(key, state);
        outputChannel.appendLine(`[Progress] REPORT before BEGIN: ${key} | ${message} (${percentage}%)`);
    }

    if (!state.reporter) {
        if (!state.pending || percentage >= state.pending.percentage) {
            state.pending = { percentage, message };
        }
        return;
    }

    const previous = state.lastReportedPercentage;
    const normalized = Math.max(previous, percentage);
    state.lastReportedPercentage = normalized;

    state.reporter.report({
        message: message,
        increment: Math.max(0, normalized - previous)  // Не допускать отрицательных значений
    });
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

    // Закрыть Progress Window (с минимальной длительностью отображения)
    let state = states.get(key);
    if (!state) {
        state = {
            resolve: null,
            reporter: null,
            lastReportedPercentage: 0,
            pending: null,
            startedAtMs: Date.now(),
            pendingEndMessage: null,
            endTimer: null
        };
        states.set(key, state);
    }

    if (!state.reporter || !state.resolve) {
        state.pendingEndMessage = message;
        return;
    }

    scheduleEnd(key, state, message, states, outputChannel);
}

function flushPending(state: ProgressState): void {
    if (!state.reporter || !state.pending) {
        return;
    }

    const previous = state.lastReportedPercentage;
    const normalized = Math.max(previous, state.pending.percentage);
    state.lastReportedPercentage = normalized;

    state.reporter.report({
        message: state.pending.message,
        increment: Math.max(0, normalized - previous)
    });

    state.pending = null;
}

function scheduleEnd(
    key: string,
    state: ProgressState,
    message: string,
    states: Map<string, ProgressState>,
    outputChannel: vscode.OutputChannel
): void {
    if (!state.resolve) {
        state.pendingEndMessage = message;
        return;
    }

    if (state.endTimer) {
        clearTimeout(state.endTimer);
        state.endTimer = null;
    }

    const elapsed = Date.now() - state.startedAtMs;
    const delay = Math.max(0, MIN_PROGRESS_VISIBLE_MS - elapsed);

    state.endTimer = setTimeout(() => {
        state.endTimer = null;
        outputChannel.appendLine(`[Progress] END (delayed): ${key} | ${message}`);
        state.resolve?.();
        states.delete(key);
    }, delay);
}
