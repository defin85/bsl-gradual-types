/**
 * Enhanced Code Actions Provider
 * 
 * Предоставляет автоматические исправления и рефакторинг
 * на основе результатов продвинутого анализа типов
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
            // Запрашиваем code actions от enhanced LSP сервера
            const result = await this.client.requestCodeActions({
                textDocument: { uri: document.uri.toString() },
                range: {
                    start: { line: range.start.line, character: range.start.character },
                    end: { line: range.end.line, character: range.end.character }
                },
                context: {
                    diagnostics: context.diagnostics.map(d => this.convertDiagnostic(d)),
                    only: context.only?.map(kind => kind.value),
                    triggerKind: context.triggerKind
                }
            });
            
            if (!result || !Array.isArray(result)) {
                return [];
            }
            
            return result
                .filter(action => action.title) // Фильтруем валидные actions
                .map(action => this.convertToVSCodeAction(action, document));
            
        } catch (error) {
            console.error('Error providing code actions:', error);
            return [];
        }
    }
    
    /**
     * Конвертация VSCode диагностики в LSP формат
     */
    private convertDiagnostic(diagnostic: vscode.Diagnostic): any {
        return {
            range: {
                start: { 
                    line: diagnostic.range.start.line, 
                    character: diagnostic.range.start.character 
                },
                end: { 
                    line: diagnostic.range.end.line, 
                    character: diagnostic.range.end.character 
                }
            },
            severity: diagnostic.severity,
            message: diagnostic.message,
            source: diagnostic.source
        };
    }
    
    /**
     * Конвертация LSP code action в VSCode формат
     */
    private convertToVSCodeAction(lspAction: any, document: vscode.TextDocument): vscode.CodeAction {
        const action = new vscode.CodeAction(lspAction.title, this.convertKind(lspAction.kind));
        
        // Конвертируем edit если есть
        if (lspAction.edit) {
            action.edit = this.convertWorkspaceEdit(lspAction.edit, document);
        }
        
        // Конвертируем command если есть
        if (lspAction.command) {
            action.command = {
                title: lspAction.command.title,
                command: lspAction.command.command,
                arguments: lspAction.command.arguments
            };
        }
        
        action.isPreferred = lspAction.isPreferred || false;
        
        // Добавляем диагностики
        if (lspAction.diagnostics) {
            action.diagnostics = lspAction.diagnostics.map((d: any) => this.convertLspDiagnostic(d));
        }
        
        return action;
    }
    
    /**
     * Конвертация LSP code action kind в VSCode формат
     */
    private convertKind(lspKind: string): vscode.CodeActionKind {
        switch (lspKind) {
            case 'quickfix':
                return vscode.CodeActionKind.QuickFix;
            case 'refactor':
                return vscode.CodeActionKind.Refactor;
            case 'refactor.extract':
                return vscode.CodeActionKind.RefactorExtract;
            case 'refactor.inline':
                return vscode.CodeActionKind.RefactorInline;
            case 'source':
                return vscode.CodeActionKind.Source;
            default:
                return vscode.CodeActionKind.Empty;
        }
    }
    
    /**
     * Конвертация LSP workspace edit в VSCode формат
     */
    private convertWorkspaceEdit(lspEdit: any, document: vscode.TextDocument): vscode.WorkspaceEdit {
        const edit = new vscode.WorkspaceEdit();
        
        if (lspEdit.changes) {
            for (const [uri, textEdits] of Object.entries(lspEdit.changes)) {
                const vscodeUri = vscode.Uri.parse(uri);
                const edits = (textEdits as any[]).map(textEdit => 
                    new vscode.TextEdit(
                        new vscode.Range(
                            textEdit.range.start.line,
                            textEdit.range.start.character,
                            textEdit.range.end.line,
                            textEdit.range.end.character
                        ),
                        textEdit.newText
                    )
                );
                edit.set(vscodeUri, edits);
            }
        }
        
        return edit;
    }
    
    /**
     * Конвертация LSP диагностики в VSCode формат
     */
    private convertLspDiagnostic(lspDiag: any): vscode.Diagnostic {
        const range = new vscode.Range(
            lspDiag.range.start.line,
            lspDiag.range.start.character,
            lspDiag.range.end.line,
            lspDiag.range.end.character
        );
        
        const severity = this.convertSeverity(lspDiag.severity);
        
        const diagnostic = new vscode.Diagnostic(range, lspDiag.message, severity);
        diagnostic.source = lspDiag.source || 'bsl-gradual-types';
        
        return diagnostic;
    }
    
    /**
     * Конвертация LSP severity в VSCode формат
     */
    private convertSeverity(lspSeverity: number): vscode.DiagnosticSeverity {
        switch (lspSeverity) {
            case 1: return vscode.DiagnosticSeverity.Error;
            case 2: return vscode.DiagnosticSeverity.Warning;
            case 3: return vscode.DiagnosticSeverity.Information;
            case 4: return vscode.DiagnosticSeverity.Hint;
            default: return vscode.DiagnosticSeverity.Information;
        }
    }
}