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
exports.registerConfigurationCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const progress_1 = require("../lsp/progress");
/**
 * Register configuration-related commands
 */
function registerConfigurationCommands(context, safeRegisterCommand, _outputChannel) {
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
        }
        catch {
            const createFile = await vscode.window.showInformationMessage('Rules configuration file not found. Would you like to create one?', 'Create Rules File');
            if (createFile) {
                try {
                    const client = (0, lsp_1.getLanguageClient)();
                    if (!client) {
                        throw new Error('LSP client is not running');
                    }
                    await client.sendRequest('workspace/executeCommand', {
                        command: 'bslAnalyzer.createRulesConfig',
                        arguments: [rulesFile.toString()]
                    });
                    const document = await vscode.workspace.openTextDocument(rulesFile);
                    await vscode.window.showTextDocument(document);
                }
                catch (error) {
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
        (0, progress_1.updateStatusBar)('BSL Analyzer: Generating reports...');
        try {
            const client = (0, lsp_1.getLanguageClient)();
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
            const openReports = await vscode.window.showInformationMessage('Reports generated successfully', 'Open Reports Folder');
            if (openReports) {
                vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(outputDir));
            }
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Report generation failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
}
exports.registerConfigurationCommands = registerConfigurationCommands;
//# sourceMappingURL=configuration.js.map