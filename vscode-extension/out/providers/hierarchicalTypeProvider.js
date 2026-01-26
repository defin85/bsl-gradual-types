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
exports.HierarchicalTypeIndexProvider = exports.HierarchicalTypeItem = void 0;
const vscode = __importStar(require("vscode"));
const typeModels_1 = require("./typeModels");
const typeTreeBuilder_1 = require("./typeTreeBuilder");
const progress_1 = require("../lsp/progress");
// Re-export for backward compatibility
var typeModels_2 = require("./typeModels");
Object.defineProperty(exports, "HierarchicalTypeItem", { enumerable: true, get: function () { return typeModels_2.HierarchicalTypeItem; } });
/**
 * Иерархический провайдер для отображения типов BSL с группировкой по категориям
 */
class HierarchicalTypeIndexProvider {
    constructor(outputChannel) {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        this.wasIndexing = false;
        this.outputChannel = outputChannel;
        this.treeBuilder = new typeTreeBuilder_1.TypeTreeBuilder(outputChannel);
        // Запускаем загрузку типов асинхронно
        this.initializeTypes();
        // Обновляем дерево после завершения индексации типов
        progress_1.progressEmitter.event((progress) => {
            const isIndexing = progress.isIndexing;
            if (this.wasIndexing && !isIndexing) {
                this.refresh();
            }
            this.wasIndexing = isIndexing;
        });
    }
    async initializeTypes() {
        await this.treeBuilder.loadTypes();
        this._onDidChangeTreeData.fire();
    }
    refresh() {
        // Асинхронная перезагрузка типов
        this.treeBuilder.loadTypes().then(() => {
            this._onDidChangeTreeData.fire();
        });
    }
    getTreeItem(element) {
        return element;
    }
    getChildren(element) {
        if (!element) {
            this.outputChannel?.appendLine('HierarchicalTypeIndexProvider: Getting root categories');
            return this.getRootCategories();
        }
        else if (element.contextValue === 'platform-group') {
            return Promise.resolve(this.treeBuilder.getPlatformCategories());
        }
        else if (element.contextValue === 'config-group') {
            return Promise.resolve(this.treeBuilder.getConfigCategories());
        }
        else if (element.contextValue === 'category') {
            return this.getCategoryTypes(element);
        }
        else if (element.contextValue === 'type') {
            return this.getTypeMembers(element);
        }
        else if (element.contextValue === 'methods-folder') {
            return this.getTypeMethods(element);
        }
        else if (element.contextValue === 'properties-folder') {
            return this.getTypeProperties(element);
        }
        return Promise.resolve([]);
    }
    async getRootCategories() {
        this.outputChannel?.appendLine(`HierarchicalTypeIndexProvider: Building categories, found ${this.treeBuilder.categoriesCount} categories`);
        const items = [];
        // Platform types group
        if (this.treeBuilder.platformTypesCount > 0) {
            const platformGroup = new typeModels_1.HierarchicalTypeItem(`🏗️ Platform 1C (${this.treeBuilder.platformTypesCount})`, vscode.TreeItemCollapsibleState.Collapsed, 'Platform types from syntax helper', 'platform-group');
            items.push(platformGroup);
        }
        // Configuration types group
        if (this.treeBuilder.configTypesCount > 0) {
            const configGroup = new typeModels_1.HierarchicalTypeItem(`📁 Configuration (${this.treeBuilder.configTypesCount})`, vscode.TreeItemCollapsibleState.Collapsed, 'Types from configuration metadata', 'config-group');
            items.push(configGroup);
        }
        // Если типов нет - показываем информационный узел
        if (items.length === 0) {
            const infoItem = new typeModels_1.HierarchicalTypeItem('⏳ Loading types...', vscode.TreeItemCollapsibleState.None, 'Types are being loaded from LSP server', 'loading');
            infoItem.tooltip = 'Types will appear after LSP server starts.\nMake sure LSP server is running.';
            items.push(infoItem);
        }
        return items;
    }
    async getCategoryTypes(element) {
        const categoryKey = element.itemData;
        if (!categoryKey)
            return [];
        return this.treeBuilder.getCategoryTypes(categoryKey);
    }
    async getTypeMembers(element) {
        const typeName = element.itemData;
        if (!typeName)
            return [];
        return this.treeBuilder.getTypeMembers(typeName);
    }
    async getTypeMethods(element) {
        const typeName = element.itemData;
        if (!typeName)
            return [];
        return this.treeBuilder.getTypeMethods(typeName);
    }
    async getTypeProperties(element) {
        const typeName = element.itemData;
        if (!typeName)
            return [];
        return this.treeBuilder.getTypeProperties(typeName);
    }
}
exports.HierarchicalTypeIndexProvider = HierarchicalTypeIndexProvider;
//# sourceMappingURL=hierarchicalTypeProvider.js.map