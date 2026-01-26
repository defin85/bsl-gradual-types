/**
 * Providers Setup Module
 *
 * Регистрация провайдеров: Diagnostics stats + (через LSP) Inlay Hints/Code Actions
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
import { EnhancedDiagnosticsProvider } from '../providers/enhancedDiagnosticsProvider';

export interface ProvidersResult {
    diagnosticsProvider: EnhancedDiagnosticsProvider;
}

/**
 * Регистрация всех enhanced providers
 */
export async function registerEnhancedProviders(
    context: vscode.ExtensionContext,
    languageClient: EnhancedLspClient,
    outputChannel: vscode.OutputChannel
): Promise<ProvidersResult> {
    outputChannel.appendLine('Registering enhanced providers...');

    // Важно: мы НЕ регистрируем кастомные providers-заглушки.
    //
    // Inlay hints и code actions должны приходить через стандартный LSP
    // (LanguageClient сам регистрирует фичи, если сервер объявляет capabilities).
    //
    // Если сервер их не поддерживает — VS Code не будет обещать эти фичи пользователю.
    void languageClient;
    void context;

    // Enhanced diagnostics provider
    const diagnosticsProvider = new EnhancedDiagnosticsProvider(languageClient, outputChannel);

    outputChannel.appendLine('Enhanced providers registered');

    return {
        diagnosticsProvider
    };
}
