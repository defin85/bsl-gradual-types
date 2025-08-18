/**
 * Simplified Code Actions Provider
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class CodeActionsProvider implements vscode.CodeActionProvider {
    private client;
    constructor(client: EnhancedLspClient);
    provideCodeActions(document: vscode.TextDocument, range: vscode.Range | vscode.Selection, context: vscode.CodeActionContext, token: vscode.CancellationToken): Promise<vscode.CodeAction[]>;
}
//# sourceMappingURL=code-actions-simple.d.ts.map