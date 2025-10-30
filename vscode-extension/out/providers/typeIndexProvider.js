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
exports.BslTypeIndexProvider = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const items_1 = require("./items");
const configHelper_1 = require("../config/configHelper");
/**
 * Provider для отображения индекса типов BSL
 */
class BslTypeIndexProvider {
    constructor(outputChannel) {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        this.outputChannel = outputChannel;
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    getChildren(element) {
        if (!element) {
            return this.getRootItems();
        }
        else {
            return this.getChildItems(element);
        }
    }
    async getRootItems() {
        const items = [];
        // Получаем информацию о кешированном индексе
        const indexInfo = await this.getIndexInfo();
        if (indexInfo) {
            items.push(new items_1.BslTypeItem(`📚 Platform Types (${indexInfo.platformTypes})`, vscode.TreeItemCollapsibleState.Collapsed, 'Platform Types', 'platform'), new items_1.BslTypeItem(`🗂️ Configuration Types (${indexInfo.configTypes})`, vscode.TreeItemCollapsibleState.Collapsed, 'Configuration Types', 'configuration'), new items_1.BslTypeItem(`🔧 Global Functions (${indexInfo.globalFunctions})`, vscode.TreeItemCollapsibleState.Collapsed, 'Global Functions', 'module'));
            // Добавляем общую статистику как отдельный элемент с типом module (для совместимости)
            const statsItem = new items_1.BslTypeItem(`📊 Total: ${indexInfo.totalTypes} types`, vscode.TreeItemCollapsibleState.None, 'Statistics', 'module', 'stats');
            statsItem.iconPath = new vscode.ThemeIcon('graph');
            items.push(statsItem);
        }
        else {
            // Если индекс не найден
            const warningItem = new items_1.BslTypeItem('⚠️ Index not found', vscode.TreeItemCollapsibleState.None, 'No index', 'module', 'warning');
            warningItem.iconPath = new vscode.ThemeIcon('warning');
            items.push(warningItem);
            const buildItem = new items_1.BslTypeItem('🔨 Build Index', vscode.TreeItemCollapsibleState.None, 'Build', 'module', 'build-action');
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
    async getChildItems(element) {
        const items = [];
        // Пока возвращаем примеры, но можно будет загрузить реальные типы из кеша
        switch (element.contextValue) {
            case 'platform':
                // Читаем платформенные типы из кеша
                const platformTypes = await this.getPlatformTypes();
                return platformTypes.slice(0, 50).map(type => new items_1.BslTypeItem(type.name, vscode.TreeItemCollapsibleState.None, type.name, 'platform', 'type'));
            case 'configuration':
                // Читаем типы конфигурации из кеша проекта
                const configTypes = await this.getConfigurationTypes();
                return configTypes.slice(0, 50).map(type => new items_1.BslTypeItem(type.name, vscode.TreeItemCollapsibleState.None, type.name, 'configuration', type.kind));
            case 'module':
                // Глобальные функции
                return [
                    new items_1.BslTypeItem('Сообщить', vscode.TreeItemCollapsibleState.None, 'Сообщить', 'module', 'function'),
                    new items_1.BslTypeItem('СокрЛП', vscode.TreeItemCollapsibleState.None, 'СокрЛП', 'module', 'function'),
                    new items_1.BslTypeItem('НачалоГода', vscode.TreeItemCollapsibleState.None, 'НачалоГода', 'module', 'function'),
                    new items_1.BslTypeItem('СтрНайти', vscode.TreeItemCollapsibleState.None, 'СтрНайти', 'module', 'function'),
                    new items_1.BslTypeItem('Тип', vscode.TreeItemCollapsibleState.None, 'Тип', 'module', 'function')
                ];
            default:
                return items;
        }
    }
    async getIndexInfo() {
        try {
            // Проверяем кеш платформы
            const homedir = require('os').homedir();
            const platformVersion = configHelper_1.BslAnalyzerConfig.platformVersion;
            const platformCachePath = path.join(homedir, '.bsl_analyzer', 'platform_cache', `${platformVersion}.jsonl`);
            let platformTypes = 0;
            if (fs.existsSync(platformCachePath)) {
                const content = fs.readFileSync(platformCachePath, 'utf-8');
                platformTypes = content.trim().split('\n').length;
            }
            // Проверяем кеш проекта
            const configPath = configHelper_1.BslAnalyzerConfig.configurationPath;
            let configTypes = 0;
            if (configPath) {
                const projectId = this.tryExtractProjectId(configPath);
                if (projectId) {
                    const projectCachePath = path.join(homedir, '.bsl_analyzer', 'project_indices', projectId, platformVersion, 'config_entities.jsonl');
                    if (fs.existsSync(projectCachePath)) {
                        const content = fs.readFileSync(projectCachePath, 'utf-8');
                        configTypes = content.trim().split('\n').filter(line => line).length;
                    }
                }
                else {
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
        }
        catch (error) {
            this.outputChannel?.appendLine(`Error reading index info: ${error}`);
            return null;
        }
    }
    async getPlatformTypes() {
        try {
            const homedir = require('os').homedir();
            const platformVersion = configHelper_1.BslAnalyzerConfig.platformVersion;
            const platformCachePath = path.join(homedir, '.bsl_analyzer', 'platform_cache', `${platformVersion}.jsonl`);
            if (fs.existsSync(platformCachePath)) {
                const content = fs.readFileSync(platformCachePath, 'utf-8');
                const lines = content.trim().split('\n');
                const types = [];
                for (const line of lines) {
                    try {
                        const entity = JSON.parse(line);
                        if (entity.display_name || entity.qualified_name) {
                            types.push({ name: entity.display_name || entity.qualified_name });
                        }
                    }
                    catch (e) {
                        // Игнорируем ошибки парсинга
                    }
                }
                return types;
            }
        }
        catch (error) {
            this.outputChannel?.appendLine(`Error reading platform types: ${error}`);
        }
        return [];
    }
    async getConfigurationTypes() {
        try {
            const configPath = configHelper_1.BslAnalyzerConfig.configurationPath;
            if (!configPath)
                return [];
            const homedir = require('os').homedir();
            const platformVersion = configHelper_1.BslAnalyzerConfig.platformVersion;
            const projectId = this.tryExtractProjectId(configPath);
            if (!projectId) {
                this.outputChannel?.appendLine('Cannot list configuration types: UUID missing');
                return [];
            }
            const projectCachePath = path.join(homedir, '.bsl_analyzer', 'project_indices', projectId, platformVersion, 'config_entities.jsonl');
            if (fs.existsSync(projectCachePath)) {
                const content = fs.readFileSync(projectCachePath, 'utf-8');
                const lines = content.trim().split('\n');
                const types = [];
                for (const line of lines) {
                    try {
                        const entity = JSON.parse(line);
                        if (entity.qualified_name) {
                            types.push({
                                name: entity.qualified_name,
                                kind: entity.entity_kind || 'type'
                            });
                        }
                    }
                    catch (e) {
                        // Игнорируем ошибки парсинга
                    }
                }
                return types;
            }
        }
        catch (error) {
            this.outputChannel?.appendLine(`Error reading configuration types: ${error}`);
        }
        return [];
    }
    tryExtractProjectId(configPath) {
        try {
            const configXmlPath = path.join(configPath, 'Configuration.xml');
            if (!fs.existsSync(configXmlPath))
                return null;
            const content = fs.readFileSync(configXmlPath, 'utf-8');
            const m = content.match(/<Configuration[^>]*uuid="([^"]+)"/i);
            if (m && m[1]) {
                const uuid = m[1].replace(/-/g, '');
                return `${path.basename(configPath)}_${uuid}`;
            }
        }
        catch (e) {
            this.outputChannel?.appendLine(`Failed to extract UUID: ${e}`);
        }
        return null;
    }
}
exports.BslTypeIndexProvider = BslTypeIndexProvider;
//# sourceMappingURL=typeIndexProvider.js.map