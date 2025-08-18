"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.PlatformDocItem = exports.BslTypeItem = exports.BslDiagnosticItem = exports.BslOverviewItem = void 0;
const vscode = __importStar(require("vscode"));
/**
 * Элемент дерева для обзора BSL
 */
class BslOverviewItem extends vscode.TreeItem {
    constructor(label, collapsibleState, contextValue) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        if (contextValue) {
            this.contextValue = contextValue;
        }
    }
}
exports.BslOverviewItem = BslOverviewItem;
/**
 * Элемент дерева для диагностики
 */
class BslDiagnosticItem extends vscode.TreeItem {
    constructor(label, collapsibleState, contextValue, severity) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.severity = severity;
        if (contextValue) {
            this.contextValue = contextValue;
        }
        // Устанавливаем иконку в зависимости от severity
        if (severity === vscode.DiagnosticSeverity.Error) {
            this.iconPath = new vscode.ThemeIcon('error');
        }
        else if (severity === vscode.DiagnosticSeverity.Warning) {
            this.iconPath = new vscode.ThemeIcon('warning');
        }
        else if (severity === vscode.DiagnosticSeverity.Information) {
            this.iconPath = new vscode.ThemeIcon('info');
        }
    }
}
exports.BslDiagnosticItem = BslDiagnosticItem;
/**
 * Элемент дерева для типов BSL
 */
class BslTypeItem extends vscode.TreeItem {
    constructor(label, collapsibleState, typeName, typeKind, contextValue) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.typeName = typeName;
        this.typeKind = typeKind;
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
exports.BslTypeItem = BslTypeItem;
/**
 * Элемент дерева для документации платформы
 */
class PlatformDocItem extends vscode.TreeItem {
    constructor(label, collapsibleState, contextValue, docPath) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.docPath = docPath;
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
exports.PlatformDocItem = PlatformDocItem;
//# sourceMappingURL=items.js.map