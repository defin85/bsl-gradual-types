import * as vscode from 'vscode';
import { HierarchicalTypeItem } from './typeModels';
import { TypeTreeBuilder } from './typeTreeBuilder';
import { progressEmitter } from '../lsp/progress';
import { getSidebarSnapshot } from './sidebarSnapshot';

// Re-export for backward compatibility
export { HierarchicalTypeItem } from './typeModels';

/**
 * Иерархический провайдер для отображения типов BSL с группировкой по категориям
 */
export class HierarchicalTypeIndexProvider implements vscode.TreeDataProvider<HierarchicalTypeItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<HierarchicalTypeItem | undefined | null | void> =
        new vscode.EventEmitter<HierarchicalTypeItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<HierarchicalTypeItem | undefined | null | void> =
        this._onDidChangeTreeData.event;

    private outputChannel: vscode.OutputChannel | undefined;
    private treeBuilder: TypeTreeBuilder;
    private wasIndexing = false;

    constructor(outputChannel?: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
        this.treeBuilder = new TypeTreeBuilder(outputChannel);
        // Запускаем загрузку типов асинхронно
        this.initializeTypes();

        // Обновляем дерево после завершения индексации типов
        progressEmitter.event((progress) => {
            const isIndexing = progress.isIndexing;
            if (this.wasIndexing && !isIndexing) {
                this.refresh();
            }
            this.wasIndexing = isIndexing;
        });
    }

    private async initializeTypes(): Promise<void> {
        await this.treeBuilder.loadTypes();
        this._onDidChangeTreeData.fire();
    }

    refresh(): void {
        // Асинхронная перезагрузка типов
        this.treeBuilder.loadTypes().then(() => {
            this._onDidChangeTreeData.fire();
        });
    }

    getTreeItem(element: HierarchicalTypeItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: HierarchicalTypeItem): Thenable<HierarchicalTypeItem[]> {
        if (!element) {
            this.outputChannel?.appendLine('HierarchicalTypeIndexProvider: Getting root categories');
            return this.getRootCategories();
        } else if (element.contextValue === 'platform-group') {
            return Promise.resolve(this.treeBuilder.getPlatformCategories());
        } else if (element.contextValue === 'config-group') {
            return Promise.resolve(this.treeBuilder.getConfigCategories());
        } else if (element.contextValue === 'category') {
            return this.getCategoryTypes(element);
        } else if (element.contextValue === 'type') {
            return this.getTypeMembers(element);
        } else if (element.contextValue === 'methods-folder') {
            return this.getTypeMethods(element);
        } else if (element.contextValue === 'properties-folder') {
            return this.getTypeProperties(element);
        }
        return Promise.resolve([]);
    }

    private async getRootCategories(): Promise<HierarchicalTypeItem[]> {
        this.outputChannel?.appendLine(
            `HierarchicalTypeIndexProvider: Building categories, found ${this.treeBuilder.categoriesCount} categories`
        );
        const items: HierarchicalTypeItem[] = [];
        const snapshot = await getSidebarSnapshot();
        const platformTypesCount = snapshot.typeRepository.status === 'live'
            ? snapshot.typeRepository.platformTypes
            : this.treeBuilder.platformTypesCount;
        const configTypesCount = snapshot.typeRepository.status === 'live'
            ? snapshot.typeRepository.configurationTypes
            : this.treeBuilder.configTypesCount;

        // Platform types group
        if (platformTypesCount > 0) {
            const platformGroup = new HierarchicalTypeItem(
                `🏗️ Platform 1C (${platformTypesCount})`,
                vscode.TreeItemCollapsibleState.Collapsed,
                'Platform types from syntax helper',
                'platform-group'
            );
            items.push(platformGroup);
        }

        // Configuration types group
        if (configTypesCount > 0) {
            const configGroup = new HierarchicalTypeItem(
                `📁 Configuration (${configTypesCount})`,
                vscode.TreeItemCollapsibleState.Collapsed,
                'Types from configuration metadata',
                'config-group'
            );
            items.push(configGroup);
        }

        // Если типов нет - показываем информационный узел
        if (items.length === 0) {
            const infoItem = new HierarchicalTypeItem(
                '⏳ Loading types...',
                vscode.TreeItemCollapsibleState.None,
                'Types are being loaded from LSP server',
                'loading'
            );
            infoItem.tooltip = 'Types will appear after LSP server starts.\nMake sure LSP server is running.';
            items.push(infoItem);
        }

        return items;
    }

    private async getCategoryTypes(element: HierarchicalTypeItem): Promise<HierarchicalTypeItem[]> {
        const categoryKey = element.itemData;
        if (!categoryKey) return [];

        return this.treeBuilder.getCategoryTypes(categoryKey);
    }

    private async getTypeMembers(element: HierarchicalTypeItem): Promise<HierarchicalTypeItem[]> {
        const typeName = element.itemData;
        if (!typeName) return [];

        return this.treeBuilder.getTypeMembers(typeName);
    }

    private async getTypeMethods(element: HierarchicalTypeItem): Promise<HierarchicalTypeItem[]> {
        const typeName = element.itemData;
        if (!typeName) return [];

        return this.treeBuilder.getTypeMethods(typeName);
    }

    private async getTypeProperties(element: HierarchicalTypeItem): Promise<HierarchicalTypeItem[]> {
        const typeName = element.itemData;
        if (!typeName) return [];

        return this.treeBuilder.getTypeProperties(typeName);
    }
}
