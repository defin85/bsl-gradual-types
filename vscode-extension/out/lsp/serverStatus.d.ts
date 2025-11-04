import * as vscode from 'vscode';
/**
 * Инициализирует модуль server status
 */
export declare function initializeServerStatus(channel: vscode.OutputChannel, statusBar: vscode.StatusBarItem): void;
/**
 * Обработчик bsl/serverStatus notification
 * Показывает $(loading~spin) icon во время загрузки типов платформы
 */
export declare function handleServerStatus(params: {
    loading: boolean;
    message?: string;
}): void;
//# sourceMappingURL=serverStatus.d.ts.map