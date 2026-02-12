import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
import { progressEmitter } from '../lsp/progress';
import { getSidebarSnapshot, invalidateSidebarSnapshot, SidebarCacheSnapshot } from './sidebarSnapshot';

type CacheSection =
    | 'cache-status'
    | 'cache-metrics'
    | 'cache-timings'
    | 'cache-size'
    | 'cache-actions';

export class CacheDashboardProvider implements vscode.TreeDataProvider<BslOverviewItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<BslOverviewItem | undefined | null | void> =
        new vscode.EventEmitter<BslOverviewItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslOverviewItem | undefined | null | void> =
        this._onDidChangeTreeData.event;

    private outputChannel: vscode.OutputChannel;
    private disposables: vscode.Disposable[] = [];

    constructor(outputChannel: vscode.OutputChannel) {
        this.outputChannel = outputChannel;

        const progressDisposable = progressEmitter.event(() => {
            this.refresh();
        });
        this.disposables.push(progressDisposable);

        const configDisposable = vscode.workspace.onDidChangeConfiguration((e) => {
            if (
                e.affectsConfiguration('bslAnalyzer.cacheEnabled') ||
                e.affectsConfiguration('bslAnalyzer.configurationPath')
            ) {
                this.refresh();
            }
        });
        this.disposables.push(configDisposable);
    }

    refresh(): void {
        invalidateSidebarSnapshot();
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: BslOverviewItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: BslOverviewItem): Thenable<BslOverviewItem[]> {
        if (!element) {
            return Promise.resolve([
                new BslOverviewItem('Status', vscode.TreeItemCollapsibleState.Expanded, 'cache-status'),
                new BslOverviewItem('Metrics', vscode.TreeItemCollapsibleState.Expanded, 'cache-metrics'),
                new BslOverviewItem('Timings', vscode.TreeItemCollapsibleState.Expanded, 'cache-timings'),
                new BslOverviewItem('Size', vscode.TreeItemCollapsibleState.Expanded, 'cache-size'),
                new BslOverviewItem('Actions', vscode.TreeItemCollapsibleState.Expanded, 'cache-actions')
            ]);
        }

        const section = element.contextValue as CacheSection;
        switch (section) {
            case 'cache-status':
                return this.getStatusItems();
            case 'cache-metrics':
                return this.getMetricsItems();
            case 'cache-timings':
                return this.getTimingItems();
            case 'cache-size':
                return this.getSizeItems();
            case 'cache-actions':
                return this.getActionItems();
            default:
                return Promise.resolve([]);
        }
    }

    private async getStatusItems(): Promise<BslOverviewItem[]> {
        const cache = await this.loadCacheSnapshot();
        if (!cache.stats) {
            return this.missingConfigItems(cache.reason);
        }
        const stats = cache.stats;

        const enabled = stats.cache_enabled ? 'Enabled' : 'Disabled';
        const envDisabled = stats.env_disabled ? 'Yes' : 'No';
        const swrEnabled = stats.swr_enabled ? 'On' : 'Off';

        return [
            new BslOverviewItem(`Cache: ${enabled}`, vscode.TreeItemCollapsibleState.None),
            new BslOverviewItem(`ENV Disabled: ${envDisabled}`, vscode.TreeItemCollapsibleState.None),
            new BslOverviewItem(`SWR: ${swrEnabled}`, vscode.TreeItemCollapsibleState.None),
            new BslOverviewItem(`Root: ${stats.cache_root}`, vscode.TreeItemCollapsibleState.None)
        ];
    }

    private async getMetricsItems(): Promise<BslOverviewItem[]> {
        const cache = await this.loadCacheSnapshot();
        if (!cache.stats) {
            return this.missingConfigItems(cache.reason);
        }
        const stats = cache.stats;

        const runtime = stats.disk.runtime;
        const total = runtime.hit_count + runtime.miss_count;
        const hitRate = total > 0 ? Math.round((runtime.hit_count / total) * 100) : 0;

        const items: BslOverviewItem[] = [
            new BslOverviewItem(
                `Disk: hits ${runtime.hit_count} / misses ${runtime.miss_count} (${hitRate}%)`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Disk stale hits: ${runtime.stale_hit_count}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `AST: hits ${stats.ast.hits} / misses ${stats.ast.misses}`,
                vscode.TreeItemCollapsibleState.None
            ),
        ];

        // NOTE: IR stats are optional (some LSP server versions don't report them yet).
        if (stats.ir) {
            items.push(
                new BslOverviewItem(
                    `IR: hits ${stats.ir.hits} / misses ${stats.ir.misses}`,
                    vscode.TreeItemCollapsibleState.None
                )
            );
        } else {
            const irMissing = new BslOverviewItem(
                'IR: n/a (not reported by LSP server)',
                vscode.TreeItemCollapsibleState.None
            );
            irMissing.tooltip = 'Update the LSP server binary to see IR cache stats (if supported).';
            items.push(irMissing);
        }

        return items;
    }

    private async getTimingItems(): Promise<BslOverviewItem[]> {
        const cache = await this.loadCacheSnapshot();
        if (!cache.stats) {
            return this.missingConfigItems(cache.reason);
        }
        const stats = cache.stats;

        const runtime = stats.disk.runtime;
        return [
            new BslOverviewItem(
                `Disk build time: ${runtime.build_time_ms_total} ms`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Disk load time: ${runtime.load_time_ms_total} ms`,
                vscode.TreeItemCollapsibleState.None
            )
        ];
    }

    private async getSizeItems(): Promise<BslOverviewItem[]> {
        const cache = await this.loadCacheSnapshot();
        if (!cache.stats) {
            return this.missingConfigItems(cache.reason);
        }
        const stats = cache.stats;

        return [
            new BslOverviewItem(
                `Disk entries: ${stats.disk.scope.entries}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Disk size: ${formatBytes(stats.disk.scope.size_bytes)}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `AST entries: ${stats.ast.entries} / ${stats.ast.capacity}`,
                vscode.TreeItemCollapsibleState.None
            )
        ];
    }

    private async getActionItems(): Promise<BslOverviewItem[]> {
        const cache = await this.loadCacheSnapshot();
        if (!cache.stats) {
            return this.missingConfigItems(cache.reason);
        }
        const stats = cache.stats;

        const items: BslOverviewItem[] = [];

        const toggleLabel = stats.cache_enabled ? 'Disable Cache' : 'Enable Cache';
        const toggleItem = new BslOverviewItem(toggleLabel, vscode.TreeItemCollapsibleState.None);
        if (!stats.env_disabled) {
            toggleItem.command = {
                command: 'bslAnalyzer.toggleCache',
                title: toggleLabel
            };
        } else {
            toggleItem.tooltip = 'Cache disabled by BSL_CACHE_DISABLE (ENV)';
        }
        toggleItem.iconPath = new vscode.ThemeIcon('circle-slash');
        items.push(toggleItem);

        const clearItem = new BslOverviewItem('Clear Cache (Project/Config)', vscode.TreeItemCollapsibleState.None);
        clearItem.command = {
            command: 'bslAnalyzer.clearCache',
            title: 'Clear Cache'
        };
        clearItem.iconPath = new vscode.ThemeIcon('trash');
        items.push(clearItem);

        const refreshItem = new BslOverviewItem('Refresh', vscode.TreeItemCollapsibleState.None);
        refreshItem.command = {
            command: 'bslAnalyzer.refreshCacheDashboard',
            title: 'Refresh Cache Dashboard'
        };
        refreshItem.iconPath = new vscode.ThemeIcon('refresh');
        items.push(refreshItem);

        return items;
    }

    private async loadCacheSnapshot(): Promise<SidebarCacheSnapshot> {
        try {
            const snapshot = await getSidebarSnapshot();
            return snapshot.cache;
        } catch (error) {
            this.outputChannel.appendLine(`[Cache Dashboard] Failed to load sidebar snapshot: ${error}`);
            return {
                status: 'n/a',
                reason: 'lsp_unavailable',
                stats: null,
            };
        }
    }

    private missingConfigItems(reason?: SidebarCacheSnapshot['reason']): BslOverviewItem[] {
        const message = this.messageForUnavailableReason(reason);
        const missing = new BslOverviewItem(message, vscode.TreeItemCollapsibleState.None);
        missing.iconPath = new vscode.ThemeIcon('warning');
        return [missing];
    }

    private messageForUnavailableReason(reason?: SidebarCacheSnapshot['reason']): string {
        if (reason === 'configuration_path_missing') {
            return 'Configuration path not set';
        }
        if (reason === 'lsp_unavailable') {
            return 'Cache metrics unavailable (LSP/cache stats not ready)';
        }
        return 'Cache metrics unavailable';
    }

    dispose(): void {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}

function formatBytes(bytes: number): string {
    if (bytes <= 0) {
        return '0 B';
    }
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const idx = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / Math.pow(1024, idx);
    return `${value.toFixed(value >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`;
}
