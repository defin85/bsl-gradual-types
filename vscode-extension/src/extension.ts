import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

// Импорт из новых модулей
import {
    BslAnalyzerConfig,
    migrateLegacySettings,
    openBslExtensionSettings,
} from './config';
import {
    initializeLspClient,
    startLanguageClient,
    stopLanguageClient,
    getLanguageClient,
    initializeAutoSignatureHelpOnCursorMove
} from './lsp';
import {
    initializeProgress,
    updateStatusBar
} from './lsp/progress';
import { logger, LogLevel } from './lsp/logger';
// MILESTONE 2.20.3: Current Context Indicator
import { initializeContextProvider } from './lsp/contextProvider';
// MILESTONE 2.20.4: Type Repository Statistics
import { initializeStatsProvider } from './lsp/statsProvider';
// MILESTONE 2.20.3: Server Status Handler (rust-analyzer approach)
import { initializeServerStatus } from './lsp/serverStatus';
import { initializeSnapshotStatus } from './lsp/snapshotStatus';
import { initializeExactIndexWarmup } from './lsp/exactIndexWarmup';
import {
    getPlatformDocsArchive,
    initializeUtils,
    autoDetectConfiguration
} from './utils';
import {
    buildIndex,
    getIndexState,
    isMethodNotFoundError,
} from './lsp/customRequests';
import {
    orchestrateStartupIndex,
} from './indexStartupOrchestration';
import {
    BslOverviewProvider,
    BslDiagnosticsProvider,
    CacheDashboardProvider,
    ObservabilityProvider,
    CompletionTimelineWebviewProvider,
    HierarchicalTypeIndexProvider,
    BslActionsWebviewProvider,
    TypeDetailsWebviewProvider,
} from './providers';
import { invalidateSidebarSnapshot } from './providers/sidebarSnapshot';
// Webview функции не используются напрямую в extension.ts
// Они используются в модуле commands
import { registerCommands as registerAllCommands, initializeCommands } from './commands';
// MILESTONE 2.9: Platform Documentation функции удалены - больше не используются

// Глобальные переменные
let indexServerPath: string;
let outputChannel: vscode.OutputChannel;
let statusBarItem: vscode.StatusBarItem;
let snapshotStatusBarItem: vscode.StatusBarItem;
let extensionContext: vscode.ExtensionContext;

// Функции прогресса теперь импортируются из модуля lsp/progress

export async function activate(context: vscode.ExtensionContext) {
    extensionContext = context;

    try {
        // Get the current version from package.json
        const currentVersion = context.extension.packageJSON.version;

        // Context is passed directly to functions that need it

        // Initialize output channel
        outputChannel = vscode.window.createOutputChannel('BSL Analyzer');
        context.subscriptions.push(outputChannel);

        // Initialize logger
        logger.initialize(outputChannel, LogLevel.Info);
        logger.info('BSL Analyzer Extension activated');

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

        snapshotStatusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
        snapshotStatusBarItem.command = 'bslAnalyzer.openSnapshotReadiness';
        snapshotStatusBarItem.hide();
        context.subscriptions.push(snapshotStatusBarItem);

        // Инициализируем модули
        initializeUtils(outputChannel);
        initializeProgress(outputChannel, statusBarItem);
        initializeServerStatus(outputChannel, statusBarItem);
        context.subscriptions.push(initializeSnapshotStatus(outputChannel, snapshotStatusBarItem));
        context.subscriptions.push(initializeExactIndexWarmup());
        // MILESTONE 2.20.3: Initialize Current Context Provider
        initializeContextProvider(context, statusBarItem);
        // MILESTONE 2.20.4: Initialize Type Repository Stats Provider
        initializeStatsProvider(context, statusBarItem);
        initializeLspClient(outputChannel);
        initializeCommands(outputChannel);
        // MILESTONE 2.9: initializePlatformDocs удалён - управление типами через LSP

        // Migrate legacy settings if needed
        await migrateLegacySettings();

        // MILESTONE 2.9: Валидация ОБЯЗАТЕЛЬНОГО параметра platformDocsArchive
        const platformDocsArchiveInfo = BslAnalyzerConfig.getPlatformDocsArchiveResolution();
        const platformDocsArchive = platformDocsArchiveInfo.value;

        // Определяем, запущены ли мы в тестовом режиме
        const isTestMode = process.env.NODE_ENV === 'test' ||
                           process.env.VSCODE_TEST_MODE === '1' ||
                           context.extensionMode === vscode.ExtensionMode.Test;

        if (!platformDocsArchive || platformDocsArchive.trim() === '') {
            if (isTestMode) {
                // В тестовом режиме только логируем (не показываем UI предупреждение)
                logger.warn('[Test Mode] platformDocsArchive not configured - using mocks');
                outputChannel.appendLine('ℹ️ [Test Mode] platformDocsArchive не настроен - используются mocks');
            } else {
                const message = platformDocsArchiveInfo.ignoredGlobalValue
                    ? '⚠️ BSL Analyzer: platformDocsArchive задан только в User/Remote scope и игнорируется для текущей рабочей области.\n\n' +
                      'Перенесите значение в настройки Workspace, иначе типы платформы 1С не будут доступны в LSP hover.'
                    : '⚠️ BSL Analyzer: platformDocsArchive не настроен!\n\n' +
                      'Это ОБЯЗАТЕЛЬНЫЙ параметр для работы TypeRepository.\n' +
                      'Без него типы платформы 1С не будут доступны в LSP hover.';
                // В production режиме показываем предупреждение
                const selection = await vscode.window.showErrorMessage(
                    message,
                    'Открыть настройки',
                    'Закрыть'
                );

                if (selection === 'Открыть настройки') {
                    void openBslExtensionSettings('platformDocsArchive');
                }

                // НЕ останавливаем активацию полностью, но показываем предупреждение
                if (platformDocsArchiveInfo.ignoredGlobalValue) {
                    outputChannel.appendLine(
                        `⚠️ platformDocsArchive задан глобально (${platformDocsArchiveInfo.ignoredGlobalValue}), но проигнорирован для текущей workspace`
                    );
                } else {
                    outputChannel.appendLine('⚠️ Extension будет работать в ограниченном режиме без платформенных типов');
                }
            }
        } else {
            outputChannel.appendLine(`✅ Platform docs archive configured: ${platformDocsArchive}`);
        }

        // Initialize configuration
        initializeConfiguration();

        // Auto-detect configuration if not set
        await autoDetectConfigurationIfNeeded();

        // Start LSP client FIRST (it may register some commands)
        // Запускаем сразу без задержки
        outputChannel.appendLine('🚀 Starting LSP server...');
        await startLanguageClient(context);
        initializeAutoSignatureHelpOnCursorMove(context);
        // ✅ ИСПРАВЛЕНО: НЕ перезаписываем статус, если идёт индексация
        // updateStatusBar обновит статус сам, когда индексация завершится
        const currentProgress = require('./lsp/progress').getCurrentProgress();
        if (!currentProgress || !currentProgress.isIndexing) {
            updateStatusBar('$(database) BSL Analyzer: Ready');
        }

        // Register sidebar providers
        registerSidebarProviders(context);

        // Register our custom commands AFTER LSP client
        await registerAllCommands(context);

        // Auto-initialize index if configured
        initializeIndexIfNeeded();

        // Show welcome message
        showWelcomeMessage();

        outputChannel.appendLine(`✅ BSL Analyzer v${currentVersion} activated successfully with auto-indexing support`);

    } catch (error) {
        outputChannel?.appendLine(`❌ Activation failed: ${error}`);
        vscode.window.showErrorMessage(`BSL Analyzer activation failed: ${error}`);
    }
}


function initializeConfiguration() {
    indexServerPath = BslAnalyzerConfig.binaryPath;

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
    const configPath = BslAnalyzerConfig.configurationPath;

    if (!configPath) {
        outputChannel.appendLine('📍 Configuration path not set, attempting auto-detection...');
        const detectedPath = await autoDetectConfiguration(outputChannel);

        if (detectedPath) {
            outputChannel.appendLine(`✅ Configuration auto-detected: ${detectedPath}`);
            // Refresh providers to use new configuration
            vscode.commands.executeCommand('bslAnalyzer.refreshTypeRepository');
        }
    } else {
        outputChannel.appendLine(`📍 Using configured path: ${configPath}`);
    }
}

async function initializeIndexIfNeeded() {
    const autoIndexBuild = BslAnalyzerConfig.autoIndexBuild;
    const configPath = BslAnalyzerConfig.configurationPath;
    const platformVersion = BslAnalyzerConfig.platformVersion;
    const platformDocsArchive = getPlatformDocsArchive();
    const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';

    await orchestrateStartupIndex({
        autoIndexBuild,
        configPath,
        platformVersion,
        platformDocsArchive,
        workspacePath,
        getIndexState: () => getIndexState({}),
        buildIndex,
        isMethodNotFoundError,
        log: (message) => outputChannel.appendLine(message),
        setStatus: (status) => updateStatusBar(status),
        showWarning: async (message) => {
            await vscode.window.showWarningMessage(message);
        },
    });
}

function showWelcomeMessage() {
    const configPath = BslAnalyzerConfig.configurationPath;
    const platformDocs = BslAnalyzerConfig.platformDocsArchive;

    if (!configPath && !platformDocs) {
        vscode.window.showInformationMessage(
            'BSL Analyzer: No configuration. Please configure 1C path and platform documentation.',
            'Open Settings'
        ).then(selection => {
            if (selection === 'Open Settings') {
                void openBslExtensionSettings();
            }
        });
    } else if (!configPath) {
        vscode.window.showInformationMessage(
            'BSL Analyzer: Please configure your 1C configuration path.',
            'Open Settings'
        ).then(selection => {
            if (selection === 'Open Settings') {
                void openBslExtensionSettings('configurationPath');
            }
        });
    } else if (!platformDocs) {
        vscode.window.showInformationMessage(
            'BSL Analyzer: Please configure platform documentation archive for full functionality.',
            'Open Settings'
        ).then(selection => {
            if (selection === 'Open Settings') {
                void openBslExtensionSettings('platformDocsArchive');
            }
        });
    } else {
        vscode.window.showInformationMessage(
            'BSL Analyzer: Configuration detected. Index orchestration is managed by LSP server.'
        );
    }
}

// Все функции организованы в модули:
// - LSP клиент в модуле lsp/
// - Webview функции в модуле webviews/
// - Провайдеры в модуле providers/
// - Команды в модуле commands/
// - Утилиты в модуле utils/

function registerSidebarProviders(context: vscode.ExtensionContext) {
    outputChannel.appendLine('📋 Registering BSL Analyzer sidebar providers...');

    try {
        // Overview provider
        outputChannel.appendLine('📋 Creating Overview provider...');
        const overviewProvider = new BslOverviewProvider(outputChannel);
        const overviewTreeView = vscode.window.createTreeView('bslAnalyzer.overview', {
            treeDataProvider: overviewProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(overviewTreeView);
        context.subscriptions.push(overviewProvider);
        outputChannel.appendLine('✅ Overview provider registered');

        // Cache Dashboard provider
        outputChannel.appendLine('📋 Creating Cache Dashboard provider...');
        const cacheDashboardProvider = new CacheDashboardProvider(outputChannel);
        const cacheDashboardTreeView = vscode.window.createTreeView('bslAnalyzer.cacheDashboard', {
            treeDataProvider: cacheDashboardProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(cacheDashboardTreeView);
        context.subscriptions.push(cacheDashboardProvider);
        outputChannel.appendLine('✅ Cache Dashboard provider registered');

        // Observability provider
        outputChannel.appendLine('📋 Creating Observability provider...');
        const observabilityProvider = new ObservabilityProvider(outputChannel);
        const observabilityTreeView = vscode.window.createTreeView('bslAnalyzer.observability', {
            treeDataProvider: observabilityProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(observabilityTreeView);
        context.subscriptions.push(observabilityProvider);
        outputChannel.appendLine('✅ Observability provider registered');

        // Completion Timeline webview provider
        outputChannel.appendLine('📋 Creating Completion Timeline webview provider...');
        const completionTimelineProvider = new CompletionTimelineWebviewProvider(outputChannel);
        const completionTimelineWebview = vscode.window.registerWebviewViewProvider(
            'bslAnalyzer.completionTimeline',
            completionTimelineProvider
        );
        context.subscriptions.push(completionTimelineWebview);
        context.subscriptions.push(completionTimelineProvider);
        outputChannel.appendLine('✅ Completion Timeline webview provider registered');

        // Diagnostics provider  
        outputChannel.appendLine('📋 Creating Diagnostics provider...');
        const diagnosticsProvider = new BslDiagnosticsProvider();
        const diagnosticsTreeView = vscode.window.createTreeView('bslAnalyzer.diagnostics', {
            treeDataProvider: diagnosticsProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(diagnosticsTreeView);
        context.subscriptions.push(diagnosticsProvider);
        outputChannel.appendLine('✅ Diagnostics provider registered');

        // Type Repository provider - показывает типы из LSP Server TypeRepository
        outputChannel.appendLine('📋 Creating Type Repository provider...');
        const typeIndexProvider = new HierarchicalTypeIndexProvider(outputChannel);
        const typeIndexTreeView = vscode.window.createTreeView('bslAnalyzer.typeRepository', {
            treeDataProvider: typeIndexProvider,
            showCollapseAll: true
        });
        context.subscriptions.push(typeIndexTreeView);
        outputChannel.appendLine('✅ Type Repository provider registered');

        // MILESTONE 2.9: Platform Documentation provider УДАЛЁН
        // Теперь единственный источник данных - TypeRepository в LSP Server
        // UI показывает типы через Custom LSP Requests (будет в Milestone 2.10)

        // Quick Actions webview provider
        outputChannel.appendLine('📋 Creating Quick Actions webview provider...');
        const actionsProvider = new BslActionsWebviewProvider(context.extensionUri, undefined, outputChannel);
        const webviewProvider = vscode.window.registerWebviewViewProvider('bslAnalyzer.actions', actionsProvider);
        context.subscriptions.push(webviewProvider);
        outputChannel.appendLine('✅ Quick Actions webview provider registered');

        // Type Details modal provider
        outputChannel.appendLine("📚 Creating Type Details modal provider...");
        const typeDetailsProvider = new TypeDetailsWebviewProvider(context.extensionUri);
        context.subscriptions.push(
            vscode.commands.registerCommand("bslAnalyzer.showTypeDetails", (typeName: string) => {
                typeDetailsProvider.showTypeDetails(typeName);
            })
        );
        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.goToDiagnostic', async (uri: vscode.Uri, diagnostic: vscode.Diagnostic) => {
                if (!uri || !diagnostic?.range) {
                    return;
                }
                const document = await vscode.workspace.openTextDocument(uri);
                const editor = await vscode.window.showTextDocument(document, { preview: false });
                editor.selection = new vscode.Selection(diagnostic.range.start, diagnostic.range.end);
                editor.revealRange(diagnostic.range, vscode.TextEditorRevealType.InCenterIfOutsideViewport);
            })
        );
        outputChannel.appendLine("✅ Type Details modal provider registered");
        // Register refresh commands
        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.refreshOverview', () => {
                outputChannel.appendLine('🔄 Refreshing Overview panel');
                invalidateSidebarSnapshot();
                overviewProvider.refresh();
                void actionsProvider.refreshSidebarSnapshot();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.refreshCacheDashboard', () => {
                outputChannel.appendLine('🔄 Refreshing Cache Dashboard panel');
                invalidateSidebarSnapshot();
                cacheDashboardProvider.refresh();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.refreshObservability', () => {
                outputChannel.appendLine('🔄 Refreshing Observability panel');
                observabilityProvider.refresh();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.openSnapshotReadiness', async () => {
                outputChannel.appendLine('🔎 Opening Snapshot Readiness diagnostics');
                try {
                    await vscode.commands.executeCommand('bslAnalyzer.observability.focus');
                } catch (error) {
                    outputChannel.appendLine(`⚠️ Failed to focus Observability panel: ${error}`);
                }
                observabilityProvider.refresh(false);
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.refreshCompletionTimeline', () => {
                outputChannel.appendLine('🔄 Refreshing Completion Timeline panel');
                completionTimelineProvider.refresh();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.toggleObservabilityAutoRefresh', async () => {
                const config = vscode.workspace.getConfiguration('bslAnalyzer');
                const current = config.get<boolean>('observabilityAutoRefresh', true);
                const target = (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0)
                    ? vscode.ConfigurationTarget.Workspace
                    : vscode.ConfigurationTarget.Global;
                await config.update('observabilityAutoRefresh', !current, target);

                outputChannel.appendLine(
                    `🔁 Observability auto refresh ${!current ? 'enabled' : 'disabled'} (${target === vscode.ConfigurationTarget.Workspace ? 'workspace' : 'global'} scope)`
                );
                observabilityProvider.refresh();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.toggleObservabilityCompactMode', async () => {
                const config = vscode.workspace.getConfiguration('bslAnalyzer');
                const current = config.get<boolean>('observabilityCompactMode', false);
                const target = (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0)
                    ? vscode.ConfigurationTarget.Workspace
                    : vscode.ConfigurationTarget.Global;
                await config.update('observabilityCompactMode', !current, target);

                outputChannel.appendLine(
                    `🧭 Observability compact mode ${!current ? 'enabled' : 'disabled'} (${target === vscode.ConfigurationTarget.Workspace ? 'workspace' : 'global'} scope)`
                );
                observabilityProvider.refresh();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.refreshDiagnostics', () => {
                outputChannel.appendLine('🔄 Refreshing Diagnostics panel');
                invalidateSidebarSnapshot();
                diagnosticsProvider.refresh();
                void actionsProvider.refreshSidebarSnapshot();
            })
        );

        context.subscriptions.push(
            vscode.commands.registerCommand('bslAnalyzer.refreshTypeRepository', () => {
                outputChannel.appendLine('🔄 Refreshing Type Repository panel');
                invalidateSidebarSnapshot();
                typeIndexProvider.refresh();
                void actionsProvider.refreshSidebarSnapshot();
            })
        );

        // MILESTONE 2.9: Platform Documentation команды УДАЛЕНЫ
        // - bslAnalyzer.refreshPlatformDocs
        // - bslAnalyzer.addPlatformDocs
        // - bslAnalyzer.removePlatformDocs
        // - bslAnalyzer.parsePlatformDocs
        // Все управление типами теперь через LSP Server TypeRepository

        outputChannel.appendLine('✅ All BSL Analyzer sidebar providers registered successfully');

        // Показываем уведомление об успешной регистрации
        vscode.window.showInformationMessage('BSL Analyzer sidebar activated! Check the Activity Bar for the BSL Analyzer icon.');

    } catch (error) {
        outputChannel.appendLine(`❌ Error registering sidebar providers: ${error}`);
        vscode.window.showErrorMessage(`Failed to register BSL Analyzer sidebar: ${error}`);
    }
}



// Функции платформенной документации перенесены в модуль platformDocs

export async function deactivate(): Promise<void> {
    const client = getLanguageClient();
    if (!client) {
        return;
    }

    try {
        // Give the client time to shut down gracefully
        const timeoutPromise = new Promise<void>((resolve) => {
            setTimeout(() => {
                outputChannel.appendLine('⚠️ LSP client shutdown timeout reached');
                resolve();
            }, 5000);
        });

        const stopPromise = stopLanguageClient().then(() => {
            outputChannel.appendLine('✅ LSP client stopped successfully');
        }).catch(error => {
            outputChannel.appendLine(`⚠️ Error stopping LSP client: ${error}`);
        });

        // Wait for either stop to complete or timeout
        await Promise.race([stopPromise, timeoutPromise]);

    } catch (error) {
        outputChannel.appendLine(`⚠️ Error during deactivation: ${error}`);
    } finally {
        outputChannel.appendLine('👋 BSL Analyzer extension deactivated');
        outputChannel.dispose();
    }
}
