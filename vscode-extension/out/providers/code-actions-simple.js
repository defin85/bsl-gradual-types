"use strict";
/**
 * Simplified Code Actions Provider
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.CodeActionsProvider = void 0;
class CodeActionsProvider {
    constructor(client) {
        this.client = client;
    }
    async provideCodeActions(document, range, context, token) {
        try {
            // Простая заглушка - в будущем будет реальная реализация
            return [];
        }
        catch (error) {
            console.error('Error providing code actions:', error);
            return [];
        }
    }
}
exports.CodeActionsProvider = CodeActionsProvider;
//# sourceMappingURL=code-actions-simple.js.map