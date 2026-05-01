import * as vscode from 'vscode';
import { BslOverviewItem } from './items';
import { BslAnalyzerConfig } from '../config/configHelper';
import { getLanguageClient } from '../lsp/client';
import { getObservabilityMetricsWithRequest } from '../lsp/customRequests';
import type { SnapshotArtifactStatus, SnapshotStatusResponse } from '../lsp/customRequests';
import { getServerStatusSnapshot, onServerStatusChange } from '../lsp/serverStatus';
import {
    getActiveSnapshotStatusHistory,
    getActiveSnapshotStatusSnapshot,
    onSnapshotStatusChange,
    sanitizeSnapshotDetail,
} from '../lsp/snapshotStatus';

type ObservabilitySection =
    | 'obs-status'
    | 'obs-snapshot'
    | 'obs-sla'
    | 'obs-latency'
    | 'obs-rates'
    | 'obs-counters'
    | 'obs-gauges'
    | 'obs-actions'
    | 'obs-snapshot-summary'
    | 'obs-snapshot-why'
    | 'obs-snapshot-artifacts'
    | 'obs-snapshot-worker'
    | 'obs-snapshot-failure'
    | 'obs-snapshot-history'
    | 'obs-snapshot-actions';

type HistogramPoint = {
    p50?: number;
    p95?: number;
    p99?: number;
    count?: number;
};

const MAX_COUNTER_ITEMS = 20;
const MAX_GAUGE_ITEMS = 20;

const KEY_LATENCIES: Array<{ key: string; label: string }> = [
    { key: 'intellisense_v2_wait_for_file_version_diagnostics', label: 'Diagnostics wait_for_file_version' },
    { key: 'intellisense_v2_syntax_diagnostics_query', label: 'Diagnostics syntax_query' },
    { key: 'intellisense_v2_semantic_diagnostics_query', label: 'Diagnostics semantic_query' },
    { key: 'intellisense_v2_wait_for_file_version_completion', label: 'Completion wait_for_file_version' },
    { key: 'intellisense_v2_snapshot_completion', label: 'Completion snapshot' },
    { key: 'intellisense_v2_ir_query_completion', label: 'Completion ir_query' },
    { key: 'intellisense_v2_wait_for_file_version_hover', label: 'Hover wait_for_file_version' },
    { key: 'intellisense_v2_snapshot_hover', label: 'Hover snapshot' },
    { key: 'intellisense_v2_ir_query_hover', label: 'Hover ir_query' },
];

const SLA_LATENCIES: Array<{ key: string; label: string }> = [
    { key: 'intellisense_v2_wait_for_file_version_completion', label: 'Completion wait_for_file_version' },
    { key: 'intellisense_v2_snapshot_completion', label: 'Completion snapshot' },
    { key: 'intellisense_v2_ir_query_completion', label: 'Completion ir_query' },
    { key: 'intellisense_v2_wait_for_file_version_hover', label: 'Hover wait_for_file_version' },
    { key: 'intellisense_v2_snapshot_hover', label: 'Hover snapshot' },
    { key: 'intellisense_v2_ir_query_hover', label: 'Hover ir_query' },
    { key: 'intellisense_v2_wait_for_file_version_diagnostics', label: 'Diagnostics wait_for_file_version' },
    { key: 'intellisense_v2_syntax_diagnostics_query', label: 'Diagnostics syntax_query' },
    { key: 'intellisense_v2_semantic_diagnostics_query', label: 'Diagnostics semantic_query' },
];

const SLA_RATES: Array<{ key: string; label: string }> = [
    { key: 'completion_error_rate', label: 'Completion error rate' },
    { key: 'completion_incomplete_rate', label: 'Completion incomplete rate' },
    { key: 'signature_help_empty_rate', label: 'SignatureHelp empty rate' },
];

export class ObservabilityProvider implements vscode.TreeDataProvider<BslOverviewItem>, vscode.Disposable {
    private _onDidChangeTreeData: vscode.EventEmitter<BslOverviewItem | undefined | null | void> =
        new vscode.EventEmitter<BslOverviewItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<BslOverviewItem | undefined | null | void> =
        this._onDidChangeTreeData.event;

    private outputChannel: vscode.OutputChannel;
    private cachedMetrics: Record<string, unknown> | null = null;
    private lastFetchAt = 0;
    private lastSuccessAt = 0;
    private lastFetchHadUsableMetrics = false;
    private nextFetchAllowedAt = 0;
    private inflight: Promise<Record<string, unknown> | null> | null = null;
    private readonly ttlMs = 1000;
    private autoRefreshEnabled = true;
    private autoRefreshIntervalMs = 3000;
    private compactModeEnabled = false;
    private autoRefreshTimer: NodeJS.Timeout | null = null;
    private disposables: vscode.Disposable[] = [];

    constructor(outputChannel: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
        this.reloadConfig();

        const configDisposable = vscode.workspace.onDidChangeConfiguration((e) => {
            if (
                e.affectsConfiguration('bslAnalyzer.observabilityAutoRefresh')
                || e.affectsConfiguration('bslAnalyzer.observabilityRefreshMs')
                || e.affectsConfiguration('bslAnalyzer.observabilityCompactMode')
            ) {
                this.reloadConfig();
                this.refresh();
            }
        });
        this.disposables.push(configDisposable);

        const serverStatusDisposable = onServerStatusChange((status) => {
            if (!status.loading) {
                this.refresh();
            }
        });
        this.disposables.push(serverStatusDisposable);

        const snapshotStatusDisposable = onSnapshotStatusChange(() => {
            this.refresh(false);
        });
        this.disposables.push(snapshotStatusDisposable);

        this.startAutoRefresh();
    }

    dispose(): void {
        this.stopAutoRefresh();
        for (const disposable of this.disposables) {
            disposable.dispose();
        }
        this.disposables = [];
    }

    refresh(forceInvalidate = true): void {
        if (forceInvalidate) {
            this.cachedMetrics = null;
            this.lastFetchAt = 0;
            this.nextFetchAllowedAt = 0;
        }
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: BslOverviewItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: BslOverviewItem): Thenable<BslOverviewItem[]> {
        if (!element) {
            if (this.compactModeEnabled) {
                return Promise.resolve([
                    new BslOverviewItem('Status', vscode.TreeItemCollapsibleState.Expanded, 'obs-status'),
                    new BslOverviewItem('Snapshot Readiness', vscode.TreeItemCollapsibleState.Expanded, 'obs-snapshot'),
                    new BslOverviewItem('SLA Metrics', vscode.TreeItemCollapsibleState.Expanded, 'obs-sla'),
                    new BslOverviewItem('Actions', vscode.TreeItemCollapsibleState.Expanded, 'obs-actions'),
                ]);
            }

            return Promise.resolve([
                new BslOverviewItem('Status', vscode.TreeItemCollapsibleState.Expanded, 'obs-status'),
                new BslOverviewItem('Snapshot Readiness', vscode.TreeItemCollapsibleState.Expanded, 'obs-snapshot'),
                new BslOverviewItem('Key Latencies', vscode.TreeItemCollapsibleState.Expanded, 'obs-latency'),
                new BslOverviewItem('Rates', vscode.TreeItemCollapsibleState.Collapsed, 'obs-rates'),
                new BslOverviewItem('Counters', vscode.TreeItemCollapsibleState.Collapsed, 'obs-counters'),
                new BslOverviewItem('Gauges', vscode.TreeItemCollapsibleState.Collapsed, 'obs-gauges'),
                new BslOverviewItem('Actions', vscode.TreeItemCollapsibleState.Expanded, 'obs-actions'),
            ]);
        }

        const section = element.contextValue as ObservabilitySection;
        switch (section) {
            case 'obs-status':
                return this.getStatusItems();
            case 'obs-snapshot':
                return Promise.resolve(this.getSnapshotItems());
            case 'obs-sla':
                return this.getSlaItems();
            case 'obs-latency':
                return this.getLatencyItems();
            case 'obs-rates':
                return this.getRateItems();
            case 'obs-counters':
                return this.getCounterItems();
            case 'obs-gauges':
                return this.getGaugeItems();
            case 'obs-actions':
                return this.getActionItems();
            case 'obs-snapshot-summary':
                return Promise.resolve(this.getSnapshotSummaryItems());
            case 'obs-snapshot-why':
                return Promise.resolve(this.getSnapshotWhyItems());
            case 'obs-snapshot-artifacts':
                return Promise.resolve(this.getSnapshotArtifactItems());
            case 'obs-snapshot-worker':
                return Promise.resolve(this.getSnapshotWorkerItems());
            case 'obs-snapshot-failure':
                return Promise.resolve(this.getSnapshotFailureItems());
            case 'obs-snapshot-history':
                return Promise.resolve(this.getSnapshotHistoryItems());
            case 'obs-snapshot-actions':
                return Promise.resolve(this.getSnapshotActionItems());
            default:
                return Promise.resolve([]);
        }
    }

    private async getStatusItems(): Promise<BslOverviewItem[]> {
        const client = getLanguageClient();
        const serverRunning = !!client?.isRunning();
        const serverLoading = getServerStatusSnapshot().loading;
        const metrics = await this.loadMetrics();

        const statusItem = new BslOverviewItem(
            `LSP Server: ${serverRunning ? 'Running' : 'Stopped'}`,
            vscode.TreeItemCollapsibleState.None
        );
        statusItem.iconPath = new vscode.ThemeIcon(serverRunning ? 'check' : 'error');

        const refreshItem = new BslOverviewItem(
            `Auto refresh: ${this.autoRefreshEnabled ? `On (${this.autoRefreshIntervalMs} ms)` : 'Off'}`,
            vscode.TreeItemCollapsibleState.None
        );
        refreshItem.iconPath = new vscode.ThemeIcon(this.autoRefreshEnabled ? 'sync' : 'circle-slash');

        const lastUpdateLabel = this.lastSuccessAt > 0
            ? formatRelativeTime(this.lastSuccessAt)
            : 'never';
        const updatedItem = new BslOverviewItem(
            `Last update: ${lastUpdateLabel}`,
            vscode.TreeItemCollapsibleState.None
        );

        const items: BslOverviewItem[] = [statusItem, refreshItem, updatedItem];

        items.push(
            new BslOverviewItem(
                `Compact mode: ${this.compactModeEnabled ? 'On' : 'Off'}`,
                vscode.TreeItemCollapsibleState.None
            )
        );

        const uptime = asNumber(metrics?.uptime_seconds);
        if (uptime !== null) {
            items.push(
                new BslOverviewItem(
                    `Uptime: ${formatDurationSeconds(uptime)}`,
                    vscode.TreeItemCollapsibleState.None
                )
            );
        }

        if (!metrics) {
            const unavailable = new BslOverviewItem(
                serverLoading
                    ? 'Metrics: deferred while server is loading'
                    : 'Metrics: unavailable (LSP unsupported or not ready)',
                vscode.TreeItemCollapsibleState.None
            );
            unavailable.iconPath = new vscode.ThemeIcon('warning');
            items.push(unavailable);
        }

        return items;
    }

    private async getSlaItems(): Promise<BslOverviewItem[]> {
        const metrics = await this.loadMetrics();
        if (!metrics) {
            return [new BslOverviewItem('No SLA metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const items: BslOverviewItem[] = [];
        const histograms = asRecord(metrics.histograms);
        if (histograms) {
            for (const metric of SLA_LATENCIES) {
                const point = asHistogram(histograms[`${metric.key}_ms`] ?? histograms[metric.key]);
                if (!point) {
                    continue;
                }
                items.push(
                    new BslOverviewItem(
                        `${metric.label}: p95=${formatMs(point.p95)} p99=${formatMs(point.p99)} (n=${formatCount(point.count)})`,
                        vscode.TreeItemCollapsibleState.None
                    )
                );
            }
        }

        const rates = asRecord(metrics.rates);
        if (rates) {
            for (const rateMetric of SLA_RATES) {
                if (!(rateMetric.key in rates)) {
                    continue;
                }
                items.push(
                    new BslOverviewItem(
                        `${rateMetric.label}: ${formatRate(rates[rateMetric.key])}`,
                        vscode.TreeItemCollapsibleState.None
                    )
                );
            }
        }

        return items.length > 0
            ? items
            : [new BslOverviewItem('No SLA metrics', vscode.TreeItemCollapsibleState.None)];
    }

    private getSnapshotItems(): BslOverviewItem[] {
        const snapshot = getActiveSnapshotStatusSnapshot();
        switch (snapshot.kind) {
            case 'inactive': {
                const item = new BslOverviewItem(
                    'Snapshot: no active BSL editor',
                    vscode.TreeItemCollapsibleState.None
                );
                item.iconPath = new vscode.ThemeIcon('circle-slash');
                return [item];
            }
            case 'unsupported': {
                const item = new BslOverviewItem(
                    'Snapshot: unsupported by current server',
                    vscode.TreeItemCollapsibleState.None
                );
                item.iconPath = new vscode.ThemeIcon('warning');
                return [item];
            }
            case 'unavailable': {
                const item = new BslOverviewItem(
                    `Snapshot: unavailable (${snapshot.message})`,
                    vscode.TreeItemCollapsibleState.None
                );
                item.iconPath = new vscode.ThemeIcon('warning');
                return [item];
            }
            case 'ok': {
                return [
                    new BslOverviewItem('Summary', vscode.TreeItemCollapsibleState.Expanded, 'obs-snapshot-summary'),
                    new BslOverviewItem('Why', vscode.TreeItemCollapsibleState.Expanded, 'obs-snapshot-why'),
                    new BslOverviewItem('Artifacts', vscode.TreeItemCollapsibleState.Expanded, 'obs-snapshot-artifacts'),
                    new BslOverviewItem('Worker', vscode.TreeItemCollapsibleState.Collapsed, 'obs-snapshot-worker'),
                    new BslOverviewItem('Last Failure', vscode.TreeItemCollapsibleState.Collapsed, 'obs-snapshot-failure'),
                    new BslOverviewItem('Recent Transitions', vscode.TreeItemCollapsibleState.Collapsed, 'obs-snapshot-history'),
                    new BslOverviewItem('Actions', vscode.TreeItemCollapsibleState.Collapsed, 'obs-snapshot-actions'),
                ];
            }
        }
    }

    private getSnapshotSummaryItems(): BslOverviewItem[] {
        const status = this.currentSnapshotStatus();
        if (!status) {
            return [new BslOverviewItem('Snapshot: unavailable', vscode.TreeItemCollapsibleState.None)];
        }

        const items = [
            new BslOverviewItem(
                `State: ${status.state}${status.exact ? ' (exact)' : ''}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Requested revision: ${status.requestedVersion ?? 'n/a'}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Ready revision: ${status.readyVersion ?? 'n/a'}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Task state: ${status.taskState}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Schema: v${status.schemaVersion}`,
                vscode.TreeItemCollapsibleState.None
            ),
            new BslOverviewItem(
                `Updated: ${formatRelativeTime(status.updatedAtMs)}`,
                vscode.TreeItemCollapsibleState.None
            ),
        ];
        return items;
    }

    private getSnapshotWhyItems(): BslOverviewItem[] {
        const status = this.currentSnapshotStatus();
        if (!status) {
            return [new BslOverviewItem('Why: unavailable', vscode.TreeItemCollapsibleState.None)];
        }

        const items: BslOverviewItem[] = [];
        if (status.reason) {
            items.push(new BslOverviewItem(`Code: ${status.reason.code}`, vscode.TreeItemCollapsibleState.None));
            const message = sanitizeSnapshotDetail(status.reason.message);
            if (message) {
                items.push(new BslOverviewItem(`Message: ${message}`, vscode.TreeItemCollapsibleState.None));
            }
        } else {
            items.push(new BslOverviewItem('Code: unavailable in legacy payload', vscode.TreeItemCollapsibleState.None));
        }
        if (status.fallbackReason) {
            items.push(
                new BslOverviewItem(
                    `Fallback: ${sanitizeSnapshotDetail(status.fallbackReason) ?? status.fallbackReason}`,
                    vscode.TreeItemCollapsibleState.None
                )
            );
        }
        if (status.recommendation) {
            items.push(new BslOverviewItem(`Recommendation: ${status.recommendation}`, vscode.TreeItemCollapsibleState.None));
        }
        return items;
    }

    private getSnapshotArtifactItems(): BslOverviewItem[] {
        const status = this.currentSnapshotStatus();
        if (!status?.artifacts) {
            return [new BslOverviewItem('Artifacts: unavailable in legacy payload', vscode.TreeItemCollapsibleState.None)];
        }

        return [
            this.snapshotArtifactItem('Shadow state', status.artifacts.shadowState),
            this.snapshotArtifactItem('Ready parse snapshot', status.artifacts.readyParseSnapshot),
            this.snapshotArtifactItem('Exact type index', status.artifacts.exactTypeIndex),
            this.snapshotArtifactItem('Completion head', status.artifacts.completionHead),
        ];
    }

    private getSnapshotWorkerItems(): BslOverviewItem[] {
        const status = this.currentSnapshotStatus();
        const worker = status?.worker;
        if (!worker) {
            return [new BslOverviewItem('Worker: not active', vscode.TreeItemCollapsibleState.None)];
        }

        const items: BslOverviewItem[] = [];
        if (typeof worker.targetVersion === 'number') {
            items.push(new BslOverviewItem(`Target revision: ${worker.targetVersion}`, vscode.TreeItemCollapsibleState.None));
        }
        if (worker.phase) {
            items.push(new BslOverviewItem(`Phase: ${worker.phase}`, vscode.TreeItemCollapsibleState.None));
        }
        if (worker.trigger) {
            items.push(new BslOverviewItem(`Trigger: ${worker.trigger}`, vscode.TreeItemCollapsibleState.None));
        }
        if (typeof worker.ageMs === 'number') {
            items.push(new BslOverviewItem(`Age: ${formatMs(worker.ageMs)}`, vscode.TreeItemCollapsibleState.None));
        }
        if (worker.cancellationReason) {
            items.push(
                new BslOverviewItem(
                    `Cancellation: ${sanitizeSnapshotDetail(worker.cancellationReason) ?? worker.cancellationReason}`,
                    vscode.TreeItemCollapsibleState.None
                )
            );
        }
        if (typeof worker.supersededByVersion === 'number') {
            items.push(new BslOverviewItem(`Superseded by: ${worker.supersededByVersion}`, vscode.TreeItemCollapsibleState.None));
        }
        return items.length > 0
            ? items
            : [new BslOverviewItem('Worker: details unavailable', vscode.TreeItemCollapsibleState.None)];
    }

    private getSnapshotFailureItems(): BslOverviewItem[] {
        const status = this.currentSnapshotStatus();
        const failure = status?.lastFailure;
        if (!failure) {
            return [new BslOverviewItem('Last failure: none for current revision', vscode.TreeItemCollapsibleState.None)];
        }

        const items = [
            new BslOverviewItem(`Stage: ${failure.stage}`, vscode.TreeItemCollapsibleState.None),
            new BslOverviewItem(`Reason: ${sanitizeSnapshotDetail(failure.reason) ?? failure.reason}`, vscode.TreeItemCollapsibleState.None),
        ];
        if (failure.message) {
            items.push(new BslOverviewItem(`Message: ${sanitizeSnapshotDetail(failure.message) ?? failure.message}`, vscode.TreeItemCollapsibleState.None));
        }
        if (typeof failure.requestedVersion === 'number') {
            items.push(new BslOverviewItem(`Requested revision: ${failure.requestedVersion}`, vscode.TreeItemCollapsibleState.None));
        }
        if (typeof failure.occurredAtMs === 'number') {
            items.push(new BslOverviewItem(`Occurred: ${formatRelativeTime(failure.occurredAtMs)}`, vscode.TreeItemCollapsibleState.None));
        }
        return items;
    }

    private getSnapshotHistoryItems(): BslOverviewItem[] {
        const history = getActiveSnapshotStatusHistory();
        if (history.length === 0) {
            return [new BslOverviewItem('Recent transitions: none', vscode.TreeItemCollapsibleState.None)];
        }

        return history
            .slice()
            .reverse()
            .map((status) => {
                const label = `${formatRelativeTime(status.updatedAtMs)}: ${status.state} requested=${status.requestedVersion ?? 'n/a'} ready=${status.readyVersion ?? 'n/a'}`;
                return new BslOverviewItem(label, vscode.TreeItemCollapsibleState.None);
            });
    }

    private getSnapshotActionItems(): BslOverviewItem[] {
        const refreshItem = new BslOverviewItem('Refresh Snapshot Status', vscode.TreeItemCollapsibleState.None);
        refreshItem.command = {
            command: 'bslAnalyzer.refreshObservability',
            title: 'Refresh Snapshot Status',
        };
        refreshItem.iconPath = new vscode.ThemeIcon('refresh');

        const timelineItem = new BslOverviewItem('Open Completion Timeline', vscode.TreeItemCollapsibleState.None);
        timelineItem.command = {
            command: 'bslAnalyzer.refreshCompletionTimeline',
            title: 'Open Completion Timeline',
        };
        timelineItem.iconPath = new vscode.ThemeIcon('timeline-view-icon');

        const exportItem = new BslOverviewItem('Export Incident Bundle', vscode.TreeItemCollapsibleState.None);
        exportItem.command = {
            command: 'bslAnalyzer.exportObservabilityIncidentBundle',
            title: 'Export Observability Incident Bundle',
        };
        exportItem.iconPath = new vscode.ThemeIcon('export');

        return [refreshItem, timelineItem, exportItem];
    }

    private currentSnapshotStatus(): SnapshotStatusResponse | undefined {
        const snapshot = getActiveSnapshotStatusSnapshot();
        return snapshot.kind === 'ok' ? snapshot.status : undefined;
    }

    private snapshotArtifactItem(label: string, artifact: SnapshotArtifactStatus | undefined): BslOverviewItem {
        if (!artifact) {
            return new BslOverviewItem(`${label}: unknown`, vscode.TreeItemCollapsibleState.None);
        }

        const parts = [`${label}: ${artifact.state}`];
        if (typeof artifact.version === 'number') {
            parts.push(`v${artifact.version}`);
        }
        if (typeof artifact.ageMs === 'number') {
            parts.push(`age=${formatMs(artifact.ageMs)}`);
        }
        const detail = sanitizeSnapshotDetail(artifact.detail);
        if (detail) {
            parts.push(detail);
        }
        return new BslOverviewItem(parts.join(' | '), vscode.TreeItemCollapsibleState.None);
    }

    private async getLatencyItems(): Promise<BslOverviewItem[]> {
        const metrics = await this.loadMetrics();
        if (!metrics) {
            return [new BslOverviewItem('No latency metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const histograms = asRecord(metrics.histograms);
        if (!histograms) {
            return [new BslOverviewItem('No histogram data', vscode.TreeItemCollapsibleState.None)];
        }

        const items: BslOverviewItem[] = [];
        for (const metric of KEY_LATENCIES) {
            const point = asHistogram(histograms[`${metric.key}_ms`] ?? histograms[metric.key]);
            if (!point) {
                continue;
            }
            items.push(
                new BslOverviewItem(
                    `${metric.label}: p50=${formatMs(point.p50)} p95=${formatMs(point.p95)} p99=${formatMs(point.p99)} (n=${formatCount(point.count)})`,
                    vscode.TreeItemCollapsibleState.None
                )
            );
        }

        return items.length > 0
            ? items
            : [new BslOverviewItem('No key latency metrics', vscode.TreeItemCollapsibleState.None)];
    }

    private async getRateItems(): Promise<BslOverviewItem[]> {
        const metrics = await this.loadMetrics();
        if (!metrics) {
            return [new BslOverviewItem('No rate metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const rates = asRecord(metrics.rates);
        if (!rates || Object.keys(rates).length === 0) {
            return [new BslOverviewItem('No rate metrics', vscode.TreeItemCollapsibleState.None)];
        }

        return Object.entries(rates)
            .sort(([left], [right]) => left.localeCompare(right))
            .map(([name, value]) => {
                return new BslOverviewItem(
                    `${name}: ${formatRate(value)}`,
                    vscode.TreeItemCollapsibleState.None
                );
            });
    }

    private async getCounterItems(): Promise<BslOverviewItem[]> {
        const metrics = await this.loadMetrics();
        if (!metrics) {
            return [new BslOverviewItem('No counter metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const counters = asRecord(metrics.counters);
        if (!counters || Object.keys(counters).length === 0) {
            return [new BslOverviewItem('No counter metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const items = this.renderNumericEntries(counters, MAX_COUNTER_ITEMS, true);
        return items.length > 0
            ? items
            : [new BslOverviewItem('No numeric counters', vscode.TreeItemCollapsibleState.None)];
    }

    private async getGaugeItems(): Promise<BslOverviewItem[]> {
        const metrics = await this.loadMetrics();
        if (!metrics) {
            return [new BslOverviewItem('No gauge metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const gauges = asRecord(metrics.gauges);
        if (!gauges || Object.keys(gauges).length === 0) {
            return [new BslOverviewItem('No gauge metrics', vscode.TreeItemCollapsibleState.None)];
        }

        const items = this.renderNumericEntries(gauges, MAX_GAUGE_ITEMS, false);
        return items.length > 0
            ? items
            : [new BslOverviewItem('No numeric gauges', vscode.TreeItemCollapsibleState.None)];
    }

    private getActionItems(): Promise<BslOverviewItem[]> {
        const refreshItem = new BslOverviewItem('Refresh', vscode.TreeItemCollapsibleState.None);
        refreshItem.command = {
            command: 'bslAnalyzer.refreshObservability',
            title: 'Refresh Observability',
        };
        refreshItem.iconPath = new vscode.ThemeIcon('refresh');

        const toggleLabel = this.autoRefreshEnabled ? 'Disable Auto Refresh' : 'Enable Auto Refresh';
        const toggleItem = new BslOverviewItem(toggleLabel, vscode.TreeItemCollapsibleState.None);
        toggleItem.command = {
            command: 'bslAnalyzer.toggleObservabilityAutoRefresh',
            title: toggleLabel,
        };
        toggleItem.iconPath = new vscode.ThemeIcon(this.autoRefreshEnabled ? 'circle-slash' : 'play');

        const compactToggleLabel = this.compactModeEnabled ? 'Disable Compact Mode' : 'Enable Compact Mode';
        const compactToggleItem = new BslOverviewItem(compactToggleLabel, vscode.TreeItemCollapsibleState.None);
        compactToggleItem.command = {
            command: 'bslAnalyzer.toggleObservabilityCompactMode',
            title: compactToggleLabel,
        };
        compactToggleItem.iconPath = new vscode.ThemeIcon(this.compactModeEnabled ? 'list-tree' : 'list-flat');

        const exportItem = new BslOverviewItem('Export Incident Bundle', vscode.TreeItemCollapsibleState.None);
        exportItem.command = {
            command: 'bslAnalyzer.exportObservabilityIncidentBundle',
            title: 'Export Observability Incident Bundle',
        };
        exportItem.iconPath = new vscode.ThemeIcon('export');

        const dumpItem = new BslOverviewItem('Dump Raw Metrics to Output', vscode.TreeItemCollapsibleState.None);
        dumpItem.command = {
            command: 'bslAnalyzer.dumpLspMetrics',
            title: 'Dump LSP Metrics',
        };
        dumpItem.iconPath = new vscode.ThemeIcon('output');

        return Promise.resolve([refreshItem, toggleItem, compactToggleItem, exportItem, dumpItem]);
    }

    private async loadMetrics(): Promise<Record<string, unknown> | null> {
        if (getServerStatusSnapshot().loading) {
            return this.cachedMetrics;
        }

        const now = Date.now();
        if (now < this.nextFetchAllowedAt) {
            return this.cachedMetrics;
        }

        if (!this.inflight) {
            this.inflight = getObservabilityMetricsWithRequest({ shape: 'sidebar' })
                .then((response) => {
                    const metrics = asRecord(response?.metrics);
                    this.lastFetchAt = Date.now();
                    if (metrics) {
                        this.lastFetchHadUsableMetrics = true;
                        this.cachedMetrics = metrics;
                        this.lastSuccessAt = this.lastFetchAt;
                        this.nextFetchAllowedAt = this.lastFetchAt + this.getFetchCacheWindowMs();
                        return metrics;
                    }
                    this.lastFetchHadUsableMetrics = false;
                    this.nextFetchAllowedAt = this.lastFetchAt + this.getFetchCacheWindowMs();
                    return this.cachedMetrics;
                })
                .catch((error) => {
                    this.lastFetchAt = Date.now();
                    this.lastFetchHadUsableMetrics = false;
                    this.nextFetchAllowedAt = this.lastFetchAt + this.getFetchCacheWindowMs();
                    this.outputChannel.appendLine(`[Observability] Failed to fetch metrics: ${error}`);
                    return this.cachedMetrics;
                })
                .finally(() => {
                    this.inflight = null;
                });
        }

        return this.inflight;
    }

    private reloadConfig(): void {
        this.autoRefreshEnabled = BslAnalyzerConfig.observabilityAutoRefresh;
        this.autoRefreshIntervalMs = clamp(
            BslAnalyzerConfig.observabilityRefreshMs,
            1000,
            60000
        );
        this.compactModeEnabled = BslAnalyzerConfig.observabilityCompactMode;
        this.startAutoRefresh();
    }

    private startAutoRefresh(): void {
        this.stopAutoRefresh();
        if (!this.autoRefreshEnabled) {
            return;
        }

        this.autoRefreshTimer = setInterval(() => {
            this.refresh(false);
        }, this.autoRefreshIntervalMs);
    }

    private getFetchCacheWindowMs(): number {
        return this.lastFetchHadUsableMetrics
            ? this.ttlMs
            : Math.max(this.ttlMs, this.autoRefreshIntervalMs);
    }

    private stopAutoRefresh(): void {
        if (this.autoRefreshTimer) {
            clearInterval(this.autoRefreshTimer);
            this.autoRefreshTimer = null;
        }
    }

    private renderNumericEntries(
        source: Record<string, unknown>,
        maxItems: number,
        sortByValueDesc: boolean
    ): BslOverviewItem[] {
        const numericEntries = Object.entries(source)
            .map(([name, value]) => [name, asNumber(value)] as const)
            .filter(([, value]) => value !== null) as Array<[string, number]>;

        if (sortByValueDesc) {
            numericEntries.sort((left, right) => {
                if (right[1] !== left[1]) {
                    return right[1] - left[1];
                }
                return left[0].localeCompare(right[0]);
            });
        } else {
            numericEntries.sort(([left], [right]) => left.localeCompare(right));
        }

        const shown = numericEntries.slice(0, maxItems);
        const items = shown.map(([name, value]) => {
            return new BslOverviewItem(
                `${name}: ${formatNumber(value)}`,
                vscode.TreeItemCollapsibleState.None
            );
        });

        if (numericEntries.length > shown.length) {
            const hidden = numericEntries.length - shown.length;
            const tail = new BslOverviewItem(
                `... +${hidden} more`,
                vscode.TreeItemCollapsibleState.None
            );
            tail.tooltip = 'Use "Dump Raw Metrics to Output" for full snapshot.';
            items.push(tail);
        }

        return items;
    }
}

function asRecord(value: unknown): Record<string, unknown> | null {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return null;
    }
    return value as Record<string, unknown>;
}

function asHistogram(value: unknown): HistogramPoint | null {
    const record = asRecord(value);
    if (!record) {
        return null;
    }
    return {
        p50: asNumber(record.p50) ?? undefined,
        p95: asNumber(record.p95) ?? undefined,
        p99: asNumber(record.p99) ?? undefined,
        count: asNumber(record.count) ?? undefined,
    };
}

function asNumber(value: unknown): number | null {
    return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function formatMs(value: number | undefined): string {
    if (value === undefined) {
        return '-';
    }
    return `${Math.round(value)} ms`;
}

function formatCount(value: number | undefined): string {
    if (value === undefined) {
        return '?';
    }
    return formatNumber(value);
}

function formatRate(value: unknown): string {
    const numeric = asNumber(value);
    if (numeric === null) {
        return String(value);
    }
    if (numeric >= 0 && numeric <= 1) {
        return `${(numeric * 100).toFixed(2)}%`;
    }
    return formatNumber(numeric);
}

function formatNumber(value: number): string {
    return Number.isInteger(value) ? value.toString() : value.toFixed(2);
}

function formatDurationSeconds(value: number): string {
    const totalSeconds = Math.max(0, Math.floor(value));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    if (hours > 0) {
        return `${hours}h ${minutes}m ${seconds}s`;
    }
    if (minutes > 0) {
        return `${minutes}m ${seconds}s`;
    }
    return `${seconds}s`;
}

function formatRelativeTime(timestampMs: number): string {
    const diffMs = Date.now() - timestampMs;
    if (diffMs < 1000) {
        return 'just now';
    }
    if (diffMs < 60000) {
        return `${Math.floor(diffMs / 1000)}s ago`;
    }
    if (diffMs < 3600000) {
        return `${Math.floor(diffMs / 60000)}m ago`;
    }
    return `${Math.floor(diffMs / 3600000)}h ago`;
}

function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
}
