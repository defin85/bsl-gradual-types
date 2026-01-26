"use strict";
/**
 * Simplified Enhanced Diagnostics Provider
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
exports.EnhancedDiagnosticsProvider = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
class EnhancedDiagnosticsProvider {
    constructor(client, outputChannel) {
        void client;
        this.outputChannel = outputChannel;
    }
    /**
     * Получение статистики диагностик
     */
    getDiagnosticsStats() {
        let errors = 0;
        let warnings = 0;
        let infos = 0;
        let hints = 0;
        // Диагностики приходят в VS Code через стандартный LSP pipeline.
        // Здесь считаем статистику по workspace diagnostics и фильтруем по BSL файлам.
        const all = vscode.languages.getDiagnostics();
        for (const [uri, diagnostics] of all) {
            if (uri.scheme !== 'file')
                continue;
            const ext = path.extname(uri.fsPath).toLowerCase();
            if (ext !== '.bsl' && ext !== '.os')
                continue;
            for (const d of diagnostics) {
                switch (d.severity) {
                    case vscode.DiagnosticSeverity.Error:
                        errors += 1;
                        break;
                    case vscode.DiagnosticSeverity.Warning:
                        warnings += 1;
                        break;
                    case vscode.DiagnosticSeverity.Information:
                        infos += 1;
                        break;
                    case vscode.DiagnosticSeverity.Hint:
                        hints += 1;
                        break;
                }
            }
        }
        return { errors, warnings, infos, hints };
    }
}
exports.EnhancedDiagnosticsProvider = EnhancedDiagnosticsProvider;
//# sourceMappingURL=enhancedDiagnosticsProvider.js.map