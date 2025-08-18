import * as vscode from 'vscode';
import { BslDiagnosticItem } from './items';

/**
 * Провайдер для дерева диагностики BSL
 */
export class BslDiagnosticsProvider implements vscode.TreeDataProvider<BslDiagnosticItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<BslDiagnosticItem | undefined | null | void> = new vscode.EventEmitter<BslDiagnosticItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslDiagnosticItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private diagnostics: vscode.Diagnostic[] = [];

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    updateDiagnostics(diagnostics: vscode.Diagnostic[]) {
        this.diagnostics = diagnostics;
        this.refresh();
    }

    getTreeItem(element: BslDiagnosticItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: BslDiagnosticItem): Thenable<BslDiagnosticItem[]> {
        if (!element) {
            // Root items - группировка по severity
            const errors = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
            const warnings = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Warning);
            const infos = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Information);
            const hints = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Hint);

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

            return Promise.resolve(items);
        } else {
            // Child items - конкретные диагностики
            let relevantDiagnostics: vscode.Diagnostic[] = [];
            
            switch (element.contextValue) {
                case 'errors':
                    relevantDiagnostics = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
                    break;
                case 'warnings':
                    relevantDiagnostics = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Warning);
                    break;
                case 'infos':
                    relevantDiagnostics = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Information);
                    break;
                case 'hints':
                    relevantDiagnostics = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Hint);
                    break;
            }

            const items = relevantDiagnostics.map(d => {
                const item = new BslDiagnosticItem(
                    d.message,
                    vscode.TreeItemCollapsibleState.None,
                    'diagnostic',
                    d.severity
                );
                
                // Добавляем информацию о позиции
                if (d.range) {
                    item.description = `Line ${d.range.start.line + 1}`;
                }
                
                // Добавляем команду для перехода к проблеме
                if (d.source) {
                    item.command = {
                        command: 'bslAnalyzer.goToDiagnostic',
                        title: 'Go to Issue',
                        arguments: [d]
                    };
                }
                
                return item;
            });

            return Promise.resolve(items);
        }
    }
}