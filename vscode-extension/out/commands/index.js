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
exports.registerObservabilityCommands = exports.registerCacheCommands = exports.registerDebugCommands = exports.registerConfigurationCommands = exports.registerIndexCommands = exports.registerSearchCommands = exports.registerAnalysisCommands = exports.registerParseConfigurationCommand = exports.registerSemanticVisualization = exports.registerCommands = exports.initializeCommands = void 0;
const vscode = __importStar(require("vscode"));
const lsp_1 = require("../lsp");
const parseConfiguration_1 = require("./parseConfiguration");
const analysis_1 = require("./analysis");
const search_1 = require("./search");
const index_commands_1 = require("./index-commands");
const configuration_1 = require("./configuration");
const debug_1 = require("./debug");
const cache_1 = require("./cache");
const observability_1 = require("./observability");
let outputChannel;
let commandsRegistered = false;
function initializeCommands(channel) {
    outputChannel = channel;
}
exports.initializeCommands = initializeCommands;
/**
 * Helper function to safely register commands with duplicate check
 */
async function safeRegisterCommand(context, commandId, callback) {
    try {
        const disposable = vscode.commands.registerCommand(commandId, callback);
        context.subscriptions.push(disposable);
        outputChannel.appendLine(`Registered command: ${commandId}`);
        return disposable;
    }
    catch (error) {
        // Если ошибка о том, что команда уже зарегистрирована - это нормально
        if (error.message && error.message.includes('already exists')) {
            outputChannel.appendLine(`Command already registered: ${commandId}, skipping...`);
            return null;
        }
        // Другие ошибки - это проблема
        outputChannel.appendLine(`Failed to register command ${commandId}: ${error}`);
        return null;
    }
}
async function registerCommands(context) {
    // Защита от двойной регистрации
    if (commandsRegistered) {
        outputChannel.appendLine('Commands already registered, skipping...');
        return;
    }
    outputChannel.appendLine('Registering BSL Analyzer commands...');
    // Create bound safeRegisterCommand for passing to modules
    const boundSafeRegister = (commandId, callback) => safeRegisterCommand(context, commandId, callback);
    // Register all command modules
    (0, analysis_1.registerAnalysisCommands)(context, boundSafeRegister, outputChannel);
    (0, search_1.registerSearchCommands)(context, boundSafeRegister, outputChannel);
    (0, index_commands_1.registerIndexCommands)(context, boundSafeRegister, outputChannel);
    (0, configuration_1.registerConfigurationCommands)(context, boundSafeRegister, outputChannel);
    (0, debug_1.registerDebugCommands)(context, boundSafeRegister, outputChannel);
    (0, cache_1.registerCacheCommands)(context, boundSafeRegister, outputChannel);
    (0, observability_1.registerObservabilityCommands)(context, boundSafeRegister, outputChannel);
    // Parse Configuration (MILESTONE 2.17)
    // Регистрация через отдельный модуль для лучшей организации кода
    const client = (0, lsp_1.getLanguageClient)();
    if (client) {
        const parseConfigDisposable = (0, parseConfiguration_1.registerParseConfigurationCommand)(context, client);
        if (parseConfigDisposable) {
            outputChannel.appendLine('Registered command: bslAnalyzer.parseConfiguration');
        }
    }
    else {
        outputChannel.appendLine('Cannot register bslAnalyzer.parseConfiguration - LSP client not ready');
    }
    // Устанавливаем флаг, что команды зарегистрированы
    commandsRegistered = true;
    outputChannel.appendLine('Successfully registered all extension commands');
}
exports.registerCommands = registerCommands;
// Re-exports
var semanticVisualization_1 = require("./semanticVisualization");
Object.defineProperty(exports, "registerSemanticVisualization", { enumerable: true, get: function () { return semanticVisualization_1.registerSemanticVisualization; } });
var parseConfiguration_2 = require("./parseConfiguration");
Object.defineProperty(exports, "registerParseConfigurationCommand", { enumerable: true, get: function () { return parseConfiguration_2.registerParseConfigurationCommand; } });
var analysis_2 = require("./analysis");
Object.defineProperty(exports, "registerAnalysisCommands", { enumerable: true, get: function () { return analysis_2.registerAnalysisCommands; } });
var search_2 = require("./search");
Object.defineProperty(exports, "registerSearchCommands", { enumerable: true, get: function () { return search_2.registerSearchCommands; } });
var index_commands_2 = require("./index-commands");
Object.defineProperty(exports, "registerIndexCommands", { enumerable: true, get: function () { return index_commands_2.registerIndexCommands; } });
var configuration_2 = require("./configuration");
Object.defineProperty(exports, "registerConfigurationCommands", { enumerable: true, get: function () { return configuration_2.registerConfigurationCommands; } });
var debug_2 = require("./debug");
Object.defineProperty(exports, "registerDebugCommands", { enumerable: true, get: function () { return debug_2.registerDebugCommands; } });
var cache_2 = require("./cache");
Object.defineProperty(exports, "registerCacheCommands", { enumerable: true, get: function () { return cache_2.registerCacheCommands; } });
var observability_2 = require("./observability");
Object.defineProperty(exports, "registerObservabilityCommands", { enumerable: true, get: function () { return observability_2.registerObservabilityCommands; } });
//# sourceMappingURL=index.js.map