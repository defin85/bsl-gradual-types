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
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Building BSL Index',
                cancellable: false
            }, async (progress) => {
                updateStatusBar('$(sync~spin) BSL: Loading platform cache...');
                progress.report({ increment: 25, message: 'Loading platform cache...' });

                updateStatusBar('$(sync~spin) BSL: Parsing configuration...');
                progress.report({ increment: 25, message: 'Parsing configuration...' });

                updateStatusBar('$(sync~spin) BSL: Building unified index...');
                progress.report({ increment: 35, message: 'Building unified index...' });

                const args = [
                    '--config', configPath,
                    '--platform-version', getPlatformVersion()
                ];

                const platformDocsArchive = getPlatformDocsArchive();
                if (platformDocsArchive) {
                    args.push('--platform-docs-archive', platformDocsArchive);
                }

                const result = await buildIndex({ workspace_path: workspacePath });

                updateStatusBar('$(sync~spin) BSL: Finalizing index...');
                progress.report({ increment: 15, message: 'Finalizing...' });

                updateStatusBar('$(check) BSL: Ready');

                const typesCount = result.types_count || 'unknown';

                vscode.window.showInformationMessage(`BSL Index built successfully with ${typesCount} types`);

                return result;
            });

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
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Incremental Index Update',
                cancellable: false
            }, async (progress) => {
                updateStatusBar('$(sync~spin) BSL: Analyzing changes...');
                progress.report({ increment: 30, message: 'Analyzing changes...' });

                updateStatusBar('$(sync~spin) BSL: Updating index...');
                progress.report({ increment: 50, message: 'Updating index...' });

                const result = await incrementalUpdate(configPath, getPlatformVersion());

                updateStatusBar('$(sync~spin) BSL: Finalizing...');
                progress.report({ increment: 20, message: 'Finalizing...' });

                updateStatusBar('$(check) BSL: Ready');

                vscode.window.showInformationMessage(`Index updated successfully: ${result.message}`);

                return result.message;
            });
        } catch (error) {
            updateStatusBar(`$(error) BSL: Incremental update failed: ${error}`);
            vscode.window.showErrorMessage(`Incremental update failed: ${error}`);
            outputChannel.appendLine(`Incremental update error: ${error}`);
        }
    });
}
