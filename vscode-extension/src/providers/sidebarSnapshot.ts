import * as path from 'path';
import * as vscode from 'vscode';
import { BslAnalyzerConfig } from '../config/configHelper';
import {
    CacheStatsResponse,
    TypeRepositoryStats,
    WorkspaceStatsResponse,
    getCacheStats,
    getTypeRepositoryStats,
    getWorkspaceStats,
} from '../lsp/customRequests';

const SNAPSHOT_TTL_MS = 1500;

export interface SidebarTypeRepositoryStats {
    totalTypes: number;
    platformTypes: number;
    configurationTypes: number;
    lastUpdateTime?: string;
    status: 'live' | 'n/a';
}

export interface SidebarDiagnosticsStats {
    total: number;
    errors: number;
    warnings: number;
    infos: number;
    hints: number;
}

export interface SidebarDiagnosticEntry {
    uri: vscode.Uri;
    diagnostic: vscode.Diagnostic;
}

export interface SidebarDiagnosticsSnapshot {
    stats: SidebarDiagnosticsStats;
    entries: SidebarDiagnosticEntry[];
}

export interface SidebarCacheSnapshot {
    status: 'live' | 'n/a';
    reason?: 'configuration_path_missing' | 'lsp_unavailable';
    stats: CacheStatsResponse | null;
}

export interface SidebarUiHealthSnapshot {
    workspace: 'live' | 'fallback';
    typeRepository: 'live' | 'n/a';
    cache: 'live' | 'n/a';
}

export interface SidebarSnapshot {
    generatedAt: string;
    workspace: {
        bslFiles: number;
        source: 'lsp' | 'fallback';
    };
    typeRepository: SidebarTypeRepositoryStats;
    diagnostics: SidebarDiagnosticsStats;
    diagnosticsSnapshot: SidebarDiagnosticsSnapshot;
    cache: SidebarCacheSnapshot;
    uiHealth: SidebarUiHealthSnapshot;
}

let cachedSnapshot: SidebarSnapshot | null = null;
let cachedAtMs = 0;
let inflightSnapshot: Promise<SidebarSnapshot> | null = null;

function isBslDocument(uri: vscode.Uri): boolean {
    if (uri.scheme !== 'file') {
        return false;
    }
    const ext = path.extname(uri.fsPath).toLowerCase();
    return ext === '.bsl' || ext === '.os';
}

export function countDiagnosticsBySeverity(diagnostics: readonly vscode.Diagnostic[]): SidebarDiagnosticsStats {
    const stats: SidebarDiagnosticsStats = {
        total: 0,
        errors: 0,
        warnings: 0,
        infos: 0,
        hints: 0,
    };

    for (const diagnostic of diagnostics) {
        stats.total += 1;
        switch (diagnostic.severity) {
            case vscode.DiagnosticSeverity.Error:
                stats.errors += 1;
                break;
            case vscode.DiagnosticSeverity.Warning:
                stats.warnings += 1;
                break;
            case vscode.DiagnosticSeverity.Information:
                stats.infos += 1;
                break;
            case vscode.DiagnosticSeverity.Hint:
                stats.hints += 1;
                break;
        }
    }

    return stats;
}

export function collectBslDiagnosticsSnapshot(): SidebarDiagnosticsSnapshot {
    const entries: SidebarDiagnosticEntry[] = [];
    const all = vscode.languages.getDiagnostics();
    for (const [uri, diagnostics] of all) {
        if (!isBslDocument(uri)) {
            continue;
        }
        for (const diagnostic of diagnostics) {
            entries.push({ uri, diagnostic });
        }
    }

    return {
        stats: countDiagnosticsBySeverity(entries.map((entry) => entry.diagnostic)),
        entries,
    };
}

function toTypeRepositoryStats(repoStats: TypeRepositoryStats | null): SidebarTypeRepositoryStats {
    if (!repoStats) {
        return {
            totalTypes: 0,
            platformTypes: 0,
            configurationTypes: 0,
            status: 'n/a',
        };
    }

    return {
        totalTypes: repoStats.totalTypes ?? 0,
        platformTypes: repoStats.platformTypes ?? 0,
        configurationTypes: repoStats.configurationTypes ?? 0,
        lastUpdateTime: repoStats.lastUpdateTime,
        status: 'live',
    };
}

function inferWorkspaceFiles(
    workspaceStats: WorkspaceStatsResponse | null,
    diagnosticsSnapshot: SidebarDiagnosticsSnapshot
): { bslFiles: number; source: 'lsp' | 'fallback' } {
    if (workspaceStats) {
        return {
            bslFiles: workspaceStats.bslFiles ?? 0,
            source: 'lsp',
        };
    }

    const uniqueBslFiles = new Set(
        diagnosticsSnapshot.entries.map((entry) => entry.uri.toString())
    );
    return {
        bslFiles: uniqueBslFiles.size,
        source: 'fallback',
    };
}

function toCacheSnapshot(
    configurationPath: string | undefined,
    cacheStats: CacheStatsResponse | null
): SidebarCacheSnapshot {
    if (!configurationPath || configurationPath.trim().length === 0) {
        return {
            status: 'n/a',
            reason: 'configuration_path_missing',
            stats: null,
        };
    }

    if (!cacheStats) {
        return {
            status: 'n/a',
            reason: 'lsp_unavailable',
            stats: null,
        };
    }

    return {
        status: 'live',
        stats: cacheStats,
    };
}

export function invalidateSidebarSnapshot(): void {
    cachedSnapshot = null;
    cachedAtMs = 0;
}

async function buildSidebarSnapshot(): Promise<SidebarSnapshot> {
    const diagnosticsSnapshot = collectBslDiagnosticsSnapshot();
    const configurationPath = BslAnalyzerConfig.configurationPath?.trim();
    const [repoStats, workspaceStats, cacheStats] = await Promise.all([
        getTypeRepositoryStats(),
        getWorkspaceStats(),
        configurationPath ? getCacheStats(configurationPath) : Promise.resolve(null),
    ]);
    const workspace = inferWorkspaceFiles(workspaceStats, diagnosticsSnapshot);
    const typeRepository = toTypeRepositoryStats(repoStats);
    const cache = toCacheSnapshot(configurationPath, cacheStats);

    return {
        generatedAt: new Date().toISOString(),
        workspace,
        typeRepository,
        diagnostics: diagnosticsSnapshot.stats,
        diagnosticsSnapshot,
        cache,
        uiHealth: {
            workspace: workspace.source === 'lsp' ? 'live' : 'fallback',
            typeRepository: typeRepository.status,
            cache: cache.status,
        },
    };
}

export async function getSidebarSnapshot(forceRefresh = false): Promise<SidebarSnapshot> {
    const now = Date.now();
    if (!forceRefresh && cachedSnapshot && now - cachedAtMs < SNAPSHOT_TTL_MS) {
        return cachedSnapshot;
    }

    if (!forceRefresh && inflightSnapshot) {
        return inflightSnapshot;
    }

    inflightSnapshot = buildSidebarSnapshot()
        .then((snapshot) => {
            cachedSnapshot = snapshot;
            cachedAtMs = Date.now();
            return snapshot;
        })
        .finally(() => {
            inflightSnapshot = null;
        });

    return inflightSnapshot;
}
