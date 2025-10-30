import * as vscode from 'vscode';
import { State } from 'vscode-languageclient/node';

/**
 * Состояние прогресса индексации
 */
export interface IndexingProgress {
    isIndexing: boolean;
    currentStep: string;
    progress: number;        // 0-100
    totalSteps: number;
    currentStepNumber: number;
    startTime?: Date;
    estimatedTimeRemaining?: string;
}

// Глобальное состояние индексации
let globalIndexingProgress: IndexingProgress = {
    isIndexing: false,
    currentStep: 'Idle',
    progress: 0,
    totalSteps: 4,
    currentStepNumber: 0
};

// Event emitter для обновления прогресса
export const progressEmitter = new vscode.EventEmitter<IndexingProgress>();

let outputChannel: vscode.OutputChannel | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;

// Throttling для UI обновлений (максимум 1 обновление каждые 500ms)
let lastUiUpdateTime = 0;
const UI_UPDATE_THROTTLE_MS = 500;
let pendingProgressUpdate: IndexingProgress | null = null;
let throttleTimeoutId: NodeJS.Timeout | undefined;

/**
 * Инициализирует модуль прогресса
 */
export function initializeProgress(channel: vscode.OutputChannel, statusBar: vscode.StatusBarItem) {
    outputChannel = channel;
    statusBarItem = statusBar;
}

/**
 * Начинает отслеживание прогресса индексации
 */
export function startIndexing(totalSteps: number = 4) {
    globalIndexingProgress = {
        isIndexing: true,
        currentStep: 'Initializing...',
        progress: 0,
        totalSteps,
        currentStepNumber: 0,
        startTime: new Date()
    };
    
    updateStatusBar(undefined, globalIndexingProgress);
    progressEmitter.fire(globalIndexingProgress);
    outputChannel?.appendLine(`🚀 Index building started with ${totalSteps} steps`);
}

/**
 * Обновляет прогресс индексации
 *
 * @param percentage - процент выполнения (0-100)
 * @param stepName - описание текущего шага
 * @param eta - оценка оставшегося времени в секундах (опционально)
 */
export function updateIndexingProgress(percentage: number, stepName: string, eta?: number) {
    if (!globalIndexingProgress.isIndexing) {
        outputChannel?.appendLine(`⚠️ updateIndexingProgress called but indexing is not active`);
        return;
    }

    const elapsed = globalIndexingProgress.startTime ?
        (new Date().getTime() - globalIndexingProgress.startTime.getTime()) / 1000 : 0;

    // Используем ETA из параметра, если передан, иначе вычисляем
    const estimatedEta = eta !== undefined
        ? eta
        : (percentage > 5 ? Math.round((elapsed * (100 / percentage)) - elapsed) : undefined);

    globalIndexingProgress = {
        ...globalIndexingProgress,
        currentStep: stepName,
        progress: Math.min(percentage, 100),
        currentStepNumber: Math.round(percentage / 25), // Примерная оценка шага (0-4)
        estimatedTimeRemaining: estimatedEta !== undefined ? `${estimatedEta}s` : 'calculating...'
    };

    // ✅ ИЗМЕНЕНИЕ: Используем throttled update вместо прямого вызова
    throttledUpdateUi(globalIndexingProgress);
}

/**
 * Обновляет UI с throttling (максимум каждые 500ms)
 * Использует "trailing edge" паттерн - последнее значение всегда показывается
 */
function throttledUpdateUi(progress: IndexingProgress): void {
    const now = Date.now();
    const timeSinceLastUpdate = now - lastUiUpdateTime;

    // Сохраняем последнее обновление
    pendingProgressUpdate = progress;

    if (timeSinceLastUpdate >= UI_UPDATE_THROTTLE_MS) {
        // Прошло достаточно времени - обновляем сразу
        flushPendingUpdate();
    } else {
        // Слишком рано - планируем отложенное обновление
        if (throttleTimeoutId !== undefined) {
            clearTimeout(throttleTimeoutId);
        }

        const delay = UI_UPDATE_THROTTLE_MS - timeSinceLastUpdate;
        throttleTimeoutId = setTimeout(() => {
            flushPendingUpdate();
        }, delay);
    }
}

/**
 * Применяет накопленное обновление к UI
 */
function flushPendingUpdate(): void {
    if (pendingProgressUpdate) {
        updateStatusBar(undefined, pendingProgressUpdate);
        progressEmitter.fire(pendingProgressUpdate);
        lastUiUpdateTime = Date.now();
        throttleTimeoutId = undefined;

        // Логируем только при реальном обновлении UI
        outputChannel?.appendLine(
            `📊 Progress: ${pendingProgressUpdate.currentStep} ` +
            `(${pendingProgressUpdate.progress}%${
                pendingProgressUpdate.estimatedTimeRemaining
                    ? `, ETA: ${pendingProgressUpdate.estimatedTimeRemaining}`
                    : ''
            })`
        );
    }
}


/**
 * Завершает отслеживание прогресса индексации
 *
 * @param message - сообщение о завершении (опционально)
 */
export function finishIndexing(message?: string) {
    // ✅ ДОБАВИТЬ: Сбросить pending updates перед завершением
    if (throttleTimeoutId !== undefined) {
        clearTimeout(throttleTimeoutId);
        throttleTimeoutId = undefined;
    }

    const elapsed = globalIndexingProgress.startTime ?
        (new Date().getTime() - globalIndexingProgress.startTime.getTime()) / 1000 : 0;

    // Определяем успешность на основе message (если содержит "✅" или "успешно")
    const success = message ? (message.includes('✅') || message.toLowerCase().includes('успешно')) : true;

    globalIndexingProgress = {
        isIndexing: false,
        currentStep: success ? 'Completed' : 'Failed',
        progress: 100,
        totalSteps: globalIndexingProgress.totalSteps,
        currentStepNumber: globalIndexingProgress.totalSteps
    };

    // ✅ ИЗМЕНЕНИЕ: Финальное обновление всегда показывается сразу (без throttling)
    updateStatusBar(success ? 'BSL Analyzer: Index Ready' : 'BSL Analyzer: Index Failed', undefined);
    progressEmitter.fire(globalIndexingProgress);
    lastUiUpdateTime = Date.now();  // ✅ ДОБАВИТЬ: сброс времени для следующей индексации

    const statusIcon = success ? '✅' : '❌';
    const displayMessage = message || `Index building ${success ? 'completed' : 'failed'}`;
    outputChannel?.appendLine(`${statusIcon} ${displayMessage} in ${elapsed.toFixed(1)}s`);

    if (success) {
        vscode.window.showInformationMessage(`BSL Index built successfully in ${elapsed.toFixed(1)}s`);
    }
}

/**
 * Обновляет статус бар
 */
export function updateStatusBar(text?: string, progress?: IndexingProgress) {
    if (!statusBarItem) {
        return;
    }
    
    if (text) {
        statusBarItem.text = text;
        statusBarItem.show();
        return;
    }
    
    if (progress && progress.isIndexing) {
        const icon = '$(sync~spin)';
        const percent = Math.round(progress.progress);
        const eta = progress.estimatedTimeRemaining ? ` - ETA: ${progress.estimatedTimeRemaining}` : '';
        statusBarItem.text = `${icon} BSL Index: ${progress.currentStep} (${percent}%${eta})`;
        statusBarItem.tooltip = `Step ${progress.currentStepNumber}/${progress.totalSteps}\nProgress: ${percent}%\n${progress.currentStep}`;
        statusBarItem.show();
    } else {
        statusBarItem.text = '$(database) BSL Analyzer';
        statusBarItem.tooltip = 'BSL Type Safety Analyzer\nClick to build index';
        statusBarItem.show();
    }
}

/**
 * Возвращает текущее состояние прогресса
 */
export function getCurrentProgress(): IndexingProgress {
    return globalIndexingProgress;
}

/**
 * Обновляет status bar в зависимости от состояния LSP сервера
 *
 * @param state - состояние LSP клиента (State.Stopped | State.Starting | State.Running)
 */
export function updateLspStatus(state: State): void {
    if (!statusBarItem) {
        console.warn('[LSP] Status bar item not initialized');
        return;
    }

    switch (state) {
        case State.Stopped:
            statusBarItem.text = '$(error) BSL: Disconnected';
            statusBarItem.tooltip = 'BSL Language Server не активен\nПроверьте логи для деталей';
            statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
            break;

        case State.Starting:
            statusBarItem.text = '$(sync~spin) BSL: Starting...';
            statusBarItem.tooltip = 'BSL Language Server запускается...';
            statusBarItem.backgroundColor = undefined;
            break;

        case State.Running:
            statusBarItem.text = '$(check) BSL: Ready';
            statusBarItem.tooltip = 'BSL Type Safety Analyzer\nLSP Server активен';
            statusBarItem.backgroundColor = undefined;
            break;

        default:
            console.warn(`[LSP] Unknown state: ${state}`);
            break;
    }

    statusBarItem.show();
}