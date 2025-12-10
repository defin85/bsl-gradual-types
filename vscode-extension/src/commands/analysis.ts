import * as vscode from 'vscode';
import { CommandHandler, CodeMetrics } from '../types';
import { getLanguageClient } from '../lsp';
import { updateStatusBar } from '../lsp/progress';
import { parseMethodCall } from '../utils';
import { showMetricsWebview, showMethodValidationWebview } from '../webviews';

/**
 * Register analysis-related commands
 */
export function registerAnalysisCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>,
    outputChannel: vscode.OutputChannel
) {
    // Analyze current file
    safeRegisterCommand('bslAnalyzer.analyzeFile', async () => {
        // Приоритет BSL файлу из visibleTextEditors (activeTextEditor может быть Output)
        const editor = vscode.window.visibleTextEditors.find(e => e.document.languageId === 'bsl') ||
            vscode.window.activeTextEditor;

        if (!editor || editor.document.languageId !== 'bsl') {
            vscode.window.showWarningMessage('Please open a BSL file to analyze');
            return;
        }

        try {
            const client = getLanguageClient();
            if (client && client.isRunning()) {
                // Форсируем повторный анализ через запрос диагностики
                await client.sendRequest('textDocument/diagnostic', {
                    textDocument: {
                        uri: editor.document.uri.toString()
                    }
                });
                vscode.window.showInformationMessage('File analysis completed');
            } else {
                outputChannel.appendLine('LSP server not running - please start it first');
                vscode.window.showWarningMessage('LSP server is not running. Please wait for it to start.');
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Analysis failed: ${error}`);
        }
    });

    // Analyze workspace
    safeRegisterCommand('bslAnalyzer.analyzeWorkspace', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showWarningMessage('No workspace folder is open');
            return;
        }

        try {
            const client = getLanguageClient();
            if (client && client.isRunning()) {
                const firstFolder = workspaceFolders[0];
                if (!firstFolder) {
                    vscode.window.showErrorMessage('No workspace folder found');
                    return;
                }
                await client.sendRequest('workspace/executeCommand', {
                    command: 'bslAnalyzer.lsp.analyzeWorkspace',
                    arguments: [firstFolder.uri.toString()]
                });
                vscode.window.showInformationMessage('Workspace analysis completed');
            } else {
                vscode.window.showErrorMessage('LSP server not running');
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Workspace analysis failed: ${error}`);
        }
    });

    // Show metrics
    safeRegisterCommand('bslAnalyzer.showMetrics', async () => {
        const editor = vscode.window.visibleTextEditors.find(e => e.document.languageId === 'bsl') ||
            vscode.window.activeTextEditor;

        if (!editor || editor.document.languageId !== 'bsl') {
            vscode.window.showWarningMessage('Please open a BSL file to show metrics');
            return;
        }

        try {
            const client = getLanguageClient();
            if (!client) {
                throw new Error('LSP client is not running');
            }
            const metrics = await client.sendRequest('workspace/executeCommand', {
                command: 'bslAnalyzer.getMetrics',
                arguments: [editor.document.uri.toString()]
            });

            showMetricsWebview(context, metrics as CodeMetrics);
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to get metrics: ${error}`);
        }
    });

    // Validate Method Call
    safeRegisterCommand('bslAnalyzer.validateMethodCall', async () => {
        const editor = vscode.window.visibleTextEditors.find(e => e.document.languageId === 'bsl') ||
            vscode.window.activeTextEditor;

        if (!editor || editor.document.languageId !== 'bsl') {
            vscode.window.showWarningMessage('Please open a BSL file and select a method call');
            return;
        }

        let selectedText = '';
        if (editor.selection && !editor.selection.isEmpty) {
            selectedText = editor.document.getText(editor.selection);
        }

        if (!selectedText) {
            vscode.window.showWarningMessage('Please select a method call to validate');
            return;
        }

        updateStatusBar('BSL Analyzer: Validating method call...');

        try {
            const methodCallInfo = parseMethodCall(selectedText);
            if (!methodCallInfo) {
                vscode.window.showWarningMessage('Invalid method call format');
                return;
            }

            const { queryType } = await import('../lsp/customRequests');
            const result = await queryType(methodCallInfo.objectName);

            const resultText = JSON.stringify(result, null, 2);
            showMethodValidationWebview(context, methodCallInfo, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Method validation failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });
}
