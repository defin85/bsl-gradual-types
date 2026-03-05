import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { setAutoReindexPaused, updateStatusBar } from '../lsp/progress';
import { getAutoReindexEnabled, getConfigurationPath, getPlatformVersion, getPlatformDocsArchive } from '../utils';
import { queryType, buildIndex, incrementalUpdate, pauseAutoReindex, resumeAutoReindex } from '../lsp/customRequests';
import { isAttachedBuildIndexResponse } from '../indexStartupOrchestration';
import { showIndexStatsWebview } from '../webviews';

/**
 * Register index-related commands (build, stats, incremental update)
 */
export function registerIndexCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>,
    outputChannel: vscode.OutputChannel
) {
    const isTestMode =
        process.env.NODE_ENV === 'test' ||
        process.env.VSCODE_TEST_MODE === '1' ||
        process.env.VSCODE_EXTENSION_MODE === 'test';

    let watchers: vscode.Disposable[] = [];
    let debounceTimer: NodeJS.Timeout | undefined;
    let inFlight = false;
    let pending = false;
    let pendingPaths = new Set<string>();
    let userPaused = false;
    let lastPauseSynced: boolean | null = null;

    const isAutoReindexPaused = () => userPaused || !getAutoReindexEnabled();

    const applyPausedStatus = () => {
        if (isAutoReindexPaused()) {
            updateStatusBar('$(debug-pause) BSL: Auto reindex paused');
        }
    };

    const syncAutoReindexState = async (reason: string) => {
        const paused = isAutoReindexPaused();
        setAutoReindexPaused(paused);
        if (paused) {
            applyPausedStatus();
        } else if (!inFlight) {
            updateStatusBar('$(check) BSL: Ready');
        }

        if (isTestMode) {
            return;
        }

        if (lastPauseSynced === paused) {
            return;
        }

        try {
            if (paused) {
                await pauseAutoReindex();
            } else {
                await resumeAutoReindex();
            }
            lastPauseSynced = paused;
            outputChannel.appendLine(
                `[AutoReindex] LSP state: ${paused ? 'paused' : 'resumed'} (${reason})`
            );
        } catch (error) {
            outputChannel.appendLine(
                `[AutoReindex] Failed to sync LSP pause state (${reason}): ${error}`
            );
        }
    };

    // Build Unified BSL Index
    safeRegisterCommand('bslAnalyzer.buildIndex', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showWarningMessage('No workspace folder found');
            return;
        }

        const configPath = getConfigurationPath();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }

        const choice = await vscode.window.showInformationMessage(
            'Building unified BSL index. This may take a few seconds...',
            'Build Index',
            'Cancel'
        );

        if (choice !== 'Build Index') {
            return;
        }

        const workspacePath = workspaceFolders[0].uri.fsPath;

        try {
            // P4: не дублируем локальный progress (Notification) поверх server-initiated $/progress.
            updateStatusBar('$(sync~spin) BSL: Building index...');

            // Логи/аргументы оставляем только для диагностики (сам build идёт через LSP).
            const args = ['--config', configPath, '--platform-version', getPlatformVersion()];
            const platformDocsArchive = getPlatformDocsArchive();
            if (platformDocsArchive) {
                args.push('--platform-docs-archive', platformDocsArchive);
            }
            outputChannel.appendLine(`BuildIndex (LSP): ${args.join(' ')}`);

            const result = await buildIndex({ workspace_path: workspacePath });

            if (isAttachedBuildIndexResponse(result)) {
                updateStatusBar('$(sync~spin) BSL: Index already running');
                outputChannel.appendLine(`BuildIndex (attached): ${result.message}`);
                vscode.window.showInformationMessage(
                    'BSL Index уже строится на сервере. Подключено к текущей операции.'
                );
                return;
            }

            if (!result.success) {
                updateStatusBar(`$(error) BSL: Index build failed: ${result.message}`);
                vscode.window.showErrorMessage(`Index build failed: ${result.message}`);
                outputChannel.appendLine(`Index build failed: ${result.message}`);
                return;
            }

            updateStatusBar('$(check) BSL: Ready');
            const typesCount = result.types_count || 'unknown';
            vscode.window.showInformationMessage(`BSL Index built successfully with ${typesCount} types`);
        } catch (error) {
            updateStatusBar(`$(error) BSL: Index build failed: ${error}`);
            vscode.window.showErrorMessage(`Index build failed: ${error}`);
            outputChannel.appendLine(`Index build error: ${error}`);
        }
    });

    // Show Index Statistics
    safeRegisterCommand('bslAnalyzer.showIndexStats', async () => {
        const configPath = getConfigurationPath();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }

        updateStatusBar('BSL Analyzer: Loading stats...');

        try {
            const result = await queryType('stats');

            const resultText = JSON.stringify(result, null, 2);
            showIndexStatsWebview(context, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to load index stats: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });

    // Incremental Index Update
    safeRegisterCommand('bslAnalyzer.incrementalUpdate', async () => {
        const configPath = getConfigurationPath();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }

        try {
            // P4: прогресс показывает сервер через $/progress.
            updateStatusBar('$(sync~spin) BSL: Incremental update...');
            const result = await incrementalUpdate(configPath, getPlatformVersion(), [], false);
            if (!result.success) {
                updateStatusBar(`$(error) BSL: Incremental update failed: ${result.message}`);
                vscode.window.showErrorMessage(`Incremental update failed: ${result.message}`);
                outputChannel.appendLine(`Incremental update failed: ${result.message}`);
                return;
            }
            pendingPaths.clear();
            updateStatusBar('$(check) BSL: Ready');
            vscode.window.showInformationMessage(`Index updated successfully: ${result.message}`);
        } catch (error) {
            updateStatusBar(`$(error) BSL: Incremental update failed: ${error}`);
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

        if (!getAutoReindexEnabled()) {
            vscode.window.showWarningMessage('BSL Analyzer: Auto reindex is disabled in settings');
            applyPausedStatus();
            return;
        }

        if (pendingPaths.size > 0) {
            await runAutoUpdate();
        } else {
            updateStatusBar('$(check) BSL: Ready');
        }
    });

    // Reindex Now (manual)
    safeRegisterCommand('bslAnalyzer.reindexNow', async () => {
        const configPath = getConfigurationPath();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }

        const changedPaths = Array.from(pendingPaths);
        pendingPaths.clear();

        try {
            updateStatusBar('$(sync~spin) BSL: Reindex now...');
            const result = await incrementalUpdate(
                configPath,
                getPlatformVersion(),
                changedPaths,
                false
            );
            if (!result.success) {
                updateStatusBar(`$(error) BSL: Reindex failed: ${result.message}`);
                vscode.window.showErrorMessage(`Reindex failed: ${result.message}`);
                outputChannel.appendLine(`Reindex failed: ${result.message}`);
                for (const path of changedPaths) {
                    pendingPaths.add(path);
                }
                applyPausedStatus();
                return;
            }
            updateStatusBar('$(check) BSL: Ready');
            vscode.window.showInformationMessage(`Reindex completed: ${result.message}`);
        } catch (error) {
            updateStatusBar(`$(error) BSL: Reindex failed: ${error}`);
            vscode.window.showErrorMessage(`Reindex failed: ${error}`);
            outputChannel.appendLine(`Reindex error: ${error}`);
            for (const path of changedPaths) {
                pendingPaths.add(path);
            }
        } finally {
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
        for (const w of watchers) w.dispose();
        watchers = [];
    };

    const scheduleAutoUpdate = (reason: string, uri?: vscode.Uri) => {
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
        if (debounceTimer) clearTimeout(debounceTimer);
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
        const configPath = getConfigurationPath();
        if (!configPath) return;
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
            updateStatusBar('$(sync~spin) BSL: Auto reindex...');
            const result = await incrementalUpdate(
                configPath,
                getPlatformVersion(),
                changedPaths,
                true
            );
            if (!result.success) {
                for (const path of changedPaths) {
                    pendingPaths.add(path);
                }
                outputChannel.appendLine(`[AutoReindex] Skipped: ${result.message}`);
                if (/paused/i.test(result.message)) {
                    applyPausedStatus();
                } else {
                    updateStatusBar(`$(error) BSL: Auto reindex failed: ${result.message}`);
                }
                return;
            }
            updateStatusBar('$(check) BSL: Ready');
            outputChannel.appendLine('[AutoReindex] Completed');
        } catch (error) {
            for (const path of changedPaths) {
                pendingPaths.add(path);
            }
            updateStatusBar(`$(error) BSL: Auto reindex failed: ${error}`);
            outputChannel.appendLine(`[AutoReindex] Failed: ${error}`);
        } finally {
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
        const configPath = getConfigurationPath();
        if (!configPath) return;

        // Минимальный набор "горячих" файлов: Configuration.xml, любые Ext/*.bsl, Form.xml.
        const patterns = [
            '**/Ext/*.bsl',
            '**/*.xml'
        ];

        for (const pattern of patterns) {
            const watcher = vscode.workspace.createFileSystemWatcher(
                new vscode.RelativePattern(configPath, pattern)
            );
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
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('bslAnalyzer.configurationPath')) {
                outputChannel.appendLine('[AutoReindex] configurationPath changed, recreating watchers');
                createWatchers();
                pendingPaths.clear();
            }
            if (e.affectsConfiguration('bslAnalyzer.autoReindexEnabled')) {
                outputChannel.appendLine(
                    `[AutoReindex] autoReindexEnabled changed: ${getAutoReindexEnabled()}`
                );
                void syncAutoReindexState('settings change');
                if (!isAutoReindexPaused() && pendingPaths.size > 0) {
                    scheduleAutoUpdate('auto reindex enabled');
                }
            }
        })
    );
}
