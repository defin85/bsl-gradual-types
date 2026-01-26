import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
/**
 * Запускает периодическую проверку состояния LSP сервера
 * @param client LSP клиент для проверки
 * @param outputChannel Канал для логирования
 */
export declare function startHealthCheck(client: LanguageClient, outputChannel: vscode.OutputChannel): void;
/**
 * Останавливает периодическую проверку состояния
 */
export declare function stopHealthCheck(): void;
//# sourceMappingURL=health-check.d.ts.map