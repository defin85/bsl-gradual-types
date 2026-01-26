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
exports.registerSearchCommands = void 0;
const vscode = __importStar(require("vscode"));
const progress_1 = require("../lsp/progress");
const customRequests_1 = require("../lsp/customRequests");
const webviews_1 = require("../webviews");
/**
 * Register type search related commands
 */
function registerSearchCommands(context, safeRegisterCommand, _outputChannel) {
    // Search BSL Type
    safeRegisterCommand('bslAnalyzer.searchType', async () => {
        const typeName = await vscode.window.showInputBox({
            prompt: 'Enter BSL type name to search (e.g., "Массив", "Справочники.Номенклатура")',
            placeHolder: 'Type name...'
        });
        if (!typeName) {
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Searching type...');
        try {
            const result = await (0, customRequests_1.queryType)(typeName);
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showTypeInfoWebview)(context, typeName, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Type search failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
    // Search Method in Type
    safeRegisterCommand('bslAnalyzer.searchMethod', async () => {
        const typeName = await vscode.window.showInputBox({
            prompt: 'Enter type name (e.g., "Массив", "Справочники.Номенклатура")',
            placeHolder: 'Type name...'
        });
        if (!typeName) {
            return;
        }
        const methodName = await vscode.window.showInputBox({
            prompt: 'Enter method name to search',
            placeHolder: 'Method name...'
        });
        if (!methodName) {
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Searching method...');
        try {
            const result = await (0, customRequests_1.queryType)(typeName);
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showMethodInfoWebview)(context, typeName, methodName, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Method search failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
    // Explore Type Methods & Properties
    safeRegisterCommand('bslAnalyzer.exploreType', async () => {
        const editor = vscode.window.visibleTextEditors.find(e => e.document.languageId === 'bsl') ||
            vscode.window.activeTextEditor;
        let typeName = '';
        if (editor && editor.selection && !editor.selection.isEmpty) {
            typeName = editor.document.getText(editor.selection);
        }
        if (!typeName) {
            typeName = await vscode.window.showInputBox({
                prompt: 'Enter type name to explore',
                placeHolder: 'Type name...'
            }) || '';
        }
        if (!typeName) {
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Loading type info...');
        try {
            const result = await (0, customRequests_1.queryType)(typeName);
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showTypeExplorerWebview)(context, typeName, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Type exploration failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
    // Check Type Compatibility
    safeRegisterCommand('bslAnalyzer.checkTypeCompatibility', async () => {
        const fromType = await vscode.window.showInputBox({
            prompt: 'Enter source type name',
            placeHolder: 'e.g., Справочники.Номенклатура'
        });
        if (!fromType) {
            return;
        }
        const toType = await vscode.window.showInputBox({
            prompt: 'Enter target type name',
            placeHolder: 'e.g., СправочникСсылка'
        });
        if (!toType) {
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Checking compatibility...');
        try {
            const result = await (0, customRequests_1.checkTypeCompatibility)(fromType, toType);
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showTypeCompatibilityWebview)(context, fromType, toType, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Type compatibility check failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
}
exports.registerSearchCommands = registerSearchCommands;
//# sourceMappingURL=search.js.map