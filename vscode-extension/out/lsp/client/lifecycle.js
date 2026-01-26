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
exports.sendCustomNotification = exports.sendCustomRequest = exports.getServerVersion = exports.isClientRunning = exports.getLanguageClient = exports.restartLanguageClient = exports.stopLanguageClient = exports.startLanguageClient = exports.initializeLifecycle = void 0;
const vscode = __importStar(require("vscode"));
const fs = __importStar(require("fs"));
const node_1 = require("vscode-languageclient/node");
const binaryPath_1 = require("../../utils/binaryPath");
const configHelper_1 = require("../../config/configHelper");
const serverStatus_1 = require("../serverStatus");
const progress_1 = require("../progress");
const server_options_1 = require("./server-options");
const client_options_1 = require("./client-options");
const progress_handler_1 = require("./progress-handler");
const health_check_1 = require("./health-check");
/**
 * Преобразует состояние LSP клиента в читаемую строку
 */
function StateToString(state) {
    switch (state) {
        case node_1.State.Stopped: return 'Stopped';
        case node_1.State.Starting: return 'Starting';
        case node_1.State.Running: return 'Running';
        default: return `Unknown(${state})`;
    }
}
/** Текущий LSP клиент */
let client = null;
/** Output channel для логирования */
let outputChannel;
/**
 * Инициализирует модуль lifecycle
 */
function initializeLifecycle(channel) {
    outputChannel = channel;
}
exports.initializeLifecycle = initializeLifecycle;
/**
 * Запускает LSP сервер
 */
async function startLanguageClient(context) {
    const serverMode = configHelper_1.BslAnalyzerConfig.serverMode;
    // Используем getBinaryPath для получения пути к LSP серверу
    let serverPath;
    try {
        serverPath = (0, binaryPath_1.getBinaryPath)('lsp-server', context);
        outputChannel.appendLine(`LSP server path resolved: ${serverPath}`);
    }
    catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Failed to locate LSP server: ${errorMessage}`);
        vscode.window.showWarningMessage('BSL Analyzer: LSP server not found. Extension features will be limited.', 'Show Details').then(selection => {
            if (selection === 'Show Details') {
                outputChannel.show();
            }
        });
        return;
    }
    // Проверяем существование файла
    if (!fs.existsSync(serverPath)) {
        outputChannel.appendLine(`LSP server file not found: ${serverPath}`);
        vscode.window.showWarningMessage('BSL Analyzer: LSP server binary not found. Please build the project first.', 'Open Build Instructions').then(selection => {
            if (selection === 'Open Build Instructions') {
                vscode.env.openExternal(vscode.Uri.parse('https://github.com/bsl-analyzer-team/bsl-type-safety-analyzer#building'));
            }
        });
        return;
    }
    outputChannel.appendLine(`Starting LSP server in ${serverMode} mode...`);
    outputChannel.appendLine(`Server path: ${serverPath}`);
    // Build options
    const serverOptions = (0, server_options_1.buildServerOptions)(serverPath, outputChannel);
    const clientOptions = (0, client_options_1.buildClientOptions)(outputChannel);
    if (serverMode === 'stdio') {
        outputChannel.appendLine(`Rust server logs: ${context.extensionPath}\\rust_lsp_server.log`);
    }
    // Create the language client
    client = new node_1.LanguageClient('bslAnalyzer', 'BSL Type Safety Analyzer', serverOptions, clientOptions);
    // Добавляем обработчики состояния ПЕРЕД запуском
    setupStateChangeHandler(client, outputChannel);
    setupConnectionErrorHandler(client, outputChannel);
    (0, progress_handler_1.setupProgressHandler)(client, outputChannel);
    // Start the client
    try {
        outputChannel.appendLine('Starting LSP client...');
        outputChannel.appendLine(`Server command: ${JSON.stringify(serverOptions)}`);
        await client.start();
        outputChannel.appendLine('LSP client started successfully');
        // Регистрируем обработчики после успешного запуска
        setupServerStatusHandler(client, outputChannel);
        registerCustomHandlers(client, outputChannel);
        // Уведомляем провайдеры об изменении статуса
        vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
        // Запускаем периодическую проверку состояния
        (0, health_check_1.startHealthCheck)(client, outputChannel);
    }
    catch (error) {
        outputChannel.appendLine(`Failed to start LSP client: ${error}`);
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
    (0, health_check_1.stopHealthCheck)();
    if (client) {
        outputChannel.appendLine('Stopping LSP client...');
        try {
            await client.stop();
            outputChannel.appendLine('LSP client stopped');
        }
        catch (error) {
            outputChannel.appendLine(`Error stopping LSP client: ${error}`);
        }
        client = null;
    }
}
exports.stopLanguageClient = stopLanguageClient;
/**
 * Перезапускает LSP сервер
 */
async function restartLanguageClient(context) {
    outputChannel.appendLine('Restarting LSP server...');
    await stopLanguageClient();
    // Уведомляем об остановке
    vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
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
 * Возвращает версию LSP сервера из initialize result
 */
function getServerVersion() {
    if (!client) {
        return undefined;
    }
    const info = client.initializeResult?.serverInfo;
    if (info && typeof info.version === 'string') {
        return info.version;
    }
    return undefined;
}
exports.getServerVersion = getServerVersion;
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
        outputChannel.appendLine(`Custom request failed: ${error}`);
        throw error;
    }
}
exports.sendCustomRequest = sendCustomRequest;
/**
 * Отправляет уведомление на сервер
 */
function sendCustomNotification(method, params) {
    if (!client || !client.isRunning()) {
        outputChannel.appendLine(`Cannot send notification: LSP client is not running`);
        return;
    }
    client.sendNotification(method, params);
}
exports.sendCustomNotification = sendCustomNotification;
// ============================================================================
// Private helpers
// ============================================================================
/**
 * Настраивает обработчик изменения состояния клиента
 */
function setupStateChangeHandler(client, outputChannel) {
    client.onDidChangeState((event) => {
        outputChannel.appendLine(`LSP Client state: ${StateToString(event.oldState)} -> ${StateToString(event.newState)}`);
        // Update status bar with LSP status indicator
        (0, progress_1.updateLspStatus)(event.newState);
        // Refresh UI when state changes
        vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
        // Show warning if server disconnected unexpectedly
        if (event.newState === node_1.State.Stopped) {
            outputChannel.appendLine('LSP server disconnected unexpectedly');
            vscode.window.showWarningMessage('BSL Analyzer: Language server disconnected', 'Restart Server').then(selection => {
                if (selection === 'Restart Server') {
                    vscode.commands.executeCommand('bslAnalyzer.restartServer');
                }
            });
        }
    });
}
/**
 * Настраивает обработчик ошибок подключения
 */
function setupConnectionErrorHandler(client, outputChannel) {
    client.onConnectionError = (error, message, count) => {
        outputChannel.appendLine(`Connection error (attempt ${count}): ${error.message}`);
        outputChannel.appendLine(`   Error stack: ${error.stack}`);
        if (message) {
            outputChannel.appendLine(`   Last message: ${JSON.stringify(message)}`);
        }
    };
}
/**
 * Настраивает обработчик статуса сервера
 */
function setupServerStatusHandler(client, outputChannel) {
    client.onNotification('bsl/serverStatus', (params) => {
        (0, serverStatus_1.handleServerStatus)(params);
    });
    outputChannel.appendLine('bsl/serverStatus handler registered');
}
/**
 * Регистрирует обработчики кастомных запросов
 */
function registerCustomHandlers(client, outputChannel) {
    // Обработчик запросов информации о типе
    client.onRequest('bsl/typeInfo', async (params) => {
        outputChannel.appendLine(`Type info request: ${JSON.stringify(params)}`);
        return null;
    });
    // Обработчик запросов валидации метода
    client.onRequest('bsl/validateMethod', async (params) => {
        outputChannel.appendLine(`Method validation request: ${JSON.stringify(params)}`);
        return null;
    });
}
//# sourceMappingURL=lifecycle.js.map