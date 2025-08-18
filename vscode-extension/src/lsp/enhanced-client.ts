/**
 * Enhanced LSP Client для BSL Gradual Type System
 * 
 * Расширенный клиент для взаимодействия с enhanced LSP сервером,
 * поддерживающий новые возможности Phase 5.0
 */

import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    RevealOutputChannelOn,
    RequestType,
    NotificationType
} from 'vscode-languageclient/node';

import { PerformanceMonitor } from '../utils/performance-monitor';

// Custom request types для enhanced функциональности
namespace EnhancedRequests {
    export const GetEnhancedHover = new RequestType<
        { uri: string; position: vscode.Position },
        { contents: { value: string } } | null,
        void
    >('bsl/enhancedHover');
    
    export const RequestPerformanceProfiling = new RequestType<
        { filePath: string },
        { humanReadableReport: string; jsonReport: string },
        void
    >('bsl/performanceProfiling');
    
    export const RequestProjectAnalysis = new RequestType<
        { projectPath: string; options: ProjectAnalysisOptions },
        ProjectAnalysisResult,
        void
    >('bsl/projectAnalysis');
    
    export const RequestCacheClear = new RequestType<
        {},
        { success: boolean; freedBytes: number },
        void
    >('bsl/clearCache');
    
    export const GetCacheStats = new RequestType<
        {},
        CacheStatsResult,
        void
    >('bsl/cacheStats');
}

// Типы для enhanced функциональности
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
export class EnhancedLspClient {
    private client: LanguageClient | null = null;
    private serverPath: string;
    private outputChannel: vscode.OutputChannel;
    private performanceMonitor: PerformanceMonitor;
    
    constructor(
        serverPath: string,
        outputChannel: vscode.OutputChannel,
        performanceMonitor: PerformanceMonitor
    ) {
        this.serverPath = serverPath;
        this.outputChannel = outputChannel;
        this.performanceMonitor = performanceMonitor;
    }
    
    /**
     * Запуск enhanced LSP клиента
     */
    async start(): Promise<void> {
        if (this.client) {
            await this.stop();
        }
        
        // Настройки сервера
        const serverOptions: ServerOptions = {
            run: {
                command: this.serverPath,
                transport: TransportKind.stdio
            },
            debug: {
                command: this.serverPath,
                transport: TransportKind.stdio,
                args: ['--debug']
            }
        };
        
        // Настройки клиента с enhanced возможностями
        const clientOptions: LanguageClientOptions = {
            documentSelector: [
                { scheme: 'file', language: 'bsl' },
                { scheme: 'file', pattern: '**/*.bsl' },
                { scheme: 'file', pattern: '**/*.os' }
            ],
            synchronize: {
                configurationSection: 'bsl',
                fileEvents: [
                    vscode.workspace.createFileSystemWatcher('**/*.bsl'),
                    vscode.workspace.createFileSystemWatcher('**/*.os'),
                    vscode.workspace.createFileSystemWatcher('**/Configuration.xml')
                ]
            },
            outputChannel: this.outputChannel,
            revealOutputChannelOn: RevealOutputChannelOn.Info,
            
            // Enhanced capabilities
            initializationOptions: {
                enableFlowSensitiveAnalysis: true,
                enableUnionTypes: true,
                enableInterproceduralAnalysis: true,
                enableTypeHints: true,
                enableCodeActions: true,
                cacheDirectory: this.getCacheDirectory(),
                performanceProfiling: this.getPerformanceSettings()
            },
            
            // Middleware для логирования performance
            middleware: {
                handleDiagnostics: (uri, diagnostics, next) => {
                    const startTime = Date.now();
                    next(uri, diagnostics);
                    const duration = Date.now() - startTime;
                    this.performanceMonitor.recordLspOperation('diagnostics', duration);
                },
                
                provideHover: async (document, position, token, next) => {
                    const startTime = Date.now();
                    const result = await next(document, position, token);
                    const duration = Date.now() - startTime;
                    this.performanceMonitor.recordLspOperation('hover', duration);
                    return result;
                },
                
                provideCompletionItem: async (document, position, context, token, next) => {
                    const startTime = Date.now();
                    const result = await next(document, position, context, token);
                    const duration = Date.now() - startTime;
                    this.performanceMonitor.recordLspOperation('completion', duration);
                    return result;
                }
            }
        };
        
        // Создаем и запускаем клиент
        this.client = new LanguageClient(
            'bslGradualTypes',
            'BSL Gradual Type System',
            serverOptions,
            clientOptions
        );
        
        // Регистрируем обработчики enhanced уведомлений
        this.registerEnhancedHandlers();
        
        await this.client.start();
        
        this.outputChannel.appendLine('✅ Enhanced LSP client connected');
    }
    
    /**
     * Остановка LSP клиента
     */
    async stop(): Promise<void> {
        if (this.client) {
            await this.client.stop();
            this.client = null;
            this.outputChannel.appendLine('🔄 Enhanced LSP client stopped');
        }
    }
    
    /**
     * Получение enhanced hover информации
     */
    async getEnhancedHover(uri: string, position: vscode.Position): Promise<any> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest(EnhancedRequests.GetEnhancedHover, {
            uri,
            position
        });
    }
    
    /**
     * Запрос performance profiling
     */
    async requestPerformanceProfiling(filePath: string): Promise<any> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest(EnhancedRequests.RequestPerformanceProfiling, {
            filePath
        });
    }
    
    /**
     * Запрос анализа проекта
     */
    async requestProjectAnalysis(
        projectPath: string, 
        options: ProjectAnalysisOptions
    ): Promise<ProjectAnalysisResult> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest(EnhancedRequests.RequestProjectAnalysis, {
            projectPath,
            options
        });
    }
    
    /**
     * Очистка кеша
     */
    async requestCacheClear(): Promise<any> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest(EnhancedRequests.RequestCacheClear, {});
    }
    
    /**
     * Получение статистики кеша
     */
    async getCacheStats(): Promise<CacheStatsResult> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest(EnhancedRequests.GetCacheStats, {});
    }
    
    /**
     * Запрос code actions
     */
    async requestCodeActions(params: any): Promise<any> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest('textDocument/codeAction', params);
    }
    
    /**
     * Запрос inlay hints
     */
    async requestInlayHints(params: any): Promise<any> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        return await this.client.sendRequest('textDocument/inlayHint', params);
    }
    
    /**
     * Запрос enhanced диагностик
     */
    async requestEnhancedDiagnostics(uri: string): Promise<any> {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        
        // TODO: Реализовать custom enhanced diagnostics request
        return [];
    }
    
    /**
     * Регистрация обработчиков enhanced уведомлений
     */
    private registerEnhancedHandlers(): void {
        if (!this.client) {
            return;
        }
        
        // Уведомления о прогрессе analysis
        this.client.onNotification('bsl/analysisProgress', (params: any) => {
            this.outputChannel.appendLine(`📊 Analysis progress: ${params.message} (${params.percentage}%)`);
        });
        
        // Уведомления о cache events
        this.client.onNotification('bsl/cacheEvent', (params: any) => {
            if (params.type === 'hit') {
                this.performanceMonitor.recordCacheHit();
            } else if (params.type === 'miss') {
                this.performanceMonitor.recordCacheMiss();
            }
        });
        
        // Уведомления о performance warnings
        this.client.onNotification('bsl/performanceWarning', (params: any) => {
            this.outputChannel.appendLine(`⚠️ Performance warning: ${params.message}`);
            
            if (params.severity === 'high') {
                vscode.window.showWarningMessage(
                    `BSL Performance Warning: ${params.message}`,
                    'Show Details'
                ).then(selection => {
                    if (selection === 'Show Details') {
                        this.outputChannel.show();
                    }
                });
            }
        });
    }
    
    /**
     * Получение директории кеша
     */
    private getCacheDirectory(): string {
        const config = vscode.workspace.getConfiguration('bsl');
        const customCacheDir = config.get<string>('analysis.cacheDirectory');
        
        if (customCacheDir) {
            return customCacheDir;
        }
        
        // Используем стандартную директорию в workspace
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (workspaceFolder) {
            return vscode.Uri.joinPath(workspaceFolder.uri, '.bsl_cache').fsPath;
        }
        
        // Fallback к temp директории
        return vscode.Uri.joinPath(vscode.Uri.file(require('os').tmpdir()), '.bsl_cache').fsPath;
    }
    
    /**
     * Получение настроек производительности
     */
    private getPerformanceSettings(): any {
        const config = vscode.workspace.getConfiguration('bsl.performance');
        
        return {
            enableProfiling: config.get('enableProfiling', false),
            maxMemoryUsageMB: config.get('maxMemoryUsageMB', 512),
            parallelAnalysisThreads: config.get('parallelAnalysisThreads', 'auto'),
            cacheLifetimeMinutes: config.get('cacheLifetimeMinutes', 60)
        };
    }
    
    /**
     * Получение информации о состоянии клиента
     */
    getClientInfo(): any {
        return {
            isRunning: this.client !== null,
            serverPath: this.serverPath,
            cacheDirectory: this.getCacheDirectory(),
            performanceSettings: this.getPerformanceSettings()
        };
    }
}