import * as vscode from 'vscode';
import { CommandHandler } from '../types';
import { getLanguageClient } from '../lsp';
import { registerParseConfigurationCommand } from './parseConfiguration';
import { registerAnalysisCommands } from './analysis';
import { registerSearchCommands } from './search';
import { registerIndexCommands } from './index-commands';
import { registerConfigurationCommands } from './configuration';
import { registerDebugCommands } from './debug';
import { registerCacheCommands } from './cache';
import { registerObservabilityCommands } from './observability';

let outputChannel: vscode.OutputChannel;
let commandsRegistered = false;

export function initializeCommands(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

/**
 * Helper function to safely register commands with duplicate check
 */
async function safeRegisterCommand(
    context: vscode.ExtensionContext,
    commandId: string,
    callback: CommandHandler
): Promise<vscode.Disposable | null> {
    try {
        const disposable = vscode.commands.registerCommand(commandId, callback);
        context.subscriptions.push(disposable);
        outputChannel.appendLine(`Registered command: ${commandId}`);
        return disposable;
    } catch (error: any) {
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

export async function registerCommands(context: vscode.ExtensionContext) {
    // Защита от двойной регистрации
    if (commandsRegistered) {
        outputChannel.appendLine('Commands already registered, skipping...');
        return;
    }

    outputChannel.appendLine('Registering BSL Analyzer commands...');

    // Create bound safeRegisterCommand for passing to modules
    const boundSafeRegister = (commandId: string, callback: CommandHandler) =>
        safeRegisterCommand(context, commandId, callback);

    // Register all command modules
    registerAnalysisCommands(context, boundSafeRegister, outputChannel);
    registerSearchCommands(context, boundSafeRegister, outputChannel);
    registerIndexCommands(context, boundSafeRegister, outputChannel);
    registerConfigurationCommands(context, boundSafeRegister, outputChannel);
    registerDebugCommands(context, boundSafeRegister, outputChannel);
    registerCacheCommands(context, boundSafeRegister, outputChannel);
    registerObservabilityCommands(context, boundSafeRegister, outputChannel);

    // Parse Configuration (MILESTONE 2.17)
    // Регистрация через отдельный модуль для лучшей организации кода
    const client = getLanguageClient();
    if (client) {
        const parseConfigDisposable = registerParseConfigurationCommand(context, client);
        if (parseConfigDisposable) {
            outputChannel.appendLine('Registered command: bslAnalyzer.parseConfiguration');
        }
    } else {
        outputChannel.appendLine('Cannot register bslAnalyzer.parseConfiguration - LSP client not ready');
    }

    // Устанавливаем флаг, что команды зарегистрированы
    commandsRegistered = true;
    outputChannel.appendLine('Successfully registered all extension commands');
}

// Re-exports
export { registerSemanticVisualization } from './semanticVisualization';
export { registerParseConfigurationCommand } from './parseConfiguration';
export { registerAnalysisCommands } from './analysis';
export { registerSearchCommands } from './search';
export { registerIndexCommands } from './index-commands';
export { registerConfigurationCommands } from './configuration';
export { registerDebugCommands } from './debug';
export { registerCacheCommands } from './cache';
export { registerObservabilityCommands } from './observability';
