import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { getLanguageClient } from '../lsp';
import { updateStatusBar } from '../lsp/progress';

/**
 * Register configuration-related commands
 */
export function registerConfigurationCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>,
    _outputChannel: vscode.OutputChannel
) {
    // Configure rules
    safeRegisterCommand('bslAnalyzer.configureRules', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showWarningMessage('No workspace folder is open');
            return;
        }

        const firstFolder = workspaceFolders[0];
        if (!firstFolder) {
            vscode.window.showWarningMessage('No workspace folder found');
            return;
        }
        const rulesFile = vscode.Uri.joinPath(firstFolder.uri, 'bsl-rules.toml');

        try {
            await vscode.workspace.fs.stat(rulesFile);
            const document = await vscode.workspace.openTextDocument(rulesFile);
            await vscode.window.showTextDocument(document);
        } catch {
            const createFile = await vscode.window.showInformationMessage(
                'Rules configuration file not found. Would you like to create one?',
                'Create Rules File'
            );

            if (createFile) {
                try {
                    const client = getLanguageClient();
                    if (!client) {
                        throw new Error('LSP client is not running');
                    }
                    await client.sendRequest('workspace/executeCommand', {
                        command: 'bslAnalyzer.createRulesConfig',
                        arguments: [rulesFile.toString()]
                    });

                    const document = await vscode.workspace.openTextDocument(rulesFile);
                    await vscode.window.showTextDocument(document);
                } catch (error) {
                    vscode.window.showErrorMessage(`Failed to create rules file: ${error}`);
                }
            }
        }
    });

    // Generate reports
    safeRegisterCommand('bslAnalyzer.generateReports', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showWarningMessage('No workspace folder is open');
            return;
        }

        const outputDir = await vscode.window.showInputBox({
            prompt: 'Enter output directory for reports',
            value: './reports'
        });

        if (!outputDir) {
            return;
        }

        updateStatusBar('BSL Analyzer: Generating reports...');

        try {
            const client = getLanguageClient();
            if (!client) {
                throw new Error('LSP client is not running');
            }
            const firstFolder = workspaceFolders[0];
            if (!firstFolder) {
                throw new Error('No workspace folder found');
            }
            await client.sendRequest('workspace/executeCommand', {
                command: 'bslAnalyzer.generateReports',
                arguments: [firstFolder.uri.toString(), outputDir]
            });

            const openReports = await vscode.window.showInformationMessage(
                'Reports generated successfully',
                'Open Reports Folder'
            );

            if (openReports) {
                vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(outputDir));
            }

            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Report generation failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });
}
