"use strict";
/**
 * Simplified Type Hints Provider
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.TypeHintsProvider = void 0;
class TypeHintsProvider {
    constructor(client) {
        this.client = client;
    }
    async provideInlayHints(document, range, token) {
        try {
            // Простая заглушка - в будущем будет реальная реализация
            return [];
        }
        catch (error) {
            console.error('Error providing inlay hints:', error);
            return [];
        }
    }
}
exports.TypeHintsProvider = TypeHintsProvider;
//# sourceMappingURL=type-hints-simple.js.map