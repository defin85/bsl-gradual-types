/**
 * Enhanced Diagnostics Provider
 *
 * Предоставляет расширенную диагностику с поддержкой
 * flow-sensitive анализа и union types
 */
import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
export declare class EnhancedDiagnosticsProvider {
    private client;
    private outputChannel;
    private diagnosticsCollection;
    private diagnosticsStats;
    constructor(client: EnhancedLspClient, outputChannel: vscode.OutputChannel);
    /**
     * Настройка обработки диагностик
     */
    private setupDiagnosticsHandling;
    /**
     * Обновление диагностик для документа
     */
    private updateDiagnosticsForDocument;
    /**
     * Конвертация LSP диагностики в VSCode формат
     */
    private convertDiagnostic;
    /**
     * Конвертация severity
     */
    private convertSeverity;
    /**
     * Обновление статистики диагностик
     */
    private updateDiagnosticsStats;
    /**
     * Обновление status bar с диагностиками
     */
    private updateStatusBar;
    /**
     * Генерация tooltip для status bar
     */
    private generateTooltip;
    /**
     * Показать детальную статистику
     */
    showDetailedStats(): void;
    /**
     * Генерация HTML для статистики
     */
    private generateStatsHtml;
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
//# sourceMappingURL=enhanced-diagnostics-provider.d.ts.map