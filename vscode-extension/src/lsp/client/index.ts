/**
 * LSP Client module - модульная структура для управления LSP клиентом
 *
 * Структура:
 * - index.ts (этот файл) - инициализация и re-exports
 * - lifecycle.ts - start, stop, restart, getClient, isClientRunning
 * - server-options.ts - buildServerOptions()
 * - client-options.ts - buildClientOptions()
 * - progress-handler.ts - setupProgressHandler()
 * - health-check.ts - startHealthCheck, stopHealthCheck
 */

import * as vscode from 'vscode';
import { initializeLifecycle } from './lifecycle';

// Re-export public API from lifecycle
export {
    startLanguageClient,
    stopLanguageClient,
    restartLanguageClient,
    getLanguageClient,
    getActiveServerLaunchInfo,
    getServerVersion,
    isClientRunning,
    sendCustomRequest,
    sendCustomNotification
} from './lifecycle';

/**
 * Инициализирует модуль LSP клиента
 * @param channel Output channel для логирования
 */
export function initializeLspClient(channel: vscode.OutputChannel): void {
    initializeLifecycle(channel);
}
