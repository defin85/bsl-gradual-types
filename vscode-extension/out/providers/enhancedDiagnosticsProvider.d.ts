/**
 * Simplified Enhanced Diagnostics Provider
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class EnhancedDiagnosticsProvider {
    private outputChannel;
    constructor(client: EnhancedLspClient, outputChannel: vscode.OutputChannel);
    /**
     * Получение статистики диагностик
     */
    getDiagnosticsStats(): {
        errors: number;
        warnings: number;
        infos: number;
        hints: number;
    };
}
//# sourceMappingURL=enhancedDiagnosticsProvider.d.ts.map