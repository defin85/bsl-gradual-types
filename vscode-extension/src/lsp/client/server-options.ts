import * as vscode from 'vscode';
import {
    ServerOptions,
    TransportKind,
    Executable
} from 'vscode-languageclient/node';
import { BslAnalyzerConfig } from '../../config/configHelper';

/**
 * Строит ServerOptions для LSP клиента
 * @param serverPath Путь к исполняемому файлу LSP сервера
 * @param outputChannel Канал для логирования
 */
export function buildServerOptions(
    serverPath: string,
    outputChannel: vscode.OutputChannel
): ServerOptions {
    const serverMode = BslAnalyzerConfig.serverMode;
    const tcpPort = BslAnalyzerConfig.serverTcpPort;

    if (serverMode === 'stdio') {
        // STDIO mode - прямой запуск (как в rust-analyzer)
        const newEnv = { ...process.env };
        newEnv.RUST_LOG = 'debug';
        newEnv.RUST_BACKTRACE = 'full';
        newEnv.BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS = String(BslAnalyzerConfig.slowClientLogMs);
        newEnv.BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS = String(BslAnalyzerConfig.diagnosticsDebounceMs);
        if (BslAnalyzerConfig.debugDiagnosticsSaveCoherence) {
            newEnv.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE = '1';
        } else {
            delete newEnv.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE;
        }

        const run: Executable = {
            command: serverPath,
            options: { env: newEnv }
        };

        outputChannel.appendLine(`STDIO mode: command = ${serverPath}`);
        outputChannel.appendLine(
            `STDIO mode: BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS=${newEnv.BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS}`
        );
        outputChannel.appendLine(
            `STDIO mode: BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS=${newEnv.BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS}`
        );
        if (newEnv.BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE === '1') {
            outputChannel.appendLine('STDIO mode: BSL_DEBUG_DIAGNOSTICS_SAVE_COHERENCE=1');
        }

        return {
            run,
            debug: run
        };
    } else {
        // TCP mode - подключаемся к серверу
        outputChannel.appendLine(`TCP mode: connecting to port ${tcpPort}...`);

        return {
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
}
