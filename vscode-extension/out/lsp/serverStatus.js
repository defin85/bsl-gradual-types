"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.handleServerStatus = exports.initializeServerStatus = void 0;
/**
 * MILESTONE 2.20.3: Server Status Handler (rust-analyzer approach)
 * Управляет status bar icon на основе loading состояния LSP server
 */
let statusBarItem;
let outputChannel;
/**
 * Инициализирует модуль server status
 */
function initializeServerStatus(channel, statusBar) {
    outputChannel = channel;
    statusBarItem = statusBar;
}
exports.initializeServerStatus = initializeServerStatus;
/**
 * Обработчик bsl/serverStatus notification
 * Показывает $(loading~spin) icon во время загрузки типов платформы
 */
function handleServerStatus(params) {
    if (!statusBarItem) {
        console.warn('[ServerStatus] statusBarItem not initialized');
        return;
    }
    if (params.loading) {
        // Показываем spinning icon во время загрузки типов
        const message = params.message || 'Загрузка типов платформы...';
        statusBarItem.text = `$(loading~spin) BSL: ${message}`;
        statusBarItem.tooltip = `BSL Language Server загружается\n${message}`;
        statusBarItem.backgroundColor = undefined;
    }
    else {
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
exports.handleServerStatus = handleServerStatus;
//# sourceMappingURL=serverStatus.js.map