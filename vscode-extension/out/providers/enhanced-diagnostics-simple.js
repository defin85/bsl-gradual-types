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
class EnhancedDiagnosticsProvider {
    constructor(client, outputChannel) {
        this.client = client;
        this.outputChannel = outputChannel;
        this.diagnosticsCollection = vscode.languages.createDiagnosticCollection('bsl-gradual-types');
    }
    /**
     * Получение статистики диагностик
     */
    getDiagnosticsStats() {
        return {
            errors: 0,
            warnings: 0,
            infos: 0,
            hints: 0
        };
    }
}
exports.EnhancedDiagnosticsProvider = EnhancedDiagnosticsProvider;
//# sourceMappingURL=enhanced-diagnostics-simple.js.map