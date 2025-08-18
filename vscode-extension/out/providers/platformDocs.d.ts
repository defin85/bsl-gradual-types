import * as vscode from 'vscode';
/**
 * Элемент дерева для документации платформы с расширенными свойствами
 */
export declare class PlatformDocItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    readonly version: string;
    readonly typesCount?: string | undefined;
    readonly archiveName?: string | undefined;
    readonly lastParsed?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, version: string, contextValue?: string, typesCount?: string | undefined, archiveName?: string | undefined, lastParsed?: string | undefined);
}
/**
 * Provider для отображения документации платформы
 */
export declare class BslPlatformDocsProvider implements vscode.TreeDataProvider<PlatformDocItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<PlatformDocItem | undefined | null | void>;
    private outputChannel;
    constructor(outputChannel?: vscode.OutputChannel);
    refresh(): void;
    getTreeItem(element: PlatformDocItem): vscode.TreeItem;
    getChildren(element?: PlatformDocItem): Thenable<PlatformDocItem[]>;
    private getAvailablePlatformVersions;
}
//# sourceMappingURL=platformDocs.d.ts.map