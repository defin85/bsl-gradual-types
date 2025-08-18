/**
 * Enhanced Diagnostics Provider
 * 
 * Предоставляет расширенную диагностику с поддержкой
 * flow-sensitive анализа и union types
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';

export class EnhancedDiagnosticsProvider {
    private client: EnhancedLspClient;
    private outputChannel: vscode.OutputChannel;
    private diagnosticsCollection: vscode.DiagnosticCollection;
    
    // Счетчики диагностик
    private diagnosticsStats = {
        errors: 0,
        warnings: 0,
        infos: 0,
        hints: 0
    };
    
    constructor(client: EnhancedLspClient, outputChannel: vscode.OutputChannel) {
        this.client = client;
        this.outputChannel = outputChannel;
        this.diagnosticsCollection = vscode.languages.createDiagnosticCollection('bsl-gradual-types');
        
        // Подписываемся на изменения диагностик от LSP
        this.setupDiagnosticsHandling();
    }
    
    /**
     * Настройка обработки диагностик
     */
    private setupDiagnosticsHandling(): void {
        // TODO: Интеграция с LSP клиентом для получения enhanced диагностик
        
        // Подписываемся на изменения документов для real-time анализа
        vscode.workspace.onDidChangeTextDocument(async (event) => {
            if (event.document.languageId === 'bsl') {
                await this.updateDiagnosticsForDocument(event.document);
            }
        });
        
        vscode.workspace.onDidOpenTextDocument(async (document) => {
            if (document.languageId === 'bsl') {
                await this.updateDiagnosticsForDocument(document);
            }
        });
        
        vscode.workspace.onDidCloseTextDocument((document) => {
            if (document.languageId === 'bsl') {
                this.diagnosticsCollection.delete(document.uri);
                this.updateDiagnosticsStats();
            }
        });
    }
    
    /**
     * Обновление диагностик для документа
     */
    private async updateDiagnosticsForDocument(document: vscode.TextDocument): Promise<void> {
        try {
            // Запрашиваем enhanced диагностики от LSP сервера
            const diagnostics = await this.client.requestEnhancedDiagnostics(document.uri.toString());
            
            if (diagnostics) {
                const vcodeDiagnostics = diagnostics.map(d => this.convertDiagnostic(d));
                this.diagnosticsCollection.set(document.uri, vcodeDiagnostics);
                this.updateDiagnosticsStats();
            }
            
        } catch (error) {
            this.outputChannel.appendLine(`❌ Error updating diagnostics: ${error}`);
        }
    }
    
    /**
     * Конвертация LSP диагностики в VSCode формат
     */
    private convertDiagnostic(lspDiag: any): vscode.Diagnostic {
        const range = new vscode.Range(
            lspDiag.range.start.line,
            lspDiag.range.start.character,
            lspDiag.range.end.line,
            lspDiag.range.end.character
        );
        
        const severity = this.convertSeverity(lspDiag.severity);
        const diagnostic = new vscode.Diagnostic(range, lspDiag.message, severity);
        
        diagnostic.source = 'bsl-gradual-types';
        diagnostic.code = lspDiag.code;
        
        // Добавляем related information для enhanced диагностик
        if (lspDiag.relatedInformation) {
            diagnostic.relatedInformation = lspDiag.relatedInformation.map((info: any) => ({
                location: new vscode.Location(
                    vscode.Uri.parse(info.location.uri),
                    new vscode.Range(
                        info.location.range.start.line,
                        info.location.range.start.character,
                        info.location.range.end.line,
                        info.location.range.end.character
                    )
                ),
                message: info.message
            }));
        }
        
        // Добавляем tags для специальных типов диагностик
        if (lspDiag.tags) {
            diagnostic.tags = lspDiag.tags.map((tag: number) => {
                switch (tag) {
                    case 1: return vscode.DiagnosticTag.Unnecessary;
                    case 2: return vscode.DiagnosticTag.Deprecated;
                    default: return undefined;
                }
            }).filter(Boolean);
        }
        
        return diagnostic;
    }
    
    /**
     * Конвертация severity
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
    
    /**
     * Обновление статистики диагностик
     */
    private updateDiagnosticsStats(): void {
        this.diagnosticsStats = { errors: 0, warnings: 0, infos: 0, hints: 0 };
        
        this.diagnosticsCollection.forEach((uri, diagnostics) => {
            for (const diagnostic of diagnostics) {
                switch (diagnostic.severity) {
                    case vscode.DiagnosticSeverity.Error:
                        this.diagnosticsStats.errors++;
                        break;
                    case vscode.DiagnosticSeverity.Warning:
                        this.diagnosticsStats.warnings++;
                        break;
                    case vscode.DiagnosticSeverity.Information:
                        this.diagnosticsStats.infos++;
                        break;
                    case vscode.DiagnosticSeverity.Hint:
                        this.diagnosticsStats.hints++;
                        break;
                }
            }
        });
        
        this.updateStatusBar();
    }
    
    /**
     * Обновление status bar с диагностиками
     */
    private updateStatusBar(): void {
        const { errors, warnings, infos, hints } = this.diagnosticsStats;
        const total = errors + warnings + infos + hints;
        
        if (total === 0) {
            this.statusBarItem.text = '$(check) BSL: No issues';
        } else {
            let text = '$(warning) BSL:';
            if (errors > 0) text += ` ${errors}❌`;
            if (warnings > 0) text += ` ${warnings}⚠️`;
            if (infos > 0) text += ` ${infos}ℹ️`;
            if (hints > 0) text += ` ${hints}💡`;
            
            this.statusBarItem.text = text;
        }
        
        this.statusBarItem.tooltip = this.generateTooltip();
        this.statusBarItem.show();
    }
    
    /**
     * Генерация tooltip для status bar
     */
    private generateTooltip(): string {
        const { errors, warnings, infos, hints } = this.diagnosticsStats;
        
        let tooltip = 'BSL Gradual Type System\n\n';
        tooltip += `Diagnostics:\n`;
        tooltip += `  Errors: ${errors}\n`;
        tooltip += `  Warnings: ${warnings}\n`;
        tooltip += `  Info: ${infos}\n`;
        tooltip += `  Hints: ${hints}\n\n`;
        
        // Добавляем performance info
        const hoverMetrics = this.operationMetrics.get('hover');
        const completionMetrics = this.operationMetrics.get('completion');
        
        if (hoverMetrics || completionMetrics) {
            tooltip += 'Performance:\n';
            if (hoverMetrics) {
                tooltip += `  Hover: ${Math.round(hoverMetrics.averageTime)}ms avg\n`;
            }
            if (completionMetrics) {
                tooltip += `  Completion: ${Math.round(completionMetrics.averageTime)}ms avg\n`;
            }
        }
        
        // Добавляем cache info
        if (this.cacheMetrics.hits + this.cacheMetrics.misses > 0) {
            tooltip += `\nCache: ${Math.round(this.cacheMetrics.hitRate * 100)}% hit rate`;
        }
        
        tooltip += '\n\nClick to show detailed performance stats';
        
        return tooltip;
    }
    
    /**
     * Показать детальную статистику
     */
    showDetailedStats(): void {
        const report = this.generateReport();
        
        const panel = vscode.window.createWebviewPanel(
            'bslPerformanceStats',
            'BSL Performance Statistics',
            vscode.ViewColumn.Active,
            { enableScripts: true }
        );
        
        panel.webview.html = this.generateStatsHtml(report);
    }
    
    /**
     * Генерация HTML для статистики
     */
    private generateStatsHtml(report: PerformanceReport): string {
        let operationsHtml = '';
        for (const op of report.operations) {
            operationsHtml += `
                <tr>
                    <td>${op.name}</td>
                    <td>${op.totalCalls}</td>
                    <td>${Math.round(op.averageTime)}ms</td>
                    <td>${Math.round(op.maxTime)}ms</td>
                    <td>${Math.round(op.recentAverage)}ms</td>
                </tr>
            `;
        }
        
        return `
            <!DOCTYPE html>
            <html>
            <head>
                <meta charset="UTF-8">
                <title>BSL Performance Statistics</title>
                <style>
                    body { font-family: var(--vscode-font-family); padding: 20px; }
                    table { width: 100%; border-collapse: collapse; margin: 20px 0; }
                    th, td { text-align: left; padding: 8px; border-bottom: 1px solid var(--vscode-panel-border); }
                    th { background: var(--vscode-editor-background); font-weight: bold; }
                    .cache-stats { background: var(--vscode-textBlockQuote-background); padding: 15px; margin: 20px 0; }
                    .recommendations { background: var(--vscode-inputValidation-warningBackground); padding: 15px; margin: 20px 0; }
                </style>
            </head>
            <body>
                <h1>📊 BSL Performance Statistics</h1>
                
                <h2>LSP Operations</h2>
                <table>
                    <thead>
                        <tr>
                            <th>Operation</th>
                            <th>Total Calls</th>
                            <th>Average Time</th>
                            <th>Max Time</th>
                            <th>Recent Average</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${operationsHtml}
                    </tbody>
                </table>
                
                <div class="cache-stats">
                    <h2>🗄️ Cache Statistics</h2>
                    <p><strong>Hits:</strong> ${report.cache.hits}</p>
                    <p><strong>Misses:</strong> ${report.cache.misses}</p>
                    <p><strong>Hit Rate:</strong> ${Math.round(report.cache.hitRate * 100)}%</p>
                </div>
                
                <div class="recommendations">
                    <h2>💡 Recommendations</h2>
                    ${report.recommendations.map(rec => `<p>${rec}</p>`).join('')}
                </div>
            </body>
            </html>
        `;
    }
    
    /**
     * Получение статистики диагностик
     */
    getDiagnosticsStats() {
        return { ...this.diagnosticsStats };
    }
}

interface PerformanceReport {
    operations: OperationReport[];
    cache: {
        hits: number;
        misses: number;
        hitRate: number;
    };
    recommendations: string[];
}