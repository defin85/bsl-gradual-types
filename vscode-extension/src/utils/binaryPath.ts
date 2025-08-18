import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { BslAnalyzerConfig } from '../config/configHelper';

let outputChannel: vscode.OutputChannel;

export function setOutputChannel(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

/**
 * Получает путь к бинарному файлу BSL Analyzer
 * @param binaryName Имя бинарного файла (без расширения)
 * @param extensionContext Контекст расширения
 * @returns Полный путь к бинарному файлу
 */
export function getBinaryPath(binaryName: string, extensionContext?: vscode.ExtensionContext): string {
    const useBundled = BslAnalyzerConfig.useBundledBinaries;
    
    // Если явно указано использовать встроенные бинарники
    if (useBundled) {
        // Сначала пробуем глобальный контекст (для development режима)
        if (extensionContext) {
            const contextBinPath = path.join(extensionContext.extensionPath, 'bin', `${binaryName}.exe`);
            if (fs.existsSync(contextBinPath)) {
                outputChannel?.appendLine(`✅ Using bundled binary from context: ${contextBinPath}`);
                return contextBinPath;
            }
        }
        
        // Затем пробуем найти установленное расширение
        const extensionPath = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer')?.extensionPath;
        if (extensionPath) {
            const bundledBinPath = path.join(extensionPath, 'bin', `${binaryName}.exe`);
            if (fs.existsSync(bundledBinPath)) {
                outputChannel?.appendLine(`✅ Using bundled binary: ${bundledBinPath}`);
                return bundledBinPath;
            }
        }
        
        // Fallback на vscode-extension/bin для development
        const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (workspacePath) {
            const devBinPath = path.join(workspacePath, 'vscode-extension', 'bin', `${binaryName}.exe`);
            if (fs.existsSync(devBinPath)) {
                outputChannel?.appendLine(`✅ Using development binary: ${devBinPath}`);
                return devBinPath;
            }
        }
    }
    
    // Если указан внешний путь к бинарникам
    const binaryPath = BslAnalyzerConfig.binaryPath;
    if (binaryPath) {
        const externalBinPath = path.join(binaryPath, `${binaryName}.exe`);
        if (fs.existsSync(externalBinPath)) {
            outputChannel?.appendLine(`✅ Using external binary: ${externalBinPath}`);
            return externalBinPath;
        }
        
        outputChannel?.appendLine(`❌ Binary not found in specified path: ${externalBinPath}`);
    }
    
    // Последняя попытка - проверить в PATH
    const pathBinary = `${binaryName}.exe`;
    outputChannel?.appendLine(`⚠️ Attempting to use binary from PATH: ${pathBinary}`);
    return pathBinary;
}