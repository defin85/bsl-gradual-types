/**
 * Enhanced LSP Client для BSL Gradual Type System
 *
 * Расширенный клиент для взаимодействия с enhanced LSP сервером,
 * поддерживающий новые возможности Phase 5.0
 */
import * as vscode from 'vscode';
import { PerformanceMonitor } from '../utils/performance-monitor';
interface ProjectAnalysisOptions {
    useParallelAnalysis: boolean;
    enableCaching: boolean;
    showProgress: boolean;
    numThreads?: number;
}
interface ProjectAnalysisResult {
    stats: {
        totalFiles: number;
        successfulFiles: number;
        totalFunctions: number;
        totalVariables: number;
        totalDiagnostics: number;
        avgAnalysisTime: string;
    };
    totalTime: string;
    files: Array<{
        path: string;
        success: boolean;
        analysisTime: string;
        diagnosticsCount: number;
    }>;
}
interface CacheStatsResult {
    memoryEntries: number;
    diskSizeBytes: number;
    hitRate: number;
    totalHits: number;
    totalMisses: number;
}
/**
 * Enhanced LSP клиент с поддержкой новых возможностей
 */
export declare class EnhancedLspClient {
    private client;
    private serverPath;
    private outputChannel;
    private performanceMonitor;
    constructor(serverPath: string, outputChannel: vscode.OutputChannel, performanceMonitor: PerformanceMonitor);
    /**
     * Запуск enhanced LSP клиента
     */
    start(): Promise<void>;
    /**
     * Остановка LSP клиента
     */
    stop(): Promise<void>;
    /**
     * Получение enhanced hover информации
     */
    getEnhancedHover(uri: string, position: vscode.Position): Promise<any>;
    /**
     * Запрос performance profiling
     */
    requestPerformanceProfiling(filePath: string): Promise<any>;
    /**
     * Запрос анализа проекта
     */
    requestProjectAnalysis(projectPath: string, options: ProjectAnalysisOptions): Promise<ProjectAnalysisResult>;
    /**
     * Очистка кеша
     */
    requestCacheClear(): Promise<any>;
    /**
     * Получение статистики кеша
     */
    getCacheStats(): Promise<CacheStatsResult>;
    /**
     * Запрос code actions
     */
    requestCodeActions(params: any): Promise<any>;
    /**
     * Запрос inlay hints
     */
    requestInlayHints(params: any): Promise<any>;
    /**
     * Запрос enhanced диагностик
     */
    requestEnhancedDiagnostics(uri: string): Promise<any>;
    /**
     * Регистрация обработчиков enhanced уведомлений
     */
    private registerEnhancedHandlers;
    /**
     * Получение директории кеша
     */
    private getCacheDirectory;
    /**
     * Получение настроек производительности
     */
    private getPerformanceSettings;
    /**
     * Получение информации о состоянии клиента
     */
    getClientInfo(): any;
}
export {};
//# sourceMappingURL=enhanced-client.d.ts.map