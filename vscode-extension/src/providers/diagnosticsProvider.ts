import * as vscode from 'vscode';
import { BslDiagnosticItem } from './items';
import { getSidebarSnapshot, invalidateSidebarSnapshot, SidebarDiagnosticEntry } from './sidebarSnapshot';

/**
 * Провайдер для дерева диагностики BSL
 */
export class BslDiagnosticsProvider implements vscode.TreeDataProvider<BslDiagnosticItem>, vscode.Disposable {
    private _onDidChangeTreeData: vscode.EventEmitter<BslDiagnosticItem | undefined | null | void> = new vscode.EventEmitter<BslDiagnosticItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslDiagnosticItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private readonly disposables: vscode.Disposable[] = [];

    constructor() {
        this.disposables.push(vscode.languages.onDidChangeDiagnostics(() => {
            invalidateSidebarSnapshot();
            this.refresh();
        }));
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    updateDiagnostics(_diagnostics: vscode.Diagnostic[]) {
        // Kept for backward compatibility with older call sites.
        this.refresh();
    }

    getTreeItem(element: BslDiagnosticItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: BslDiagnosticItem): Thenable<BslDiagnosticItem[]> {
        return this.getChildrenFromSnapshot(element);
    }

    private async getChildrenFromSnapshot(element?: BslDiagnosticItem): Promise<BslDiagnosticItem[]> {
        const snapshot = await getSidebarSnapshot();
        const diagnostics = snapshot.diagnosticsSnapshot.entries;

        if (!element) {
            // Root items - группировка по severity
            const errors = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Error);
            const warnings = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Warning);
            const infos = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Information);
            const hints = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Hint);

            const items: BslDiagnosticItem[] = [];
            
            if (errors.length > 0) {
                items.push(new BslDiagnosticItem(
                    `Errors (${errors.length})`,
                    vscode.TreeItemCollapsibleState.Expanded,
                    'errors',
                    vscode.DiagnosticSeverity.Error
                ));
            }
            
            if (warnings.length > 0) {
                items.push(new BslDiagnosticItem(
                    `Warnings (${warnings.length})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'warnings',
                    vscode.DiagnosticSeverity.Warning
                ));
            }
            
            if (infos.length > 0) {
                items.push(new BslDiagnosticItem(
                    `Information (${infos.length})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'infos',
                    vscode.DiagnosticSeverity.Information
                ));
            }
            
            if (hints.length > 0) {
                items.push(new BslDiagnosticItem(
                    `Hints (${hints.length})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'hints',
                    vscode.DiagnosticSeverity.Hint
                ));
            }

            if (items.length === 0) {
                items.push(new BslDiagnosticItem(
                    'No issues found',
                    vscode.TreeItemCollapsibleState.None,
                    'no-issues'
                ));
            }

            return items;
        } else {
            // Child items - конкретные диагностики
            let relevantDiagnostics: SidebarDiagnosticEntry[] = [];
            
            switch (element.contextValue) {
                case 'errors':
                    relevantDiagnostics = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Error);
                    break;
                case 'warnings':
                    relevantDiagnostics = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Warning);
                    break;
                case 'infos':
                    relevantDiagnostics = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Information);
                    break;
                case 'hints':
                    relevantDiagnostics = diagnostics.filter(d => d.diagnostic.severity === vscode.DiagnosticSeverity.Hint);
                    break;
            }

            return relevantDiagnostics.map(({ uri, diagnostic }) => {
                const item = new BslDiagnosticItem(
                    diagnostic.message,
                    vscode.TreeItemCollapsibleState.None,
                    'diagnostic',
                    diagnostic.severity
                );
                
                // Добавляем информацию о позиции
                if (diagnostic.range) {
                    item.description = `${uri.fsPath.split(/[\\\\/]/).pop() ?? uri.fsPath}:${diagnostic.range.start.line + 1}`;
                }
                
                // Добавляем команду для перехода к проблеме
                if (diagnostic.range) {
                    item.command = {
                        command: 'bslAnalyzer.goToDiagnostic',
                        title: 'Go to Issue',
                        arguments: [uri, diagnostic]
                    };
                }
                
                return item;
            });
        }
    }

    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables.length = 0;
    }
}
