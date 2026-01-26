"use strict";
/**
 * Providers Setup Module
 *
 * Регистрация провайдеров: Diagnostics stats + (через LSP) Inlay Hints/Code Actions
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerEnhancedProviders = void 0;
const enhancedDiagnosticsProvider_1 = require("../providers/enhancedDiagnosticsProvider");
/**
 * Регистрация всех enhanced providers
 */
async function registerEnhancedProviders(context, languageClient, outputChannel) {
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
    const diagnosticsProvider = new enhancedDiagnosticsProvider_1.EnhancedDiagnosticsProvider(languageClient, outputChannel);
    outputChannel.appendLine('Enhanced providers registered');
    return {
        diagnosticsProvider
    };
}
exports.registerEnhancedProviders = registerEnhancedProviders;
//# sourceMappingURL=providers.js.map