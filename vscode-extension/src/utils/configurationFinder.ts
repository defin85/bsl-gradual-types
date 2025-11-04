import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import { logger } from '../lsp/logger';

export interface ConfigurationInfo {
    path: string;
    name: string;
    isExtension: boolean;
    uuid?: string;
}

/**
 * Находит все конфигурации 1С в директории
 */
export async function findConfigurations(rootPath: string): Promise<ConfigurationInfo[]> {
    const configurations: ConfigurationInfo[] = [];
    
    try {
        const entries = await fs.promises.readdir(rootPath, { withFileTypes: true });
        
        for (const entry of entries) {
            if (entry.isDirectory()) {
                const configPath = path.join(rootPath, entry.name);
                const configXmlPath = path.join(configPath, 'Configuration.xml');
                
                if (fs.existsSync(configXmlPath)) {
                    const configInfo = await analyzeConfiguration(configXmlPath);
                    if (configInfo) {
                        configurations.push({
                            ...configInfo,
                            path: configPath,
                            name: entry.name
                        });
                    }
                }
            }
        }
    } catch (error) {
        logger.error(`Error scanning for configurations: ${error}`);
    }
    
    return configurations;
}

/**
 * Анализирует Configuration.xml и определяет тип конфигурации
 */
async function analyzeConfiguration(xmlPath: string): Promise<{ isExtension: boolean; uuid?: string } | null> {
    try {
        const content = await fs.promises.readFile(xmlPath, 'utf-8');
        
        // Проверяем, является ли это расширением
        const isExtension = content.includes('<ConfigurationExtensionPurpose>');
        
        // Извлекаем UUID конфигурации
        const uuidMatch = content.match(/<Configuration[^>]*uuid="([^"]+)"/);
        const uuid = uuidMatch ? uuidMatch[1] : undefined;
        
        return { isExtension, uuid };
    } catch (error) {
        logger.error(`Error analyzing configuration: ${error}`);
        return null;
    }
}

/**
 * Находит основную конфигурацию в workspace
 */
export async function findMainConfiguration(): Promise<ConfigurationInfo | null> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return null;
    }
    
    for (const folder of workspaceFolders) {
        // Ищем конфигурации в корне workspace
        let configurations = await findConfigurations(folder.uri.fsPath);
        
        // Если не нашли в корне, ищем в стандартных директориях
        if (configurations.length === 0) {
            const standardDirs = ['conf', 'src', 'configuration', 'Конфигурация'];
            for (const dir of standardDirs) {
                const dirPath = path.join(folder.uri.fsPath, dir);
                if (fs.existsSync(dirPath)) {
                    configurations = await findConfigurations(dirPath);
                    if (configurations.length > 0) break;
                }
            }
        }
        
        // Фильтруем только основные конфигурации (не расширения)
        const mainConfigs = configurations.filter(c => !c.isExtension);
        
        if (mainConfigs.length > 0) {
            // Если несколько основных конфигураций, берем первую
            // В будущем можно добавить диалог выбора
            return mainConfigs[0];
        }
    }
    
    return null;
}

/**
 * Показывает диалог выбора конфигурации
 */
export async function selectConfiguration(configurations: ConfigurationInfo[]): Promise<ConfigurationInfo | null> {
    const items = configurations.map(config => ({
        label: config.name,
        description: config.isExtension ? '📦 Расширение' : '🏢 Основная конфигурация',
        detail: config.path,
        config
    }));
    
    const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Выберите конфигурацию для индексации',
        title: 'BSL Analyzer: Выбор конфигурации'
    });
    
    return selected ? selected.config : null;
}

/**
 * Автоматически определяет и устанавливает путь к конфигурации
 */
export async function autoDetectConfiguration(outputChannel?: vscode.OutputChannel): Promise<string | null> {
    outputChannel?.appendLine('🔍 Searching for 1C configuration in workspace...');
    
    const mainConfig = await findMainConfiguration();
    
    if (mainConfig) {
        outputChannel?.appendLine(`✅ Found main configuration: ${mainConfig.name} at ${mainConfig.path}`);
        
        // Сохраняем в настройках
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        await config.update('configurationPath', mainConfig.path, vscode.ConfigurationTarget.Workspace);
        
        return mainConfig.path;
    } else {
        outputChannel?.appendLine('❌ No 1C configuration found in workspace');
        
        // Предлагаем выбрать вручную
        const result = await vscode.window.showInformationMessage(
            'Конфигурация 1С не найдена автоматически',
            'Выбрать папку',
            'Пропустить'
        );
        
        if (result === 'Выбрать папку') {
            const uri = await vscode.window.showOpenDialog({
                canSelectFolders: true,
                canSelectFiles: false,
                canSelectMany: false,
                openLabel: 'Выбрать конфигурацию',
                title: 'Выберите папку с конфигурацией 1С'
            });
            
            if (uri && uri.length > 0) {
                const configPath = uri[0].fsPath;
                const config = vscode.workspace.getConfiguration('bslAnalyzer');
                await config.update('configurationPath', configPath, vscode.ConfigurationTarget.Workspace);
                return configPath;
            }
        }
    }
    
    return null;
}