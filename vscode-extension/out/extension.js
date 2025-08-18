"use strict";
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
exports.deactivate = exports.activate = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
// Импорт из новых модулей
const config_1 = require("./config");
const lsp_1 = require("./lsp");
const progress_1 = require("./lsp/progress");
const utils_1 = require("./utils");
const providers_1 = require("./providers");
// Webview функции не используются напрямую в extension.ts
// Они используются в модуле commands
const commands_1 = require("./commands");
const platformDocs_1 = require("./platformDocs");
// Глобальные переменные
let indexServerPath;
let outputChannel;
let statusBarItem;
let extensionContext;
// Функции прогресса теперь импортируются из модуля lsp/progress
async function activate(context) {
    extensionContext = context;
    try {
        // Get the current version from package.json
        const currentVersion = context.extension.packageJSON.version;
        // Context is passed directly to functions that need it
        // Initialize output channel
        outputChannel = vscode.window.createOutputChannel('BSL Analyzer');
        context.subscriptions.push(outputChannel);
        outputChannel.appendLine(`🚀 BSL Analyzer v${currentVersion} activation started (with modular architecture)`);
        outputChannel.appendLine(`Extension path: ${context.extensionPath}`);
        // Show immediate notification for debugging
        vscode.window.showInformationMessage(`BSL Analyzer v${currentVersion} is activating...`);
        outputChannel.show(); // Показываем Output канал для отладки
        // Create status bar item first
        statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
        statusBarItem.command = 'bslAnalyzer.analyzeFile';
        statusBarItem.text = 'BSL Analyzer: Starting...';
        statusBarItem.tooltip = 'Click to analyze current file (via LSP)';
        statusBarItem.show();
        context.subscriptions.push(statusBarItem);
        // Инициализируем модули
        (0, utils_1.initializeUtils)(outputChannel);
        (0, progress_1.initializeProgress)(outputChannel, statusBarItem);
        (0, lsp_1.initializeLspClient)(outputChannel);
        (0, commands_1.initializeCommands)(outputChannel);
        (0, platformDocs_1.initializePlatformDocs)(outputChannel);
        // Migrate legacy settings if needed
        await (0, config_1.migrateLegacySettings)();
        // Initialize configuration
        initializeConfiguration();
        // Auto-detect configuration if not set
        await autoDetectConfigurationIfNeeded();
        // Start LSP client FIRST (it may register some commands)
        // Запускаем с задержкой для стабильности
        setTimeout(async () => {
            outputChannel.appendLine('🚀 Starting LSP server with delay...');
            await (0, lsp_1.startLanguageClient)(context);
            // Обновляем статус бар после успешного запуска
            (0, progress_1.updateStatusBar)('$(database) BSL Analyzer: Ready');
        }, 1000);
        // Register sidebar providers
        registerSidebarProviders(context);
        // Register our custom commands AFTER LSP client
        await (0, commands_1.registerCommands)(context);
        // Auto-initialize index if configured
        initializeIndexIfNeeded();
        // Show welcome message
        showWelcomeMessage();
        outputChannel.appendLine(`✅ BSL Analyzer v${currentVersion} activated successfully with auto-indexing support`);
    }
    catch (error) {
        outputChannel?.appendLine(`❌ Activation failed: ${error}`);
        vscode.window.showErrorMessage(`BSL Analyzer activation failed: ${error}`);
    }
}
exports.activate = activate;
function initializeConfiguration() {
    indexServerPath = config_1.BslAnalyzerConfig.binaryPath;
    if (!indexServerPath) {
        // First, try bundled binaries from extension context
        // Use extensionContext which is available globally in this scope
        const extensionPath = extensionContext?.extensionPath;
        if (extensionPath) {
            const bundledBinPath = path.join(extensionPath, 'bin');
            if (fs.existsSync(bundledBinPath)) {
                indexServerPath = bundledBinPath;
                outputChannel.appendLine(`Using bundled BSL Analyzer binaries at: ${indexServerPath}`);
            }
        }
        // No fallback - extension must be self-contained
        if (!indexServerPath) {
            outputChannel.appendLine(`❌ BSL Analyzer binaries not found in extension.`);
            outputChannel.appendLine(`💡 Please run 'npm run copy:binaries' to update extension binaries.`);
        }
    }
}
async function autoDetectConfigurationIfNeeded() {
    const configPath = config_1.BslAnalyzerConfig.configurationPath;
    if (!configPath) {
        outputChannel.appendLine('📍 Configuration path not set, attempting auto-detection...');
        const detectedPath = await (0, utils_1.autoDetectConfiguration)(outputChannel);
        if (detectedPath) {
            outputChannel.appendLine(`✅ Configuration auto-detected: ${detectedPath}`);
            // Refresh providers to use new configuration
            vscode.commands.executeCommand('bslAnalyzer.refreshTypeIndex');
        }
    }
    else {
        outputChannel.appendLine(`📍 Using configured path: ${configPath}`);
    }
}
async function initializeIndexIfNeeded() {
    const autoIndexBuild = config_1.BslAnalyzerConfig.autoIndexBuild;
    const configPath = config_1.BslAnalyzerConfig.configurationPath;
    if (!autoIndexBuild) {
        outputChannel.appendLine('ℹ️ Auto-index build is disabled');
        return;
    }
    if (!configPath) {
        outputChannel.appendLine('⚠️ Configuration path not set - user must configure it');
        (0, progress_1.updateStatusBar)('BSL Analyzer: No Config');
        return;
    }
    // Check if we have valid UUID for this configuration
    const projectId = extractUuidProjectId(configPath);
    if (!projectId) {
        outputChannel.appendLine('❌ Cannot find UUID in Configuration.xml - no fallback, index cannot be built');
        (0, progress_1.updateStatusBar)('BSL Analyzer: Invalid Config');
        return;
    }
    // Check if index already exists in cache
    const platformVersion = config_1.BslAnalyzerConfig.platformVersion;
    const indexCachePath = path.join(require('os').homedir(), '.bsl_analyzer', 'project_indices', projectId, platformVersion);
    if (fs.existsSync(path.join(indexCachePath, 'unified_index.json'))) {
        outputChannel.appendLine(`✅ Index found in cache: ${projectId}/${platformVersion}`);
        (0, progress_1.updateStatusBar)('BSL Analyzer: Index Ready');
        return;
    }
    // Check if platform documentation is configured
    const platformDocsArchive = (0, utils_1.getPlatformDocsArchive)();
    if (!platformDocsArchive) {
        outputChannel.appendLine('❌ Platform documentation not configured - cannot build full index');
        outputChannel.appendLine('💡 User must specify platform documentation archive in settings');
        (0, progress_1.updateStatusBar)('BSL Analyzer: No Platform Docs');
        // Don't build index without platform docs - it would be incomplete
        return;
    }
    outputChannel.appendLine('🚀 Building BSL index with user-configured settings...');
    // Build index with user-provided configuration
    try {
        (0, progress_1.startIndexing)(4);
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: 'Building BSL Index',
            cancellable: false
        }, async (progress) => {
            (0, progress_1.updateIndexingProgress)(1, 'Loading platform documentation...', 10);
            progress.report({ increment: 25, message: 'Loading platform documentation...' });
            (0, progress_1.updateIndexingProgress)(2, 'Parsing configuration...', 35);
            progress.report({ increment: 25, message: 'Parsing configuration...' });
            (0, progress_1.updateIndexingProgress)(3, 'Building unified index...', 70);
            progress.report({ increment: 35, message: 'Building unified index...' });
            outputChannel.appendLine(`📁 Configuration: ${configPath}`);
            outputChannel.appendLine(`📚 Platform docs: ${platformDocsArchive}`);
            outputChannel.appendLine(`🔢 Platform version: ${platformVersion}`);
            const args = [
                '--config', configPath,
                '--platform-version', platformVersion,
                '--platform-docs-archive', platformDocsArchive
            ];
            await (0, utils_1.executeBslCommand)('build_unified_index', args);
            (0, progress_1.updateIndexingProgress)(4, 'Finalizing index...', 90);
            progress.report({ increment: 15, message: 'Finalizing...' });
            (0, progress_1.finishIndexing)(true);
            outputChannel.appendLine('✅ Index build completed successfully');
            (0, progress_1.updateStatusBar)('BSL Analyzer: Index Ready');
        });
    }
    catch (error) {
        (0, progress_1.finishIndexing)(false);
        outputChannel.appendLine(`❌ Index build failed: ${error}`);
        (0, progress_1.updateStatusBar)('BSL Analyzer: Build Failed');
    }
}
function showWelcomeMessage() {
    const configPath = config_1.BslAnalyzerConfig.configurationPath;
    const platformDocs = config_1.BslAnalyzerConfig.platformDocsArchive;
    if (!configPath && !platformDocs) {
        vscode.window.showInformationMessage('BSL Analyzer: No configuration. Please configure 1C path and platform documentation.', 'Open Settings').then(selection => {
            if (selection === 'Open Settings') {
                vscode.commands.executeCommand('workbench.action.openSettings', 'bslAnalyzer');
            }
        });
    }
    else if (!configPath) {
        vscode.window.showInformationMessage('BSL Analyzer: Please configure your 1C configuration path.', 'Open Settings').then(selection => {
            if (selection === 'Open Settings') {
                vscode.commands.executeCommand('workbench.action.openSettings', 'bslAnalyzer.configurationPath');
            }
        });
    }
    else if (!platformDocs) {
        vscode.window.showInformationMessage('BSL Analyzer: Please configure platform documentation archive for full functionality.', 'Open Settings').then(selection => {
            if (selection === 'Open Settings') {
                vscode.commands.executeCommand('workbench.action.openSettings', 'bslAnalyzer.platformDocsArchive');
            }
        });
    }
    else {
        // Everything is configured
        const homedir = require('os').homedir();
        const platformVersion = config_1.BslAnalyzerConfig.platformVersion;
        const projectId = extractUuidProjectId(configPath);
        if (projectId) {
            const indexPath = path.join(homedir, '.bsl_analyzer', 'project_indices', projectId, platformVersion, 'unified_index.json');
            if (fs.existsSync(indexPath)) {
                vscode.window.showInformationMessage('BSL Analyzer: Index loaded from cache. Ready to use!');
            }
            else {
                vscode.window.showInformationMessage('BSL Analyzer: Configuration detected. Building index...');
            }
        }
        else {
            vscode.window.showWarningMessage('BSL Analyzer: Invalid configuration (no UUID). Please check your Configuration.xml');
        }
    }
}
// UUID-based project identifier (must match Rust naming scheme; no fallback)
function extractUuidProjectId(configPath) {
    try {
        const cfgXml = path.join(configPath, 'Configuration.xml');
        if (!fs.existsSync(cfgXml))
            return null;
        const content = fs.readFileSync(cfgXml, 'utf-8');
        const m = content.match(/<Configuration[^>]*uuid="([^"]+)"/i);
        if (m && m[1]) {
            const uuid = m[1].replace(/-/g, '');
            return `${path.basename(configPath)}_${uuid}`;
        }
    }
    catch (e) {
        outputChannel.appendLine(`Failed to extract UUID: ${e}`);
    }
    return null;
}
// Все функции организованы в модули:
// - LSP клиент в модуле lsp/
// - Webview функции в модуле webviews/
// - Провайдеры в модуле providers/
// - Команды в модуле commands/
// - Утилиты в модуле utils/
function registerSidebarProviders(context) {
    outputChannel.appendLine('📋 Registering BSL Analyzer sidebar providers...');
    try {
        // Overview provider
        outputChannel.appendLine('📋 Creating Overview provider...');
        const overviewProvider = new providers_1.BslOverviewProvider(outputChannel);
        const overviewTreeView = vscode.window.createTreeView('bslAnalyzer.overview', {
            treeDataProvider: overviewProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(overviewTreeView);
        outputChannel.appendLine('✅ Overview provider registered');
        // Diagnostics provider  
        outputChannel.appendLine('📋 Creating Diagnostics provider...');
        const diagnosticsProvider = new providers_1.BslDiagnosticsProvider();
        const diagnosticsTreeView = vscode.window.createTreeView('bslAnalyzer.diagnostics', {
            treeDataProvider: diagnosticsProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(diagnosticsTreeView);
        outputChannel.appendLine('✅ Diagnostics provider registered');
        // Type Index provider - используем новый иерархический провайдер
        outputChannel.appendLine('📋 Creating Hierarchical Type Index provider...');
        const typeIndexProvider = new providers_1.HierarchicalTypeIndexProvider(outputChannel);
        const typeIndexTreeView = vscode.window.createTreeView('bslAnalyzer.typeIndex', {
            treeDataProvider: typeIndexProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(typeIndexTreeView);
        outputChannel.appendLine('✅ Hierarchical Type Index provider registered');
        // Platform Documentation provider
        outputChannel.appendLine('📋 Creating Platform Documentation provider...');
        const platformDocsProvider = new providers_1.BslPlatformDocsProvider(outputChannel);
        const platformDocsTreeView = vscode.window.createTreeView('bslAnalyzer.platformDocs', {
            treeDataProvider: platformDocsProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(platformDocsTreeView);
        outputChannel.appendLine('✅ Platform Documentation provider registered');
        // Quick Actions webview provider
        outputChannel.appendLine('📋 Creating Quick Actions webview provider...');
        const actionsProvider = new providers_1.BslActionsWebviewProvider(context.extensionUri);
        const webviewProvider = vscode.window.registerWebviewViewProvider('bslAnalyzer.actions', actionsProvider);
        context.subscriptions.push(webviewProvider);
        outputChannel.appendLine('✅ Quick Actions webview provider registered');
        // Register refresh commands
        context.subscriptions.push(vscode.commands.registerCommand('bslAnalyzer.refreshOverview', () => {
            outputChannel.appendLine('🔄 Refreshing Overview panel');
            overviewProvider.refresh();
        }));
        context.subscriptions.push(vscode.commands.registerCommand('bslAnalyzer.refreshDiagnostics', () => {
            outputChannel.appendLine('🔄 Refreshing Diagnostics panel');
            diagnosticsProvider.refresh();
        }));
        context.subscriptions.push(vscode.commands.registerCommand('bslAnalyzer.refreshTypeIndex', () => {
            outputChannel.appendLine('🔄 Refreshing Type Index panel');
            typeIndexProvider.refresh();
        }));
        context.subscriptions.push(vscode.commands.registerCommand('bslAnalyzer.refreshPlatformDocs', () => {
            outputChannel.appendLine('🔄 Refreshing Platform Docs panel');
            platformDocsProvider.refresh();
        }));
        // Регистрируем команду добавления документации
        outputChannel.appendLine('Registering bslAnalyzer.addPlatformDocs command...');
        try {
            const addDocsDisposable = vscode.commands.registerCommand('bslAnalyzer.addPlatformDocs', async () => {
                outputChannel.appendLine('📁 Command executed: Adding platform documentation...');
                await (0, platformDocs_1.addPlatformDocumentation)(platformDocsProvider);
            });
            context.subscriptions.push(addDocsDisposable);
            outputChannel.appendLine('✅ Successfully registered bslAnalyzer.addPlatformDocs');
        }
        catch (error) {
            outputChannel.appendLine(`❌ Failed to register bslAnalyzer.addPlatformDocs: ${error}`);
        }
        context.subscriptions.push(vscode.commands.registerCommand('bslAnalyzer.removePlatformDocs', async (item) => {
            if (item && item.version) {
                outputChannel.appendLine(`🗑️ Removing platform docs for version: ${item.version}`);
                await (0, platformDocs_1.removePlatformDocumentation)(item.version, platformDocsProvider);
            }
        }));
        context.subscriptions.push(vscode.commands.registerCommand('bslAnalyzer.parsePlatformDocs', async (item) => {
            if (item && item.version) {
                outputChannel.appendLine(`⚙️ Parsing platform docs for version: ${item.version}`);
                await (0, platformDocs_1.parsePlatformDocumentation)(item.version);
            }
        }));
        outputChannel.appendLine('✅ All BSL Analyzer sidebar providers registered successfully');
        // Показываем уведомление об успешной регистрации
        vscode.window.showInformationMessage('BSL Analyzer sidebar activated! Check the Activity Bar for the BSL Analyzer icon.');
    }
    catch (error) {
        outputChannel.appendLine(`❌ Error registering sidebar providers: ${error}`);
        vscode.window.showErrorMessage(`Failed to register BSL Analyzer sidebar: ${error}`);
    }
}
// Функции платформенной документации перенесены в модуль platformDocs
async function deactivate() {
    const client = (0, lsp_1.getLanguageClient)();
    if (!client) {
        return;
    }
    try {
        // Give the client time to shut down gracefully
        const timeoutPromise = new Promise((resolve) => {
            setTimeout(() => {
                outputChannel.appendLine('⚠️ LSP client shutdown timeout reached');
                resolve();
            }, 5000);
        });
        const stopPromise = (0, lsp_1.stopLanguageClient)().then(() => {
            outputChannel.appendLine('✅ LSP client stopped successfully');
        }).catch(error => {
            outputChannel.appendLine(`⚠️ Error stopping LSP client: ${error}`);
        });
        // Wait for either stop to complete or timeout
        await Promise.race([stopPromise, timeoutPromise]);
    }
    catch (error) {
        outputChannel.appendLine(`⚠️ Error during deactivation: ${error}`);
    }
    finally {
        outputChannel.appendLine('👋 BSL Analyzer extension deactivated');
        outputChannel.dispose();
    }
}
exports.deactivate = deactivate;
//# sourceMappingURL=extension.js.map