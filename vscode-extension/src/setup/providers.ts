/**
 * Providers Setup Module
 *
 * Регистрация провайдеров: Type Hints, Code Actions, Diagnostics
 */

import * as vscode from 'vscode';
import { EnhancedLspClient } from '../lsp/enhanced-client';
import { TypeHintsProvider } from '../providers/type-hints-simple';
import { CodeActionsProvider } from '../providers/code-actions-simple';
import { EnhancedDiagnosticsProvider } from '../providers/enhanced-diagnostics-simple';

export interface ProvidersResult {
    typeHintsProvider: TypeHintsProvider;
    codeActionsProvider: CodeActionsProvider;
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

    // Type hints provider (inlay hints)
    const typeHintsProvider = new TypeHintsProvider(languageClient);
    context.subscriptions.push(
        vscode.languages.registerInlayHintsProvider(
            { scheme: 'file', language: 'bsl' },
            typeHintsProvider
        )
    );

    // Enhanced code actions provider
    const codeActionsProvider = new CodeActionsProvider(languageClient);
    context.subscriptions.push(
        vscode.languages.registerCodeActionsProvider(
            { scheme: 'file', language: 'bsl' },
            codeActionsProvider,
            {
                providedCodeActionKinds: [
                    vscode.CodeActionKind.QuickFix,
                    vscode.CodeActionKind.Refactor,
                    vscode.CodeActionKind.RefactorExtract,
                ]
            }
        )
    );

    // Enhanced diagnostics provider
    const diagnosticsProvider = new EnhancedDiagnosticsProvider(languageClient, outputChannel);

    outputChannel.appendLine('Enhanced providers registered');

    return {
        typeHintsProvider,
        codeActionsProvider,
        diagnosticsProvider
    };
}
