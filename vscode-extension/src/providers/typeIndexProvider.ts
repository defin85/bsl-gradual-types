import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { BslTypeItem } from './items';
import { BslAnalyzerConfig } from '../config/configHelper';

/**
 * Provider для отображения индекса типов BSL
 */
export class BslTypeIndexProvider implements vscode.TreeDataProvider<BslTypeItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<BslTypeItem | undefined | null | void> = new vscode.EventEmitter<BslTypeItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslTypeItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private outputChannel: vscode.OutputChannel | undefined;

    constructor(outputChannel?: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: BslTypeItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: BslTypeItem): Thenable<BslTypeItem[]> {
        if (!element) {
            return this.getRootItems();
        } else {
            return this.getChildItems(element);
        }
    }

    private async getRootItems(): Promise<BslTypeItem[]> {
        const items: BslTypeItem[] = [];

        // Получаем информацию о кешированном индексе
        const indexInfo = await this.getIndexInfo();

        if (indexInfo) {
            items.push(
                new BslTypeItem(
                    `📚 Platform Types (${indexInfo.platformTypes})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'Platform Types',
                    'platform'
                ),
                new BslTypeItem(
                    `🗂️ Configuration Types (${indexInfo.configTypes})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'Configuration Types',
                    'configuration'
                ),
                new BslTypeItem(
                    `🔧 Global Functions (${indexInfo.globalFunctions})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'Global Functions',
                    'module'
                )
            );

            // Добавляем общую статистику как отдельный элемент с типом module (для совместимости)
            const statsItem = new BslTypeItem(
                `📊 Total: ${indexInfo.totalTypes} types`,
                vscode.TreeItemCollapsibleState.None,
                'Statistics',
                'module',
                'stats'
            );
            statsItem.iconPath = new vscode.ThemeIcon('graph');
            items.push(statsItem);
        } else {
            // Если индекс не найден
            const warningItem = new BslTypeItem(
                '⚠️ Index not found',
                vscode.TreeItemCollapsibleState.None,
                'No index',
                'module',
                'warning'
            );
            warningItem.iconPath = new vscode.ThemeIcon('warning');
            items.push(warningItem);

            const buildItem = new BslTypeItem(
                '🔨 Build Index',
                vscode.TreeItemCollapsibleState.None,
                'Build',
                'module',
                'build-action'
            );
            buildItem.iconPath = new vscode.ThemeIcon('tools');
            buildItem.command = {
                command: 'bslAnalyzer.buildIndex',
                title: 'Build Index',
                arguments: []
            };
            items.push(buildItem);
        }

        return items;
    }

    private async getChildItems(element: BslTypeItem): Promise<BslTypeItem[]> {
        const items: BslTypeItem[] = [];

        // Пока возвращаем примеры, но можно будет загрузить реальные типы из кеша
        switch (element.contextValue) {
            case 'platform':
                // Читаем платформенные типы из кеша
                const platformTypes = await this.getPlatformTypes();
                return platformTypes.slice(0, 50).map(type =>
                    new BslTypeItem(type.name, vscode.TreeItemCollapsibleState.None, type.name, 'platform', 'type')
                );

            case 'configuration':
                // Читаем типы конфигурации из кеша проекта
                const configTypes = await this.getConfigurationTypes();
                return configTypes.slice(0, 50).map(type =>
                    new BslTypeItem(type.name, vscode.TreeItemCollapsibleState.None, type.name, 'configuration', type.kind)
                );

            case 'module':
                // Глобальные функции
                return [
                    new BslTypeItem('Сообщить', vscode.TreeItemCollapsibleState.None, 'Сообщить', 'module', 'function'),
                    new BslTypeItem('СокрЛП', vscode.TreeItemCollapsibleState.None, 'СокрЛП', 'module', 'function'),
                    new BslTypeItem('НачалоГода', vscode.TreeItemCollapsibleState.None, 'НачалоГода', 'module', 'function'),
                    new BslTypeItem('СтрНайти', vscode.TreeItemCollapsibleState.None, 'СтрНайти', 'module', 'function'),
                    new BslTypeItem('Тип', vscode.TreeItemCollapsibleState.None, 'Тип', 'module', 'function')
                ];

            default:
                return items;
        }
    }

    private async getIndexInfo(): Promise<{
        platformTypes: number;
        configTypes: number;
        globalFunctions: number;
        totalTypes: number;
    } | null> {
        try {
            // Проверяем кеш платформы
            const homedir = require('os').homedir();
            const platformVersion = BslAnalyzerConfig.platformVersion;
            const platformCachePath = path.join(homedir, '.bsl_analyzer', 'platform_cache', `${platformVersion}.jsonl`);

            let platformTypes = 0;
            if (fs.existsSync(platformCachePath)) {
                const content = fs.readFileSync(platformCachePath, 'utf-8');
                platformTypes = content.trim().split('\n').length;
            }

            // Проверяем кеш проекта
            const configPath = BslAnalyzerConfig.configurationPath;
            let configTypes = 0;

            if (configPath) {
                const projectId = this.tryExtractProjectId(configPath);
                if (projectId) {
                    const projectCachePath = path.join(
                        homedir,
                        '.bsl_analyzer',
                        'project_indices',
                        projectId,
                        platformVersion,
                        'config_entities.jsonl'
                    );
                    if (fs.existsSync(projectCachePath)) {
                        const content = fs.readFileSync(projectCachePath, 'utf-8');
                        configTypes = content.trim().split('\n').filter(line => line).length;
                    }
                } else {
                    this.outputChannel?.appendLine('UUID not found in Configuration.xml – configuration types cache path unresolved');
                }
            }

            // Глобальные функции (примерное количество)
            const globalFunctions = 150; // Примерно столько глобальных функций в 1С

            return {
                platformTypes,
                configTypes,
                globalFunctions,
                totalTypes: platformTypes + configTypes + globalFunctions
            };
        } catch (error) {
            this.outputChannel?.appendLine(`Error reading index info: ${error}`);
            return null;
        }
    }

    private async getPlatformTypes(): Promise<{ name: string }[]> {
        try {
            const homedir = require('os').homedir();
            const platformVersion = BslAnalyzerConfig.platformVersion;
            const platformCachePath = path.join(homedir, '.bsl_analyzer', 'platform_cache', `${platformVersion}.jsonl`);

            if (fs.existsSync(platformCachePath)) {
                const content = fs.readFileSync(platformCachePath, 'utf-8');
                const lines = content.trim().split('\n');
                const types: { name: string }[] = [];

                for (const line of lines) {
                    try {
                        const entity = JSON.parse(line);
                        if (entity.display_name || entity.qualified_name) {
                            types.push({ name: entity.display_name || entity.qualified_name });
                        }
                    } catch (e) {
                        // Игнорируем ошибки парсинга
                    }
                }

                return types;
            }
        } catch (error) {
            this.outputChannel?.appendLine(`Error reading platform types: ${error}`);
        }
        return [];
    }

    private async getConfigurationTypes(): Promise<{ name: string; kind: string }[]> {
        try {
            const configPath = BslAnalyzerConfig.configurationPath;
            if (!configPath) return [];

            const homedir = require('os').homedir();
            const platformVersion = BslAnalyzerConfig.platformVersion;
            const projectId = this.tryExtractProjectId(configPath);
            if (!projectId) {
                this.outputChannel?.appendLine('Cannot list configuration types: UUID missing');
                return [];
            }
            const projectCachePath = path.join(
                homedir,
                '.bsl_analyzer',
                'project_indices',
                projectId,
                platformVersion,
                'config_entities.jsonl'
            );

            if (fs.existsSync(projectCachePath)) {
                const content = fs.readFileSync(projectCachePath, 'utf-8');
                const lines = content.trim().split('\n');
                const types: { name: string; kind: string }[] = [];

                for (const line of lines) {
                    try {
                        const entity = JSON.parse(line);
                        if (entity.qualified_name) {
                            types.push({
                                name: entity.qualified_name,
                                kind: entity.entity_kind || 'type'
                            });
                        }
                    } catch (e) {
                        // Игнорируем ошибки парсинга
                    }
                }

                return types;
            }
        } catch (error) {
            this.outputChannel?.appendLine(`Error reading configuration types: ${error}`);
        }
        return [];
    }
    private tryExtractProjectId(configPath: string): string | null {
        try {
            const configXmlPath = path.join(configPath, 'Configuration.xml');
            if (!fs.existsSync(configXmlPath)) return null;
            const content = fs.readFileSync(configXmlPath, 'utf-8');
            const m = content.match(/<Configuration[^>]*uuid="([^"]+)"/i);
            if (m && m[1]) {
                const uuid = m[1].replace(/-/g, '');
                return `${path.basename(configPath)}_${uuid}`;
            }
        } catch (e) {
            this.outputChannel?.appendLine(`Failed to extract UUID: ${e}`);
        }
        return null;
    }
}