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
exports.sendCustomNotification = exports.sendCustomRequest = exports.isClientRunning = exports.getLanguageClient = exports.restartLanguageClient = exports.stopLanguageClient = exports.startLanguageClient = exports.initializeLspClient = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const binaryPath_1 = require("../utils/binaryPath");
const configHelper_1 = require("../config/configHelper");
const progress_1 = require("./progress");
const fs = __importStar(require("fs"));
let client = null;
let outputChannel;
let healthCheckInterval = null;
/**
 * Инициализирует модуль LSP клиента
 */
function initializeLspClient(channel) {
    outputChannel = channel;
}
exports.initializeLspClient = initializeLspClient;
/**
 * Запускает LSP сервер
 */
async function startLanguageClient(context) {
    const serverMode = configHelper_1.BslAnalyzerConfig.serverMode;
    const tcpPort = configHelper_1.BslAnalyzerConfig.serverTcpPort;
    const traceLevel = configHelper_1.BslAnalyzerConfig.serverTrace;
    // Используем getBinaryPath для получения пути к LSP серверу
    let serverPath;
    try {
        // Всегда используем общую логику выбора бинарников
        serverPath = (0, binaryPath_1.getBinaryPath)('lsp_server', context);
        outputChannel.appendLine(`🚀 LSP server path resolved: ${serverPath}`);
    }
    catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`❌ Failed to locate LSP server: ${errorMessage}`);
        vscode.window.showWarningMessage('BSL Analyzer: LSP server not found. Extension features will be limited.', 'Show Details').then(selection => {
            if (selection === 'Show Details') {
                outputChannel.show();
            }
        });
        return;
    }
    // Проверяем существование файла
    if (!fs.existsSync(serverPath)) {
        outputChannel.appendLine(`❌ LSP server file not found: ${serverPath}`);
        vscode.window.showWarningMessage('BSL Analyzer: LSP server binary not found. Please build the project first.', 'Open Build Instructions').then(selection => {
            if (selection === 'Open Build Instructions') {
                vscode.env.openExternal(vscode.Uri.parse('https://github.com/bsl-analyzer-team/bsl-type-safety-analyzer#building'));
            }
        });
        return;
    }
    outputChannel.appendLine(`🔧 Starting LSP server in ${serverMode} mode...`);
    outputChannel.appendLine(`📍 Server path: ${serverPath}`);
    // Server options configuration
    let serverOptions;
    if (serverMode === 'stdio') {
        // STDIO mode - запускаем сервер как процесс
        const execOptions = {
            env: {
                ...process.env,
                RUST_LOG: 'info',
                RUST_BACKTRACE: '1'
            }
        };
        serverOptions = {
            run: {
                command: serverPath,
                args: [],
                options: execOptions
            },
            debug: {
                command: serverPath,
                args: [],
                options: execOptions
            }
        };
    }
    else {
        // TCP mode - подключаемся к серверу
        outputChannel.appendLine(`📡 Connecting to LSP server on port ${tcpPort}...`);
        serverOptions = {
            run: {
                transport: node_1.TransportKind.socket,
                port: tcpPort
            },
            debug: {
                transport: node_1.TransportKind.socket,
                port: tcpPort
            }
        };
    }
    // Client options configuration
    const clientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'bsl' },
            { scheme: 'untitled', language: 'bsl' }
        ],
        synchronize: {
            fileEvents: [
                vscode.workspace.createFileSystemWatcher('**/*.bsl'),
                vscode.workspace.createFileSystemWatcher('**/*.os'),
                vscode.workspace.createFileSystemWatcher('**/Configuration.xml')
            ],
            configurationSection: 'bslAnalyzer'
        },
        outputChannel: outputChannel,
        revealOutputChannelOn: node_1.RevealOutputChannelOn.Never,
        traceOutputChannel: outputChannel,
        middleware: {
            // Перехватываем workspace-related notifications
            workspace: {
                configuration: (params, token, next) => {
                    outputChannel.appendLine(`📊 Configuration request: ${JSON.stringify(params)}`);
                    return next(params, token);
                }
            }
        }
    };
    // Устанавливаем уровень трассировки
    if (traceLevel && traceLevel !== 'off') {
        // Convert string to Trace enum
        if (traceLevel === 'messages') {
            clientOptions.trace = node_1.Trace.Messages;
        }
        else if (traceLevel === 'verbose') {
            clientOptions.trace = node_1.Trace.Verbose;
        }
    }
    // Create the language client
    client = new node_1.LanguageClient('bslAnalyzer', 'BSL Type Safety Analyzer', serverOptions, clientOptions);
    // Start the client
    try {
        outputChannel.appendLine('🚀 Starting LSP client...');
        await client.start();
        outputChannel.appendLine('✅ LSP client started successfully');
        // Регистрируем обработчики custom requests
        registerCustomHandlers();
        // Регистрируем обработчик прогресса индексации
        client.onNotification('bsl/indexingProgress', (params) => {
            handleIndexingProgress(params);
        });
        // Регистрируем обработчик изменения состояния клиента
        client.onDidChangeState((event) => {
            outputChannel.appendLine(`📊 LSP Client state changed: ${event.oldState} -> ${event.newState}`);
            // Обновляем UI при изменении состояния
            vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
            // Если сервер отключился неожиданно
            if (event.newState === 1) { // Stopped state
                outputChannel.appendLine('⚠️ LSP server disconnected unexpectedly');
                vscode.window.showWarningMessage('BSL Analyzer: Language server disconnected', 'Restart Server').then(selection => {
                    if (selection === 'Restart Server') {
                        vscode.commands.executeCommand('bslAnalyzer.restartServer');
                    }
                });
                // Обновляем статус бар
                (0, progress_1.updateStatusBar)('$(error) BSL Analyzer: Disconnected');
            }
            else if (event.newState === 2) { // Running state
                (0, progress_1.updateStatusBar)('$(database) BSL Analyzer: Ready');
            }
        });
        // Уведомляем провайдеры об изменении статуса
        vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
        // Запускаем периодическую проверку состояния (каждые 30 секунд)
        startHealthCheck();
    }
    catch (error) {
        outputChannel.appendLine(`❌ Failed to start LSP client: ${error}`);
        vscode.window.showErrorMessage(`Failed to start BSL Analyzer: ${error}`);
        (0, progress_1.updateStatusBar)('$(error) BSL Analyzer: Failed to start');
    }
}
exports.startLanguageClient = startLanguageClient;
/**
 * Останавливает LSP сервер
 */
async function stopLanguageClient() {
    // Останавливаем health check
    stopHealthCheck();
    if (client) {
        outputChannel.appendLine('🛑 Stopping LSP client...');
        try {
            await client.stop();
            outputChannel.appendLine('✅ LSP client stopped');
        }
        catch (error) {
            outputChannel.appendLine(`⚠️ Error stopping LSP client: ${error}`);
        }
        client = null;
    }
}
exports.stopLanguageClient = stopLanguageClient;
/**
 * Перезапускает LSP сервер
 */
async function restartLanguageClient(context) {
    outputChannel.appendLine('🔄 Restarting LSP server...');
    await stopLanguageClient();
    // Уведомляем об остановке
    vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
    // Небольшая задержка перед перезапуском
    await new Promise(resolve => setTimeout(resolve, 500));
    await startLanguageClient(context);
}
exports.restartLanguageClient = restartLanguageClient;
/**
 * Возвращает текущий клиент LSP
 */
function getLanguageClient() {
    return client;
}
exports.getLanguageClient = getLanguageClient;
/**
 * Проверяет, запущен ли LSP клиент
 */
function isClientRunning() {
    return client !== null && client.isRunning();
}
exports.isClientRunning = isClientRunning;
/**
 * Регистрирует обработчики кастомных запросов
 */
function registerCustomHandlers() {
    if (!client)
        return;
    // Обработчик запросов информации о типе
    client.onRequest('bsl/typeInfo', async (params) => {
        outputChannel.appendLine(`📋 Type info request: ${JSON.stringify(params)}`);
        // Здесь можно добавить обработку запроса
        return null;
    });
    // Обработчик запросов валидации метода
    client.onRequest('bsl/validateMethod', async (params) => {
        outputChannel.appendLine(`✓ Method validation request: ${JSON.stringify(params)}`);
        // Здесь можно добавить обработку запроса
        return null;
    });
}
/**
 * Обработчик прогресса индексации от сервера
 */
function handleIndexingProgress(params) {
    outputChannel.appendLine(`📊 Indexing progress: Step ${params.step}/${params.totalSteps} - ${params.message} (${params.percentage}%)`);
    // Здесь можно обновить UI с прогрессом
    // Например, вызвать event emitter для обновления status bar
}
/**
 * Отправляет запрос на сервер для выполнения кастомной команды
 */
async function sendCustomRequest(method, params) {
    if (!client || !client.isRunning()) {
        throw new Error('LSP client is not running');
    }
    try {
        const result = await client.sendRequest(method, params);
        return result;
    }
    catch (error) {
        outputChannel.appendLine(`❌ Custom request failed: ${error}`);
        throw error;
    }
}
exports.sendCustomRequest = sendCustomRequest;
/**
 * Отправляет уведомление на сервер
 */
function sendCustomNotification(method, params) {
    if (!client || !client.isRunning()) {
        outputChannel.appendLine(`⚠️ Cannot send notification: LSP client is not running`);
        return;
    }
    client.sendNotification(method, params);
}
exports.sendCustomNotification = sendCustomNotification;
/**
 * Запускает периодическую проверку состояния LSP сервера
 */
function startHealthCheck() {
    // Останавливаем предыдущий интервал, если он есть
    stopHealthCheck();
    // Проверяем состояние каждые 30 секунд
    healthCheckInterval = setInterval(() => {
        if (client) {
            const isRunning = client.isRunning();
            if (!isRunning) {
                outputChannel.appendLine('⚠️ Health check: LSP client is not running');
                (0, progress_1.updateStatusBar)('$(error) BSL Analyzer: Disconnected');
                vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
                // Показываем уведомление только один раз
                stopHealthCheck();
                vscode.window.showWarningMessage('BSL Analyzer: Language server stopped unexpectedly', 'Restart Server', 'Dismiss').then(selection => {
                    if (selection === 'Restart Server') {
                        vscode.commands.executeCommand('bslAnalyzer.restartServer');
                    }
                });
            }
        }
    }, 30000); // 30 секунд
}
/**
 * Останавливает периодическую проверку состояния
 */
function stopHealthCheck() {
    if (healthCheckInterval) {
        clearInterval(healthCheckInterval);
        healthCheckInterval = null;
    }
}
//# sourceMappingURL=client.js.map