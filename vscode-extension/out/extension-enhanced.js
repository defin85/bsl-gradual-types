"use strict";
/**
 * BSL Gradual Type System - Enhanced VSCode Extension
 *
 * Интегрирует VSCode с enhanced LSP сервером, предоставляя:
 * - Flow-sensitive type analysis
 * - Union types с инкрементальным парсингом
 * - Code actions и type hints
 * - Performance profiling integration
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
exports.getEnhancedPackageContributions = exports.deactivate = exports.activate = void 0;
const vscode = __importStar(require("vscode"));
// Enhanced imports для новой функциональности
const enhanced_client_1 = require("./lsp/enhanced-client");
const type_hints_simple_1 = require("./providers/type-hints-simple");
const code_actions_simple_1 = require("./providers/code-actions-simple");
const performance_monitor_1 = require("./utils/performance-monitor");
const enhanced_diagnostics_simple_1 = require("./providers/enhanced-diagnostics-simple");
// Импорты из старого проекта (адаптированные)
const configHelper_1 = require("./config/configHelper");
const binaryPath_1 = require("./utils/binaryPath");
// Глобальные переменные
let languageClient = null;
let outputChannel;
let statusBarItem;
let performanceMonitor;
let extensionContext;
// Providers
let typeHintsProvider;
let codeActionsProvider;
let diagnosticsProvider;
/**
 * Активация расширения
 */
async function activate(context) {
    extensionContext = context;
    // Создаем output channel
    outputChannel = vscode.window.createOutputChannel('BSL Gradual Types');
    context.subscriptions.push(outputChannel);
    // Создаем status bar item
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.text = "$(loading~spin) BSL: Initializing...";
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    // Инициализируем performance monitor
    performanceMonitor = new performance_monitor_1.PerformanceMonitor(outputChannel);
    try {
        outputChannel.appendLine('🚀 Activating BSL Gradual Type System...');
        // Загружаем конфигурацию
        await loadConfiguration();
        // Инициализируем enhanced LSP клиент
        await initializeEnhancedLsp();
        // Регистрируем providers
        await registerEnhancedProviders();
        // Регистрируем команды
        registerEnhancedCommands();
        // Устанавливаем final status
        statusBarItem.text = "$(check) BSL: Ready";
        statusBarItem.tooltip = "BSL Gradual Type System активен";
        outputChannel.appendLine('✅ BSL Gradual Type System activated successfully!');
        // Показываем welcome message при первом запуске
        await showWelcomeMessage();
    }
    catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`❌ Activation failed: ${errorMessage}`);
        statusBarItem.text = "$(error) BSL: Error";
        statusBarItem.tooltip = `Error: ${errorMessage}`;
        vscode.window.showErrorMessage(`BSL Gradual Type System activation failed: ${errorMessage}`);
    }
}
exports.activate = activate;
/**
 * Деактивация расширения
 */
async function deactivate() {
    outputChannel.appendLine('🔄 Deactivating BSL Gradual Type System...');
    try {
        // Останавливаем LSP клиент
        if (languageClient) {
            await languageClient.stop();
            languageClient = null;
        }
        // Cleanup performance monitor
        performanceMonitor?.dispose();
        outputChannel.appendLine('✅ BSL Gradual Type System deactivated successfully');
    }
    catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`⚠️ Deactivation warning: ${errorMessage}`);
    }
}
exports.deactivate = deactivate;
/**
 * Загрузка конфигурации
 */
async function loadConfiguration() {
    outputChannel.appendLine('📋 Loading configuration...');
    // Миграция legacy настроек если нужно
    await migrateLegacySettings();
    // Валидация конфигурации
    const config = configHelper_1.BslAnalyzerConfig;
    if (!config.isValid()) {
        throw new Error('Invalid configuration detected');
    }
    outputChannel.appendLine(`✅ Configuration loaded: ${JSON.stringify(config.summary(), null, 2)}`);
}
/**
 * Инициализация enhanced LSP клиента
 */
async function initializeEnhancedLsp() {
    outputChannel.appendLine('🔗 Initializing Enhanced LSP client...');
    // Получаем путь к enhanced LSP серверу
    const serverPath = (0, binaryPath_1.getBinaryPath)('lsp-server', extensionContext);
    if (!serverPath) {
        throw new Error('Enhanced LSP server binary not found');
    }
    outputChannel.appendLine(`📍 Enhanced LSP server path: ${serverPath}`);
    // Создаем enhanced LSP клиент
    languageClient = new enhanced_client_1.EnhancedLspClient(serverPath, outputChannel, performanceMonitor);
    // Запускаем клиент
    await languageClient.start();
    outputChannel.appendLine('✅ Enhanced LSP client started successfully');
}
/**
 * Регистрация enhanced providers
 */
async function registerEnhancedProviders() {
    outputChannel.appendLine('🔌 Registering enhanced providers...');
    if (!languageClient) {
        throw new Error('Language client not initialized');
    }
    // Type hints provider (inlay hints)
    typeHintsProvider = new type_hints_simple_1.TypeHintsProvider(languageClient);
    extensionContext.subscriptions.push(vscode.languages.registerInlayHintsProvider({ scheme: 'file', language: 'bsl' }, typeHintsProvider));
    // Enhanced code actions provider
    codeActionsProvider = new code_actions_simple_1.CodeActionsProvider(languageClient);
    extensionContext.subscriptions.push(vscode.languages.registerCodeActionsProvider({ scheme: 'file', language: 'bsl' }, codeActionsProvider, {
        providedCodeActionKinds: [
            vscode.CodeActionKind.QuickFix,
            vscode.CodeActionKind.Refactor,
            vscode.CodeActionKind.RefactorExtract,
        ]
    }));
    // Enhanced diagnostics provider
    diagnosticsProvider = new enhanced_diagnostics_simple_1.EnhancedDiagnosticsProvider(languageClient, outputChannel);
    outputChannel.appendLine('✅ Enhanced providers registered');
}
/**
 * Регистрация enhanced команд
 */
function registerEnhancedCommands() {
    outputChannel.appendLine('⚙️ Registering enhanced commands...');
    // Команда для показа type информации
    extensionContext.subscriptions.push(vscode.commands.registerCommand('bsl.showTypeInfo', async () => {
        await showTypeInfoAtCursor();
    }));
    // Команда для запуска performance profiling
    extensionContext.subscriptions.push(vscode.commands.registerCommand('bsl.runPerformanceProfiling', async () => {
        await runPerformanceProfiling();
    }));
    // Команда для анализа проекта
    extensionContext.subscriptions.push(vscode.commands.registerCommand('bsl.analyzeProject', async () => {
        await analyzeCurrentProject();
    }));
    // Команда для показа type hints настроек
    extensionContext.subscriptions.push(vscode.commands.registerCommand('bsl.configureTypeHints', async () => {
        await configureTypeHints();
    }));
    // Команда для cache управления
    extensionContext.subscriptions.push(vscode.commands.registerCommand('bsl.clearAnalysisCache', async () => {
        await clearAnalysisCache();
    }));
    outputChannel.appendLine('✅ Enhanced commands registered');
}
/**
 * Показать type информацию под курсором
 */
async function showTypeInfoAtCursor() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || !languageClient) {
        return;
    }
    const position = editor.selection.active;
    const document = editor.document;
    try {
        // Запрашиваем enhanced hover информацию
        const hover = await languageClient.getEnhancedHover(document.uri.toString(), position);
        if (hover) {
            // Показываем в webview panel для rich content
            const panel = vscode.window.createWebviewPanel('bslTypeInfo', 'BSL Type Information', vscode.ViewColumn.Beside, { enableScripts: true });
            panel.webview.html = generateTypeInfoHtml(hover);
        }
        else {
            vscode.window.showInformationMessage('No type information available at cursor');
        }
    }
    catch (error) {
        outputChannel.appendLine(`❌ Error getting type info: ${error}`);
        vscode.window.showErrorMessage('Failed to get type information');
    }
}
/**
 * Запуск performance profiling текущего файла
 */
async function runPerformanceProfiling() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No active BSL file');
        return;
    }
    const filePath = editor.document.uri.fsPath;
    try {
        statusBarItem.text = "$(loading~spin) BSL: Profiling...";
        // Запускаем profiler через наш LSP сервер
        const result = await languageClient?.requestPerformanceProfiling(filePath);
        if (result) {
            // Показываем результаты в output channel
            outputChannel.show();
            outputChannel.appendLine('📊 Performance Profiling Results:');
            outputChannel.appendLine(result.humanReadableReport);
            vscode.window.showInformationMessage(`Performance profiling completed. Check output for details.`, 'Show Output').then(selection => {
                if (selection === 'Show Output') {
                    outputChannel.show();
                }
            });
        }
        statusBarItem.text = "$(check) BSL: Ready";
    }
    catch (error) {
        statusBarItem.text = "$(error) BSL: Error";
        outputChannel.appendLine(`❌ Profiling error: ${error}`);
        vscode.window.showErrorMessage('Performance profiling failed');
    }
}
/**
 * Анализ текущего проекта
 */
async function analyzeCurrentProject() {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        vscode.window.showWarningMessage('No workspace folder open');
        return;
    }
    const projectPath = workspaceFolder.uri.fsPath;
    try {
        statusBarItem.text = "$(loading~spin) BSL: Analyzing project...";
        // Показываем progress notification
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "Analyzing BSL project",
            cancellable: true
        }, async (progress, token) => {
            progress.report({ increment: 0, message: "Starting analysis..." });
            const result = await languageClient?.requestProjectAnalysis(projectPath, {
                useParallelAnalysis: true,
                enableCaching: true,
                showProgress: true
            });
            if (result) {
                progress.report({ increment: 100, message: "Analysis completed" });
                // Показываем результаты
                const message = `Analysis completed!\n` +
                    `Files: ${result.stats.totalFiles}\n` +
                    `Functions: ${result.stats.totalFunctions}\n` +
                    `Diagnostics: ${result.stats.totalDiagnostics}`;
                vscode.window.showInformationMessage(message, 'Show Details').then(selection => {
                    if (selection === 'Show Details') {
                        showProjectAnalysisResults(result);
                    }
                });
            }
        });
        statusBarItem.text = "$(check) BSL: Ready";
    }
    catch (error) {
        statusBarItem.text = "$(error) BSL: Error";
        outputChannel.appendLine(`❌ Project analysis error: ${error}`);
        vscode.window.showErrorMessage('Project analysis failed');
    }
}
/**
 * Настройка type hints
 */
async function configureTypeHints() {
    const config = vscode.workspace.getConfiguration('bsl.typeHints');
    const options = [
        { label: 'Show variable types', setting: 'showVariableTypes' },
        { label: 'Show return types', setting: 'showReturnTypes' },
        { label: 'Show union details', setting: 'showUnionDetails' },
        { label: 'Show parameter types', setting: 'showParameterTypes' }
    ];
    const quickPick = vscode.window.createQuickPick();
    quickPick.items = options.map(opt => ({
        label: opt.label,
        description: config.get(opt.setting) ? '✅ Enabled' : '❌ Disabled',
        detail: opt.setting
    }));
    quickPick.canSelectMany = true;
    quickPick.title = 'Configure Type Hints';
    quickPick.onDidAccept(() => {
        const selected = quickPick.selectedItems;
        options.forEach(opt => {
            const isSelected = selected.some(item => item.detail === opt.setting);
            config.update(opt.setting, isSelected, vscode.ConfigurationTarget.Global);
        });
        vscode.window.showInformationMessage('Type hints configuration updated');
        quickPick.hide();
    });
    quickPick.show();
}
/**
 * Очистка cache анализа
 */
async function clearAnalysisCache() {
    try {
        const result = await languageClient?.requestCacheClear();
        if (result?.success) {
            vscode.window.showInformationMessage(`Analysis cache cleared. Freed ${result.freedBytes} bytes.`);
        }
        else {
            vscode.window.showWarningMessage('Failed to clear analysis cache');
        }
    }
    catch (error) {
        outputChannel.appendLine(`❌ Cache clear error: ${error}`);
        vscode.window.showErrorMessage('Failed to clear cache');
    }
}
/**
 * Показать welcome message при первом запуске
 */
async function showWelcomeMessage() {
    const config = vscode.workspace.getConfiguration('bsl');
    const hasShownWelcome = config.get('hasShownWelcome', false);
    if (!hasShownWelcome) {
        const selection = await vscode.window.showInformationMessage('🎉 Welcome to BSL Gradual Type System! ' +
            'This is a production-ready type system with flow-sensitive analysis, ' +
            'union types, and enhanced LSP features.', 'Show Features', 'Configure', "Don't show again");
        switch (selection) {
            case 'Show Features':
                await showFeaturesOverview();
                break;
            case 'Configure':
                await vscode.commands.executeCommand('workbench.action.openSettings', 'bsl');
                break;
            case "Don't show again":
                await config.update('hasShownWelcome', true, vscode.ConfigurationTarget.Global);
                break;
        }
    }
}
/**
 * Показать обзор возможностей
 */
async function showFeaturesOverview() {
    const panel = vscode.window.createWebviewPanel('bslFeatures', 'BSL Gradual Type System Features', vscode.ViewColumn.Active, { enableScripts: true });
    panel.webview.html = `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>BSL Features</title>
            <style>
                body { font-family: var(--vscode-font-family); padding: 20px; }
                .feature { margin: 20px 0; padding: 15px; border-left: 4px solid var(--vscode-accent); }
                .feature h3 { margin-top: 0; color: var(--vscode-accent); }
                .performance { background: var(--vscode-terminal-ansiGreen); color: white; padding: 5px 10px; border-radius: 3px; }
            </style>
        </head>
        <body>
            <h1>🚀 BSL Gradual Type System v1.0.0</h1>
            
            <div class="feature">
                <h3>🔍 Flow-Sensitive Analysis</h3>
                <p>Отслеживает изменения типов переменных по мере выполнения программы</p>
            </div>
            
            <div class="feature">
                <h3>🔗 Union Types</h3>
                <p>Полноценные Union типы с нормализацией и взвешенными вероятностями</p>
            </div>
            
            <div class="feature">
                <h3>⚡ Enhanced LSP</h3>
                <p>Инкрементальный парсинг, умное автодополнение, real-time диагностика</p>
            </div>
            
            <div class="feature">
                <h3>🎯 Type Hints</h3>
                <p>Inline отображение типов прямо в коде</p>
            </div>
            
            <div class="feature">
                <h3>🛠️ Code Actions</h3>
                <p>Автоматические исправления и рефакторинг предложения</p>
            </div>
            
            <div class="performance">
                📊 Performance: Парсинг ~189μs | Type Checking ~125μs | Flow Analysis ~175ns
            </div>
        </body>
        </html>
    `;
}
/**
 * Показать результаты анализа проекта
 */
async function showProjectAnalysisResults(results) {
    const panel = vscode.window.createWebviewPanel('bslProjectResults', 'Project Analysis Results', vscode.ViewColumn.Active, { enableScripts: true });
    panel.webview.html = `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>Project Analysis Results</title>
            <style>
                body { font-family: var(--vscode-font-family); padding: 20px; }
                .stat { display: flex; justify-content: space-between; margin: 10px 0; }
                .stat-value { font-weight: bold; color: var(--vscode-accent); }
            </style>
        </head>
        <body>
            <h1>📊 Project Analysis Results</h1>
            
            <div class="stat">
                <span>📁 Total Files:</span>
                <span class="stat-value">${results.stats.totalFiles}</span>
            </div>
            <div class="stat">
                <span>✅ Successful:</span>
                <span class="stat-value">${results.stats.successfulFiles}</span>
            </div>
            <div class="stat">
                <span>🔧 Functions Found:</span>
                <span class="stat-value">${results.stats.totalFunctions}</span>
            </div>
            <div class="stat">
                <span>📦 Variables Found:</span>
                <span class="stat-value">${results.stats.totalVariables}</span>
            </div>
            <div class="stat">
                <span>🚨 Diagnostics:</span>
                <span class="stat-value">${results.stats.totalDiagnostics}</span>
            </div>
            <div class="stat">
                <span>⏱️ Analysis Time:</span>
                <span class="stat-value">${results.totalTime}</span>
            </div>
            
            <h2>🎯 Performance</h2>
            <div class="stat">
                <span>📈 Average per file:</span>
                <span class="stat-value">${results.stats.avgAnalysisTime}</span>
            </div>
        </body>
        </html>
    `;
}
/**
 * Генерация HTML для type info
 */
function generateTypeInfoHtml(hover) {
    return `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>Type Information</title>
            <style>
                body { 
                    font-family: var(--vscode-font-family); 
                    padding: 20px; 
                    background: var(--vscode-editor-background);
                    color: var(--vscode-editor-foreground);
                }
                .type-info { 
                    background: var(--vscode-textBlockQuote-background); 
                    padding: 15px; 
                    border-radius: 5px; 
                    border-left: 4px solid var(--vscode-accent);
                }
                .confidence { 
                    color: var(--vscode-terminal-ansiGreen); 
                    font-weight: bold; 
                }
                .source { 
                    color: var(--vscode-terminal-ansiBlue); 
                    font-style: italic; 
                }
            </style>
        </head>
        <body>
            <div class="type-info">
                ${hover.contents.value}
            </div>
        </body>
        </html>
    `;
}
/**
 * Обновление package.json с enhanced функциональностью
 */
function getEnhancedPackageContributions() {
    return {
        commands: [
            {
                command: "bsl.showTypeInfo",
                title: "Show Type Information",
                category: "BSL"
            },
            {
                command: "bsl.runPerformanceProfiling",
                title: "Run Performance Profiling",
                category: "BSL"
            },
            {
                command: "bsl.analyzeProject",
                title: "Analyze Project",
                category: "BSL"
            },
            {
                command: "bsl.configureTypeHints",
                title: "Configure Type Hints",
                category: "BSL"
            },
            {
                command: "bsl.clearAnalysisCache",
                title: "Clear Analysis Cache",
                category: "BSL"
            }
        ],
        configuration: {
            type: "object",
            title: "BSL Gradual Type System",
            properties: {
                "bsl.typeHints.showVariableTypes": {
                    type: "boolean",
                    default: true,
                    description: "Show type hints for variables"
                },
                "bsl.typeHints.showReturnTypes": {
                    type: "boolean",
                    default: true,
                    description: "Show type hints for function return types"
                },
                "bsl.typeHints.showUnionDetails": {
                    type: "boolean",
                    default: true,
                    description: "Show detailed information for Union types"
                },
                "bsl.typeHints.minCertainty": {
                    type: "number",
                    default: 0.7,
                    minimum: 0.0,
                    maximum: 1.0,
                    description: "Minimum certainty level to show type hints"
                },
                "bsl.performance.enableProfiling": {
                    type: "boolean",
                    default: false,
                    description: "Enable automatic performance profiling"
                },
                "bsl.analysis.useParallelProcessing": {
                    type: "boolean",
                    default: true,
                    description: "Use parallel processing for project analysis"
                },
                "bsl.analysis.enableCaching": {
                    type: "boolean",
                    default: true,
                    description: "Enable caching of analysis results"
                }
            }
        }
    };
}
exports.getEnhancedPackageContributions = getEnhancedPackageContributions;
//# sourceMappingURL=extension-enhanced.js.map