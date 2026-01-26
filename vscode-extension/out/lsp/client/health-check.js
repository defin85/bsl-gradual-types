"use strict";
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
exports.stopHealthCheck = exports.startHealthCheck = void 0;
const vscode = __importStar(require("vscode"));
const progress_1 = require("../progress");
/** Интервал health check */
let healthCheckInterval = null;
/**
 * Запускает периодическую проверку состояния LSP сервера
 * @param client LSP клиент для проверки
 * @param outputChannel Канал для логирования
 */
function startHealthCheck(client, outputChannel) {
    // Останавливаем предыдущий интервал, если он есть
    stopHealthCheck();
    // Проверяем состояние каждые 30 секунд
    healthCheckInterval = setInterval(() => {
        if (client) {
            const isRunning = client.isRunning();
            if (!isRunning) {
                outputChannel.appendLine('Health check: LSP client is not running');
                (0, progress_1.updateStatusBar)('$(error) BSL Analyzer: Disconnected');
                vscode.commands.executeCommand('bslAnalyzer.refreshOverview');
                // Показываем уведомление только один раз
                stopHealthCheck();
                vscode.window.showWarningMessage('BSL Analyzer: Language server stopped unexpectedly', 'Restart Server', 'Dismiss').then(selection => {
                    if (selection === 'Restart Server') {
                        vscode.commands.executeCommand('bslAnalyzer.restartServer');
                    }
                });
            }
        }
    }, 30000); // 30 секунд
}
exports.startHealthCheck = startHealthCheck;
/**
 * Останавливает периодическую проверку состояния
 */
function stopHealthCheck() {
    if (healthCheckInterval) {
        clearInterval(healthCheckInterval);
        healthCheckInterval = null;
    }
}
exports.stopHealthCheck = stopHealthCheck;
//# sourceMappingURL=health-check.js.map