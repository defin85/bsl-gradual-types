/**
 * Performance Monitor для VSCode Extension
 *
 * Мониторит производительность операций LSP и предоставляет
 * статистику для оптимизации пользовательского опыта
 */
import * as vscode from 'vscode';
export declare class PerformanceMonitor {
    private operationMetrics;
    private cacheMetrics;
    private outputChannel;
    private statusBarItem;
    private enabled;
    constructor(outputChannel: vscode.OutputChannel);
    /**
     * Включение мониторинга
     */
    enable(): void;
    /**
     * Отключение мониторинга
     */
    disable(): void;
    /**
     * Обновление cache hit rate
     */
    private updateCacheHitRate;
    /**
     * Обновление status bar
     */
    private updateStatusBar;
    /**
     * Генерация отчета о производительности
     */
    generateReport(): PerformanceReport;
    /**
     * Генерация рекомендаций по производительности
     */
    private generateRecommendations;
    /**
     * Очистка метрик
     */
    reset(): void;
    /**
     * Cleanup
     */
    dispose(): void;
}
interface OperationReport {
    name: string;
    totalCalls: number;
    averageTime: number;
    maxTime: number;
    minTime: number;
    recentAverage: number;
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
export {};
//# sourceMappingURL=performance-monitor.d.ts.map