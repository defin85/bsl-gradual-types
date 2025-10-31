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
exports.updateLspStatus = exports.getCurrentProgress = exports.updateStatusBar = exports.finishIndexing = exports.updateIndexingProgress = exports.startIndexing = exports.initializeProgress = exports.progressEmitter = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
// Глобальное состояние индексации
let globalIndexingProgress = {
    isIndexing: false,
    currentStep: 'Idle',
    progress: 0,
    totalSteps: 4,
    currentStepNumber: 0
};
// Event emitter для обновления прогресса
exports.progressEmitter = new vscode.EventEmitter();
let outputChannel;
let statusBarItem;
// Throttling для UI обновлений (максимум 1 обновление каждые 500ms)
let lastUiUpdateTime = 0;
const UI_UPDATE_THROTTLE_MS = 500;
let pendingProgressUpdate = null;
let throttleTimeoutId;
/**
 * Инициализирует модуль прогресса
 */
function initializeProgress(channel, statusBar) {
    outputChannel = channel;
    statusBarItem = statusBar;
}
exports.initializeProgress = initializeProgress;
/**
 * Начинает отслеживание прогресса индексации
 */
function startIndexing(totalSteps = 4) {
    globalIndexingProgress = {
        isIndexing: true,
        currentStep: 'Initializing...',
        progress: 0,
        totalSteps,
        currentStepNumber: 0,
        startTime: new Date()
    };
    updateStatusBar(undefined, globalIndexingProgress);
    exports.progressEmitter.fire(globalIndexingProgress);
    outputChannel?.appendLine(`🚀 Index building started with ${totalSteps} steps`);
}
exports.startIndexing = startIndexing;
/**
 * Обновляет прогресс индексации
 *
 * @param percentage - процент выполнения (0-100)
 * @param stepName - описание текущего шага
 * @param eta - оценка оставшегося времени в секундах (опционально)
 */
function updateIndexingProgress(percentage, stepName, eta) {
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
        currentStepNumber: Math.round(percentage / 25),
        estimatedTimeRemaining: estimatedEta !== undefined ? `${estimatedEta}s` : 'calculating...'
    };
    // ✅ ИЗМЕНЕНИЕ: Используем throttled update вместо прямого вызова
    throttledUpdateUi(globalIndexingProgress);
}
exports.updateIndexingProgress = updateIndexingProgress;
/**
 * Обновляет UI с throttling (максимум каждые 500ms)
 * Использует "trailing edge" паттерн - последнее значение всегда показывается
 */
function throttledUpdateUi(progress) {
    const now = Date.now();
    const timeSinceLastUpdate = now - lastUiUpdateTime;
    // Сохраняем последнее обновление
    pendingProgressUpdate = progress;
    if (timeSinceLastUpdate >= UI_UPDATE_THROTTLE_MS) {
        // Прошло достаточно времени - обновляем сразу
        flushPendingUpdate();
    }
    else {
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
function flushPendingUpdate() {
    if (pendingProgressUpdate) {
        updateStatusBar(undefined, pendingProgressUpdate);
        exports.progressEmitter.fire(pendingProgressUpdate);
        lastUiUpdateTime = Date.now();
        throttleTimeoutId = undefined;
        // Логируем только при реальном обновлении UI
        outputChannel?.appendLine(`📊 Progress: ${pendingProgressUpdate.currentStep} ` +
            `(${pendingProgressUpdate.progress}%${pendingProgressUpdate.estimatedTimeRemaining
                ? `, ETA: ${pendingProgressUpdate.estimatedTimeRemaining}`
                : ''})`);
    }
}
/**
 * Завершает отслеживание прогресса индексации
 *
 * @param message - сообщение о завершении (опционально)
 */
function finishIndexing(message) {
    // ✅ ИСПРАВЛЕНИЕ: Применяем накопленное обновление перед очисткой
    if (throttleTimeoutId !== undefined) {
        clearTimeout(throttleTimeoutId);
        flushPendingUpdate();
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
    exports.progressEmitter.fire(globalIndexingProgress);
    lastUiUpdateTime = Date.now(); // ✅ ДОБАВИТЬ: сброс времени для следующей индексации
    const statusIcon = success ? '✅' : '❌';
    const displayMessage = message || `Index building ${success ? 'completed' : 'failed'}`;
    outputChannel?.appendLine(`${statusIcon} ${displayMessage} in ${elapsed.toFixed(1)}s`);
    if (success) {
        vscode.window.showInformationMessage(`BSL Index built successfully in ${elapsed.toFixed(1)}s`);
    }
}
exports.finishIndexing = finishIndexing;
/**
 * Обновляет статус бар
 */
function updateStatusBar(text, progress) {
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
    }
    else {
        statusBarItem.text = '$(database) BSL Analyzer';
        statusBarItem.tooltip = 'BSL Type Safety Analyzer\nClick to build index';
        statusBarItem.show();
    }
}
exports.updateStatusBar = updateStatusBar;
/**
 * Возвращает текущее состояние прогресса
 */
function getCurrentProgress() {
    return globalIndexingProgress;
}
exports.getCurrentProgress = getCurrentProgress;
/**
 * Обновляет status bar в зависимости от состояния LSP сервера
 *
 * @param state - состояние LSP клиента (State.Stopped | State.Starting | State.Running)
 */
function updateLspStatus(state) {
    if (!statusBarItem) {
        console.warn('[LSP] Status bar item not initialized');
        return;
    }
    switch (state) {
        case node_1.State.Stopped:
            statusBarItem.text = '$(error) BSL: Disconnected';
            statusBarItem.tooltip = 'BSL Language Server не активен\nПроверьте логи для деталей';
            statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
            break;
        case node_1.State.Starting:
            statusBarItem.text = '$(sync~spin) BSL: Starting...';
            statusBarItem.tooltip = 'BSL Language Server запускается...';
            statusBarItem.backgroundColor = undefined;
            break;
        case node_1.State.Running:
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
exports.updateLspStatus = updateLspStatus;
//# sourceMappingURL=progress.js.map