import * as vscode from 'vscode';
import { BslAnalyzerConfig } from '../config';
import * as fs from 'fs';

let outputChannel: vscode.OutputChannel | undefined;

export function setOutputChannel(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

/**
 * Получить путь к конфигурации
 */
export function getConfigurationPath(): string {
    return BslAnalyzerConfig.configurationPath;
}

/**
 * Получить версию платформы
 */
export function getPlatformVersion(): string {
    return BslAnalyzerConfig.platformVersion;
}

/**
 * Получить путь к архиву документации платформы
 */
export function getPlatformDocsArchive(): string {
    const userArchive = BslAnalyzerConfig.platformDocsArchive;
    
    if (userArchive && fs.existsSync(userArchive)) {
        outputChannel?.appendLine(`📚 Using user-specified platform documentation: ${userArchive}`);
        return userArchive;
    }
    
    if (!userArchive) {
        outputChannel?.appendLine(`⚠️ Platform documentation not configured. Some features may be limited.`);
        outputChannel?.appendLine(`💡 Specify path to rebuilt.shcntx_ru.zip or rebuilt.shlang_ru.zip in settings.`);
    } else {
        outputChannel?.appendLine(`❌ Platform documentation not found at: ${userArchive}`);
    }
    
    return '';
}