/**
 * Simplified Enhanced Diagnostics Provider
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class EnhancedDiagnosticsProvider {
    private client;
    private outputChannel;
    private diagnosticsCollection;
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
//# sourceMappingURL=enhanced-diagnostics-simple.d.ts.map