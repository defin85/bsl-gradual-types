import * as vscode from 'vscode';
/**
 * Элемент иерархического дерева типов
 */
export declare class HierarchicalTypeItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    readonly typeName: string;
    readonly typeContext: string;
    readonly itemData?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, typeName: string, typeContext: string, itemData?: string | undefined);
}
/**
 * Иерархический провайдер для отображения типов BSL с группировкой по категориям
 */
export declare class HierarchicalTypeIndexProvider implements vscode.TreeDataProvider<HierarchicalTypeItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<HierarchicalTypeItem | undefined | null | void>;
    private outputChannel;
    private platformTypes;
    private configTypes;
    private typeCategories;
    constructor(outputChannel?: vscode.OutputChannel);
    refresh(): void;
    getTreeItem(element: HierarchicalTypeItem): vscode.TreeItem;
    getChildren(element?: HierarchicalTypeItem): Thenable<HierarchicalTypeItem[]>;
    private loadTypes;
    private loadPlatformTypes;
    private loadConfigurationTypes;
    private extractUuidProjectId;
    private categorizeTypes;
    private matchesCategory;
    private getConfigCategory;
    private getCategoryIcon;
    private getPlatformCategories;
    private getConfigCategories;
    private getRootCategories;
    private getCategoryTypes;
    private hasMembers;
    private getTypeMembers;
    private getTypeMethods;
    private getTypeProperties;
    private formatMethodTooltip;
    private formatPropertyTooltip;
}
//# sourceMappingURL=hierarchicalTypeProvider.d.ts.map