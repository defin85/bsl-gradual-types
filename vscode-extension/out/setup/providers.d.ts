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
export declare function registerEnhancedProviders(context: vscode.ExtensionContext, languageClient: EnhancedLspClient, outputChannel: vscode.OutputChannel): Promise<ProvidersResult>;
//# sourceMappingURL=providers.d.ts.map