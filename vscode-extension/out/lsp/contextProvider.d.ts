import * as vscode from 'vscode';
export interface CurrentContext {
    functionName?: string;
    functionKind: 'function' | 'procedure' | 'none';
    params?: string[];
    returnType?: string;
}
/**
 * Инициализирует отслеживание текущего контекста в редакторе
 *
 * @param context - VSCode extension context
 * @param statusBar - Status bar item для обновления tooltip
 */
export declare function initializeContextProvider(context: vscode.ExtensionContext, statusBar: vscode.StatusBarItem): void;
//# sourceMappingURL=contextProvider.d.ts.map