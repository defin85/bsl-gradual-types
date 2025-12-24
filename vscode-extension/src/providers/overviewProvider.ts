import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
import { progressEmitter, getCurrentProgress } from '../lsp/progress';
import { getServerVersion, isClientRunning } from '../lsp/client';
import { BslAnalyzerConfig } from '../config/configHelper';
import { getTypeRepositoryStats, getWorkspaceStats } from '../lsp/customRequests';

/**
 * Провайдер для дерева обзора BSL Analyzer
 */
export class BslOverviewProvider implements vscode.TreeDataProvider<BslOverviewItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<BslOverviewItem | undefined | null | void> = new vscode.EventEmitter<BslOverviewItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslOverviewItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private outputChannel: vscode.OutputChannel;

    constructor(outputChannel: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
        
        // Подписываемся на изменения прогресса индексации
        progressEmitter.event(() => {
            this.refresh();
        });
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: BslOverviewItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: BslOverviewItem): Thenable<BslOverviewItem[]> {
        if (!element) {
            // Root items
            return Promise.resolve([
                new BslOverviewItem('Workspace Analysis', vscode.TreeItemCollapsibleState.Expanded, 'workspace'),
                new BslOverviewItem('LSP Server Status', vscode.TreeItemCollapsibleState.Expanded, 'server'),
                new BslOverviewItem('Configuration', vscode.TreeItemCollapsibleState.Expanded, 'config')
            ]);
        } else {
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

    private async getWorkspaceItems(): Promise<BslOverviewItem[]> {
        const stats = await getWorkspaceStats();
        const repoStats = await getTypeRepositoryStats();
        const fileCount = stats ? stats.bslFiles : 0;
        const issuesCount = stats ? stats.diagnostics : 0;
        const lastAnalysis = repoStats?.lastUpdateTime
            ? formatUpdateTime(repoStats.lastUpdateTime)
            : 'Never';

        const workspaceItems = [
            new BslOverviewItem(`BSL Files: ${fileCount}`, vscode.TreeItemCollapsibleState.None, 'file-count'),
            new BslOverviewItem(`Last Analysis: ${lastAnalysis}`, vscode.TreeItemCollapsibleState.None, 'last-analysis'),
            new BslOverviewItem(`Issues Found: ${issuesCount}`, vscode.TreeItemCollapsibleState.None, 'issues')
        ];
        
        // Добавляем информацию об индексации если она активна
        const progress = getCurrentProgress();
        if (progress.isIndexing) {
            const progressIcon = '$(loading~spin)';
            const progressText = `${progressIcon} ${progress.currentStep} (${progress.progress}%)`;
            const progressItem = new BslOverviewItem(progressText, vscode.TreeItemCollapsibleState.None, 'indexing-progress');
            progressItem.tooltip = `${progress.currentStep}\nProgress: ${progress.progress}%`;
            workspaceItems.unshift(progressItem); // Добавляем в начало
        }
        
        return Promise.resolve(workspaceItems);
    }

    private async getServerItems(): Promise<BslOverviewItem[]> {
        // Проверка статуса LSP сервера
        const serverStatus = isClientRunning() ? 'Running' : 'Stopped';
        const statusIcon = isClientRunning() ? '$(check)' : '$(error)';
        const statusColor = isClientRunning() ? '✅' : '⚠️';

        const repoStats = await getTypeRepositoryStats();
        const totalTypes = repoStats?.totalTypes ?? 0;
        const platformTypes = repoStats?.platformTypes ?? 0;
        const configTypes = repoStats?.configurationTypes ?? 0;
        const platformVersion = BslAnalyzerConfig.platformVersion || 'Unknown';
        const lspVersion = getServerVersion() || 'Unknown';

        return [
            new BslOverviewItem(`${statusIcon} Status: ${serverStatus}`, vscode.TreeItemCollapsibleState.None, 'status'),
            new BslOverviewItem(
                `TypeRepository: ${totalTypes} (Platform ${platformTypes}, Config ${configTypes})`,
                vscode.TreeItemCollapsibleState.None,
                'index-count'
            ),
            new BslOverviewItem(`Platform: ${platformVersion}`, vscode.TreeItemCollapsibleState.None, 'platform'),
            new BslOverviewItem(`LSP Version: ${lspVersion}`, vscode.TreeItemCollapsibleState.None, 'lsp-version')
        ];
    }

    private getConfigItems(): Thenable<BslOverviewItem[]> {
        const configPath = BslAnalyzerConfig.configurationPath || 'Not configured';
        const realTimeEnabled = BslAnalyzerConfig.enableRealTimeAnalysis ? 'Enabled' : 'Disabled';
        const metricsEnabled = BslAnalyzerConfig.enableMetrics ? 'Enabled' : 'Disabled';
        
        return Promise.resolve([
            new BslOverviewItem(`Configuration: ${configPath}`, vscode.TreeItemCollapsibleState.None, 'config-path'),
            new BslOverviewItem(`Real-time Analysis: ${realTimeEnabled}`, vscode.TreeItemCollapsibleState.None, 'real-time'),
            new BslOverviewItem(`Metrics: ${metricsEnabled}`, vscode.TreeItemCollapsibleState.None, 'metrics')
        ]);
    }
}

function formatUpdateTime(isoTimestamp: string): string {
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
    } catch {
        return 'unknown';
    }
}
