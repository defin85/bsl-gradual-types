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
exports.registerIndexCommands = void 0;
const vscode = __importStar(require("vscode"));
const progress_1 = require("../lsp/progress");
const utils_1 = require("../utils");
const customRequests_1 = require("../lsp/customRequests");
const webviews_1 = require("../webviews");
/**
 * Register index-related commands (build, stats, incremental update)
 */
function registerIndexCommands(context, safeRegisterCommand, outputChannel) {
    const isTestMode = process.env.NODE_ENV === 'test' ||
        process.env.VSCODE_TEST_MODE === '1' ||
        process.env.VSCODE_EXTENSION_MODE === 'test';
    let watchers = [];
    let debounceTimer;
    let inFlight = false;
    let pending = false;
    let pendingPaths = new Set();
    let userPaused = false;
    let lastPauseSynced = null;
    const isAutoReindexPaused = () => userPaused || !(0, utils_1.getAutoReindexEnabled)();
    const applyPausedStatus = () => {
        if (isAutoReindexPaused()) {
            (0, progress_1.updateStatusBar)('$(debug-pause) BSL: Auto reindex paused');
        }
    };
    const syncAutoReindexState = async (reason) => {
        const paused = isAutoReindexPaused();
        (0, progress_1.setAutoReindexPaused)(paused);
        if (paused) {
            applyPausedStatus();
        }
        else if (!inFlight) {
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
        }
        if (isTestMode) {
            return;
        }
        if (lastPauseSynced === paused) {
            return;
        }
        try {
            if (paused) {
                await (0, customRequests_1.pauseAutoReindex)();
            }
            else {
                await (0, customRequests_1.resumeAutoReindex)();
            }
            lastPauseSynced = paused;
            outputChannel.appendLine(`[AutoReindex] LSP state: ${paused ? 'paused' : 'resumed'} (${reason})`);
        }
        catch (error) {
            outputChannel.appendLine(`[AutoReindex] Failed to sync LSP pause state (${reason}): ${error}`);
        }
    };
    // Build Unified BSL Index
    safeRegisterCommand('bslAnalyzer.buildIndex', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showWarningMessage('No workspace folder found');
            return;
        }
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }
        const choice = await vscode.window.showInformationMessage('Building unified BSL index. This may take a few seconds...', 'Build Index', 'Cancel');
        if (choice !== 'Build Index') {
            return;
        }
        const workspacePath = workspaceFolders[0].uri.fsPath;
        try {
            // P4: не дублируем локальный progress (Notification) поверх server-initiated $/progress.
            (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Building index...');
            // Логи/аргументы оставляем только для диагностики (сам build идёт через LSP).
            const args = ['--config', configPath, '--platform-version', (0, utils_1.getPlatformVersion)()];
            const platformDocsArchive = (0, utils_1.getPlatformDocsArchive)();
            if (platformDocsArchive) {
                args.push('--platform-docs-archive', platformDocsArchive);
            }
            outputChannel.appendLine(`BuildIndex (LSP): ${args.join(' ')}`);
            const result = await (0, customRequests_1.buildIndex)({ workspace_path: workspacePath });
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
            const typesCount = result.types_count || 'unknown';
            vscode.window.showInformationMessage(`BSL Index built successfully with ${typesCount} types`);
        }
        catch (error) {
            (0, progress_1.updateStatusBar)(`$(error) BSL: Index build failed: ${error}`);
            vscode.window.showErrorMessage(`Index build failed: ${error}`);
            outputChannel.appendLine(`Index build error: ${error}`);
        }
    });
    // Show Index Statistics
    safeRegisterCommand('bslAnalyzer.showIndexStats', async () => {
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Loading stats...');
        try {
            const result = await (0, customRequests_1.queryType)('stats');
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showIndexStatsWebview)(context, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Failed to load index stats: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
    // Incremental Index Update
    safeRegisterCommand('bslAnalyzer.incrementalUpdate', async () => {
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }
        try {
            // P4: прогресс показывает сервер через $/progress.
            (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Incremental update...');
            const result = await (0, customRequests_1.incrementalUpdate)(configPath, (0, utils_1.getPlatformVersion)(), [], false);
            if (!result.success) {
                (0, progress_1.updateStatusBar)(`$(error) BSL: Incremental update failed: ${result.message}`);
                vscode.window.showErrorMessage(`Incremental update failed: ${result.message}`);
                outputChannel.appendLine(`Incremental update failed: ${result.message}`);
                return;
            }
            pendingPaths.clear();
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
            vscode.window.showInformationMessage(`Index updated successfully: ${result.message}`);
        }
        catch (error) {
            (0, progress_1.updateStatusBar)(`$(error) BSL: Incremental update failed: ${error}`);
            vscode.window.showErrorMessage(`Incremental update failed: ${error}`);
            outputChannel.appendLine(`Incremental update error: ${error}`);
        }
    });
    // Pause Auto Reindex
    safeRegisterCommand('bslAnalyzer.pauseAutoReindex', async () => {
        userPaused = true;
        await syncAutoReindexState('user pause');
        outputChannel.appendLine('[AutoReindex] Paused by user');
        vscode.window.showInformationMessage('BSL Analyzer: Auto reindex paused');
    });
    // Resume Auto Reindex
    safeRegisterCommand('bslAnalyzer.resumeAutoReindex', async () => {
        userPaused = false;
        await syncAutoReindexState('user resume');
        outputChannel.appendLine('[AutoReindex] Resumed by user');
        if (!(0, utils_1.getAutoReindexEnabled)()) {
            vscode.window.showWarningMessage('BSL Analyzer: Auto reindex is disabled in settings');
            applyPausedStatus();
            return;
        }
        if (pendingPaths.size > 0) {
            await runAutoUpdate();
        }
        else {
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
        }
    });
    // Reindex Now (manual)
    safeRegisterCommand('bslAnalyzer.reindexNow', async () => {
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }
        const changedPaths = Array.from(pendingPaths);
        pendingPaths.clear();
        try {
            (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Reindex now...');
            const result = await (0, customRequests_1.incrementalUpdate)(configPath, (0, utils_1.getPlatformVersion)(), changedPaths, false);
            if (!result.success) {
                (0, progress_1.updateStatusBar)(`$(error) BSL: Reindex failed: ${result.message}`);
                vscode.window.showErrorMessage(`Reindex failed: ${result.message}`);
                outputChannel.appendLine(`Reindex failed: ${result.message}`);
                for (const path of changedPaths) {
                    pendingPaths.add(path);
                }
                applyPausedStatus();
                return;
            }
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
            vscode.window.showInformationMessage(`Reindex completed: ${result.message}`);
        }
        catch (error) {
            (0, progress_1.updateStatusBar)(`$(error) BSL: Reindex failed: ${error}`);
            vscode.window.showErrorMessage(`Reindex failed: ${error}`);
            outputChannel.appendLine(`Reindex error: ${error}`);
            for (const path of changedPaths) {
                pendingPaths.add(path);
            }
        }
        finally {
            applyPausedStatus();
        }
    });
    // P5: авто-реиндексация при изменениях файлов конфигурации (debounce + single-flight).
    // В тестовом режиме отключаем, чтобы не провоцировать фоновые запросы/таймауты.
    if (isTestMode) {
        outputChannel.appendLine('[AutoReindex] Disabled in test mode');
        void syncAutoReindexState('test mode');
        return;
    }
    const disposeWatchers = () => {
        for (const w of watchers)
            w.dispose();
        watchers = [];
    };
    const scheduleAutoUpdate = (reason, uri) => {
        if (uri) {
            pendingPaths.add(uri.fsPath);
        }
        outputChannel.appendLine(`[AutoReindex] Schedule: ${reason}`);
        if (isTestMode) {
            return;
        }
        if (isAutoReindexPaused()) {
            outputChannel.appendLine('[AutoReindex] Paused - changes queued');
            applyPausedStatus();
            return;
        }
        if (debounceTimer)
            clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => void runAutoUpdate(), 1200);
    };
    const runAutoUpdate = async () => {
        if (isTestMode) {
            return;
        }
        if (isAutoReindexPaused()) {
            applyPausedStatus();
            return;
        }
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath)
            return;
        if (pendingPaths.size === 0) {
            return;
        }
        if (inFlight) {
            pending = true;
            return;
        }
        inFlight = true;
        pending = false;
        const changedPaths = Array.from(pendingPaths);
        pendingPaths.clear();
        try {
            (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Auto reindex...');
            const result = await (0, customRequests_1.incrementalUpdate)(configPath, (0, utils_1.getPlatformVersion)(), changedPaths, true);
            if (!result.success) {
                for (const path of changedPaths) {
                    pendingPaths.add(path);
                }
                outputChannel.appendLine(`[AutoReindex] Skipped: ${result.message}`);
                if (/paused/i.test(result.message)) {
                    applyPausedStatus();
                }
                else {
                    (0, progress_1.updateStatusBar)(`$(error) BSL: Auto reindex failed: ${result.message}`);
                }
                return;
            }
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
            outputChannel.appendLine('[AutoReindex] Completed');
        }
        catch (error) {
            for (const path of changedPaths) {
                pendingPaths.add(path);
            }
            (0, progress_1.updateStatusBar)(`$(error) BSL: Auto reindex failed: ${error}`);
            outputChannel.appendLine(`[AutoReindex] Failed: ${error}`);
        }
        finally {
            inFlight = false;
            if (pending) {
                pending = false;
                scheduleAutoUpdate('pending changes while in-flight');
            }
            applyPausedStatus();
        }
    };
    const createWatchers = () => {
        disposeWatchers();
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath)
            return;
        // Минимальный набор "горячих" файлов: Configuration.xml, любые Ext/*.bsl, Form.xml.
        const patterns = [
            '**/Ext/*.bsl',
            '**/*.xml'
        ];
        for (const pattern of patterns) {
            const watcher = vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(configPath, pattern));
            watcher.onDidCreate((uri) => scheduleAutoUpdate(`create: ${uri.fsPath}`, uri));
            watcher.onDidChange((uri) => scheduleAutoUpdate(`change: ${uri.fsPath}`, uri));
            watcher.onDidDelete((uri) => scheduleAutoUpdate(`delete: ${uri.fsPath}`, uri));
            watchers.push(watcher);
        }
        outputChannel.appendLine(`[AutoReindex] Watchers installed for: ${configPath}`);
    };
    createWatchers();
    void syncAutoReindexState('startup');
    context.subscriptions.push({ dispose: disposeWatchers });
    context.subscriptions.push(vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('bslAnalyzer.configurationPath')) {
            outputChannel.appendLine('[AutoReindex] configurationPath changed, recreating watchers');
            createWatchers();
            pendingPaths.clear();
        }
        if (e.affectsConfiguration('bslAnalyzer.autoReindexEnabled')) {
            outputChannel.appendLine(`[AutoReindex] autoReindexEnabled changed: ${(0, utils_1.getAutoReindexEnabled)()}`);
            void syncAutoReindexState('settings change');
            if (!isAutoReindexPaused() && pendingPaths.size > 0) {
                scheduleAutoUpdate('auto reindex enabled');
            }
        }
    }));
}
exports.registerIndexCommands = registerIndexCommands;
//# sourceMappingURL=index-commands.js.map