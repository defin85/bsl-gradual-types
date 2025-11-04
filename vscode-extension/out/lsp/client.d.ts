import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
/**
 * Инициализирует модуль LSP клиента
 */
export declare function initializeLspClient(channel: vscode.OutputChannel): void;
/**
 * Запускает LSP сервер
 */
export declare function startLanguageClient(context: vscode.ExtensionContext): Promise<void>;
/**
 * Останавливает LSP сервер
 */
export declare function stopLanguageClient(): Promise<void>;
/**
 * Перезапускает LSP сервер
 */
export declare function restartLanguageClient(context: vscode.ExtensionContext): Promise<void>;
/**
 * Возвращает текущий клиент LSP
 */
export declare function getLanguageClient(): LanguageClient | null;
/**
 * Проверяет, запущен ли LSP клиент
 */
export declare function isClientRunning(): boolean;
/**
 * Отправляет запрос на сервер для выполнения кастомной команды
 */
export declare function sendCustomRequest<T = unknown>(method: string, params?: unknown): Promise<T>;
/**
 * Отправляет уведомление на сервер
 */
export declare function sendCustomNotification(method: string, params?: unknown): void;
//# sourceMappingURL=client.d.ts.map