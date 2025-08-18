/**
 * Simplified Type Hints Provider
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class TypeHintsProvider implements vscode.InlayHintsProvider {
    private client;
    constructor(client: EnhancedLspClient);
    provideInlayHints(document: vscode.TextDocument, range: vscode.Range, token: vscode.CancellationToken): Promise<vscode.InlayHint[]>;
}
//# sourceMappingURL=type-hints-simple.d.ts.map