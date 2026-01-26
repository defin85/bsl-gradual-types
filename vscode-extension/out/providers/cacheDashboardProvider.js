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
exports.CacheDashboardProvider = void 0;
const vscode = __importStar(require("vscode"));
const items_1 = require("./items");
const configHelper_1 = require("../config/configHelper");
const customRequests_1 = require("../lsp/customRequests");
const progress_1 = require("../lsp/progress");
class CacheDashboardProvider {
    constructor(outputChannel) {
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
        this.cachedStats = null;
        this.lastFetchAt = 0;
        this.inflight = null;
        this.ttlMs = 3000;
        this.disposables = [];
        this.outputChannel = outputChannel;
        const progressDisposable = progress_1.progressEmitter.event(() => {
            this.refresh();
        });
        this.disposables.push(progressDisposable);
        const configDisposable = vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('bslAnalyzer.cacheEnabled') ||
                e.affectsConfiguration('bslAnalyzer.configurationPath')) {
                this.refresh();
            }
        });
        this.disposables.push(configDisposable);
    }
    refresh() {
        this.cachedStats = null;
        this.lastFetchAt = 0;
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    getChildren(element) {
        if (!element) {
            return Promise.resolve([
                new items_1.BslOverviewItem('Status', vscode.TreeItemCollapsibleState.Expanded, 'cache-status'),
                new items_1.BslOverviewItem('Metrics', vscode.TreeItemCollapsibleState.Expanded, 'cache-metrics'),
                new items_1.BslOverviewItem('Timings', vscode.TreeItemCollapsibleState.Expanded, 'cache-timings'),
                new items_1.BslOverviewItem('Size', vscode.TreeItemCollapsibleState.Expanded, 'cache-size'),
                new items_1.BslOverviewItem('Actions', vscode.TreeItemCollapsibleState.Expanded, 'cache-actions')
            ]);
        }
        const section = element.contextValue;
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
    async getStatusItems() {
        const stats = await this.loadStats();
        if (!stats) {
            return this.missingConfigItems();
        }
        const enabled = stats.cache_enabled ? 'Enabled' : 'Disabled';
        const envDisabled = stats.env_disabled ? 'Yes' : 'No';
        const swrEnabled = stats.swr_enabled ? 'On' : 'Off';
        return [
            new items_1.BslOverviewItem(`Cache: ${enabled}`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`ENV Disabled: ${envDisabled}`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`SWR: ${swrEnabled}`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`Root: ${stats.cache_root}`, vscode.TreeItemCollapsibleState.None)
        ];
    }
    async getMetricsItems() {
        const stats = await this.loadStats();
        if (!stats) {
            return this.missingConfigItems();
        }
        const runtime = stats.disk.runtime;
        const total = runtime.hit_count + runtime.miss_count;
        const hitRate = total > 0 ? Math.round((runtime.hit_count / total) * 100) : 0;
        const items = [
            new items_1.BslOverviewItem(`Disk: hits ${runtime.hit_count} / misses ${runtime.miss_count} (${hitRate}%)`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`Disk stale hits: ${runtime.stale_hit_count}`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`AST: hits ${stats.ast.hits} / misses ${stats.ast.misses}`, vscode.TreeItemCollapsibleState.None),
        ];
        // NOTE: IR stats are optional (some LSP server versions don't report them yet).
        if (stats.ir) {
            items.push(new items_1.BslOverviewItem(`IR: hits ${stats.ir.hits} / misses ${stats.ir.misses}`, vscode.TreeItemCollapsibleState.None));
        }
        else {
            const irMissing = new items_1.BslOverviewItem('IR: n/a (not reported by LSP server)', vscode.TreeItemCollapsibleState.None);
            irMissing.tooltip = 'Update the LSP server binary to see IR cache stats (if supported).';
            items.push(irMissing);
        }
        return items;
    }
    async getTimingItems() {
        const stats = await this.loadStats();
        if (!stats) {
            return this.missingConfigItems();
        }
        const runtime = stats.disk.runtime;
        return [
            new items_1.BslOverviewItem(`Disk build time: ${runtime.build_time_ms_total} ms`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`Disk load time: ${runtime.load_time_ms_total} ms`, vscode.TreeItemCollapsibleState.None)
        ];
    }
    async getSizeItems() {
        const stats = await this.loadStats();
        if (!stats) {
            return this.missingConfigItems();
        }
        return [
            new items_1.BslOverviewItem(`Disk entries: ${stats.disk.scope.entries}`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`Disk size: ${formatBytes(stats.disk.scope.size_bytes)}`, vscode.TreeItemCollapsibleState.None),
            new items_1.BslOverviewItem(`AST entries: ${stats.ast.entries} / ${stats.ast.capacity}`, vscode.TreeItemCollapsibleState.None)
        ];
    }
    async getActionItems() {
        const stats = await this.loadStats();
        if (!stats) {
            return this.missingConfigItems();
        }
        const items = [];
        const toggleLabel = stats.cache_enabled ? 'Disable Cache' : 'Enable Cache';
        const toggleItem = new items_1.BslOverviewItem(toggleLabel, vscode.TreeItemCollapsibleState.None);
        if (!stats.env_disabled) {
            toggleItem.command = {
                command: 'bslAnalyzer.toggleCache',
                title: toggleLabel
            };
        }
        else {
            toggleItem.tooltip = 'Cache disabled by BSL_CACHE_DISABLE (ENV)';
        }
        toggleItem.iconPath = new vscode.ThemeIcon('circle-slash');
        items.push(toggleItem);
        const clearItem = new items_1.BslOverviewItem('Clear Cache (Project/Config)', vscode.TreeItemCollapsibleState.None);
        clearItem.command = {
            command: 'bslAnalyzer.clearCache',
            title: 'Clear Cache'
        };
        clearItem.iconPath = new vscode.ThemeIcon('trash');
        items.push(clearItem);
        const refreshItem = new items_1.BslOverviewItem('Refresh', vscode.TreeItemCollapsibleState.None);
        refreshItem.command = {
            command: 'bslAnalyzer.refreshCacheDashboard',
            title: 'Refresh Cache Dashboard'
        };
        refreshItem.iconPath = new vscode.ThemeIcon('refresh');
        items.push(refreshItem);
        return items;
    }
    async loadStats() {
        const configPath = configHelper_1.BslAnalyzerConfig.configurationPath;
        if (!configPath) {
            return null;
        }
        const now = Date.now();
        if (this.cachedStats && now - this.lastFetchAt < this.ttlMs) {
            return this.cachedStats;
        }
        if (!this.inflight) {
            this.inflight = (0, customRequests_1.getCacheStats)(configPath)
                .catch((error) => {
                this.outputChannel.appendLine(`[Cache Dashboard] Failed to load stats: ${error}`);
                return null;
            })
                .finally(() => {
                this.inflight = null;
            });
        }
        const result = await this.inflight;
        if (result) {
            this.cachedStats = result;
            this.lastFetchAt = now;
        }
        return result;
    }
    missingConfigItems() {
        const missing = new items_1.BslOverviewItem('Configuration path not set', vscode.TreeItemCollapsibleState.None);
        missing.iconPath = new vscode.ThemeIcon('warning');
        return [missing];
    }
    dispose() {
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }
}
exports.CacheDashboardProvider = CacheDashboardProvider;
function formatBytes(bytes) {
    if (bytes <= 0) {
        return '0 B';
    }
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const idx = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / Math.pow(1024, idx);
    return `${value.toFixed(value >= 10 || idx === 0 ? 0 : 1)} ${units[idx]}`;
}
//# sourceMappingURL=cacheDashboardProvider.js.map