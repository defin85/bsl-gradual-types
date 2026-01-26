/**
 * Simplified Enhanced Diagnostics Provider
 */

import * as vscode from 'vscode';
import * as path from 'path';
import { EnhancedLspClient } from '../lsp/enhanced-client';

export class EnhancedDiagnosticsProvider {
    private outputChannel: vscode.OutputChannel;
    
    constructor(client: EnhancedLspClient, outputChannel: vscode.OutputChannel) {
        void client;
        this.outputChannel = outputChannel;
    }
    
    /**
     * Получение статистики диагностик
     */
    getDiagnosticsStats() {
        let errors = 0;
        let warnings = 0;
        let infos = 0;
        let hints = 0;

        // Диагностики приходят в VS Code через стандартный LSP pipeline.
        // Здесь считаем статистику по workspace diagnostics и фильтруем по BSL файлам.
        const all = vscode.languages.getDiagnostics();
        for (const [uri, diagnostics] of all) {
            if (uri.scheme !== 'file') continue;
            const ext = path.extname(uri.fsPath).toLowerCase();
            if (ext !== '.bsl' && ext !== '.os') continue;

            for (const d of diagnostics) {
                switch (d.severity) {
                    case vscode.DiagnosticSeverity.Error:
                        errors += 1;
                        break;
                    case vscode.DiagnosticSeverity.Warning:
                        warnings += 1;
                        break;
                    case vscode.DiagnosticSeverity.Information:
                        infos += 1;
                        break;
                    case vscode.DiagnosticSeverity.Hint:
                        hints += 1;
                        break;
                }
            }
        }

        return { errors, warnings, infos, hints };
    }
}
