/**
 * Enhanced Code Actions Provider
 *
 * Предоставляет автоматические исправления и рефакторинг
 * на основе результатов продвинутого анализа типов
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class CodeActionsProvider implements vscode.CodeActionProvider {
    private client;
    constructor(client: EnhancedLspClient);
    provideCodeActions(document: vscode.TextDocument, range: vscode.Range | vscode.Selection, context: vscode.CodeActionContext, token: vscode.CancellationToken): Promise<vscode.CodeAction[]>;
    /**
     * Конвертация VSCode диагностики в LSP формат
     */
    private convertDiagnostic;
    /**
     * Конвертация LSP code action в VSCode формат
     */
    private convertToVSCodeAction;
    /**
     * Конвертация LSP code action kind в VSCode формат
     */
    private convertKind;
    /**
     * Конвертация LSP workspace edit в VSCode формат
     */
    private convertWorkspaceEdit;
    /**
     * Конвертация LSP диагностики в VSCode формат
     */
    private convertLspDiagnostic;
    /**
     * Конвертация LSP severity в VSCode формат
     */
    private convertSeverity;
}
//# sourceMappingURL=code-actions-provider.d.ts.map