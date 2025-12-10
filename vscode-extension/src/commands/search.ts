import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { updateStatusBar } from '../lsp/progress';
import { queryType, checkTypeCompatibility } from '../lsp/customRequests';
import {
    showTypeInfoWebview,
    showMethodInfoWebview,
    showTypeExplorerWebview,
    showTypeCompatibilityWebview
} from '../webviews';

/**
 * Register type search related commands
 */
export function registerSearchCommands(
    context: vscode.ExtensionContext,
    safeRegisterCommand: (commandId: string, callback: CommandHandler) => Promise<vscode.Disposable | null>,
    _outputChannel: vscode.OutputChannel
) {
    // Search BSL Type
    safeRegisterCommand('bslAnalyzer.searchType', async () => {
        const typeName = await vscode.window.showInputBox({
            prompt: 'Enter BSL type name to search (e.g., "Массив", "Справочники.Номенклатура")',
            placeHolder: 'Type name...'
        });

        if (!typeName) {
            return;
        }

        updateStatusBar('BSL Analyzer: Searching type...');

        try {
            const result = await queryType(typeName);

            const resultText = JSON.stringify(result, null, 2);
            showTypeInfoWebview(context, typeName, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Type search failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
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

        updateStatusBar('BSL Analyzer: Searching method...');

        try {
            const result = await queryType(typeName);

            const resultText = JSON.stringify(result, null, 2);
            showMethodInfoWebview(context, typeName, methodName, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Method search failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
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

        updateStatusBar('BSL Analyzer: Loading type info...');

        try {
            const result = await queryType(typeName);

            const resultText = JSON.stringify(result, null, 2);
            showTypeExplorerWebview(context, typeName, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Type exploration failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
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

        updateStatusBar('BSL Analyzer: Checking compatibility...');

        try {
            const result = await checkTypeCompatibility(fromType, toType);

            const resultText = JSON.stringify(result, null, 2);
            showTypeCompatibilityWebview(context, fromType, toType, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Type compatibility check failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });
}
