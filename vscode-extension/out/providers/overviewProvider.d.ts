import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
/**
 * Провайдер для дерева обзора BSL Analyzer
 */
export declare class BslOverviewProvider implements vscode.TreeDataProvider<BslOverviewItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<BslOverviewItem | undefined | null | void>;
    private outputChannel;
    constructor(outputChannel: vscode.OutputChannel);
    refresh(): void;
    getTreeItem(element: BslOverviewItem): vscode.TreeItem;
    getChildren(element?: BslOverviewItem): Thenable<BslOverviewItem[]>;
    private getWorkspaceItems;
    private getServerItems;
    private getConfigItems;
}
//# sourceMappingURL=overviewProvider.d.ts.map