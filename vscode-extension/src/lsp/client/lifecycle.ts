import * as vscode from 'vscode';
import * as fs from 'fs';
import {
    LanguageClient,
    State
} from 'vscode-languageclient/node';
import {
    TypeInfoParams,
    ValidateMethodParams,
    ServerStatusParams
} from '../../types';
import { getBinaryPath } from '../../utils/binaryPath';
import { BslAnalyzerConfig } from '../../config/configHelper';
import { handleServerStatus } from '../serverStatus';
import { updateStatusBar, updateLspStatus } from '../progress';
import { buildServerOptions } from './server-options';
import { buildClientOptions } from './client-options';
import {
    instrumentCompletionProbeTransport,
    registerCompletionProbeSelectionObserver,
} from './completionProbeRuntime';
import { setupProgressHandler } from './progress-handler';
import { startHealthCheck, stopHealthCheck } from './health-check';
import { getSharedCompletionProbeRecorder } from '../../providers/completionProbeRecorder';

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

/** Текущий LSP клиент */
let client: LanguageClient | null = null;
let completionTriggerWarningShown = false;
let completionProbeSelectionDisposable: vscode.Disposable | undefined;

/** Output channel для логирования */
let outputChannel: vscode.OutputChannel;

/**
 * Инициализирует модуль lifecycle
 */
export function initializeLifecycle(channel: vscode.OutputChannel): void {
    outputChannel = channel;
    completionTriggerWarningShown = false;
}

/**
 * Запускает LSP сервер
 */
export async function startLanguageClient(context: vscode.ExtensionContext): Promise<void> {
    const serverMode = BslAnalyzerConfig.serverMode;

    // Используем getBinaryPath для получения пути к LSP серверу
    let serverPath: string;
    try {
        serverPath = getBinaryPath('lsp-server', context);
        outputChannel.appendLine(`LSP server path resolved: ${serverPath}`);
    } catch (error: unknown) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Failed to locate LSP server: ${errorMessage}`);
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
        outputChannel.appendLine(`LSP server file not found: ${serverPath}`);
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

    outputChannel.appendLine(`Starting LSP server in ${serverMode} mode...`);
    outputChannel.appendLine(`Server path: ${serverPath}`);

    // Build options
    const serverOptions = buildServerOptions(serverPath, outputChannel);
    const completionProbeRecorder = getSharedCompletionProbeRecorder();
    const clientOptions = buildClientOptions(outputChannel, completionProbeRecorder);

    if (serverMode === 'stdio') {
        outputChannel.appendLine(`Rust server logs: ${context.extensionPath}\\rust_lsp_server.log`);
    }

    // Create the language client
    client = new LanguageClient(
        'bslAnalyzer',
        'BSL Type Safety Analyzer',
        serverOptions,
        clientOptions
    );
    instrumentCompletionProbeTransport(client as unknown as { sendRequest: (...args: any[]) => Promise<unknown> }, completionProbeRecorder);
    completionProbeSelectionDisposable?.dispose();
    completionProbeSelectionDisposable = registerCompletionProbeSelectionObserver(completionProbeRecorder);

    // Добавляем обработчики состояния ПЕРЕД запуском
    setupStateChangeHandler(client, outputChannel);
    setupConnectionErrorHandler(client, outputChannel);
    setupProgressHandler(client, outputChannel);

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
        startHealthCheck(client, outputChannel);
        await warnIfCompletionTriggerDisabled(outputChannel);

    } catch (error) {
        outputChannel.appendLine(`Failed to start LSP client: ${error}`);
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
    completionProbeSelectionDisposable?.dispose();
    completionProbeSelectionDisposable = undefined;

    if (client) {
        outputChannel.appendLine('Stopping LSP client...');
        try {
            await client.stop();
            outputChannel.appendLine('LSP client stopped');
        } catch (error) {
            outputChannel.appendLine(`Error stopping LSP client: ${error}`);
        }
        client = null;
    }
}

/**
 * Перезапускает LSP сервер
 */
export async function restartLanguageClient(context: vscode.ExtensionContext): Promise<void> {
    outputChannel.appendLine('Restarting LSP server...');
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
 * Возвращает версию LSP сервера из initialize result
 */
export function getServerVersion(): string | undefined {
    if (!client) {
        return undefined;
    }
    const info = (client as any).initializeResult?.serverInfo;
    if (info && typeof info.version === 'string') {
        return info.version;
    }
    return undefined;
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
        outputChannel.appendLine(`Custom request failed: ${error}`);
        throw error;
    }
}

/**
 * Отправляет уведомление на сервер
 */
export function sendCustomNotification(method: string, params?: unknown): void {
    if (!client || !client.isRunning()) {
        outputChannel.appendLine(`Cannot send notification: LSP client is not running`);
        return;
    }

    client.sendNotification(method, params);
}

// ============================================================================
// Private helpers
// ============================================================================

/**
 * Настраивает обработчик изменения состояния клиента
 */
function setupStateChangeHandler(
    client: LanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    client.onDidChangeState((event) => {
        outputChannel.appendLine(`LSP Client state: ${StateToString(event.oldState)} -> ${StateToString(event.newState)}`);

        // Update status bar with LSP status indicator
        updateLspStatus(event.newState);

        // Refresh UI when state changes
        vscode.commands.executeCommand('bslAnalyzer.refreshOverview');

        // Show warning if server disconnected unexpectedly
        if (event.newState === State.Stopped) {
            outputChannel.appendLine('LSP server disconnected unexpectedly');
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
}

/**
 * Настраивает обработчик ошибок подключения
 */
function setupConnectionErrorHandler(
    client: LanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    (client as any).onConnectionError = (error: Error, message: any, count: number) => {
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
function setupServerStatusHandler(
    client: LanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    client.onNotification('bsl/serverStatus', (params: ServerStatusParams) => {
        handleServerStatus(params);
    });
    outputChannel.appendLine('bsl/serverStatus handler registered');
}

/**
 * Регистрирует обработчики кастомных запросов
 */
function registerCustomHandlers(
    client: LanguageClient,
    outputChannel: vscode.OutputChannel
): void {
    // Обработчик запросов информации о типе
    client.onRequest('bsl/typeInfo', async (params: TypeInfoParams) => {
        outputChannel.appendLine(`Type info request: ${JSON.stringify(params)}`);
        return null;
    });

    // Обработчик запросов валидации метода
    client.onRequest('bsl/validateMethod', async (params: ValidateMethodParams) => {
        outputChannel.appendLine(`Method validation request: ${JSON.stringify(params)}`);
        return null;
    });
}

function completionTriggerConfigScopeUri(): vscode.Uri | undefined {
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor?.document.languageId === 'bsl') {
        return activeEditor.document.uri;
    }

    const visibleBslEditor = vscode.window.visibleTextEditors.find(
        (editor) => editor.document.languageId === 'bsl',
    );
    return visibleBslEditor?.document.uri;
}

export function isCompletionTriggerEnabledForBsl(scopeUri?: vscode.Uri): boolean {
    const enabled = vscode.workspace
        .getConfiguration('editor', scopeUri)
        .get<boolean>('suggestOnTriggerCharacters', true);
    return enabled !== false;
}

export interface CompletionTriggerWarningPayload {
    scopeText: string;
    outputLines: string[];
    userMessage: string;
    actionLabel: string;
    settingsQuery: string;
}

export function buildCompletionTriggerWarningPayload(
    triggerEnabled: boolean,
    scopeUri?: vscode.Uri,
): CompletionTriggerWarningPayload | null {
    if (triggerEnabled) {
        return null;
    }

    const scopeText = scopeUri ? scopeUri.toString() : 'global';
    return {
        scopeText,
        outputLines: [
            '⚠️ Completion auto-trigger for BSL is disabled: editor.suggestOnTriggerCharacters=false',
            `   Effective scope: ${scopeText}`,
            '   Remediation: enable editor.suggestOnTriggerCharacters (User/Workspace/[bsl]).',
        ],
        userMessage:
            'BSL Analyzer: editor.suggestOnTriggerCharacters=false. Completion по "." не будет автозапускаться.',
        actionLabel: 'Open Settings',
        settingsQuery: 'editor.suggestOnTriggerCharacters',
    };
}

async function warnIfCompletionTriggerDisabled(
    outputChannel: vscode.OutputChannel,
): Promise<void> {
    if (completionTriggerWarningShown) {
        return;
    }

    const scopeUri = completionTriggerConfigScopeUri();
    const payload = buildCompletionTriggerWarningPayload(
        isCompletionTriggerEnabledForBsl(scopeUri),
        scopeUri,
    );
    if (!payload) {
        return;
    }

    completionTriggerWarningShown = true;
    for (const line of payload.outputLines) {
        outputChannel.appendLine(line);
    }

    const selection = await vscode.window.showWarningMessage(
        payload.userMessage,
        payload.actionLabel,
    );
    if (selection === payload.actionLabel) {
        await vscode.commands.executeCommand(
            'workbench.action.openSettings',
            payload.settingsQuery,
        );
    }
}
