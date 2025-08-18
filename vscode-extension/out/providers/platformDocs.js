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
exports.BslPlatformDocsProvider = exports.PlatformDocItem = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
/**
 * Элемент дерева для документации платформы с расширенными свойствами
 */
class PlatformDocItem extends vscode.TreeItem {
    constructor(label, collapsibleState, version, contextValue, typesCount, archiveName, lastParsed) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.version = version;
        this.typesCount = typesCount;
        this.archiveName = archiveName;
        this.lastParsed = lastParsed;
        if (contextValue) {
            this.contextValue = contextValue;
        }
        if (version && contextValue === 'version') {
            this.tooltip = `Platform ${version}: ${typesCount || '?'} types`;
        }
    }
}
exports.PlatformDocItem = PlatformDocItem;
/**
 * Provider для отображения документации платформы
 */
class BslPlatformDocsProvider {
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
            // Показываем доступные версии платформы из кеша
            return this.getAvailablePlatformVersions();
        }
        else {
            // Показываем детали для конкретной версии
            const details = [];
            // Показываем количество типов
            details.push(new PlatformDocItem(`📊 Types: ${element.typesCount || 'Unknown'}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            // Показываем информацию об архивах
            if (element.archiveName === 'Both archives') {
                details.push(new PlatformDocItem(`✅ Status: Complete (shcntx + shlang)`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`📂 Archive: shcntx_ru.zip`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`📂 Archive: shlang_ru.zip`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            }
            else if (element.archiveName && element.archiveName.includes('shcntx')) {
                details.push(new PlatformDocItem(`📂 Archive: ${element.archiveName}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`⚠️ Missing: shlang_ru.zip (primitive types)`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            }
            else if (element.archiveName && element.archiveName.includes('shlang')) {
                details.push(new PlatformDocItem(`📂 Archive: ${element.archiveName}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`⚠️ Missing: shcntx_ru.zip (object types)`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            }
            else {
                details.push(new PlatformDocItem(`📦 Archive: ${element.archiveName || 'Unknown'}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            }
            // Показываем дату добавления
            details.push(new PlatformDocItem(`🕒 Added: ${element.lastParsed || 'Unknown'}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            // Добавляем кнопку удаления
            const removeItem = new PlatformDocItem(`🗑️ Remove this version`, vscode.TreeItemCollapsibleState.None, element.version, 'remove-version');
            removeItem.command = {
                command: 'bslAnalyzer.removePlatformDocs',
                title: 'Remove Platform Documentation',
                arguments: [element]
            };
            details.push(removeItem);
            return Promise.resolve(details);
        }
    }
    async getAvailablePlatformVersions() {
        const items = [];
        // Проверяем наличие кеша платформенной документации
        const homedir = require('os').homedir();
        const cacheDir = path.join(homedir, '.bsl_analyzer', 'platform_cache');
        if (fs.existsSync(cacheDir)) {
            // Читаем список версий из кеша
            const files = fs.readdirSync(cacheDir);
            // Поддерживаем оба формата: с префиксом "v" и без него
            const versionFiles = files.filter(f => f.match(/^v?[\d.]+\.jsonl$/));
            for (const versionFile of versionFiles) {
                const version = versionFile.replace(/^v/, '').replace('.jsonl', '');
                // Пытаемся прочитать количество типов из файла
                let typesCount = '?';
                let archiveInfo = 'Unknown';
                try {
                    const filePath = path.join(cacheDir, versionFile);
                    const content = fs.readFileSync(filePath, 'utf-8');
                    const lines = content.trim().split('\n');
                    typesCount = lines.length.toLocaleString();
                    // Анализируем содержимое для определения типа архивов
                    let hasObjectTypes = false;
                    let hasPrimitiveTypes = false;
                    for (const line of lines.slice(0, 100)) { // Проверяем первые 100 строк
                        try {
                            const entity = JSON.parse(line);
                            if (entity.name) {
                                // Проверка на объектные типы (из shcntx)
                                if (entity.name.includes('Массив') || entity.name.includes('Array') ||
                                    entity.name.includes('ТаблицаЗначений') || entity.name.includes('ValueTable')) {
                                    hasObjectTypes = true;
                                }
                                // Проверка на примитивные типы (из shlang)
                                if (entity.name === 'Число' || entity.name === 'Number' ||
                                    entity.name === 'Строка' || entity.name === 'String' ||
                                    entity.name === 'Булево' || entity.name === 'Boolean') {
                                    hasPrimitiveTypes = true;
                                }
                            }
                        }
                        catch (e) {
                            // Игнорируем ошибки парсинга
                        }
                    }
                    if (hasObjectTypes && hasPrimitiveTypes) {
                        archiveInfo = 'Both archives';
                    }
                    else if (hasObjectTypes) {
                        archiveInfo = 'shcntx_ru.zip';
                    }
                    else if (hasPrimitiveTypes) {
                        archiveInfo = 'shlang_ru.zip';
                    }
                }
                catch (e) {
                    this.outputChannel?.appendLine(`Error reading platform cache: ${e}`);
                }
                const lastModified = fs.statSync(path.join(cacheDir, versionFile)).mtime.toLocaleDateString();
                // Логируем найденную версию
                this.outputChannel?.appendLine(`Found platform docs: v${version} - ${typesCount} types, archive: ${archiveInfo}`);
                items.push(new PlatformDocItem(`📋 Platform ${version} (${typesCount} types)`, vscode.TreeItemCollapsibleState.Expanded, version, 'version', typesCount, archiveInfo, lastModified));
            }
        }
        // Всегда добавляем кнопку для добавления документации
        const addDocsItem = new PlatformDocItem('➕ Add Platform Documentation...', vscode.TreeItemCollapsibleState.None, '', 'add-docs');
        addDocsItem.command = {
            command: 'bslAnalyzer.addPlatformDocs',
            title: 'Add Platform Documentation',
            arguments: []
        };
        items.push(addDocsItem);
        return items;
    }
}
exports.BslPlatformDocsProvider = BslPlatformDocsProvider;
//# sourceMappingURL=platformDocs.js.map