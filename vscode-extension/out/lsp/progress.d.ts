import * as vscode from 'vscode';
import { State } from 'vscode-languageclient/node';
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
/**
 * Обновляет status bar в зависимости от состояния LSP сервера
 *
 * @param state - состояние LSP клиента (State.Stopped | State.Starting | State.Running)
 */
export declare function updateLspStatus(state: State): void;
//# sourceMappingURL=progress.d.ts.map