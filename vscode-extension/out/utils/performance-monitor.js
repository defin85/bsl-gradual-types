"use strict";
/**
 * Performance Monitor для VSCode Extension
 *
 * Мониторит производительность операций LSP и предоставляет
 * статистику для оптимизации пользовательского опыта
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PerformanceMonitor = void 0;
const vscode = __importStar(require("vscode"));
class PerformanceMonitor {
    constructor(outputChannel) {
        this.operationMetrics = new Map();
        this.cacheMetrics = { hits: 0, misses: 0, hitRate: 0 };
        this.enabled = false;
        this.outputChannel = outputChannel;
        // Создаем status bar item для performance info
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 10);
        this.statusBarItem.command = 'bsl.showPerformanceStats';
        this.updateStatusBar();
    }
    /**
     * Включение мониторинга
     */
    enable() {
        this.enabled = true;
        this.outputChannel.appendLine('📊 Performance monitoring enabled');
        this.statusBarItem.show();
    }
    /**
     * Отключение мониторинга
     */
    disable() {
        this.enabled = false;
        this.statusBarItem.hide();
        this.outputChannel.appendLine('📊 Performance monitoring disabled');
    }
    /**
     * Запись операции LSP
     */
    recordLspOperation(operation, durationMs) {
        if (!this.enabled) {
            return;
        }
        const metrics = this.operationMetrics.get(operation) || {
            totalCalls: 0,
            totalTime: 0,
            averageTime: 0,
            maxTime: 0,
            minTime: Infinity,
            recentTimes: []
        };
        metrics.totalCalls++;
        metrics.totalTime += durationMs;
        metrics.averageTime = metrics.totalTime / metrics.totalCalls;
        metrics.maxTime = Math.max(metrics.maxTime, durationMs);
        metrics.minTime = Math.min(metrics.minTime, durationMs);
        // Сохраняем последние 10 измерений
        metrics.recentTimes.push(durationMs);
        if (metrics.recentTimes.length > 10) {
            metrics.recentTimes.shift();
        }
        this.operationMetrics.set(operation, metrics);
        // Обновляем status bar если операция медленная
        if (durationMs > 1000) { // > 1 секунды
            this.updateStatusBar();
            this.outputChannel.appendLine(`⚠️ Slow operation detected: ${operation} took ${durationMs}ms`);
        }
    }
    /**
     * Запись cache hit
     */
    recordCacheHit() {
        if (!this.enabled) {
            return;
        }
        this.cacheMetrics.hits++;
        this.updateCacheHitRate();
    }
    /**
     * Запись cache miss
     */
    recordCacheMiss() {
        if (!this.enabled) {
            return;
        }
        this.cacheMetrics.misses++;
        this.updateCacheHitRate();
    }
    /**
     * Обновление cache hit rate
     */
    updateCacheHitRate() {
        const total = this.cacheMetrics.hits + this.cacheMetrics.misses;
        this.cacheMetrics.hitRate = total > 0 ? this.cacheMetrics.hits / total : 0;
    }
    /**
     * Обновление status bar
     */
    updateStatusBar() {
        if (!this.enabled) {
            this.statusBarItem.hide();
            return;
        }
        const hoverMetrics = this.operationMetrics.get('hover');
        const completionMetrics = this.operationMetrics.get('completion');
        let text = '$(graph) BSL';
        let tooltip = 'BSL Performance Statistics\n\n';
        if (hoverMetrics) {
            const avgHover = Math.round(hoverMetrics.averageTime);
            text += ` H:${avgHover}ms`;
            tooltip += `Hover: ${avgHover}ms avg (${hoverMetrics.totalCalls} calls)\n`;
        }
        if (completionMetrics) {
            const avgCompletion = Math.round(completionMetrics.averageTime);
            text += ` C:${avgCompletion}ms`;
            tooltip += `Completion: ${avgCompletion}ms avg (${completionMetrics.totalCalls} calls)\n`;
        }
        if (this.cacheMetrics.hits + this.cacheMetrics.misses > 0) {
            const hitRatePercent = Math.round(this.cacheMetrics.hitRate * 100);
            text += ` 🗄️${hitRatePercent}%`;
            tooltip += `Cache hit rate: ${hitRatePercent}%\n`;
        }
        this.statusBarItem.text = text;
        this.statusBarItem.tooltip = tooltip;
        this.statusBarItem.show();
    }
    /**
     * Запись операции LSP (placeholder)
     */
    recordLspOperation(operation, durationMs) {
        // Простая заглушка
        console.log(`LSP Operation: ${operation} took ${durationMs}ms`);
    }
    /**
     * Запись cache hit
     */
    recordCacheHit() {
        this.cacheMetrics.hits++;
        this.updateCacheHitRate();
    }
    /**
     * Запись cache miss
     */
    recordCacheMiss() {
        this.cacheMetrics.misses++;
        this.updateCacheHitRate();
    }
    /**
     * Генерация отчета о производительности
     */
    generateReport() {
        const operations = [];
        for (const [operation, metrics] of this.operationMetrics) {
            operations.push({
                name: operation,
                totalCalls: metrics.totalCalls,
                averageTime: metrics.averageTime,
                maxTime: metrics.maxTime,
                minTime: metrics.minTime === Infinity ? 0 : metrics.minTime,
                recentAverage: metrics.recentTimes.length > 0
                    ? metrics.recentTimes.reduce((a, b) => a + b, 0) / metrics.recentTimes.length
                    : 0
            });
        }
        return {
            operations,
            cache: {
                hits: this.cacheMetrics.hits,
                misses: this.cacheMetrics.misses,
                hitRate: this.cacheMetrics.hitRate
            },
            recommendations: this.generateRecommendations()
        };
    }
    /**
     * Генерация рекомендаций по производительности
     */
    generateRecommendations() {
        const recommendations = [];
        // Проверяем медленные операции
        for (const [operation, metrics] of this.operationMetrics) {
            if (metrics.averageTime > 500) { // > 500ms
                recommendations.push(`⚠️ Operation '${operation}' is slow (${Math.round(metrics.averageTime)}ms avg). ` +
                    `Consider enabling caching or reducing file size.`);
            }
        }
        // Проверяем cache performance
        if (this.cacheMetrics.hitRate < 0.5 && (this.cacheMetrics.hits + this.cacheMetrics.misses) > 10) {
            recommendations.push(`🗄️ Low cache hit rate (${Math.round(this.cacheMetrics.hitRate * 100)}%). ` +
                `Consider reviewing cache settings or increasing cache size.`);
        }
        // Проверяем общую производительность
        const hoverMetrics = this.operationMetrics.get('hover');
        if (hoverMetrics && hoverMetrics.averageTime > 200) {
            recommendations.push(`🔍 Hover responses are slow (${Math.round(hoverMetrics.averageTime)}ms). ` +
                `This may impact user experience.`);
        }
        if (recommendations.length === 0) {
            recommendations.push('✅ Performance looks good! No issues detected.');
        }
        return recommendations;
    }
    /**
     * Очистка метрик
     */
    reset() {
        this.operationMetrics.clear();
        this.cacheMetrics = { hits: 0, misses: 0, hitRate: 0 };
        this.updateStatusBar();
        this.outputChannel.appendLine('🔄 Performance metrics reset');
    }
    /**
     * Cleanup
     */
    dispose() {
        this.statusBarItem.dispose();
    }
}
exports.PerformanceMonitor = PerformanceMonitor;
//# sourceMappingURL=performance-monitor.js.map