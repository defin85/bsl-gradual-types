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
exports.registerDebugCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const progress_1 = require("../lsp/progress");
/**
 * Register debug and utility commands
 */
function registerDebugCommands(context, safeRegisterCommand, outputChannel) {
    // Restart server
    safeRegisterCommand('bslAnalyzer.restartServer', async () => {
        (0, progress_1.updateStatusBar)('BSL Analyzer: Restarting...');
        outputChannel.appendLine('Restarting LSP server...');
        try {
            await (0, lsp_1.stopLanguageClient)();
            outputChannel.appendLine('Starting new LSP client...');
            await (0, lsp_1.startLanguageClient)(context);
            vscode.window.showInformationMessage('BSL Analyzer server restarted');
            outputChannel.appendLine('LSP server restart completed');
        }
        catch (error) {
            outputChannel.appendLine(`Failed to restart LSP server: ${error}`);
            vscode.window.showErrorMessage(`Failed to restart server: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Restart Failed');
        }
    });
    // Test Progress System (debug only)
    safeRegisterCommand('bslAnalyzer.testProgress', async () => {
        outputChannel.appendLine('');
        outputChannel.appendLine('===================================================');
        outputChannel.appendLine('TESTING PROGRESS SYSTEM (Enhanced Debug Mode)');
        outputChannel.appendLine('===================================================');
        outputChannel.appendLine('');
        const totalSteps = 20;
        const stepDelay = 500;
        outputChannel.appendLine(`Configuration:`);
        outputChannel.appendLine(`   - Total steps: ${totalSteps}`);
        outputChannel.appendLine(`   - Delay per step: ${stepDelay}ms`);
        outputChannel.appendLine(`   - UI Throttling: 500ms (matches production)`);
        outputChannel.appendLine('');
        outputChannel.appendLine('Starting test progress...');
        (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Testing progress system...');
        outputChannel.appendLine('   Progress started');
        outputChannel.appendLine('');
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: 'Testing Progress System',
            cancellable: false
        }, async (progress) => {
            const mockTypes = [
                'Строка', 'Число', 'Дата', 'Булево', 'Массив',
                'Структура', 'Соответствие', 'СписокЗначений', 'ТаблицаЗначений',
                'Справочники.Контрагенты', 'Документы.РеализацияТоваровУслуг',
                'РегистрыСведений.Цены', 'Обработки.ЗагрузкаДанных',
                'HTTPСоединение', 'XMLЧтение', 'XMLЗапись',
                'ФайловыйПоток', 'ЧтениеJSON', 'ЗаписьJSON', 'ДвоичныеДанные'
            ];
            for (let i = 1; i <= totalSteps; i++) {
                const currentType = mockTypes[i - 1] || `Type${i}`;
                const progressPercent = Math.floor((i / totalSteps) * 100);
                const eta = Math.floor(((totalSteps - i) * stepDelay) / 1000);
                outputChannel.appendLine(`---------------------------------------------------`);
                outputChannel.appendLine(`Step ${i}/${totalSteps} (${progressPercent}%)`);
                outputChannel.appendLine(`   Current Type: ${currentType}`);
                outputChannel.appendLine(`   ETA: ${eta}s`);
                const stepName = `Parsing type ${i}/${totalSteps}: ${currentType}`;
                outputChannel.appendLine(`   Calling updateStatusBar("${stepName}")...`);
                (0, progress_1.updateStatusBar)(`$(sync~spin) BSL: ${stepName}`);
                outputChannel.appendLine(`   Updating VSCode notification...`);
                progress.report({
                    increment: Math.floor(100 / totalSteps),
                    message: `${currentType} (${progressPercent}%)`
                });
                outputChannel.appendLine(`   Sleeping ${stepDelay}ms...`);
                await new Promise(resolve => setTimeout(resolve, stepDelay));
                outputChannel.appendLine(`   Step ${i} completed`);
            }
            outputChannel.appendLine('');
            outputChannel.appendLine('Finishing progress...');
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
            outputChannel.appendLine('   Progress finished');
        });
        outputChannel.appendLine('');
        outputChannel.appendLine('===================================================');
        outputChannel.appendLine('PROGRESS SYSTEM TEST COMPLETED');
        outputChannel.appendLine('===================================================');
        outputChannel.appendLine('');
        outputChannel.appendLine('Summary:');
        outputChannel.appendLine(`   - Total steps processed: ${totalSteps}`);
        outputChannel.appendLine(`   - Total time: ~${(totalSteps * stepDelay) / 1000}s`);
        outputChannel.appendLine(`   - Status: SUCCESS`);
        outputChannel.appendLine('');
        outputChannel.appendLine('Check the status bar at the bottom for visual progress!');
        outputChannel.appendLine('');
    });
    // Show Semantic Visualization (MILESTONE 2.16)
    safeRegisterCommand('bsl-gradual-types.showSemanticVisualization', async () => {
        const editor = vscode.window.visibleTextEditors.find(e => e.document.languageId === 'bsl') ||
            vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'bsl') {
            vscode.window.showWarningMessage('No BSL file is open');
            return;
        }
        const client = (0, lsp_1.getLanguageClient)();
        if (!client || !client.isRunning()) {
            vscode.window.showErrorMessage('LSP server is not running. Please wait or restart the server.');
            return;
        }
        const uri = editor.document.uri.toString();
        const fileName = editor.document.fileName.split(/[/\\]/).pop() || 'unknown.bsl';
        const panel = vscode.window.createWebviewPanel('bslSemanticVisualization', `Semantic Tree: ${fileName}`, vscode.ViewColumn.Two, {
            enableScripts: true,
            retainContextWhenHidden: true
        });
        // Loading indicator
        panel.webview.html = `<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
        }
        .spinner {
            border: 4px solid rgba(0, 0, 0, 0.1);
            border-left-color: var(--vscode-progressBar-background);
            border-radius: 50%;
            width: 40px;
            height: 40px;
            animation: spin 1s linear infinite;
        }
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <div>
        <div class="spinner"></div>
        <p>Loading semantic tree...</p>
    </div>
</body>
</html>`;
        try {
            const isDark = vscode.window.activeColorTheme.kind === vscode.ColorThemeKind.Dark;
            const theme = isDark ? 'dark' : 'light';
            const response = await client.sendRequest('workspace/executeCommand', {
                command: 'bsl.getSemanticHtml',
                arguments: [{
                        uri: uri,
                        theme: theme,
                        compact: false
                    }]
            });
            panel.webview.html = response.html;
        }
        catch (error) {
            const errorMessage = error?.toString() || 'Unknown error';
            panel.webview.html = `<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
            padding: 20px;
        }
        h1 {
            color: var(--vscode-errorForeground);
        }
        pre {
            background: var(--vscode-textCodeBlock-background);
            padding: 10px;
            border-radius: 4px;
            overflow-x: auto;
        }
    </style>
</head>
<body>
    <h1>Error</h1>
    <pre>${errorMessage}</pre>
</body>
</html>`;
            vscode.window.showErrorMessage(`Error getting semantic tree: ${errorMessage}`);
        }
    });
    // MILESTONE 2.20.3: Register bsl.getCurrentContext command (proxy to LSP)
    safeRegisterCommand('bsl.getCurrentContext', async (params) => {
        const client = (0, lsp_1.getLanguageClient)();
        if (!client || !client.isRunning()) {
            outputChannel.appendLine('bsl.getCurrentContext: LSP client not running');
            return null;
        }
        try {
            outputChannel.appendLine(`bsl.getCurrentContext called with params: ${JSON.stringify(params)}`);
            const result = await client.sendRequest('workspace/executeCommand', {
                command: 'bsl.getCurrentContext',
                arguments: [params]
            });
            outputChannel.appendLine(`bsl.getCurrentContext result: ${JSON.stringify(result)}`);
            return result;
        }
        catch (error) {
            outputChannel.appendLine(`Error calling bsl.getCurrentContext: ${error}`);
            return null;
        }
    });
}
exports.registerDebugCommands = registerDebugCommands;
//# sourceMappingURL=debug.js.map