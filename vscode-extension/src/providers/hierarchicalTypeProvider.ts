import * as vscode from 'vscode';
import { HierarchicalTypeItem } from './typeModels';
import { TypeTreeBuilder } from './typeTreeBuilder';

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

    constructor(outputChannel?: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
        this.treeBuilder = new TypeTreeBuilder(outputChannel);
        this.treeBuilder.loadTypes();
    }

    refresh(): void {
        this.treeBuilder.loadTypes();
        this._onDidChangeTreeData.fire();
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

        // TODO Milestone 2.10: Показывать типы из LSP через Custom Request
        // ВРЕМЕННО: показываем заглушку вместо Type Index
        const stubItem = new HierarchicalTypeItem(
            'Type Index',
            vscode.TreeItemCollapsibleState.None,
            'type-index-disabled',
            'empty'
        );
        stubItem.tooltip =
            'Type Index (Milestone 2.9).\n\n' +
            'LSP hover.\n\n' +
            'Milestone 2.10 Type Index LSP Server\n' +
            'Custom Request (bsl/getAllTypes) JSONL.\n\n' +
            '.';
        items.push(stubItem);

        // СТАРЫЙ КОД (закомментирован) - будет восстановлен в Milestone 2.10
        // const configPath = BslAnalyzerConfig.configurationPath;
        // const platformDocs = BslAnalyzerConfig.platformDocsArchive;
        //
        // // Platform types group
        // if (this.treeBuilder.platformTypesCount > 0) {
        //     const platformGroup = new HierarchicalTypeItem(
        //         `Platform 1C (${this.treeBuilder.platformTypesCount})`,
        //         vscode.TreeItemCollapsibleState.Collapsed,
        //         '',
        //         'platform-group'
        //     );
        //     items.push(platformGroup);
        // }
        //
        // // Configuration types group
        // if (this.treeBuilder.configTypesCount > 0) {
        //     const configGroup = new HierarchicalTypeItem(
        //         `Configuration (${this.treeBuilder.configTypesCount})`,
        //         vscode.TreeItemCollapsibleState.Collapsed,
        //         '',
        //         'config-group'
        //     );
        //     items.push(configGroup);
        // }

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
