import * as vscode from 'vscode';
import { logger } from './logger';
import { ServerStatusParams } from '../types';

/**
 * MILESTONE 2.20.3: Server Status Handler (rust-analyzer approach)
 *
 * Управляет Status Bar ТОЛЬКО для initial loading state.
 *
 * РАЗДЕЛЕНИЕ ОТВЕТСТВЕННОСТИ:
 * - bsl/serverStatus (этот файл) → $(loading~spin) при запуске LSP Server
 * - $/progress (vscode-languageclient) → Прогресс индексации (автоматический Progress Window)
 *
 * Этот handler показывает loading icon в начале загрузки типов платформы,
 * затем автоматический $/progress handler от vscode-languageclient берёт управление на себя.
 */

let statusBarItem: vscode.StatusBarItem | undefined;
let outputChannel: vscode.OutputChannel | undefined;
let currentStatus: ServerStatusParams = { loading: false };
const statusListeners = new Set<(params: ServerStatusParams) => void>();

/**
 * Инициализирует модуль server status
 */
export function initializeServerStatus(channel: vscode.OutputChannel, statusBar: vscode.StatusBarItem) {
    outputChannel = channel;
    statusBarItem = statusBar;
}

export function getServerStatusSnapshot(): ServerStatusParams {
    return { ...currentStatus };
}

export function isServerLoading(): boolean {
    return currentStatus.loading;
}

export function onServerStatusChange(
    listener: (params: ServerStatusParams) => void
): vscode.Disposable {
    statusListeners.add(listener);
    return new vscode.Disposable(() => {
        statusListeners.delete(listener);
    });
}

/**
 * Обработчик bsl/serverStatus notification
 * Показывает $(loading~spin) icon во время загрузки типов платформы
 */
export function handleServerStatus(params: ServerStatusParams): void {
    currentStatus = {
        loading: params.loading,
        message: params.message,
    };
    for (const listener of statusListeners) {
        listener(currentStatus);
    }

    if (!statusBarItem) {
        logger.warn('[ServerStatus] statusBarItem not initialized');
        return;
    }

    if (params.loading) {
        // Показываем spinning icon во время загрузки типов
        const message = params.message || 'Загрузка типов платформы...';
        statusBarItem.text = `$(loading~spin) BSL: ${message}`;
        statusBarItem.tooltip = `BSL Language Server загружается\n${message}`;
        statusBarItem.backgroundColor = undefined;
    } else {
        // Возвращаемся к обычному состоянию
        statusBarItem.text = '$(check) BSL: Ready';
        statusBarItem.tooltip = 'BSL Type Safety Analyzer\nLSP Server готов';
        statusBarItem.backgroundColor = undefined;
    }

    statusBarItem.show();

    // Логируем для отладки
    const msg = params.message || 'N/A';
    outputChannel?.appendLine(`📊 [ServerStatus] loading=${params.loading}, message=${msg}`);
}

export function resetServerStatusForTests(): void {
    currentStatus = { loading: false };
    statusListeners.clear();
    statusBarItem = undefined;
    outputChannel = undefined;
}
