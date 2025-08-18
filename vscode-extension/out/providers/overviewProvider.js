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
exports.BslOverviewProvider = void 0;
const vscode = __importStar(require("vscode"));
const items_1 = require("./items");
const progress_1 = require("../lsp/progress");
const client_1 = require("../lsp/client");
const configHelper_1 = require("../config/configHelper");
/**
 * Провайдер для дерева обзора BSL Analyzer
 */
class BslOverviewProvider {
    constructor(outputChannel) {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        this.outputChannel = outputChannel;
        // Подписываемся на изменения прогресса индексации
        progress_1.progressEmitter.event(() => {
            this.refresh();
        });
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    getChildren(element) {
        if (!element) {
            // Root items
            return Promise.resolve([
                new items_1.BslOverviewItem('Workspace Analysis', vscode.TreeItemCollapsibleState.Expanded, 'workspace'),
                new items_1.BslOverviewItem('LSP Server Status', vscode.TreeItemCollapsibleState.Expanded, 'server'),
                new items_1.BslOverviewItem('Configuration', vscode.TreeItemCollapsibleState.Expanded, 'config')
            ]);
        }
        else {
            switch (element.contextValue) {
                case 'workspace':
                    return this.getWorkspaceItems();
                case 'server':
                    return this.getServerItems();
                case 'config':
                    return this.getConfigItems();
                default:
                    return Promise.resolve([]);
            }
        }
    }
    getWorkspaceItems() {
        const workspaceItems = [
            new items_1.BslOverviewItem('BSL Files: Scanning...', vscode.TreeItemCollapsibleState.None, 'file-count'),
            new items_1.BslOverviewItem('Last Analysis: Never', vscode.TreeItemCollapsibleState.None, 'last-analysis'),
            new items_1.BslOverviewItem('Issues Found: 0', vscode.TreeItemCollapsibleState.None, 'issues')
        ];
        // Добавляем информацию об индексации если она активна
        const progress = (0, progress_1.getCurrentProgress)();
        if (progress.isIndexing) {
            const progressIcon = '$(loading~spin)';
            const progressText = `${progressIcon} ${progress.currentStep} (${progress.progress}%)`;
            const progressItem = new items_1.BslOverviewItem(progressText, vscode.TreeItemCollapsibleState.None, 'indexing-progress');
            progressItem.tooltip = `Step ${progress.currentStepNumber}/${progress.totalSteps}${progress.estimatedTimeRemaining ? `\nETA: ${progress.estimatedTimeRemaining}` : ''}`;
            workspaceItems.unshift(progressItem); // Добавляем в начало
        }
        return Promise.resolve(workspaceItems);
    }
    getServerItems() {
        // Проверка статуса LSP сервера
        const serverStatus = (0, client_1.isClientRunning)() ? 'Running' : 'Stopped';
        const statusIcon = (0, client_1.isClientRunning)() ? '$(check)' : '$(error)';
        const statusColor = (0, client_1.isClientRunning)() ? '✅' : '⚠️';
        this.outputChannel.appendLine(`${statusColor} LSP Status Check: ${serverStatus} (isClientRunning=${(0, client_1.isClientRunning)()})`);
        return Promise.resolve([
            new items_1.BslOverviewItem(`${statusIcon} Status: ${serverStatus}`, vscode.TreeItemCollapsibleState.None, 'status'),
            new items_1.BslOverviewItem('UnifiedBslIndex: Loading...', vscode.TreeItemCollapsibleState.None, 'index-count'),
            new items_1.BslOverviewItem('Platform: 8.3.25', vscode.TreeItemCollapsibleState.None, 'platform')
        ]);
    }
    getConfigItems() {
        const configPath = configHelper_1.BslAnalyzerConfig.configurationPath || 'Not configured';
        const realTimeEnabled = configHelper_1.BslAnalyzerConfig.enableRealTimeAnalysis ? 'Enabled' : 'Disabled';
        const metricsEnabled = configHelper_1.BslAnalyzerConfig.enableMetrics ? 'Enabled' : 'Disabled';
        return Promise.resolve([
            new items_1.BslOverviewItem(`Configuration: ${configPath}`, vscode.TreeItemCollapsibleState.None, 'config-path'),
            new items_1.BslOverviewItem(`Real-time Analysis: ${realTimeEnabled}`, vscode.TreeItemCollapsibleState.None, 'real-time'),
            new items_1.BslOverviewItem(`Metrics: ${metricsEnabled}`, vscode.TreeItemCollapsibleState.None, 'metrics')
        ]);
    }
}
exports.BslOverviewProvider = BslOverviewProvider;
//# sourceMappingURL=overviewProvider.js.map