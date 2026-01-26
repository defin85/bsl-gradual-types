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
exports.setAutoReindexPaused = exports.updateLspStatus = exports.setIndexingProgress = exports.getCurrentProgress = exports.updateStatusBar = exports.initializeProgress = exports.progressEmitter = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const logger_1 = require("./logger");
// Глобальное состояние индексации
let globalIndexingProgress = {
    isIndexing: false,
    currentStep: 'Idle',
    progress: 0
};
let autoReindexPaused = false;
const AUTO_REINDEX_PAUSED_TEXT = '$(debug-pause) BSL: Auto reindex paused';
const AUTO_REINDEX_PAUSED_TOOLTIP = 'Auto reindex paused';
// Event emitter для обновления прогресса
exports.progressEmitter = new vscode.EventEmitter();
let outputChannel;
let statusBarItem;
/**
 * Инициализирует модуль прогресса
 */
function initializeProgress(channel, statusBar) {
    outputChannel = channel;
    statusBarItem = statusBar;
}
exports.initializeProgress = initializeProgress;
/**
 * Обновляет статус бар
 */
function updateStatusBar(text, progress) {
    if (!statusBarItem) {
        return;
    }
    if (text) {
        const resolvedText = autoReindexPaused && /\bReady\b/.test(text)
            ? AUTO_REINDEX_PAUSED_TEXT
            : text;
        statusBarItem.text = resolvedText;
        // Установить tooltip из text (удаляя иконки VSCode)
        const cleanText = resolvedText.replace(/\$\([^)]+\)/g, '').trim();
        statusBarItem.tooltip = cleanText;
        statusBarItem.show();
        return;
    }
    if (progress && progress.isIndexing) {
        const icon = '$(sync~spin)';
        const percent = Math.round(progress.progress);
        statusBarItem.text = `${icon} BSL Index: ${progress.currentStep} (${percent}%)`;
        statusBarItem.tooltip = `Progress: ${percent}%\n${progress.currentStep}`;
        statusBarItem.show();
    }
    else {
        if (autoReindexPaused) {
            statusBarItem.text = AUTO_REINDEX_PAUSED_TEXT;
            statusBarItem.tooltip = AUTO_REINDEX_PAUSED_TOOLTIP;
        }
        else {
            statusBarItem.text = '$(database) BSL Analyzer';
            statusBarItem.tooltip = 'BSL Type Safety Analyzer\nClick to build index';
        }
        statusBarItem.show();
    }
}
exports.updateStatusBar = updateStatusBar;
/**
 * Возвращает текущее состояние прогресса
 */
function getCurrentProgress() {
    return globalIndexingProgress;
}
exports.getCurrentProgress = getCurrentProgress;
function setIndexingProgress(progress) {
    globalIndexingProgress = progress;
    exports.progressEmitter.fire(progress);
}
exports.setIndexingProgress = setIndexingProgress;
/**
 * Обновляет status bar в зависимости от состояния LSP сервера
 *
 * @param state - состояние LSP клиента (State.Stopped | State.Starting | State.Running)
 */
function updateLspStatus(state) {
    if (!statusBarItem) {
        logger_1.logger.warn('[Progress] Status bar item not initialized for updateLspStatus - call initializeProgress() first');
        return;
    }
    switch (state) {
        case node_1.State.Stopped:
            statusBarItem.text = '$(error) BSL: Disconnected';
            statusBarItem.tooltip = 'BSL Language Server не активен\nПроверьте логи для деталей';
            statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
            break;
        case node_1.State.Starting:
            statusBarItem.text = '$(sync~spin) BSL: Starting...';
            statusBarItem.tooltip = 'BSL Language Server запускается...';
            statusBarItem.backgroundColor = undefined;
            break;
        case node_1.State.Running:
            if (autoReindexPaused) {
                statusBarItem.text = AUTO_REINDEX_PAUSED_TEXT;
                statusBarItem.tooltip = AUTO_REINDEX_PAUSED_TOOLTIP;
                statusBarItem.backgroundColor = undefined;
            }
            else {
                statusBarItem.text = '$(check) BSL: Ready';
                statusBarItem.tooltip = 'BSL Type Safety Analyzer\nLSP Server активен';
                statusBarItem.backgroundColor = undefined;
            }
            break;
        default:
            logger_1.logger.warn(`[Progress] Unknown LSP state: ${state}`);
            break;
    }
    statusBarItem.show();
}
exports.updateLspStatus = updateLspStatus;
function setAutoReindexPaused(paused) {
    autoReindexPaused = paused;
}
exports.setAutoReindexPaused = setAutoReindexPaused;
//# sourceMappingURL=progress.js.map