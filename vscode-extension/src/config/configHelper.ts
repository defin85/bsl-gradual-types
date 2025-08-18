import * as vscode from 'vscode';

/**
 * Вспомогательный класс для работы с конфигурацией BSL Analyzer
 * Использует плоскую структуру настроек, организованную в категории
 */
export class BslAnalyzerConfig {
    private static getConfig() {
        return vscode.workspace.getConfiguration('bslAnalyzer');
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
    
    // Настройки бинарников
    static get useBundledBinaries(): boolean {
        return this.getConfig().get<boolean>('useBundledBinaries', true);
    }
    
    static get binaryPath(): string {
        return this.getConfig().get<string>('binaryPath', '');
    }
    
    // Настройки индексации
    static get configurationPath(): string {
        return this.getConfig().get<string>('configurationPath', '');
    }
    
    static get platformVersion(): string {
        return this.getConfig().get<string>('platformVersion', '8.3.25');
    }
    
    static get platformDocsArchive(): string {
        return this.getConfig().get<string>('platformDocsArchive', '');
    }
    
    static get autoIndexBuild(): boolean {
        return this.getConfig().get<boolean>('autoIndexBuild', false);
    }
    
    // Настройки анализа
    static get rulesConfig(): string {
        return this.getConfig().get<string>('rulesConfig', '');
    }
    
    static get enableMetrics(): boolean {
        return this.getConfig().get<boolean>('enableMetrics', true);
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