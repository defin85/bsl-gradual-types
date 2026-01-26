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
exports.buildClientOptions = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const configHelper_1 = require("../../config/configHelper");
/**
 * Строит LanguageClientOptions для LSP клиента
 * @param outputChannel Канал для логирования
 */
function buildClientOptions(outputChannel) {
    // MILESTONE 2.10: Подготавливаем initializationOptions для передачи в LSP
    const initializationOptions = {
        platformDocsArchive: configHelper_1.BslAnalyzerConfig.platformDocsArchive,
        configurationPath: configHelper_1.BslAnalyzerConfig.configurationPath,
        platformVersion: configHelper_1.BslAnalyzerConfig.platformVersion,
        cacheEnabled: configHelper_1.BslAnalyzerConfig.cacheEnabled
    };
    outputChannel.appendLine(`Sending initializationOptions to LSP:`);
    outputChannel.appendLine(`   platformDocsArchive: ${initializationOptions.platformDocsArchive || 'NOT SET'}`);
    outputChannel.appendLine(`   configurationPath: ${initializationOptions.configurationPath || 'NOT SET'}`);
    outputChannel.appendLine(`   platformVersion: ${initializationOptions.platformVersion || 'NOT SET'}`);
    outputChannel.appendLine(`   cacheEnabled: ${initializationOptions.cacheEnabled}`);
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
            // MILESTONE 3.6: Синхронизируем ОБЕ секции настроек (bslAnalyzer + bsl)
            configurationSection: ['bslAnalyzer', 'bsl']
        },
        // MILESTONE 2.10: Передаём initializationOptions в LSP
        initializationOptions: initializationOptions,
        outputChannel: outputChannel,
        revealOutputChannelOn: node_1.RevealOutputChannelOn.Never,
        traceOutputChannel: outputChannel,
        middleware: {
            // Перехватываем workspace-related notifications
            workspace: {
                configuration: (params, token, next) => {
                    outputChannel.appendLine(`Configuration request: ${JSON.stringify(params)}`);
                    return next(params, token);
                }
            }
        }
    };
    // ПРИНУДИТЕЛЬНО устанавливаем VERBOSE tracing для отладки Work Done Progress
    clientOptions.trace = node_1.Trace.Verbose;
    outputChannel.appendLine('TRACE: Verbose logging enabled');
    return clientOptions;
}
exports.buildClientOptions = buildClientOptions;
//# sourceMappingURL=client-options.js.map