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
exports.BslDiagnosticsProvider = void 0;
const vscode = __importStar(require("vscode"));
const items_1 = require("./items");
/**
 * Провайдер для дерева диагностики BSL
 */
class BslDiagnosticsProvider {
    constructor() {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        this.diagnostics = [];
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    updateDiagnostics(diagnostics) {
        this.diagnostics = diagnostics;
        this.refresh();
    }
    getTreeItem(element) {
        return element;
    }
    getChildren(element) {
        if (!element) {
            // Root items - группировка по severity
            const errors = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Error);
            const warnings = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Warning);
            const infos = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Information);
            const hints = this.diagnostics.filter(d => d.severity === vscode.DiagnosticSeverity.Hint);
            const items = [];
            if (errors.length > 0) {
                items.push(new items_1.BslDiagnosticItem(`Errors (${errors.length})`, vscode.TreeItemCollapsibleState.Expanded, 'errors', vscode.DiagnosticSeverity.Error));
            }
            if (warnings.length > 0) {
                items.push(new items_1.BslDiagnosticItem(`Warnings (${warnings.length})`, vscode.TreeItemCollapsibleState.Collapsed, 'warnings', vscode.DiagnosticSeverity.Warning));
            }
            if (infos.length > 0) {
                items.push(new items_1.BslDiagnosticItem(`Information (${infos.length})`, vscode.TreeItemCollapsibleState.Collapsed, 'infos', vscode.DiagnosticSeverity.Information));
            }
            if (hints.length > 0) {
                items.push(new items_1.BslDiagnosticItem(`Hints (${hints.length})`, vscode.TreeItemCollapsibleState.Collapsed, 'hints', vscode.DiagnosticSeverity.Hint));
            }
            if (items.length === 0) {
                items.push(new items_1.BslDiagnosticItem('No issues found', vscode.TreeItemCollapsibleState.None, 'no-issues'));
            }
            return Promise.resolve(items);
        }
        else {
            // Child items - конкретные диагностики
            let relevantDiagnostics = [];
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
                const item = new items_1.BslDiagnosticItem(d.message, vscode.TreeItemCollapsibleState.None, 'diagnostic', d.severity);
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
exports.BslDiagnosticsProvider = BslDiagnosticsProvider;
//# sourceMappingURL=diagnosticsProvider.js.map