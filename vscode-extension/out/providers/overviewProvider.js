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
const customRequests_1 = require("../lsp/customRequests");
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
    async getWorkspaceItems() {
        const stats = await (0, customRequests_1.getWorkspaceStats)();
        const repoStats = await (0, customRequests_1.getTypeRepositoryStats)();
        const fileCount = stats ? stats.bslFiles : 0;
        const issuesCount = stats ? stats.diagnostics : 0;
        const lastAnalysis = repoStats?.lastUpdateTime
            ? formatUpdateTime(repoStats.lastUpdateTime)
            : 'Never';
        const workspaceItems = [
            new items_1.BslOverviewItem(`BSL Files: ${fileCount}`, vscode.TreeItemCollapsibleState.None, 'file-count'),
            new items_1.BslOverviewItem(`Last Analysis: ${lastAnalysis}`, vscode.TreeItemCollapsibleState.None, 'last-analysis'),
            new items_1.BslOverviewItem(`Issues Found: ${issuesCount}`, vscode.TreeItemCollapsibleState.None, 'issues')
        ];
        // Добавляем информацию об индексации если она активна
        const progress = (0, progress_1.getCurrentProgress)();
        if (progress.isIndexing) {
            const progressIcon = '$(loading~spin)';
            const progressText = `${progressIcon} ${progress.currentStep} (${progress.progress}%)`;
            const progressItem = new items_1.BslOverviewItem(progressText, vscode.TreeItemCollapsibleState.None, 'indexing-progress');
            progressItem.tooltip = `${progress.currentStep}\nProgress: ${progress.progress}%`;
            workspaceItems.unshift(progressItem); // Добавляем в начало
        }
        return Promise.resolve(workspaceItems);
    }
    async getServerItems() {
        // Проверка статуса LSP сервера
        const serverStatus = (0, client_1.isClientRunning)() ? 'Running' : 'Stopped';
        const statusIcon = (0, client_1.isClientRunning)() ? '$(check)' : '$(error)';
        const statusColor = (0, client_1.isClientRunning)() ? '✅' : '⚠️';
        const repoStats = await (0, customRequests_1.getTypeRepositoryStats)();
        const totalTypes = repoStats?.totalTypes ?? 0;
        const platformTypes = repoStats?.platformTypes ?? 0;
        const configTypes = repoStats?.configurationTypes ?? 0;
        const platformVersion = configHelper_1.BslAnalyzerConfig.platformVersion || 'Unknown';
        const lspVersion = (0, client_1.getServerVersion)() || 'Unknown';
        return [
            new items_1.BslOverviewItem(`${statusIcon} Status: ${serverStatus}`, vscode.TreeItemCollapsibleState.None, 'status'),
            new items_1.BslOverviewItem(`TypeRepository: ${totalTypes} (Platform ${platformTypes}, Config ${configTypes})`, vscode.TreeItemCollapsibleState.None, 'index-count'),
            new items_1.BslOverviewItem(`Platform: ${platformVersion}`, vscode.TreeItemCollapsibleState.None, 'platform'),
            new items_1.BslOverviewItem(`LSP Version: ${lspVersion}`, vscode.TreeItemCollapsibleState.None, 'lsp-version')
        ];
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
function formatUpdateTime(isoTimestamp) {
    try {
        const date = new Date(isoTimestamp);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffMinutes = Math.floor(diffMs / 60000);
        if (diffMinutes < 1) {
            return 'just now';
        }
        if (diffMinutes < 60) {
            return `${diffMinutes} min ago`;
        }
        const hours = Math.floor(diffMinutes / 60);
        return `${hours} h ago`;
    }
    catch {
        return 'unknown';
    }
}
//# sourceMappingURL=overviewProvider.js.map