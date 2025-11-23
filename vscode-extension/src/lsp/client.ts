import * as vscode from 'vscode';
import {
    TypeInfoParams,
    ValidateMethodParams,
    ServerStatusParams
} from '../types';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
    RevealOutputChannelOn,
    Trace,
    Executable,
    State
} from 'vscode-languageclient/node';
import { getBinaryPath } from '../utils/binaryPath';
import { BslAnalyzerConfig } from '../config/configHelper';
import { handleServerStatus, initializeServerStatus } from './serverStatus';
import { updateStatusBar, updateLspStatus } from './progress';
import * as fs from 'fs';

/**
 * Преобразует состояние LSP клиента в читаемую строку
 */
function StateToString(state: State): string {
    switch (state) {
        case State.Stopped: return 'Stopped';
        case State.Starting: return 'Starting';
        case State.Running: return 'Running';
        default: return `Unknown(${state})`;
    }
}

let client: LanguageClient | null = null;
let outputChannel: vscode.OutputChannel;
let healthCheckInterval: NodeJS.Timeout | null = null;

/**
 * Инициализирует модуль LSP клиента
 */
export function initializeLspClient(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

/**
 * Запускает LSP сервер
 */
export async function startLanguageClient(context: vscode.ExtensionContext): Promise<void> {
    const serverMode = BslAnalyzerConfig.serverMode;
    const tcpPort = BslAnalyzerConfig.serverTcpPort;
    const traceLevel = BslAnalyzerConfig.serverTrace;

    // Используем getBinaryPath для получения пути к LSP серверу
    let serverPath: string;
    try {
        // Всегда используем общую логику выбора бинарников
        serverPath = getBinaryPath('lsp-server', context);
        outputChannel.appendLine(`🚀 LSP server path resolved: ${serverPath}`);
    } catch (error: unknown) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`❌ Failed to locate LSP server: ${errorMessage}`);
        vscode.window.showWarningMessage(
            'BSL Analyzer: LSP server not found. Extension features will be limited.',
            'Show Details'
        ).then(selection => {
            if (selection === 'Show Details') {
                outputChannel.show();
            }
        });
        return;
    }

    // Проверяем существование файла
    if (!fs.existsSync(serverPath)) {
        outputChannel.appendLine(`❌ LSP server file not found: ${serverPath}`);
        vscode.window.showWarningMessage(
            'BSL Analyzer: LSP server binary not found. Please build the project first.',
            'Open Build Instructions'
        ).then(selection => {
            if (selection === 'Open Build Instructions') {
                vscode.env.openExternal(vscode.Uri.parse('https://github.com/bsl-analyzer-team/bsl-type-safety-analyzer#building'));
            }
        });
        return;
    }

    outputChannel.appendLine(`🔧 Starting LSP server in ${serverMode} mode...`);
    outputChannel.appendLine(`📍 Server path: ${serverPath}`);

    // Server options configuration
    let serverOptions: ServerOptions;

    if (serverMode === 'stdio') {
        // STDIO mode - прямой запуск (как в rust-analyzer)
        const newEnv = { ...process.env };
        newEnv.RUST_LOG = 'debug';
        newEnv.RUST_BACKTRACE = 'full';

        const run: Executable = {
            command: serverPath,
            options: { env: newEnv }
        };

        serverOptions = {
            run,
            debug: run
        };

        outputChannel.appendLine(`📝 Rust server logs: ${context.extensionPath}\\rust_lsp_server.log`);
    } else {
        // TCP mode - подключаемся к серверу
        outputChannel.appendLine(`📡 Connecting to LSP server on port ${tcpPort}...`);
        serverOptions = {
            run: {
                transport: TransportKind.socket,
                port: tcpPort
            } as any,
            debug: {
                transport: TransportKind.socket,
                port: tcpPort
            } as any
        };
    }

    // ✅ MILESTONE 2.10: Подготавливаем initializationOptions для передачи в LSP
    const initializationOptions = {
        platformDocsArchive: BslAnalyzerConfig.platformDocsArchive,
        configurationPath: BslAnalyzerConfig.configurationPath,
        platformVersion: BslAnalyzerConfig.platformVersion
    };

    outputChannel.appendLine(`📤 Sending initializationOptions to LSP:`);
    outputChannel.appendLine(`   platformDocsArchive: ${initializationOptions.platformDocsArchive || 'NOT SET'}`);
    outputChannel.appendLine(`   configurationPath: ${initializationOptions.configurationPath || 'NOT SET'}`);
    outputChannel.appendLine(`   platformVersion: ${initializationOptions.platformVersion || 'NOT SET'}`);

    // Client options configuration
    const clientOptions: LanguageClientOptions = {
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
            // MILESTONE 3.6: Синхронизируем ОБЕ секции настроек (bslAnalyzer + bsl)
            configurationSection: ['bslAnalyzer', 'bsl']
        },
        // ✅ MILESTONE 2.10: Передаём initializationOptions в LSP
        initializationOptions: initializationOptions,
        // ❌ УБРАНО: progressOnInitialization работает ТОЛЬКО для прогресса во время initialize request
        // Наш прогресс создаётся ПОСЛЕ в initialized() callback, поэтому нужен custom handler
        outputChannel: outputChannel,
        revealOutputChannelOn: RevealOutputChannelOn.Never,
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

    // ✅ ПРИНУДИТЕЛЬНО устанавливаем VERBOSE tracing для отладки Work Done Progress
    (clientOptions as any).trace = Trace.Verbose;
    outputChannel.appendLine('🔍 TRACE: Verbose logging enabled');

    // Create the language client
    client = new LanguageClient(
        'bslAnalyzer',
        'BSL Type Safety Analyzer',
        serverOptions,
        clientOptions
    );

    // Добавляем детальные обработчики ошибок ПЕРЕД запуском
    // MILESTONE 2.20.1: Track LSP server status changes
    client.onDidChangeState((event) => {
        outputChannel.appendLine(`🔄 LSP Client state: ${StateToString(event.oldState)} → ${StateToString(event.newState)}`);

        // Update status bar with LSP status indicator (red background for Stopped)
        updateLspStatus(event.newState);

        // Refresh UI when state changes
        vscode.commands.executeCommand('bslAnalyzer.refreshOverview');

        // Show warning if server disconnected unexpectedly
        if (event.newState === State.Stopped) {
            outputChannel.appendLine('⚠️ LSP server disconnected unexpectedly');
            vscode.window.showWarningMessage(
                'BSL Analyzer: Language server disconnected',
                'Restart Server'
            ).then(selection => {
                if (selection === 'Restart Server') {
                    vscode.commands.executeCommand('bslAnalyzer.restartServer');
                }
            });
        }
    });

    // Обработчик ошибок подключения
    (client as any).onConnectionError = (error: Error, message: any, count: number) => {
        outputChannel.appendLine(`❌ Connection error (attempt ${count}): ${error.message}`);
        outputChannel.appendLine(`   Error stack: ${error.stack}`);
        if (message) {
            outputChannel.appendLine(`   Last message: ${JSON.stringify(message)}`);
        }
    };

    // ✅ MILESTONE 2.20: Автоматическая обработка Work Done Progress
    // vscode-languageclient автоматически показывает Progress Window для $/progress notifications
    // Не нужно регистрировать custom handler — это anti-pattern (см. rust-analyzer approach)
    //
    // РАЗДЕЛЕНИЕ ОТВЕТСТВЕННОСТИ:
    // - $/progress (vscode-languageclient) → Автоматический Progress Window + Status Bar
    // - bsl/serverStatus (custom) → Loading icon при запуске LSP Server

    // Start the client
    try {
        outputChannel.appendLine('🚀 Starting LSP client...');
        outputChannel.appendLine(`   Server command: ${JSON.stringify(serverOptions)}`);

        await client!.start();

        outputChannel.appendLine('✅ LSP client started successfully');

        // MILESTONE 2.20.3: Обработчик server status (для loading icon)
        client.onNotification('bsl/serverStatus', (params: ServerStatusParams) => {
            handleServerStatus(params);
        });

        outputChannel.appendLine('✅ bsl/serverStatus handler registered');

        // MILESTONE 2.20.4: Обработчик Work Done Progress (для Progress Window)
        // vscode-languageclient НЕ показывает Progress Window автоматически для server-initiated progress
        // (progress tokens, созданные LSP Server через window/workDoneProgress/create)
        // Нужна явная регистрация обработчика для показа Progress Window

        let activeProgressResolve: ((value: void) => void) | null = null;
        let activeProgressReporter: vscode.Progress<{ message?: string; increment?: number }> | null = null;
        let lastReportedPercentage: number = 0;

        client.onNotification('$/progress', (params: any) => {
            const token = params.token;
            const value = params.value;

            if (value.kind === 'begin') {
                outputChannel.appendLine(`📊 [Progress] BEGIN: ${value.title}`);

                // Завершаем старый прогресс если он ещё активен (из-за crash/restart LSP)
                if (activeProgressResolve) {
                    outputChannel.appendLine('🧹 Clearing previous progress before starting new one');
                    activeProgressResolve();
                    activeProgressResolve = null;
                    activeProgressReporter = null;
                }

                // Сброс состояния
                lastReportedPercentage = 0;

                // Показать Progress в Status Bar (всегда видно)
                vscode.window.withProgress({
                    location: vscode.ProgressLocation.Window,
                    title: value.title,
                    cancellable: false
                }, async (progress) => {
                    // Сохранить reporter для использования в REPORT
                    activeProgressReporter = progress;

                    // Начальное сообщение
                    progress.report({
                        message: value.message || 'Инициализация...',
                        increment: 0
                    });

                    // Ждём завершения прогресса
                    return new Promise<void>((resolve) => {
                        activeProgressResolve = resolve;
                    });
                });
            }

            if (value.kind === 'report') {
                const percentage = value.percentage || 0;
                const message = value.message || '';

                outputChannel.appendLine(`📊 [Progress] REPORT: ${message} (${percentage}%)`);

                // Обновить Progress Window
                if (activeProgressReporter) {
                    const increment = percentage - lastReportedPercentage;
                    lastReportedPercentage = percentage;

                    activeProgressReporter.report({
                        message: message,
                        increment: Math.max(0, increment)  // Не допускать отрицательных значений
                    });
                }
            }

            if (value.kind === 'end') {
                const message = value.message || 'Завершено';

                outputChannel.appendLine(`📊 [Progress] END: ${message}`);

                // Закрыть Progress Window
                if (activeProgressResolve) {
                    activeProgressResolve();
                    activeProgressResolve = null;
                }

                // Очистить состояние
                activeProgressReporter = null;
                lastReportedPercentage = 0;
            }
        });

        outputChannel.appendLine('✅ $/progress handler registered');

        // Регистрируем обработчики custom requests
        registerCustomHandlers();

        // Уведомляем провайдеры об изменении статуса
        vscode.commands.executeCommand('bslAnalyzer.refreshOverview');

        // Запускаем периодическую проверку состояния (каждые 30 секунд)
        startHealthCheck();

    } catch (error) {
        outputChannel.appendLine(`❌ Failed to start LSP client: ${error}`);
        vscode.window.showErrorMessage(`Failed to start BSL Analyzer: ${error}`);
        updateStatusBar('$(error) BSL Analyzer: Failed to start');
    }
}

/**
 * Останавливает LSP сервер
 */
export async function stopLanguageClient(): Promise<void> {
    // Останавливаем health check
    stopHealthCheck();

    if (client) {
        outputChannel.appendLine('🛑 Stopping LSP client...');
        try {
            await client.stop();
            outputChannel.appendLine('✅ LSP client stopped');
        } catch (error) {
            outputChannel.appendLine(`⚠️ Error stopping LSP client: ${error}`);
        }
        client = null;
    }
}

/**
 * Перезапускает LSP сервер
 */
export async function restartLanguageClient(context: vscode.ExtensionContext): Promise<void> {
    outputChannel.appendLine('🔄 Restarting LSP server...');
    await stopLanguageClient();
    // Уведомляем об остановке
    vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
    await startLanguageClient(context);
}

/**
 * Возвращает текущий клиент LSP
 */
export function getLanguageClient(): LanguageClient | null {
    return client;
}

/**
 * Проверяет, запущен ли LSP клиент
 */
export function isClientRunning(): boolean {
    return client !== null && client.isRunning();
}

/**
 * Регистрирует обработчики кастомных запросов
 */
function registerCustomHandlers() {
    if (!client) return;

    // Обработчик запросов информации о типе
    client.onRequest('bsl/typeInfo', async (params: TypeInfoParams) => {
        outputChannel.appendLine(`📋 Type info request: ${JSON.stringify(params)}`);
        // Здесь можно добавить обработку запроса
        return null;
    });

    // Обработчик запросов валидации метода
    client.onRequest('bsl/validateMethod', async (params: ValidateMethodParams) => {
        outputChannel.appendLine(`✓ Method validation request: ${JSON.stringify(params)}`);
        // Здесь можно добавить обработку запроса
        return null;
    });
}

/**
 * Отправляет запрос на сервер для выполнения кастомной команды
 */
export async function sendCustomRequest<T = unknown>(method: string, params?: unknown): Promise<T> {
    if (!client || !client.isRunning()) {
        throw new Error('LSP client is not running');
    }

    try {
        const result = await client.sendRequest(method, params);
        return result as T;
    } catch (error) {
        outputChannel.appendLine(`❌ Custom request failed: ${error}`);
        throw error;
    }
}

/**
 * Отправляет уведомление на сервер
 */
export function sendCustomNotification(method: string, params?: unknown): void {
    if (!client || !client.isRunning()) {
        outputChannel.appendLine(`⚠️ Cannot send notification: LSP client is not running`);
        return;
    }

    client.sendNotification(method, params);
}

/**
 * Запускает периодическую проверку состояния LSP сервера
 */
function startHealthCheck(): void {
    // Останавливаем предыдущий интервал, если он есть
    stopHealthCheck();

    // Проверяем состояние каждые 30 секунд
    healthCheckInterval = setInterval(() => {
        if (client) {
            const isRunning = client.isRunning();
            if (!isRunning) {
                outputChannel.appendLine('⚠️ Health check: LSP client is not running');
                updateStatusBar('$(error) BSL Analyzer: Disconnected');
                vscode.commands.executeCommand('bslAnalyzer.refreshOverview');

                // Показываем уведомление только один раз
                stopHealthCheck();
                vscode.window.showWarningMessage(
                    'BSL Analyzer: Language server stopped unexpectedly',
                    'Restart Server',
                    'Dismiss'
                ).then(selection => {
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
function stopHealthCheck(): void {
    if (healthCheckInterval) {
        clearInterval(healthCheckInterval);
        healthCheckInterval = null;
    }
}