import * as vscode from 'vscode';
/**
 * Состояние прогресса индексации
 */
export interface IndexingProgress {
    isIndexing: boolean;
    currentStep: string;
    progress: number;
    totalSteps: number;
    currentStepNumber: number;
    startTime?: Date;
    estimatedTimeRemaining?: string;
}
export declare const progressEmitter: vscode.EventEmitter<IndexingProgress>;
/**
 * Инициализирует модуль прогресса
 */
export declare function initializeProgress(channel: vscode.OutputChannel, statusBar: vscode.StatusBarItem): void;
/**
 * Начинает отслеживание прогресса индексации
 */
export declare function startIndexing(totalSteps?: number): void;
/**
 * Обновляет прогресс индексации
 */
export declare function updateIndexingProgress(stepNumber: number, stepName: string, progress: number): void;
/**
 * Завершает отслеживание прогресса индексации
 */
export declare function finishIndexing(success?: boolean): void;
/**
 * Обновляет статус бар
 */
export declare function updateStatusBar(text?: string, progress?: IndexingProgress): void;
/**
 * Возвращает текущее состояние прогресса
 */
export declare function getCurrentProgress(): IndexingProgress;
//# sourceMappingURL=progress.d.ts.map