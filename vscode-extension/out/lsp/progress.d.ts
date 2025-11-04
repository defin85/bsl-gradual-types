import * as vscode from 'vscode';
import { State } from 'vscode-languageclient/node';
/**
 * MILESTONE 2.20: Упрощённая система прогресса
 *
 * После рефакторинга этот модуль управляет только Status Bar.
 * Progress Window показывается автоматически через vscode-languageclient.
 *
 * УДАЛЕНО:
 * - startIndexing(), updateIndexingProgress(), finishIndexing() - вызывались из мёртвого кода
 * - Throttling логика (500ms) - больше не нужна
 * - Детальное управление прогрессом - делегировано vscode-languageclient
 */
/**
 * Состояние прогресса индексации
 */
export interface IndexingProgress {
    isIndexing: boolean;
    currentStep: string;
    progress: number;
}
export declare const progressEmitter: vscode.EventEmitter<IndexingProgress>;
/**
 * Инициализирует модуль прогресса
 */
export declare function initializeProgress(channel: vscode.OutputChannel, statusBar: vscode.StatusBarItem): void;
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