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
exports.migrateLegacySettings = exports.BslAnalyzerConfig = void 0;
const vscode = __importStar(require("vscode"));
/**
 * Вспомогательный класс для работы с конфигурацией BSL Analyzer
 * Использует плоскую структуру настроек, организованную в категории
 */
class BslAnalyzerConfig {
    static getConfig() {
        return vscode.workspace.getConfiguration('bslAnalyzer');
    }
    // Основные настройки
    static get enabled() {
        return this.getConfig().get('enabled', true);
    }
    static get enableRealTimeAnalysis() {
        return this.getConfig().get('enableRealTimeAnalysis', true);
    }
    static get maxFileSize() {
        return this.getConfig().get('maxFileSize', 1048576);
    }
    // Настройки сервера
    static get serverMode() {
        return this.getConfig().get('serverMode', 'stdio');
    }
    static get serverTcpPort() {
        return this.getConfig().get('serverTcpPort', 8080);
    }
    static get serverTrace() {
        return this.getConfig().get('serverTrace', 'off');
    }
    // Настройки бинарников
    static get useBundledBinaries() {
        return this.getConfig().get('useBundledBinaries', true);
    }
    static get binaryPath() {
        return this.getConfig().get('binaryPath', '');
    }
    // Настройки индексации
    static get configurationPath() {
        return this.getConfig().get('configurationPath', '');
    }
    static get platformVersion() {
        return this.getConfig().get('platformVersion', '8.3.25');
    }
    static get platformDocsArchive() {
        return this.getConfig().get('platformDocsArchive', '');
    }
    static get autoIndexBuild() {
        return this.getConfig().get('autoIndexBuild', false);
    }
    // Настройки анализа
    static get rulesConfig() {
        return this.getConfig().get('rulesConfig', '');
    }
    static get enableMetrics() {
        return this.getConfig().get('enableMetrics', true);
    }
    // Enhanced methods для новой функциональности
    static isValid() {
        // Проверяем что основные настройки корректны
        return this.enabled && this.binaryPath.length > 0;
    }
    static summary() {
        return {
            enabled: this.enabled,
            serverMode: this.serverMode,
            serverTcpPort: this.serverTcpPort,
            binaryPath: this.binaryPath,
            configurationPath: this.configurationPath,
            enableRealTimeAnalysis: this.enableRealTimeAnalysis
        };
    }
}
exports.BslAnalyzerConfig = BslAnalyzerConfig;
/**
 * Мапинг старых настроек на новые (если были изменения имен)
 */
const LEGACY_CONFIG_MAP = {
    'indexServerPath': 'binaryPath',
    'tcpPort': 'serverTcpPort',
    'trace.server': 'serverTrace',
    // Для вложенных настроек (если кто-то уже использовал экспериментальную версию)
    'general.enableRealTimeAnalysis': 'enableRealTimeAnalysis',
    'general.maxFileSize': 'maxFileSize',
    'server.mode': 'serverMode',
    'server.tcpPort': 'serverTcpPort',
    'server.trace': 'serverTrace',
    'binaries.useBundled': 'useBundledBinaries',
    'binaries.path': 'binaryPath',
    'index.configurationPath': 'configurationPath',
    'index.platformVersion': 'platformVersion',
    'index.platformDocsArchive': 'platformDocsArchive',
    'index.autoIndexBuild': 'autoIndexBuild',
    'analysis.rulesConfig': 'rulesConfig',
    'analysis.enableMetrics': 'enableMetrics'
};
/**
 * Мигрирует старые настройки на новые имена
 */
async function migrateLegacySettings() {
    const config = vscode.workspace.getConfiguration('bslAnalyzer');
    let migratedCount = 0;
    for (const [oldKey, newKey] of Object.entries(LEGACY_CONFIG_MAP)) {
        const inspection = config.inspect(oldKey);
        if (inspection) {
            // Мигрируем глобальные настройки
            if (inspection.globalValue !== undefined) {
                await config.update(newKey, inspection.globalValue, vscode.ConfigurationTarget.Global);
                await config.update(oldKey, undefined, vscode.ConfigurationTarget.Global);
                migratedCount++;
            }
            // Мигрируем настройки рабочей области
            if (inspection.workspaceValue !== undefined) {
                await config.update(newKey, inspection.workspaceValue, vscode.ConfigurationTarget.Workspace);
                await config.update(oldKey, undefined, vscode.ConfigurationTarget.Workspace);
                migratedCount++;
            }
            // Мигрируем настройки папки рабочей области
            if (inspection.workspaceFolderValue !== undefined) {
                await config.update(newKey, inspection.workspaceFolderValue, vscode.ConfigurationTarget.WorkspaceFolder);
                await config.update(oldKey, undefined, vscode.ConfigurationTarget.WorkspaceFolder);
                migratedCount++;
            }
        }
    }
    if (migratedCount > 0) {
        vscode.window.showInformationMessage(`BSL Analyzer: Мигрировано ${migratedCount} устаревших настроек.`);
    }
}
exports.migrateLegacySettings = migrateLegacySettings;
//# sourceMappingURL=configHelper.js.map