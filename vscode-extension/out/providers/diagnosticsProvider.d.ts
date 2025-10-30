import * as vscode from 'vscode';
import { BslDiagnosticItem } from './items';
/**
 * Провайдер для дерева диагностики BSL
 */
export declare class BslDiagnosticsProvider implements vscode.TreeDataProvider<BslDiagnosticItem> {
    private _onDidChangeTreeData;
    readonly onDidChangeTreeData: vscode.Event<BslDiagnosticItem | undefined | null | void>;
    private diagnostics;
    refresh(): void;
    updateDiagnostics(diagnostics: vscode.Diagnostic[]): void;
    getTreeItem(element: BslDiagnosticItem): vscode.TreeItem;
    getChildren(element?: BslDiagnosticItem): Thenable<BslDiagnosticItem[]>;
}
//# sourceMappingURL=diagnosticsProvider.d.ts.map