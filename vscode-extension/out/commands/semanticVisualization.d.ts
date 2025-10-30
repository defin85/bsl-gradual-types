import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
/**
 * Регистрирует команду для показа семантического дерева BSL модуля
 *
 * Использует LSP custom request `bsl/getSemanticHtml` для получения
 * готового HTML с семантическим деревом, таблицей символов и метриками.
 *
 * @param client - LSP клиент для общения с backend
 * @returns Disposable для cleanup при деактивации extension
 */
export declare function registerSemanticVisualization(client: LanguageClient): vscode.Disposable;
//# sourceMappingURL=semanticVisualization.d.ts.map