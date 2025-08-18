"use strict";
/**
 * Enhanced LSP Client для BSL Gradual Type System
 *
 * Расширенный клиент для взаимодействия с enhanced LSP сервером,
 * поддерживающий новые возможности Phase 5.0
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
exports.EnhancedLspClient = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
// Custom request types для enhanced функциональности
var EnhancedRequests;
(function (EnhancedRequests) {
    EnhancedRequests.GetEnhancedHover = new node_1.RequestType('bsl/enhancedHover');
    EnhancedRequests.RequestPerformanceProfiling = new node_1.RequestType('bsl/performanceProfiling');
    EnhancedRequests.RequestProjectAnalysis = new node_1.RequestType('bsl/projectAnalysis');
    EnhancedRequests.RequestCacheClear = new node_1.RequestType('bsl/clearCache');
    EnhancedRequests.GetCacheStats = new node_1.RequestType('bsl/cacheStats');
})(EnhancedRequests || (EnhancedRequests = {}));
/**
 * Enhanced LSP клиент с поддержкой новых возможностей
 */
class EnhancedLspClient {
    constructor(serverPath, outputChannel, performanceMonitor) {
        this.client = null;
        this.serverPath = serverPath;
        this.outputChannel = outputChannel;
        this.performanceMonitor = performanceMonitor;
    }
    /**
     * Запуск enhanced LSP клиента
     */
    async start() {
        if (this.client) {
            await this.stop();
        }
        // Настройки сервера
        const serverOptions = {
            run: {
                command: this.serverPath,
                transport: node_1.TransportKind.stdio
            },
            debug: {
                command: this.serverPath,
                transport: node_1.TransportKind.stdio,
                args: ['--debug']
            }
        };
        // Настройки клиента с enhanced возможностями
        const clientOptions = {
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
            revealOutputChannelOn: node_1.RevealOutputChannelOn.Info,
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
        this.client = new node_1.LanguageClient('bslGradualTypes', 'BSL Gradual Type System', serverOptions, clientOptions);
        // Регистрируем обработчики enhanced уведомлений
        this.registerEnhancedHandlers();
        await this.client.start();
        this.outputChannel.appendLine('✅ Enhanced LSP client connected');
    }
    /**
     * Остановка LSP клиента
     */
    async stop() {
        if (this.client) {
            await this.client.stop();
            this.client = null;
            this.outputChannel.appendLine('🔄 Enhanced LSP client stopped');
        }
    }
    /**
     * Получение enhanced hover информации
     */
    async getEnhancedHover(uri, position) {
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
    async requestPerformanceProfiling(filePath) {
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
    async requestProjectAnalysis(projectPath, options) {
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
    async requestCacheClear() {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        return await this.client.sendRequest(EnhancedRequests.RequestCacheClear, {});
    }
    /**
     * Получение статистики кеша
     */
    async getCacheStats() {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        return await this.client.sendRequest(EnhancedRequests.GetCacheStats, {});
    }
    /**
     * Запрос code actions
     */
    async requestCodeActions(params) {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        return await this.client.sendRequest('textDocument/codeAction', params);
    }
    /**
     * Запрос inlay hints
     */
    async requestInlayHints(params) {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        return await this.client.sendRequest('textDocument/inlayHint', params);
    }
    /**
     * Запрос enhanced диагностик
     */
    async requestEnhancedDiagnostics(uri) {
        if (!this.client) {
            throw new Error('LSP client not initialized');
        }
        // TODO: Реализовать custom enhanced diagnostics request
        return [];
    }
    /**
     * Регистрация обработчиков enhanced уведомлений
     */
    registerEnhancedHandlers() {
        if (!this.client) {
            return;
        }
        // Уведомления о прогрессе analysis
        this.client.onNotification('bsl/analysisProgress', (params) => {
            this.outputChannel.appendLine(`📊 Analysis progress: ${params.message} (${params.percentage}%)`);
        });
        // Уведомления о cache events
        this.client.onNotification('bsl/cacheEvent', (params) => {
            if (params.type === 'hit') {
                this.performanceMonitor.recordCacheHit();
            }
            else if (params.type === 'miss') {
                this.performanceMonitor.recordCacheMiss();
            }
        });
        // Уведомления о performance warnings
        this.client.onNotification('bsl/performanceWarning', (params) => {
            this.outputChannel.appendLine(`⚠️ Performance warning: ${params.message}`);
            if (params.severity === 'high') {
                vscode.window.showWarningMessage(`BSL Performance Warning: ${params.message}`, 'Show Details').then(selection => {
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
    getCacheDirectory() {
        const config = vscode.workspace.getConfiguration('bsl');
        const customCacheDir = config.get('analysis.cacheDirectory');
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
    getPerformanceSettings() {
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
    getClientInfo() {
        return {
            isRunning: this.client !== null,
            serverPath: this.serverPath,
            cacheDirectory: this.getCacheDirectory(),
            performanceSettings: this.getPerformanceSettings()
        };
    }
}
exports.EnhancedLspClient = EnhancedLspClient;
//# sourceMappingURL=enhanced-client.js.map