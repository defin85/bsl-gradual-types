import * as vscode from 'vscode';
import { CommandHandler, CodeMetrics } from '../types';
import { 
    getLanguageClient,
    startLanguageClient,
    stopLanguageClient
} from '../lsp';
import {
    startIndexing,
    updateIndexingProgress,
    finishIndexing,
    updateStatusBar
} from '../lsp/progress';
import {
    parseMethodCall,
    getConfigurationPath,
    getPlatformVersion,
    getPlatformDocsArchive
} from '../utils';
import { queryType, buildIndex, validateMethod, checkTypeCompatibility, incrementalUpdate } from '../lsp/customRequests';
import {
    showTypeInfoWebview,
    showMethodInfoWebview,
    showTypeExplorerWebview,
    showIndexStatsWebview,
    showMethodValidationWebview,
    showTypeCompatibilityWebview,
    showMetricsWebview
} from '../webviews';

let outputChannel: vscode.OutputChannel;
let commandsRegistered = false;

export function initializeCommands(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

export async function registerCommands(context: vscode.ExtensionContext) {
    // Защита от двойной регистрации
    if (commandsRegistered) {
        outputChannel.appendLine('⚠️ Commands already registered, skipping...');
        return;
    }
    
    outputChannel.appendLine('📝 Registering BSL Analyzer commands...');
    
    // Helper function to safely register commands with duplicate check
    const safeRegisterCommand = async (commandId: string, callback: CommandHandler) => {
        try {
            const disposable = vscode.commands.registerCommand(commandId, callback);
            context.subscriptions.push(disposable);
            outputChannel.appendLine(`✅ Registered command: ${commandId}`);
            return disposable;
        } catch (error: any) {
            // Если ошибка о том, что команда уже зарегистрирована - это нормально
            if (error.message && error.message.includes('already exists')) {
                outputChannel.appendLine(`⚠️ Command already registered: ${commandId}, skipping...`);
                return null;
            }
            // Другие ошибки - это проблема
            outputChannel.appendLine(`❌ Failed to register command ${commandId}: ${error}`);
            return null;
        }
    };
    
    // Analyze current file - команда расширения больше не нужна
    // LSP сервер автоматически анализирует файлы при открытии/изменении
    // Но оставляем для явного вызова анализа
    await safeRegisterCommand('bslAnalyzer.analyzeFile', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'bsl') {
            vscode.window.showWarningMessage('Please open a BSL file to analyze');
            return;
        }

        try {
            const client = getLanguageClient();
            if (client && client.isRunning()) {
                // Форсируем повторный анализ через запрос диагностики
                // LSP сервер и так анализирует файлы автоматически
                await client.sendRequest('textDocument/diagnostic', {
                    textDocument: {
                        uri: editor.document.uri.toString()
                    }
                });
                vscode.window.showInformationMessage('✅ File analysis completed');
            } else {
                // ✅ ЗАМЕНА CLI → LSP: bsl-analyzer заменён на LSP анализ
                outputChannel.appendLine('⚠️ LSP server not running - please start it first');
                vscode.window.showWarningMessage('LSP server is not running. Please wait for it to start.');
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Analysis failed: ${error}`);
        }
    });

    // Analyze workspace
    await safeRegisterCommand('bslAnalyzer.analyzeWorkspace', async () => {
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
                vscode.window.showInformationMessage('✅ Workspace analysis completed');
            } else {
                vscode.window.showErrorMessage('LSP server not running');
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Workspace analysis failed: ${error}`);
        }
    });

    // Generate reports
    await safeRegisterCommand('bslAnalyzer.generateReports', async () => {
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

    // Show metrics
    await safeRegisterCommand('bslAnalyzer.showMetrics', async () => {
        const editor = vscode.window.activeTextEditor;
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

    // Configure rules
    await safeRegisterCommand('bslAnalyzer.configureRules', async () => {
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

    // Search BSL Type
    await safeRegisterCommand('bslAnalyzer.searchType', async () => {
        const typeName = await vscode.window.showInputBox({
            prompt: 'Enter BSL type name to search (e.g., "Массив", "Справочники.Номенклатура")',
            placeHolder: 'Type name...'
        });

        if (!typeName) {
            return;
        }

        updateStatusBar('BSL Analyzer: Searching type...');

        try {
            // ✅ ЗАМЕНА CLI → LSP: используем LSP custom request вместо fork процесса
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
    await safeRegisterCommand('bslAnalyzer.searchMethod', async () => {
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
            // ✅ ЗАМЕНА CLI → LSP: query_type #2
            const result = await queryType(typeName);

            const resultText = JSON.stringify(result, null, 2);
            showMethodInfoWebview(context, typeName, methodName, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Method search failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });

    // Build Unified BSL Index
    await safeRegisterCommand('bslAnalyzer.buildIndex', async () => {
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

        startIndexing(4);

        try {
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Building BSL Index',
                cancellable: false
            }, async (progress) => {
                updateIndexingProgress(1, 'Loading platform cache...', 10);
                progress.report({ increment: 25, message: 'Loading platform cache...' });
                await new Promise(resolve => setTimeout(resolve, 500));
                
                updateIndexingProgress(2, 'Parsing configuration...', 35);
                progress.report({ increment: 25, message: 'Parsing configuration...' });
                await new Promise(resolve => setTimeout(resolve, 500));
                
                updateIndexingProgress(3, 'Building unified index...', 70);
                progress.report({ increment: 35, message: 'Building unified index...' });
                
                const args = [
                    '--config', configPath,
                    '--platform-version', getPlatformVersion()
                ];
                
                const platformDocsArchive = getPlatformDocsArchive();
                if (platformDocsArchive) {
                    args.push('--platform-docs-archive', platformDocsArchive);
                }
                
                // ✅ ЗАМЕНА CLI → LSP: build_unified_index #1
                const result = await buildIndex(configPath);

                updateIndexingProgress(4, 'Finalizing index...', 90);
                progress.report({ increment: 15, message: 'Finalizing...' });
                
                finishIndexing(true);
                
                let typesCount = 'unknown';
                const typesMatch = result.match(/(\d+)\s+entities/i);
                if (typesMatch && typesMatch[1]) {
                    typesCount = typesMatch[1];
                }
                
                vscode.window.showInformationMessage(`✅ BSL Index built successfully with ${typesCount} types`);
                
                return result;
            });

        } catch (error) {
            finishIndexing(false);
            vscode.window.showErrorMessage(`Index build failed: ${error}`);
            outputChannel.appendLine(`Index build error: ${error}`);
        }
    });

    // Show Index Statistics
    await safeRegisterCommand('bslAnalyzer.showIndexStats', async () => {
        const configPath = getConfigurationPath();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }

        updateStatusBar('BSL Analyzer: Loading stats...');

        try {
            // ✅ ЗАМЕНА CLI → LSP: query_type #3
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
    await safeRegisterCommand('bslAnalyzer.incrementalUpdate', async () => {
        const configPath = getConfigurationPath();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }

        startIndexing(3);

        try {
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Incremental Index Update',
                cancellable: false
            }, async (progress) => {
                updateIndexingProgress(1, 'Analyzing changes...', 20);
                progress.report({ increment: 30, message: 'Analyzing changes...' });
                await new Promise(resolve => setTimeout(resolve, 400));
                
                updateIndexingProgress(2, 'Updating index...', 60);
                progress.report({ increment: 50, message: 'Updating index...' });
                await new Promise(resolve => setTimeout(resolve, 600));

                // ✅ ЗАМЕНА CLI → LSP: incremental_update #11
                const result = await incrementalUpdate(configPath, getPlatformVersion());

                updateIndexingProgress(3, 'Finalizing...', 95);
                progress.report({ increment: 20, message: 'Finalizing...' });

                finishIndexing(true);

                vscode.window.showInformationMessage(`✅ Index updated successfully: ${result.message}`);

                return result.message;
            });
        } catch (error) {
            finishIndexing(false);
            vscode.window.showErrorMessage(`Incremental update failed: ${error}`);
            outputChannel.appendLine(`Incremental update error: ${error}`);
        }
    });

    // Explore Type Methods & Properties
    await safeRegisterCommand('bslAnalyzer.exploreType', async () => {
        const editor = vscode.window.activeTextEditor;
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
            // ✅ ЗАМЕНА CLI → LSP: query_type #4
            const result = await queryType(typeName);

            const resultText = JSON.stringify(result, null, 2);
            showTypeExplorerWebview(context, typeName, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Type exploration failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });

    // Validate Method Call
    await safeRegisterCommand('bslAnalyzer.validateMethodCall', async () => {
        const editor = vscode.window.activeTextEditor;
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

            // ✅ ЗАМЕНА CLI → LSP: query_type #5
            const result = await queryType(methodCallInfo.objectName);

            const resultText = JSON.stringify(result, null, 2);
            showMethodValidationWebview(context, methodCallInfo, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Method validation failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });

    // Check Type Compatibility
    await safeRegisterCommand('bslAnalyzer.checkTypeCompatibility', async () => {
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
            // ✅ ЗАМЕНА CLI → LSP: check_type_compatibility
            const result = await checkTypeCompatibility(fromType, toType);

            const resultText = JSON.stringify(result, null, 2);
            showTypeCompatibilityWebview(context, fromType, toType, resultText);
            updateStatusBar('BSL Analyzer: Ready');
        } catch (error) {
            vscode.window.showErrorMessage(`Type compatibility check failed: ${error}`);
            updateStatusBar('BSL Analyzer: Error');
        }
    });

    // Restart server
    await safeRegisterCommand('bslAnalyzer.restartServer', async () => {
        updateStatusBar('BSL Analyzer: Restarting...');
        outputChannel.appendLine('🔄 Restarting LSP server...');
        
        try {
            await stopLanguageClient();
            await new Promise(resolve => setTimeout(resolve, 1000));
            
            outputChannel.appendLine('🚀 Starting new LSP client...');
            await startLanguageClient(context);
            
            vscode.window.showInformationMessage('✅ BSL Analyzer server restarted');
            outputChannel.appendLine('✅ LSP server restart completed');
        } catch (error) {
            outputChannel.appendLine(`❌ Failed to restart LSP server: ${error}`);
            vscode.window.showErrorMessage(`Failed to restart server: ${error}`);
            updateStatusBar('BSL Analyzer: Restart Failed');
        }
    });

    // Test Progress System (debug only)
    await safeRegisterCommand('bslAnalyzer.testProgress', async () => {
        outputChannel.appendLine('🧪 Testing progress system...');
        
        startIndexing(5);
        
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: 'Testing Progress System',
            cancellable: false
        }, async (progress) => {
            for (let i = 1; i <= 5; i++) {
                const stepName = `Step ${i}: Processing...`;
                const progressPercent = Math.floor((i / 5) * 100);
                
                updateIndexingProgress(i, stepName, progressPercent);
                progress.report({ 
                    increment: 20, 
                    message: stepName 
                });
                
                await new Promise(resolve => setTimeout(resolve, 2000));
            }
            
            finishIndexing(true);
        });
        
        outputChannel.appendLine('✅ Progress system test completed');
    });

    // Устанавливаем флаг, что команды зарегистрированы
    commandsRegistered = true;
    outputChannel.appendLine('✅ Successfully registered 15 extension commands');
}

