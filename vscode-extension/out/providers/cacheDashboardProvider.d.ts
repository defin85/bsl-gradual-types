import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
export declare class CacheDashboardProvider implements vscode.TreeDataProvider<BslOverviewItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<BslOverviewItem | undefined | null | void>;
    private outputChannel;
    private cachedStats;
    private lastFetchAt;
    private inflight;
    private readonly ttlMs;
    private disposables;
    constructor(outputChannel: vscode.OutputChannel);
    refresh(): void;
    getTreeItem(element: BslOverviewItem): vscode.TreeItem;
    getChildren(element?: BslOverviewItem): Thenable<BslOverviewItem[]>;
    private getStatusItems;
    private getMetricsItems;
    private getTimingItems;
    private getSizeItems;
    private getActionItems;
    private loadStats;
    private missingConfigItems;
    dispose(): void;
}
//# sourceMappingURL=cacheDashboardProvider.d.ts.map