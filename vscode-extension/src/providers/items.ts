import * as vscode from 'vscode';

/**
 * Элемент дерева для обзора BSL
 */
export class BslOverviewItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        contextValue?: string
    ) {
        super(label, collapsibleState);
        if (contextValue) {
            this.contextValue = contextValue;
        }
    }
}

/**
 * Элемент дерева для диагностики
 */
export class BslDiagnosticItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        contextValue?: string,
        public readonly severity?: vscode.DiagnosticSeverity
    ) {
        super(label, collapsibleState);
        if (contextValue) {
            this.contextValue = contextValue;
        }
        
        // Устанавливаем иконку в зависимости от severity
        if (severity === vscode.DiagnosticSeverity.Error) {
            this.iconPath = new vscode.ThemeIcon('error');
        } else if (severity === vscode.DiagnosticSeverity.Warning) {
            this.iconPath = new vscode.ThemeIcon('warning');
        } else if (severity === vscode.DiagnosticSeverity.Information) {
            this.iconPath = new vscode.ThemeIcon('info');
        }
    }
}

/**
 * Элемент дерева для типов BSL
 */
export class BslTypeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly typeName: string,
        public readonly typeKind: 'platform' | 'configuration' | 'module',
        contextValue?: string
    ) {
        super(label, collapsibleState);
        this.contextValue = contextValue || typeKind;
        this.tooltip = `${typeName} (${typeKind})`;
        
        // Устанавливаем иконку в зависимости от типа
        switch (typeKind) {
            case 'platform':
                this.iconPath = new vscode.ThemeIcon('symbol-class');
                break;
            case 'configuration':
                this.iconPath = new vscode.ThemeIcon('symbol-namespace');
                break;
            case 'module':
                this.iconPath = new vscode.ThemeIcon('symbol-module');
                break;
        }
    }
}

/**
 * Элемент дерева для документации платформы
 */
export class PlatformDocItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        contextValue?: string,
        public readonly docPath?: string
    ) {
        super(label, collapsibleState);
        if (contextValue) {
            this.contextValue = contextValue;
        }
        
        if (docPath) {
            this.tooltip = docPath;
            this.command = {
                command: 'bslAnalyzer.openPlatformDoc',
                title: 'Open Documentation',
                arguments: [docPath]
            };
        }
    }
}