import * as vscode from 'vscode';
import {
    TypeInfoParams,
    ValidateMethodParams,
    IndexingProgressParams,
    ProgressParams,
    WorkDoneProgressBegin,
    WorkDoneProgressReport,
    WorkDoneProgressEnd,
    ParsedProgressMessage
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
import { updateStatusBar, updateLspStatus, startIndexing, updateIndexingProgress, finishIndexing } from './progress';
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
        serverPath = getBinaryPath('lsp_server', context);
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
            configurationSection: 'bslAnalyzer'
        },
        // ✅ MILESTONE 2.10: Передаём initializationOptions в LSP
        initializationOptions: initializationOptions,
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
    
    // Устанавливаем уровень трассировки
    if (traceLevel && traceLevel !== 'off') {
        // Convert string to Trace enum
        if (traceLevel === 'messages') {
            (clientOptions as any).trace = Trace.Messages;
        } else if (traceLevel === 'verbose') {
            (clientOptions as any).trace = Trace.Verbose;
        }
    }
    
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

    // Start the client with progress notification
    try {
        outputChannel.appendLine('🚀 Starting LSP client...');
        outputChannel.appendLine(`   Server command: ${JSON.stringify(serverOptions)}`);

        // MILESTONE 2.9: Показываем прогресс парсинга документации
        // Используем Window (status bar) вместо Notification для меньшей навязчивости
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Window,
            title: "BSL Analyzer: Запуск LSP сервера",
            cancellable: false
        }, async (progress) => {
            progress.report({ increment: 0, message: "Инициализация..." });

            await client!.start();

            progress.report({ increment: 50, message: "Парсинг документации платформы 1С..." });

            // Даём серверу время на парсинг (он делает это при старте)
            await new Promise(resolve => setTimeout(resolve, 2000));

            progress.report({ increment: 100, message: "Готов к работе" });
        });

        outputChannel.appendLine('✅ LSP client started successfully');

        // Регистрируем обработчики custom requests
        registerCustomHandlers();

        // Регистрируем обработчик прогресса индексации
        client.onNotification('bsl/indexingProgress', (params: IndexingProgressParams) => {
            handleIndexingProgress(params);
        });

        // MILESTONE 2.20.2.4: Подписка на $/progress notification для Work Done Progress
        client.onNotification('$/progress', (params: ProgressParams) => {
            const { token } = params;

            // Обрабатываем только notifications с token "bsl-load-types-*"
            if (typeof token === 'string' && token.startsWith('bsl-load-types-')) {
                outputChannel.appendLine(`[$/progress] Received notification for token: ${token}`);
                handleWorkDoneProgress(params);
            }
        });

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
    // Небольшая задержка перед перезапуском
    await new Promise(resolve => setTimeout(resolve, 500));
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
 * MILESTONE 2.20.2.4: Парсинг прогресса из message string
 *
 * Ожидаемые форматы:
 * - "Тип 150/3927 - Справочники.Контрагенты - ETA: 42s"
 * - "Тип 150/3927 - Справочники.Контрагенты"
 * - "Тип 150/3927"
 * - "✅ Загружено 3927 типов за 87.3s"
 */
export function parseProgressMessage(message: string): ParsedProgressMessage {
    const result: ParsedProgressMessage = {
        originalMessage: message
    };

    // Пробуем распарсить формат: "Тип 150/3927 - Справочники.Контрагенты - ETA: 42s"
    const match = message.match(/Тип (\d+)\/(\d+)(?: - ([^-]+))?(?: - ETA: (\d+)s)?/);
    if (match) {
        result.currentItem = parseInt(match[1], 10);
        result.totalItems = parseInt(match[2], 10);

        // Название элемента (если есть)
        if (match[3]) {
            result.itemName = match[3].trim();
        }

        // ETA (если есть)
        if (match[4]) {
            result.eta = parseInt(match[4], 10);
        }
    }

    return result;
}

/**
 * MILESTONE 2.20.2.4: Обработчик $/progress notification от LSP Server
 *
 * Обрабатывает Work Done Progress notifications для индексации типов платформы 1С
 */
function handleWorkDoneProgress(params: ProgressParams): void {
    const { token, value } = params;

    outputChannel.appendLine(`[$/progress] Token: ${token}, Kind: ${value.kind}`);

    if (value.kind === 'begin') {
        const beginValue = value as WorkDoneProgressBegin;
        outputChannel.appendLine(`[$/progress] BEGIN - Title: ${beginValue.title}, Message: ${beginValue.message || 'N/A'}, Percentage: ${beginValue.percentage || 0}%`);

        // Запускаем отслеживание индексации
        startIndexing();
    }
    else if (value.kind === 'report') {
        const reportValue = value as WorkDoneProgressReport;
        const message = reportValue.message || '';
        const percentage = reportValue.percentage || 0;

        outputChannel.appendLine(`[$/progress] REPORT - Message: ${message}, Percentage: ${percentage}%`);

        // Парсим детали из message
        const parsed = parseProgressMessage(message);

        if (parsed.currentItem && parsed.totalItems) {
            // Формируем красивое описание прогресса
            const stepName = parsed.itemName
                ? `Тип ${parsed.currentItem}/${parsed.totalItems} - ${parsed.itemName}`
                : `Тип ${parsed.currentItem}/${parsed.totalItems}`;

            // Обновляем прогресс индексации
            updateIndexingProgress(
                percentage,
                stepName,
                parsed.eta
            );
        } else {
            // Fallback: показываем message как есть
            updateIndexingProgress(percentage, message, undefined);
        }
    }
    else if (value.kind === 'end') {
        const endValue = value as WorkDoneProgressEnd;
        const message = endValue.message || 'Завершено';

        outputChannel.appendLine(`[$/progress] END - Message: ${message}`);

        // Завершаем отслеживание индексации
        finishIndexing(message);
    }
    else {
        outputChannel.appendLine(`[$/progress] WARN - Unknown progress kind: ${(value as any).kind}`);
    }
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
 * Обработчик прогресса индексации от сервера
 */
function handleIndexingProgress(params: IndexingProgressParams) {
    outputChannel.appendLine(`📊 Indexing progress: Step ${params.step}/${params.totalSteps} - ${params.message} (${params.percentage}%)`);
    
    // Здесь можно обновить UI с прогрессом
    // Например, вызвать event emitter для обновления status bar
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