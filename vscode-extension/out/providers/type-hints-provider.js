"use strict";
/**
 * Type Hints Provider для VSCode
 *
 * Предоставляет inline type hints на основе результатов
 * flow-sensitive анализа и union types
 */
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
exports.TypeHintsProvider = void 0;
const vscode = __importStar(require("vscode"));
class TypeHintsProvider {
    constructor(client) {
        this.client = client;
    }
    async provideInlayHints(document, range, token) {
        try {
            // Запрашиваем type hints от enhanced LSP сервера
            const result = await this.client.requestInlayHints({
                textDocument: { uri: document.uri.toString() },
                range: {
                    start: { line: range.start.line, character: range.start.character },
                    end: { line: range.end.line, character: range.end.character }
                }
            });
            if (!result) {
                return [];
            }
            return result.map(hint => this.convertToVSCodeInlayHint(hint));
        }
        catch (error) {
            console.error('Error providing inlay hints:', error);
            return [];
        }
    }
    /**
     * Конвертация LSP inlay hint в VSCode формат
     */
    convertToVSCodeInlayHint(lspHint) {
        const position = new vscode.Position(lspHint.position.line, lspHint.position.character);
        const hint = new vscode.InlayHint(position, lspHint.label, lspHint.kind === 1 ? vscode.InlayHintKind.Type : vscode.InlayHintKind.Parameter);
        if (lspHint.tooltip) {
            hint.tooltip = lspHint.tooltip;
        }
        hint.paddingLeft = lspHint.paddingLeft || false;
        hint.paddingRight = lspHint.paddingRight || false;
        return hint;
    }
    /**
     * Обновление настроек type hints
     */
    updateSettings() {
        // Уведомляем LSP сервер об изменении настроек
        // TODO: Реализовать отправку updated settings в LSP сервер
    }
}
exports.TypeHintsProvider = TypeHintsProvider;
//# sourceMappingURL=type-hints-provider.js.map