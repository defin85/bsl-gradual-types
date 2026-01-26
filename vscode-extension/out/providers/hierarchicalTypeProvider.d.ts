import * as vscode from 'vscode';
import { HierarchicalTypeItem } from './typeModels';
export { HierarchicalTypeItem } from './typeModels';
/**
 * Иерархический провайдер для отображения типов BSL с группировкой по категориям
 */
export declare class HierarchicalTypeIndexProvider implements vscode.TreeDataProvider<HierarchicalTypeItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<HierarchicalTypeItem | undefined | null | void>;
    private outputChannel;
    private treeBuilder;
    private wasIndexing;
    constructor(outputChannel?: vscode.OutputChannel);
    private initializeTypes;
    refresh(): void;
    getTreeItem(element: HierarchicalTypeItem): vscode.TreeItem;
    getChildren(element?: HierarchicalTypeItem): Thenable<HierarchicalTypeItem[]>;
    private getRootCategories;
    private getCategoryTypes;
    private getTypeMembers;
    private getTypeMethods;
    private getTypeProperties;
}
//# sourceMappingURL=hierarchicalTypeProvider.d.ts.map