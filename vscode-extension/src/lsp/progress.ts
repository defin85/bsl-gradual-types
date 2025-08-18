import * as vscode from 'vscode';

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
 */
export function updateIndexingProgress(stepNumber: number, stepName: string, progress: number) {
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
    progressEmitter.fire(globalIndexingProgress);
    outputChannel?.appendLine(`📊 Step ${stepNumber}/${globalIndexingProgress.totalSteps}: ${stepName} (${progress}%)`);
}

/**
 * Завершает отслеживание прогресса индексации
 */
export function finishIndexing(success: boolean = true) {
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
    progressEmitter.fire(globalIndexingProgress);
    
    const statusIcon = success ? '✅' : '❌';
    outputChannel?.appendLine(`${statusIcon} Index building ${success ? 'completed' : 'failed'} in ${elapsed.toFixed(1)}s`);
    
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