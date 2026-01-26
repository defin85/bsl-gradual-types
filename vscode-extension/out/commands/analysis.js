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
exports.registerAnalysisCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const progress_1 = require("../lsp/progress");
const utils_1 = require("../utils");
const webviews_1 = require("../webviews");
/**
 * Register analysis-related commands
 */
function registerAnalysisCommands(context, safeRegisterCommand, outputChannel) {
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
            const client = (0, lsp_1.getLanguageClient)();
            if (client && client.isRunning()) {
                // Форсируем повторный анализ через запрос диагностики
                await client.sendRequest('textDocument/diagnostic', {
                    textDocument: {
                        uri: editor.document.uri.toString()
                    }
                });
                vscode.window.showInformationMessage('File analysis completed');
            }
            else {
                outputChannel.appendLine('LSP server not running - please start it first');
                vscode.window.showWarningMessage('LSP server is not running. Please wait for it to start.');
            }
        }
        catch (error) {
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
            const client = (0, lsp_1.getLanguageClient)();
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
            }
            else {
                vscode.window.showErrorMessage('LSP server not running');
            }
        }
        catch (error) {
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
            const client = (0, lsp_1.getLanguageClient)();
            if (!client) {
                throw new Error('LSP client is not running');
            }
            const metrics = await client.sendRequest('workspace/executeCommand', {
                command: 'bslAnalyzer.getMetrics',
                arguments: [editor.document.uri.toString()]
            });
            (0, webviews_1.showMetricsWebview)(context, metrics);
        }
        catch (error) {
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
        (0, progress_1.updateStatusBar)('BSL Analyzer: Validating method call...');
        try {
            const methodCallInfo = (0, utils_1.parseMethodCall)(selectedText);
            if (!methodCallInfo) {
                vscode.window.showWarningMessage('Invalid method call format');
                return;
            }
            const { queryType } = await Promise.resolve().then(() => __importStar(require('../lsp/customRequests')));
            const result = await queryType(methodCallInfo.objectName);
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showMethodValidationWebview)(context, methodCallInfo, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Method validation failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
}
exports.registerAnalysisCommands = registerAnalysisCommands;
//# sourceMappingURL=analysis.js.map