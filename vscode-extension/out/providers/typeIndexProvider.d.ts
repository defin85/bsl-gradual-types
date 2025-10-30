import * as vscode from 'vscode';
import { BslTypeItem } from './items';
/**
 * Provider для отображения индекса типов BSL
 */
export declare class BslTypeIndexProvider implements vscode.TreeDataProvider<BslTypeItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<BslTypeItem | undefined | null | void>;
    private outputChannel;
    constructor(outputChannel?: vscode.OutputChannel);
    refresh(): void;
    getTreeItem(element: BslTypeItem): vscode.TreeItem;
    getChildren(element?: BslTypeItem): Thenable<BslTypeItem[]>;
    private getRootItems;
    private getChildItems;
    private getIndexInfo;
    private getPlatformTypes;
    private getConfigurationTypes;
    private tryExtractProjectId;
}
//# sourceMappingURL=typeIndexProvider.d.ts.map