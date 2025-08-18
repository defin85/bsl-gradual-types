/**
 * Type Hints Provider для VSCode
 *
 * Предоставляет inline type hints на основе результатов
 * flow-sensitive анализа и union types
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class TypeHintsProvider implements vscode.InlayHintsProvider {
    private client;
    constructor(client: EnhancedLspClient);
    provideInlayHints(document: vscode.TextDocument, range: vscode.Range, token: vscode.CancellationToken): Promise<vscode.InlayHint[]>;
    /**
     * Конвертация LSP inlay hint в VSCode формат
     */
    private convertToVSCodeInlayHint;
    /**
     * Обновление настроек type hints
     */
    updateSettings(): void;
}
//# sourceMappingURL=type-hints-provider.d.ts.map