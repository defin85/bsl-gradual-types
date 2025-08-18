/**
 * Simplified Code Actions Provider
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';

export class CodeActionsProvider implements vscode.CodeActionProvider {
    private client: EnhancedLspClient;
    
    constructor(client: EnhancedLspClient) {
        this.client = client;
    }
    
    async provideCodeActions(
        document: vscode.TextDocument,
        range: vscode.Range | vscode.Selection,
        context: vscode.CodeActionContext,
        token: vscode.CancellationToken
    ): Promise<vscode.CodeAction[]> {
        try {
            // Простая заглушка - в будущем будет реальная реализация
            return [];
        } catch (error) {
            console.error('Error providing code actions:', error);
            return [];
        }
    }
}