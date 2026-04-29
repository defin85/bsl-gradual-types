import * as vscode from 'vscode';
import * as path from 'path';
import { CommandHandler } from '../types';
import { getLanguageClient } from '../lsp';
import { updateStatusBar } from '../lsp/progress';
import { BslAnalyzerConfig } from '../config/configHelper';

export const DEFAULT_RULES_CONFIG_CONTENT = `[semantic.common_module_factories]
builtin_bsp = true

# Add project-specific helpers with:
# [[semantic.common_module_factories.rules]]
# id = "custom-library-module"
# owner = "CommonModules.MyLibrary"
# method = "Module"
# argument_index = 0
# target_mode = "common_module"
# enabled = true
`;

export function resolveRulesConfigUri(workspaceFolder: vscode.WorkspaceFolder): vscode.Uri {
    const configuredPath = BslAnalyzerConfig.rulesConfig.trim();
    if (!configuredPath) {
        return vscode.Uri.joinPath(workspaceFolder.uri, 'bsl-rules.toml');
    }

    if (/^[a-z][a-z0-9+.-]*:/i.test(configuredPath)) {
        return vscode.Uri.parse(configuredPath);
    }

    if (path.isAbsolute(configuredPath)) {
        return vscode.Uri.file(configuredPath);
    }

    return vscode.Uri.joinPath(workspaceFolder.uri, configuredPath);
}

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
        const rulesFile = resolveRulesConfigUri(firstFolder);

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
                    await vscode.workspace.fs.writeFile(
                        rulesFile,
                        Buffer.from(DEFAULT_RULES_CONFIG_CONTENT, 'utf8')
                    );
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
