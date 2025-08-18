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
exports.getCurrentProgress = exports.updateStatusBar = exports.finishIndexing = exports.updateIndexingProgress = exports.startIndexing = exports.initializeProgress = exports.progressEmitter = void 0;
const vscode = __importStar(require("vscode"));
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
 */
function updateIndexingProgress(stepNumber, stepName, progress) {
    if (!globalIndexingProgress.isIndexing) {
        outputChannel?.appendLine(`⚠️ updateIndexingProgress called but indexing is not active`);
        return;
    }
    const elapsed = globalIndexingProgress.startTime ?
        (new Date().getTime() - globalIndexingProgress.startTime.getTime()) / 1000 : 0;
    // Простая оценка времени: elapsed * (100 / progress) - elapsed
    const eta = progress > 5 ? Math.round((elapsed * (100 / progress)) - elapsed) : undefined;
    globalIndexingProgress = {
        ...globalIndexingProgress,
        currentStep: stepName,
        progress: Math.min(progress, 100),
        currentStepNumber: stepNumber,
        estimatedTimeRemaining: eta ? `${eta}s` : 'calculating...'
    };
    updateStatusBar(undefined, globalIndexingProgress);
    exports.progressEmitter.fire(globalIndexingProgress);
    outputChannel?.appendLine(`📊 Step ${stepNumber}/${globalIndexingProgress.totalSteps}: ${stepName} (${progress}%)`);
}
exports.updateIndexingProgress = updateIndexingProgress;
/**
 * Завершает отслеживание прогресса индексации
 */
function finishIndexing(success = true) {
    const elapsed = globalIndexingProgress.startTime ?
        (new Date().getTime() - globalIndexingProgress.startTime.getTime()) / 1000 : 0;
    globalIndexingProgress = {
        isIndexing: false,
        currentStep: success ? 'Completed' : 'Failed',
        progress: 100,
        totalSteps: globalIndexingProgress.totalSteps,
        currentStepNumber: globalIndexingProgress.totalSteps
    };
    updateStatusBar(success ? 'BSL Analyzer: Index Ready' : 'BSL Analyzer: Index Failed', undefined);
    exports.progressEmitter.fire(globalIndexingProgress);
    const statusIcon = success ? '✅' : '❌';
    outputChannel?.appendLine(`${statusIcon} Index building ${success ? 'completed' : 'failed'} in ${elapsed.toFixed(1)}s`);
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
//# sourceMappingURL=progress.js.map