import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { updateStatusBar } from '../lsp/progress';
import { getConfigurationPath, getPlatformVersion, getPlatformDocsArchive } from '../utils';
import { queryType, buildIndex, incrementalUpdate } from '../lsp/customRequests';
import { showIndexStatsWebview } from '../webviews';

/**
 * Register index-related commands (build, stats, incremental update)
 */
export function registerIndexCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>,
    outputChannel: vscode.OutputChannel
) {
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
            const result = await incrementalUpdate(configPath, getPlatformVersion());
            updateStatusBar('$(check) BSL: Ready');
            vscode.window.showInformationMessage(`Index updated successfully: ${result.message}`);
        } catch (error) {
            updateStatusBar(`$(error) BSL: Incremental update failed: ${error}`);
            vscode.window.showErrorMessage(`Incremental update failed: ${error}`);
            outputChannel.appendLine(`Incremental update error: ${error}`);
        }
    });

    // P5: авто-реиндексация при изменениях файлов конфигурации (debounce + single-flight).
    // В тестовом режиме отключаем, чтобы не провоцировать фоновые запросы/таймауты.
    const isTestMode =
        process.env.NODE_ENV === 'test' ||
        process.env.VSCODE_TEST_MODE === '1' ||
        process.env.VSCODE_EXTENSION_MODE === 'test';

    if (isTestMode) {
        outputChannel.appendLine('[AutoReindex] Disabled in test mode');
        return;
    }

    let watchers: vscode.Disposable[] = [];
    let debounceTimer: NodeJS.Timeout | undefined;
    let inFlight = false;
    let pending = false;

    const disposeWatchers = () => {
        for (const w of watchers) w.dispose();
        watchers = [];
    };

    const scheduleAutoUpdate = (reason: string) => {
        outputChannel.appendLine(`[AutoReindex] Schedule: ${reason}`);
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => void runAutoUpdate(), 1200);
    };

    const runAutoUpdate = async () => {
        const configPath = getConfigurationPath();
        if (!configPath) return;

        if (inFlight) {
            pending = true;
            return;
        }

        inFlight = true;
        pending = false;

        try {
            updateStatusBar('$(sync~spin) BSL: Auto reindex...');
            await incrementalUpdate(configPath, getPlatformVersion());
            updateStatusBar('$(check) BSL: Ready');
            outputChannel.appendLine('[AutoReindex] Completed');
        } catch (error) {
            updateStatusBar(`$(error) BSL: Auto reindex failed: ${error}`);
            outputChannel.appendLine(`[AutoReindex] Failed: ${error}`);
        } finally {
            inFlight = false;
            if (pending) {
                pending = false;
                scheduleAutoUpdate('pending changes while in-flight');
            }
        }
    };

    const createWatchers = () => {
        disposeWatchers();
        const configPath = getConfigurationPath();
        if (!configPath) return;

        // Минимальный набор "горячих" файлов: Configuration.xml, любые Ext/*.bsl, Form.xml.
        const patterns = [
            'Configuration.xml',
            '**/Ext/*.bsl',
            '**/Form.xml'
        ];

        for (const pattern of patterns) {
            const watcher = vscode.workspace.createFileSystemWatcher(
                new vscode.RelativePattern(configPath, pattern)
            );
            watcher.onDidCreate(() => scheduleAutoUpdate(`create: ${pattern}`));
            watcher.onDidChange(() => scheduleAutoUpdate(`change: ${pattern}`));
            watcher.onDidDelete(() => scheduleAutoUpdate(`delete: ${pattern}`));
            watchers.push(watcher);
        }

        outputChannel.appendLine(`[AutoReindex] Watchers installed for: ${configPath}`);
    };

    createWatchers();
    context.subscriptions.push({ dispose: disposeWatchers });
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('bslAnalyzer.configurationPath')) {
                outputChannel.appendLine('[AutoReindex] configurationPath changed, recreating watchers');
                createWatchers();
            }
        })
    );
}
