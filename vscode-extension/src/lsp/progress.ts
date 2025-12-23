import * as vscode from 'vscode';
import { State } from 'vscode-languageclient/node';
import { logger } from './logger';

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
    progress: number;        // 0-100
}

// Глобальное состояние индексации
let globalIndexingProgress: IndexingProgress = {
    isIndexing: false,
    currentStep: 'Idle',
    progress: 0
};

let autoReindexPaused = false;

const AUTO_REINDEX_PAUSED_TEXT = '$(debug-pause) BSL: Auto reindex paused';
const AUTO_REINDEX_PAUSED_TOOLTIP = 'Auto reindex paused';

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
 * Обновляет статус бар
 */
export function updateStatusBar(text?: string, progress?: IndexingProgress) {
    if (!statusBarItem) {
        return;
    }

    if (text) {
        const resolvedText =
            autoReindexPaused && /\bReady\b/.test(text)
                ? AUTO_REINDEX_PAUSED_TEXT
                : text;
        statusBarItem.text = resolvedText;
        // Установить tooltip из text (удаляя иконки VSCode)
        const cleanText = resolvedText.replace(/\$\([^)]+\)/g, '').trim();
        statusBarItem.tooltip = cleanText;
        statusBarItem.show();
        return;
    }

    if (progress && progress.isIndexing) {
        const icon = '$(sync~spin)';
        const percent = Math.round(progress.progress);
        statusBarItem.text = `${icon} BSL Index: ${progress.currentStep} (${percent}%)`;
        statusBarItem.tooltip = `Progress: ${percent}%\n${progress.currentStep}`;
        statusBarItem.show();
    } else {
        if (autoReindexPaused) {
            statusBarItem.text = AUTO_REINDEX_PAUSED_TEXT;
            statusBarItem.tooltip = AUTO_REINDEX_PAUSED_TOOLTIP;
        } else {
            statusBarItem.text = '$(database) BSL Analyzer';
            statusBarItem.tooltip = 'BSL Type Safety Analyzer\nClick to build index';
        }
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
        logger.warn('[Progress] Status bar item not initialized for updateLspStatus - call initializeProgress() first');
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
            if (autoReindexPaused) {
                statusBarItem.text = AUTO_REINDEX_PAUSED_TEXT;
                statusBarItem.tooltip = AUTO_REINDEX_PAUSED_TOOLTIP;
                statusBarItem.backgroundColor = undefined;
            } else {
                statusBarItem.text = '$(check) BSL: Ready';
                statusBarItem.tooltip = 'BSL Type Safety Analyzer\nLSP Server активен';
                statusBarItem.backgroundColor = undefined;
            }
            break;

        default:
            logger.warn(`[Progress] Unknown LSP state: ${state}`);
            break;
    }

    statusBarItem.show();
}

export function setAutoReindexPaused(paused: boolean): void {
    autoReindexPaused = paused;
}
