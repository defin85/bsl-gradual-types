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
exports.registerCommands = exports.initializeCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const progress_1 = require("../lsp/progress");
const utils_1 = require("../utils");
const customRequests_1 = require("../lsp/customRequests");
const webviews_1 = require("../webviews");
const parseConfiguration_1 = require("./parseConfiguration");
let outputChannel;
let commandsRegistered = false;
function initializeCommands(channel) {
    outputChannel = channel;
}
exports.initializeCommands = initializeCommands;
async function registerCommands(context) {
    // Защита от двойной регистрации
    if (commandsRegistered) {
        outputChannel.appendLine('⚠️ Commands already registered, skipping...');
        return;
    }
    outputChannel.appendLine('📝 Registering BSL Analyzer commands...');
    // Helper function to safely register commands with duplicate check
    const safeRegisterCommand = async (commandId, callback) => {
        try {
            const disposable = vscode.commands.registerCommand(commandId, callback);
            context.subscriptions.push(disposable);
            outputChannel.appendLine(`✅ Registered command: ${commandId}`);
            return disposable;
        }
        catch (error) {
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
        // ✅ ИСПРАВЛЕНИЕ: Приоритет BSL файлу из visibleTextEditors (activeTextEditor может быть Output)
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
                // LSP сервер и так анализирует файлы автоматически
                await client.sendRequest('textDocument/diagnostic', {
                    textDocument: {
                        uri: editor.document.uri.toString()
                    }
                });
                vscode.window.showInformationMessage('✅ File analysis completed');
            }
            else {
                // ✅ ЗАМЕНА CLI → LSP: bsl-analyzer заменён на LSP анализ
                outputChannel.appendLine('⚠️ LSP server not running - please start it first');
                vscode.window.showWarningMessage('LSP server is not running. Please wait for it to start.');
            }
        }
        catch (error) {
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
                vscode.window.showInformationMessage('✅ Workspace analysis completed');
            }
            else {
                vscode.window.showErrorMessage('LSP server not running');
            }
        }
        catch (error) {
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
    // Show metrics
    await safeRegisterCommand('bslAnalyzer.showMetrics', async () => {
        // ✅ ИСПРАВЛЕНИЕ: Приоритет BSL файлу из visibleTextEditors (activeTextEditor может быть Output)
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
    // Search BSL Type
    await safeRegisterCommand('bslAnalyzer.searchType', async () => {
        const typeName = await vscode.window.showInputBox({
            prompt: 'Enter BSL type name to search (e.g., "Массив", "Справочники.Номенклатура")',
            placeHolder: 'Type name...'
        });
        if (!typeName) {
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Searching type...');
        try {
            // ✅ ЗАМЕНА CLI → LSP: используем LSP custom request вместо fork процесса
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
        (0, progress_1.updateStatusBar)('BSL Analyzer: Searching method...');
        try {
            // ✅ ЗАМЕНА CLI → LSP: query_type #2
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
    // Build Unified BSL Index
    await safeRegisterCommand('bslAnalyzer.buildIndex', async () => {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showWarningMessage('No workspace folder found');
            return;
        }
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }
        const choice = await vscode.window.showInformationMessage('Building unified BSL index. This may take a few seconds...', 'Build Index', 'Cancel');
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
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Loading platform cache...');
                progress.report({ increment: 25, message: 'Loading platform cache...' });
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Parsing configuration...');
                progress.report({ increment: 25, message: 'Parsing configuration...' });
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Building unified index...');
                progress.report({ increment: 35, message: 'Building unified index...' });
                const args = [
                    '--config', configPath,
                    '--platform-version', (0, utils_1.getPlatformVersion)()
                ];
                const platformDocsArchive = (0, utils_1.getPlatformDocsArchive)();
                if (platformDocsArchive) {
                    args.push('--platform-docs-archive', platformDocsArchive);
                }
                // ✅ ЗАМЕНА CLI → LSP: build_unified_index #1
                const result = await (0, customRequests_1.buildIndex)({ workspace_path: workspacePath });
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Finalizing index...');
                progress.report({ increment: 15, message: 'Finalizing...' });
                (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
                const typesCount = result.types_count || 'unknown';
                vscode.window.showInformationMessage(`✅ BSL Index built successfully with ${typesCount} types`);
                return result;
            });
        }
        catch (error) {
            (0, progress_1.updateStatusBar)(`$(error) BSL: Index build failed: ${error}`);
            vscode.window.showErrorMessage(`Index build failed: ${error}`);
            outputChannel.appendLine(`Index build error: ${error}`);
        }
    });
    // Show Index Statistics
    await safeRegisterCommand('bslAnalyzer.showIndexStats', async () => {
        const configPath = (0, utils_1.getConfigurationPath)();
        if (!configPath) {
            vscode.window.showWarningMessage('Please configure the 1C configuration path in settings');
            return;
        }
        (0, progress_1.updateStatusBar)('BSL Analyzer: Loading stats...');
        try {
            // ✅ ЗАМЕНА CLI → LSP: query_type #3
            const result = await (0, customRequests_1.queryType)('stats');
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showIndexStatsWebview)(context, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Failed to load index stats: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
        }
    });
    // Incremental Index Update
    await safeRegisterCommand('bslAnalyzer.incrementalUpdate', async () => {
        const configPath = (0, utils_1.getConfigurationPath)();
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
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Analyzing changes...');
                progress.report({ increment: 30, message: 'Analyzing changes...' });
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Updating index...');
                progress.report({ increment: 50, message: 'Updating index...' });
                // ✅ ЗАМЕНА CLI → LSP: incremental_update #11
                const result = await (0, customRequests_1.incrementalUpdate)(configPath, (0, utils_1.getPlatformVersion)());
                (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Finalizing...');
                progress.report({ increment: 20, message: 'Finalizing...' });
                (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
                vscode.window.showInformationMessage(`✅ Index updated successfully: ${result.message}`);
                return result.message;
            });
        }
        catch (error) {
            (0, progress_1.updateStatusBar)(`$(error) BSL: Incremental update failed: ${error}`);
            vscode.window.showErrorMessage(`Incremental update failed: ${error}`);
            outputChannel.appendLine(`Incremental update error: ${error}`);
        }
    });
    // Explore Type Methods & Properties
    await safeRegisterCommand('bslAnalyzer.exploreType', async () => {
        // ✅ ИСПРАВЛЕНИЕ: Приоритет BSL файлу из visibleTextEditors (activeTextEditor может быть Output)
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
            // ✅ ЗАМЕНА CLI → LSP: query_type #4
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
    // Validate Method Call
    await safeRegisterCommand('bslAnalyzer.validateMethodCall', async () => {
        // ✅ ИСПРАВЛЕНИЕ: Приоритет BSL файлу из visibleTextEditors (activeTextEditor может быть Output)
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
            // ✅ ЗАМЕНА CLI → LSP: query_type #5
            const result = await (0, customRequests_1.queryType)(methodCallInfo.objectName);
            const resultText = JSON.stringify(result, null, 2);
            (0, webviews_1.showMethodValidationWebview)(context, methodCallInfo, resultText);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Ready');
        }
        catch (error) {
            vscode.window.showErrorMessage(`Method validation failed: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Error');
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
        (0, progress_1.updateStatusBar)('BSL Analyzer: Checking compatibility...');
        try {
            // ✅ ЗАМЕНА CLI → LSP: check_type_compatibility
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
    // Restart server
    await safeRegisterCommand('bslAnalyzer.restartServer', async () => {
        (0, progress_1.updateStatusBar)('BSL Analyzer: Restarting...');
        outputChannel.appendLine('🔄 Restarting LSP server...');
        try {
            await (0, lsp_1.stopLanguageClient)();
            outputChannel.appendLine('🚀 Starting new LSP client...');
            await (0, lsp_1.startLanguageClient)(context);
            vscode.window.showInformationMessage('✅ BSL Analyzer server restarted');
            outputChannel.appendLine('✅ LSP server restart completed');
        }
        catch (error) {
            outputChannel.appendLine(`❌ Failed to restart LSP server: ${error}`);
            vscode.window.showErrorMessage(`Failed to restart server: ${error}`);
            (0, progress_1.updateStatusBar)('BSL Analyzer: Restart Failed');
        }
    });
    // Test Progress System (debug only)
    await safeRegisterCommand('bslAnalyzer.testProgress', async () => {
        outputChannel.appendLine('');
        outputChannel.appendLine('═══════════════════════════════════════════════════════');
        outputChannel.appendLine('🧪 TESTING PROGRESS SYSTEM (Enhanced Debug Mode)');
        outputChannel.appendLine('═══════════════════════════════════════════════════════');
        outputChannel.appendLine('');
        const totalSteps = 20; // Эмулируем парсинг 20 типов
        const stepDelay = 500; // 500ms между обновлениями (как в реальном throttling)
        outputChannel.appendLine(`📊 Configuration:`);
        outputChannel.appendLine(`   - Total steps: ${totalSteps}`);
        outputChannel.appendLine(`   - Delay per step: ${stepDelay}ms`);
        outputChannel.appendLine(`   - UI Throttling: 500ms (matches production)`);
        outputChannel.appendLine('');
        outputChannel.appendLine('🚀 Starting test progress...');
        (0, progress_1.updateStatusBar)('$(sync~spin) BSL: Testing progress system...');
        outputChannel.appendLine('   ✓ Progress started');
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
                const currentType = mockTypes[i - 1] || `Тип${i}`;
                const progressPercent = Math.floor((i / totalSteps) * 100);
                const eta = Math.floor(((totalSteps - i) * stepDelay) / 1000);
                // Детальное логирование каждого шага
                outputChannel.appendLine(`━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`);
                outputChannel.appendLine(`📦 Step ${i}/${totalSteps} (${progressPercent}%)`);
                outputChannel.appendLine(`   Current Type: ${currentType}`);
                outputChannel.appendLine(`   ETA: ${eta}s`);
                const stepName = `Парсинг типа ${i}/${totalSteps}: ${currentType}`;
                outputChannel.appendLine(`   Calling updateStatusBar("${stepName}")...`);
                (0, progress_1.updateStatusBar)(`$(sync~spin) BSL: ${stepName}`);
                outputChannel.appendLine(`   Updating VSCode notification...`);
                progress.report({
                    increment: Math.floor(100 / totalSteps),
                    message: `${currentType} (${progressPercent}%)`
                });
                outputChannel.appendLine(`   ⏳ Sleeping ${stepDelay}ms...`);
                await new Promise(resolve => setTimeout(resolve, stepDelay));
                outputChannel.appendLine(`   ✓ Step ${i} completed`);
            }
            outputChannel.appendLine('');
            outputChannel.appendLine('🏁 Finishing progress...');
            (0, progress_1.updateStatusBar)('$(check) BSL: Ready');
            outputChannel.appendLine('   ✓ Progress finished');
        });
        outputChannel.appendLine('');
        outputChannel.appendLine('═══════════════════════════════════════════════════════');
        outputChannel.appendLine('✅ PROGRESS SYSTEM TEST COMPLETED');
        outputChannel.appendLine('═══════════════════════════════════════════════════════');
        outputChannel.appendLine('');
        outputChannel.appendLine('📋 Summary:');
        outputChannel.appendLine(`   - Total steps processed: ${totalSteps}`);
        outputChannel.appendLine(`   - Total time: ~${(totalSteps * stepDelay) / 1000}s`);
        outputChannel.appendLine(`   - Status: SUCCESS ✓`);
        outputChannel.appendLine('');
        outputChannel.appendLine('💡 Check the status bar at the bottom for visual progress!');
        outputChannel.appendLine('');
    });
    // Parse Configuration (MILESTONE 2.17)
    // Регистрация через отдельный модуль для лучшей организации кода
    const client = (0, lsp_1.getLanguageClient)();
    if (client) {
        const parseConfigDisposable = (0, parseConfiguration_1.registerParseConfigurationCommand)(context, client);
        if (parseConfigDisposable) {
            outputChannel.appendLine('✅ Registered command: bslAnalyzer.parseConfiguration');
        }
    }
    else {
        outputChannel.appendLine('⚠️ Cannot register bslAnalyzer.parseConfiguration - LSP client not ready');
    }
    // Show Semantic Visualization (MILESTONE 2.16)
    // Регистрируем команду всегда, проверку client делаем внутри
    await safeRegisterCommand('bsl-gradual-types.showSemanticVisualization', async () => {
        // ✅ ИСПРАВЛЕНИЕ: Приоритет отдаём BSL файлу из visibleTextEditors, а не activeTextEditor
        // Проблема: activeTextEditor может быть Output панелью с languageId "Log"
        const editor = vscode.window.visibleTextEditors.find(e => e.document.languageId === 'bsl') ||
            vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== 'bsl') {
            vscode.window.showWarningMessage('Нет открытого BSL файла');
            return;
        }
        // Проверяем наличие LSP client
        const client = (0, lsp_1.getLanguageClient)();
        if (!client || !client.isRunning()) {
            vscode.window.showErrorMessage('LSP server не запущен. Пожалуйста, подождите или перезапустите сервер.');
            return;
        }
        const uri = editor.document.uri.toString();
        const fileName = editor.document.fileName.split(/[/\\]/).pop() || 'unknown.bsl';
        const panel = vscode.window.createWebviewPanel('bslSemanticVisualization', `Семантическое дерево: ${fileName}`, vscode.ViewColumn.Two, {
            enableScripts: true,
            retainContextWhenHidden: true
        });
        // Loading индикатор
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
        <p>Загрузка семантического дерева...</p>
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
    <h1>❌ Ошибка</h1>
    <pre>${errorMessage}</pre>
</body>
</html>`;
            vscode.window.showErrorMessage(`Ошибка получения семантического дерева: ${errorMessage}`);
        }
    });
    // Устанавливаем флаг, что команды зарегистрированы
    commandsRegistered = true;
    outputChannel.appendLine('✅ Successfully registered 16 extension commands');
}
exports.registerCommands = registerCommands;
//# sourceMappingURL=index.js.map