import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
import { progressEmitter, getCurrentProgress } from '../lsp/progress';
import { getServerVersion, isClientRunning } from '../lsp/client';
import { BslAnalyzerConfig } from '../config/configHelper';
import { getSidebarSnapshot, invalidateSidebarSnapshot } from './sidebarSnapshot';

/**
 * Провайдер для дерева обзора BSL Analyzer
 */
export class BslOverviewProvider implements vscode.TreeDataProvider<BslOverviewItem>, vscode.Disposable {
    private _onDidChangeTreeData: vscode.EventEmitter<BslOverviewItem | undefined | null | void> = new vscode.EventEmitter<BslOverviewItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslOverviewItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private outputChannel: vscode.OutputChannel;
    private readonly disposables: vscode.Disposable[] = [];

    constructor(outputChannel: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
        
        // Подписываемся на изменения прогресса индексации
        this.disposables.push(progressEmitter.event(() => {
            invalidateSidebarSnapshot();
            this.refresh();
        }));
        this.disposables.push(vscode.languages.onDidChangeDiagnostics(() => {
            invalidateSidebarSnapshot();
            this.refresh();
        }));
        this.disposables.push(vscode.workspace.onDidChangeConfiguration((e) => {
            if (
                e.affectsConfiguration('bslAnalyzer.configurationPath')
                || e.affectsConfiguration('bslAnalyzer.platformVersion')
                || e.affectsConfiguration('bslAnalyzer.platformDocsArchive')
            ) {
                invalidateSidebarSnapshot();
                this.refresh();
            }
        }));
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
        const snapshot = await getSidebarSnapshot();
        const fileCount = snapshot.workspace.bslFiles;
        const issuesCount = snapshot.diagnostics.total;
        const lastAnalysis = snapshot.typeRepository.lastUpdateTime
            ? formatUpdateTime(snapshot.typeRepository.lastUpdateTime)
            : 'Never';

        const workspaceItems = [
            new BslOverviewItem(`BSL Files: ${fileCount}`, vscode.TreeItemCollapsibleState.None, 'file-count'),
            new BslOverviewItem(`Last Analysis: ${lastAnalysis}`, vscode.TreeItemCollapsibleState.None, 'last-analysis'),
            new BslOverviewItem(`Issues Found: ${issuesCount}`, vscode.TreeItemCollapsibleState.None, 'issues')
        ];
        
        // Добавляем информацию об индексации если она активна
        const progress = getCurrentProgress();
        if (progress.isIndexing) {
            const progressText = `Indexing: ${progress.currentStep} (${progress.progress}%)`;
            const progressItem = new BslOverviewItem(progressText, vscode.TreeItemCollapsibleState.None, 'indexing-progress');
            progressItem.iconPath = new vscode.ThemeIcon('sync~spin');
            progressItem.tooltip = `${progress.currentStep}\nProgress: ${progress.progress}%`;
            workspaceItems.unshift(progressItem); // Добавляем в начало
        }
        
        return Promise.resolve(workspaceItems);
    }

    private async getServerItems(): Promise<BslOverviewItem[]> {
        // Проверка статуса LSP сервера
        const isRunning = isClientRunning();
        const serverStatus = isRunning ? 'Running' : 'Stopped';
        const snapshot = await getSidebarSnapshot();
        const totalTypes = snapshot.typeRepository.totalTypes;
        const platformTypes = snapshot.typeRepository.platformTypes;
        const configTypes = snapshot.typeRepository.configurationTypes;
        const platformVersion = BslAnalyzerConfig.platformVersion || 'Unknown';
        const lspVersion = getServerVersion() || 'Unknown';
        const statusItem = new BslOverviewItem(`Status: ${serverStatus}`, vscode.TreeItemCollapsibleState.None, 'status');
        statusItem.iconPath = new vscode.ThemeIcon(isRunning ? 'check' : 'error');

        const typeRepositoryText = snapshot.typeRepository.status === 'live'
            ? `TypeRepository: ${totalTypes} (Platform ${platformTypes}, Config ${configTypes})`
            : 'TypeRepository: n/a';

        return [
            statusItem,
            new BslOverviewItem(
                typeRepositoryText,
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

    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables.length = 0;
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
