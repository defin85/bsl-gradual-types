/**
 * Type Hints Provider для VSCode
 * 
 * Предоставляет inline type hints на основе результатов
 * flow-sensitive анализа и union types
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';

export class TypeHintsProvider implements vscode.InlayHintsProvider {
    private client: EnhancedLspClient;
    
    constructor(client: EnhancedLspClient) {
        this.client = client;
    }
    
    async provideInlayHints(
        document: vscode.TextDocument,
        range: vscode.Range,
        token: vscode.CancellationToken
    ): Promise<vscode.InlayHint[]> {
        try {
            // Запрашиваем type hints от enhanced LSP сервера
            const result = await this.client.requestInlayHints({
                textDocument: { uri: document.uri.toString() },
                range: {
                    start: { line: range.start.line, character: range.start.character },
                    end: { line: range.end.line, character: range.end.character }
                }
            });
            
            if (!result) {
                return [];
            }
            
            return result.map(hint => this.convertToVSCodeInlayHint(hint));
            
        } catch (error) {
            console.error('Error providing inlay hints:', error);
            return [];
        }
    }
    
    /**
     * Конвертация LSP inlay hint в VSCode формат
     */
    private convertToVSCodeInlayHint(lspHint: any): vscode.InlayHint {
        const position = new vscode.Position(lspHint.position.line, lspHint.position.character);
        
        const hint = new vscode.InlayHint(
            position,
            lspHint.label,
            lspHint.kind === 1 ? vscode.InlayHintKind.Type : vscode.InlayHintKind.Parameter
        );
        
        if (lspHint.tooltip) {
            hint.tooltip = lspHint.tooltip;
        }
        
        hint.paddingLeft = lspHint.paddingLeft || false;
        hint.paddingRight = lspHint.paddingRight || false;
        
        return hint;
    }
    
    /**
     * Обновление настроек type hints
     */
    updateSettings(): void {
        // Уведомляем LSP сервер об изменении настроек
        // TODO: Реализовать отправку updated settings в LSP сервер
    }
}