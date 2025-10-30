import * as vscode from 'vscode';
/**
 * Элемент дерева для обзора BSL
 */
export declare class BslOverviewItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, contextValue?: string);
}
/**
 * Элемент дерева для диагностики
 */
export declare class BslDiagnosticItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    readonly severity?: vscode.DiagnosticSeverity | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, contextValue?: string, severity?: vscode.DiagnosticSeverity | undefined);
}
/**
 * Элемент дерева для типов BSL
 */
export declare class BslTypeItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    readonly typeName: string;
    readonly typeKind: 'platform' | 'configuration' | 'module';
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, typeName: string, typeKind: 'platform' | 'configuration' | 'module', contextValue?: string);
}
/**
 * Элемент дерева для документации платформы
 */
export declare class PlatformDocItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    readonly docPath?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, contextValue?: string, docPath?: string | undefined);
}
//# sourceMappingURL=items.d.ts.map