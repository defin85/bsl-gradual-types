"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.buildServerOptions = void 0;
const node_1 = require("vscode-languageclient/node");
const configHelper_1 = require("../../config/configHelper");
/**
 * Строит ServerOptions для LSP клиента
 * @param serverPath Путь к исполняемому файлу LSP сервера
 * @param outputChannel Канал для логирования
 */
function buildServerOptions(serverPath, outputChannel) {
    const serverMode = configHelper_1.BslAnalyzerConfig.serverMode;
    const tcpPort = configHelper_1.BslAnalyzerConfig.serverTcpPort;
    if (serverMode === 'stdio') {
        // STDIO mode - прямой запуск (как в rust-analyzer)
        const newEnv = { ...process.env };
        newEnv.RUST_LOG = 'debug';
        newEnv.RUST_BACKTRACE = 'full';
        newEnv.BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS = String(configHelper_1.BslAnalyzerConfig.slowClientLogMs);
        const run = {
            command: serverPath,
            options: { env: newEnv }
        };
        outputChannel.appendLine(`STDIO mode: command = ${serverPath}`);
        outputChannel.appendLine(`STDIO mode: BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS=${newEnv.BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS}`);
        return {
            run,
            debug: run
        };
    }
    else {
        // TCP mode - подключаемся к серверу
        outputChannel.appendLine(`TCP mode: connecting to port ${tcpPort}...`);
        return {
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
}
exports.buildServerOptions = buildServerOptions;
//# sourceMappingURL=server-options.js.map