import * as vscode from 'vscode';

export type RepoBoundConfigInspection<T> = {
    defaultValue?: T;
    globalValue?: T;
    workspaceValue?: T;
    workspaceFolderValue?: T;
};

export function resolveRepoBoundConfigValue<T>(
    inspection: RepoBoundConfigInspection<T> | undefined,
    fallback: T,
    workspaceOpen: boolean
): T {
    if (!inspection) {
        return fallback;
    }

    if (workspaceOpen) {
        return inspection.workspaceFolderValue
            ?? inspection.workspaceValue
            ?? inspection.defaultValue
            ?? fallback;
    }

    return inspection.workspaceFolderValue
        ?? inspection.workspaceValue
        ?? inspection.globalValue
        ?? inspection.defaultValue
        ?? fallback;
}

/**
 * Вспомогательный класс для работы с конфигурацией BSL Analyzer
 * Использует плоскую структуру настроек, организованную в категории
 */
export class BslAnalyzerConfig {
    private static getConfig() {
        return vscode.workspace.getConfiguration('bslAnalyzer');
    }

    private static getRepoBoundConfig<T>(key: string, fallback: T): T {
        const inspection = this.getConfig().inspect<T>(key);
        const workspaceOpen = (vscode.workspace.workspaceFolders?.length ?? 0) > 0;
        return resolveRepoBoundConfigValue(inspection, fallback, workspaceOpen);
    }
    
    // Основные настройки
    static get enabled(): boolean {
        return this.getConfig().get<boolean>('enabled', true);
    }
    
    static get enableRealTimeAnalysis(): boolean {
        return this.getConfig().get<boolean>('enableRealTimeAnalysis', true);
    }
    
    static get maxFileSize(): number {
        return this.getConfig().get<number>('maxFileSize', 1048576);
    }

    static get autoSignatureHelpOnCursorMove(): boolean {
        return this.getConfig().get<boolean>('autoSignatureHelpOnCursorMove', true);
    }
    
    // Настройки сервера
    static get serverMode(): string {
        return this.getConfig().get<string>('serverMode', 'stdio');
    }
    
    static get serverTcpPort(): number {
        return this.getConfig().get<number>('serverTcpPort', 8080);
    }
    
    static get serverTrace(): string {
        return this.getConfig().get<string>('serverTrace', 'off');
    }

    static get slowClientLogMs(): number {
        return this.getConfig().get<number>('slowClientLogMs', 2000);
    }

    static get diagnosticsDebounceMs(): number {
        return this.getConfig().get<number>('diagnosticsDebounceMs', 250);
    }

    static get observabilityAutoRefresh(): boolean {
        return this.getConfig().get<boolean>('observabilityAutoRefresh', true);
    }

    static get observabilityRefreshMs(): number {
        return this.getConfig().get<number>('observabilityRefreshMs', 3000);
    }

    static get observabilityCompactMode(): boolean {
        return this.getConfig().get<boolean>('observabilityCompactMode', false);
    }

    // Настройки бинарников
    static get useBundledBinaries(): boolean {
        return this.getConfig().get<boolean>('useBundledBinaries', true);
    }
    
    static get binaryPath(): string {
        return this.getConfig().get<string>('binaryPath', '');
    }
    
    // Настройки индексации
    static get configurationPath(): string {
        return this.getRepoBoundConfig('configurationPath', '');
    }
    
    static get platformVersion(): string {
        return this.getRepoBoundConfig('platformVersion', '8.3.25');
    }
    
    static get platformDocsArchive(): string {
        return this.getRepoBoundConfig('platformDocsArchive', '');
    }
    
    static get autoIndexBuild(): boolean {
        return this.getConfig().get<boolean>('autoIndexBuild', false);
    }

    static get autoReindexEnabled(): boolean {
        return this.getConfig().get<boolean>('autoReindexEnabled', true);
    }
    
    // Настройки анализа
    static get rulesConfig(): string {
        return this.getConfig().get<string>('rulesConfig', '');
    }
    
    static get enableMetrics(): boolean {
        return this.getConfig().get<boolean>('enableMetrics', true);
    }

    static get cacheEnabled(): boolean {
        return this.getConfig().get<boolean>('cacheEnabled', true);
    }
    
    // Enhanced methods для новой функциональности
    static isValid(): boolean {
        // Проверяем что основные настройки корректны
        return this.enabled && this.binaryPath.length > 0;
    }
    
    static summary(): any {
        return {
            enabled: this.enabled,
            serverMode: this.serverMode,
            serverTcpPort: this.serverTcpPort,
            slowClientLogMs: this.slowClientLogMs,
            diagnosticsDebounceMs: this.diagnosticsDebounceMs,
            observabilityAutoRefresh: this.observabilityAutoRefresh,
            observabilityRefreshMs: this.observabilityRefreshMs,
            observabilityCompactMode: this.observabilityCompactMode,
            binaryPath: this.binaryPath,
            configurationPath: this.configurationPath,
            enableRealTimeAnalysis: this.enableRealTimeAnalysis,
            cacheEnabled: this.cacheEnabled
        };
    }
}

/**
 * Мапинг старых настроек на новые (если были изменения имен)
 */
const LEGACY_CONFIG_MAP: { [oldKey: string]: string } = {
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
    'index.autoReindexEnabled': 'autoReindexEnabled',
    'autoReindex.enabled': 'autoReindexEnabled',
    'analysis.rulesConfig': 'rulesConfig',
    'analysis.enableMetrics': 'enableMetrics'
};

/**
 * Мигрирует старые настройки на новые имена
 */
export async function migrateLegacySettings(): Promise<void> {
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
        vscode.window.showInformationMessage(
            `BSL Analyzer: Мигрировано ${migratedCount} устаревших настроек.`
        );
    }
}
