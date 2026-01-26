import * as vscode from 'vscode';
import { ServerOptions } from 'vscode-languageclient/node';
/**
 * Строит ServerOptions для LSP клиента
 * @param serverPath Путь к исполняемому файлу LSP сервера
 * @param outputChannel Канал для логирования
 */
export declare function buildServerOptions(serverPath: string, outputChannel: vscode.OutputChannel): ServerOptions;
//# sourceMappingURL=server-options.d.ts.map