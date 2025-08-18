import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
import { progressEmitter, getCurrentProgress } from '../lsp/progress';
import { isClientRunning } from '../lsp/client';
import { BslAnalyzerConfig } from '../config/configHelper';

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

    private getWorkspaceItems(): Thenable<BslOverviewItem[]> {
        const workspaceItems = [
            new BslOverviewItem('BSL Files: Scanning...', vscode.TreeItemCollapsibleState.None, 'file-count'),
            new BslOverviewItem('Last Analysis: Never', vscode.TreeItemCollapsibleState.None, 'last-analysis'),
            new BslOverviewItem('Issues Found: 0', vscode.TreeItemCollapsibleState.None, 'issues')
        ];
        
        // Добавляем информацию об индексации если она активна
        const progress = getCurrentProgress();
        if (progress.isIndexing) {
            const progressIcon = '$(loading~spin)';
            const progressText = `${progressIcon} ${progress.currentStep} (${progress.progress}%)`;
            const progressItem = new BslOverviewItem(progressText, vscode.TreeItemCollapsibleState.None, 'indexing-progress');
            progressItem.tooltip = `Step ${progress.currentStepNumber}/${progress.totalSteps}${progress.estimatedTimeRemaining ? `\nETA: ${progress.estimatedTimeRemaining}` : ''}`;
            workspaceItems.unshift(progressItem); // Добавляем в начало
        }
        
        return Promise.resolve(workspaceItems);
    }

    private getServerItems(): Thenable<BslOverviewItem[]> {
        // Проверка статуса LSP сервера
        const serverStatus = isClientRunning() ? 'Running' : 'Stopped';
        const statusIcon = isClientRunning() ? '$(check)' : '$(error)';
        const statusColor = isClientRunning() ? '✅' : '⚠️';
        
        this.outputChannel.appendLine(`${statusColor} LSP Status Check: ${serverStatus} (isClientRunning=${isClientRunning()})`);
        
        return Promise.resolve([
            new BslOverviewItem(`${statusIcon} Status: ${serverStatus}`, vscode.TreeItemCollapsibleState.None, 'status'),
            new BslOverviewItem('UnifiedBslIndex: Loading...', vscode.TreeItemCollapsibleState.None, 'index-count'),
            new BslOverviewItem('Platform: 8.3.25', vscode.TreeItemCollapsibleState.None, 'platform')
        ]);
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