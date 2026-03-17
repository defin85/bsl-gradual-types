/**
 * LSP Custom Requests - заменяют CLI бинарники
 *
 * Все запросы идут через LSP server вместо fork процессов
 */

import { logger } from './logger';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface QueryTypeParams {
    type_name: string;
}

// ============================================================================
// Query Type Response (Расширенная версия для Type Details Modal)
// ============================================================================

export interface MethodInfo {
    name: string;
    englishName?: string;
    returnType?: string;
    params: ParamInfo[];
    description?: string;
    isDeprecated: boolean;
    isConstructor: boolean;
}

export interface ParamInfo {
    name: string;
    paramType: string;
    isOptional: boolean;
    defaultValue?: string;
}

export interface PropertyInfo {
    name: string;
    propType: string;
    isReadonly: boolean;
    description?: string;
}

export interface QueryTypeResponse {
    typeName: string;       // camelCase (serde rename_all)
    found: boolean;

    // Базовая информация о типе
    certainty?: string;     // "Known (100%)", "Inferred (50%)"
    facet?: string;         // "Manager", "Object", "Reference"
    description?: string;

    // Полная документация типа
    methods?: MethodInfo[];
    properties?: PropertyInfo[];
    facets?: string[];

    // Обратная совместимость (deprecated)
    details?: string;
}

// ============================================================================
// Other Request/Response Types
// ============================================================================

export interface BuildIndexParams {
    workspace_path: string;
}

export interface BuildIndexResponse {
    success: boolean;
    types_count: number;
    message: string;
}

export interface GetIndexStateParams {}

export type IndexStateKind = 'idle' | 'running' | 'ready' | 'failed';

export interface GetIndexStateResponse {
    version: number;
    state: IndexStateKind;
    ready: boolean;
    active_operation: 'startup' | 'buildIndex' | null;
    operation_id: string | null;
    message: string | null;
    updated_at_ms: number;
}

export interface ValidateMethodParams {
    object_type: string;
    method_name: string;
    arguments: string[];
}

export interface ValidateMethodResponse {
    valid: boolean;
    message: string;
}

export interface CheckCompatibilityParams {
    source_type: string;
    target_type: string;
}

export interface CheckCompatibilityResponse {
    compatible: boolean;
    message: string;
}

export interface IncrementalUpdateParams {
    config_path: string;
    platform_version: string;
    changed_paths?: string[];
    is_auto?: boolean;
}

export interface IncrementalUpdateResponse {
    success: boolean;
    message: string;
}

export interface AutoReindexStateResponse {
    success: boolean;
    paused: boolean;
    message: string;
}

export interface ExtractPlatformDocsParams {
    archive_path: string;
    platform_version: string;
    force: boolean;
}

export interface ExtractPlatformDocsResponse {
    success: boolean;
    types_count: number;
    message: string;
}

export interface CacheScopeDto {
    project_id: string;
    config_set_id: string;
    config_ids: string[];
}

export interface DiskCacheRuntimeStats {
    hit_count: number;
    miss_count: number;
    stale_hit_count: number;
    load_time_ms_total: number;
    build_time_ms_total: number;
    stored_entries: number;
    expired_entries: number;
    evicted_entries: number;
}

export interface DiskCacheScopeStats {
    entries: number;
    size_bytes: number;
}

export interface DiskCacheStatsReport {
    runtime: DiskCacheRuntimeStats;
    scope: DiskCacheScopeStats;
}

export interface AstCacheStats {
    hits: number;
    misses: number;
    evictions: number;
    entries: number;
    capacity: number;
}

export interface IrStats {
    hits: number;
    misses: number;
    evictions: number;
}

export interface CacheStatsResponse {
    cache_enabled: boolean;
    env_disabled: boolean;
    swr_enabled: boolean;
    cache_root: string;
    scope: CacheScopeDto;
    disk: DiskCacheStatsReport;
    ast: AstCacheStats;
    // Backward/forward compatibility: some LSP versions don't report IR cache stats yet.
    ir?: IrStats;
}

// ============================================================================
// Search Types (LSP Integration для Quick Actions)
// ============================================================================

export interface SearchTypesRequest {
    query: string;
    limit?: number;
}

export interface TypeSearchResult {
    name: string;
    english_name?: string;
    facet: string;
    certainty: string;
    description?: string;
}

export interface SearchTypesResponse {
    types: TypeSearchResult[];
    total: number;
}

// ============================================================================
// Helper Functions - прямые вызовы LSP custom requests
// ============================================================================

/**
 * Запрос информации о типе через LSP
 * Заменяет: executeBslCommand('query_type', ...)
 */
export async function queryType(typeName: string): Promise<QueryTypeResponse> {
    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        throw new Error('LSP client not available');
    }

    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.queryType',  // ← Execute Command (как searchTypes)
            arguments: [{
                type_name: typeName
            }]
        });
        return result as QueryTypeResponse;
    } catch (error) {
        logger.error('Failed to query type via LSP', error);
        throw error;
    }
}

/**
 * Построение индекса типов через LSP
 * Заменяет: executeBslCommand('build_unified_index', ...)
 */
export async function buildIndex(params: BuildIndexParams): Promise<BuildIndexResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<BuildIndexResponse>('bsl/buildIndex', params);
}

/**
 * Текущее состояние full-index на стороне LSP (server-driven source of truth)
 */
export async function getIndexState(
    params: GetIndexStateParams = {}
): Promise<GetIndexStateResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<GetIndexStateResponse>('bsl/getIndexState', params);
}

/**
 * Совместимость с legacy LSP, где `bsl/getIndexState` может отсутствовать.
 */
export function isMethodNotFoundError(error: unknown): boolean {
    const asRecord = (value: unknown): Record<string, unknown> | null => {
        if (typeof value !== 'object' || value === null) {
            return null;
        }
        return value as Record<string, unknown>;
    };

    const readCode = (value: unknown): number | undefined => {
        const record = asRecord(value);
        if (!record) {
            return undefined;
        }
        const code = record.code;
        return typeof code === 'number' ? code : undefined;
    };

    const directCode = readCode(error);
    if (directCode === -32601) {
        return true;
    }

    const errorRecord = asRecord(error);
    const nestedErrorCode = readCode(errorRecord?.error);
    if (nestedErrorCode === -32601) {
        return true;
    }

    const message = error instanceof Error ? error.message : String(error);
    return /method not found/i.test(message);
}

function isTimeoutError(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error);
    return /timed out/i.test(message);
}

async function withRequestTimeout<T>(
    request: Promise<T>,
    timeoutMs: number,
    label: string
): Promise<T> {
    let timeoutHandle: NodeJS.Timeout | undefined;
    try {
        return await Promise.race([
            request,
            new Promise<T>((_, reject) => {
                timeoutHandle = setTimeout(() => {
                    reject(new Error(`${label} timed out after ${timeoutMs}ms`));
                }, timeoutMs);
            })
        ]);
    } finally {
        if (timeoutHandle) {
            clearTimeout(timeoutHandle);
        }
    }
}

/**
 * Валидация вызова метода через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
export async function validateMethod(
    objectType: string,
    methodName: string,
    args: string[]
): Promise<ValidateMethodResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<ValidateMethodResponse>('bsl/validateMethod', {
        object_type: objectType,
        method_name: methodName,
        arguments: args
    });
}

/**
 * Проверка совместимости типов через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
export async function checkTypeCompatibility(
    sourceType: string,
    targetType: string
): Promise<CheckCompatibilityResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<CheckCompatibilityResponse>('bsl/checkTypeCompatibility', {
        source_type: sourceType,
        target_type: targetType
    });
}

/**
 * Анализ файла через LSP (уже работает через textDocument/didOpen)
 * Заменяет: executeBslCommand('bsl-analyzer', ...)
 */
export async function analyzeFile(filePath: string): Promise<void> {
    // Файл анализируется автоматически при открытии через LSP
    // Дополнительно можно отправить custom request если нужно
    logger.debug(`File ${filePath} will be analyzed via LSP textDocument/didOpen`);
}

/**
 * Инкрементальное обновление индекса через LSP
 * Заменяет: executeBslCommand('incremental_update', ...)
 */
export async function incrementalUpdate(
    configPath: string,
    platformVersion: string,
    changedPaths?: string[],
    isAuto?: boolean
): Promise<IncrementalUpdateResponse> {
    const { sendCustomRequest } = await import('./client');
    const params: IncrementalUpdateParams = {
        config_path: configPath,
        platform_version: platformVersion,
        changed_paths: changedPaths
    };
    if (isAuto !== undefined) {
        params.is_auto = isAuto;
    }
    return await sendCustomRequest<IncrementalUpdateResponse>('bsl/incrementalUpdate', params);
}

/**
 * Пауза авто-реиндексации через LSP
 */
export async function pauseAutoReindex(): Promise<AutoReindexStateResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<AutoReindexStateResponse>('bsl/pauseAutoReindex', {});
}

/**
 * Возобновление авто-реиндексации через LSP
 */
export async function resumeAutoReindex(): Promise<AutoReindexStateResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<AutoReindexStateResponse>('bsl/resumeAutoReindex', {});
}

/**
 * Извлечение платформенной документации через LSP
 * Заменяет: executeBslCommand('extract_platform_docs', ...)
 */
export async function extractPlatformDocs(
    archivePath: string,
    platformVersion: string,
    force: boolean = false
): Promise<ExtractPlatformDocsResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<ExtractPlatformDocsResponse>('bsl/extractPlatformDocs', {
        archive_path: archivePath,
        platform_version: platformVersion,
        force
    });
}

/**
 * Поиск типов в TypeRepository через LSP
 * Заменяет: mock данные в Quick Actions Webview
 *
 * @param query - поисковый запрос (partial match, case-insensitive)
 * @param limit - максимум результатов (по умолчанию 15)
 * @returns массив найденных типов
 */
export async function searchTypes(
    query: string,
    limit?: number
): Promise<SearchTypesResponse> {
    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        throw new Error('LSP client not available');
    }

    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.searchTypes',
            arguments: [{
                query,
                limit: limit || 15
            }]
        });

        return result as SearchTypesResponse;
    } catch (error) {
        logger.error('Failed to search types via LSP', error);
        throw error;
    }
}

// ============================================================================
// MILESTONE 2.20.4: Type Repository Statistics & Get All Types
// ============================================================================

/**
 * Статистика TypeRepository
 */
export interface TypeRepositoryStats {
    totalTypes: number;
    platformTypes: number;
    configurationTypes: number;
    lastUpdateTime?: string;  // ISO 8601 timestamp
}

/**
 * Статистика workspace (файлы и диагностика)
 */
export interface WorkspaceStatsResponse {
    bslFiles: number;
    diagnostics: number;
}

/**
 * Снимок метрик наблюдаемости (counters/gauges/histograms)
 * Формат соответствует `SimpleMetrics.export_metrics()` на стороне Rust.
 */
export interface ObservabilityMetricsResponse {
    metrics: any;
}

export interface ObservabilityMetricsRequest {
    shape?: 'full' | 'sidebar';
}

export type CompletionTimelineStageStatus = 'completed' | 'cancelled' | 'failed' | 'skipped';

export interface CompletionTimelineRequest {
    limit?: number;
    request_id?: string;
}

export interface CompletionTimelineStageTrace {
    name: string;
    status: CompletionTimelineStageStatus;
    started_offset_ms: number;
    duration_ms: number;
}

export interface CompletionTimelineTurnHolderTrace {
    request_id?: string;
    file_seq: number;
    request_epoch: number;
    trigger_mode: string;
    version_hint?: number;
    age_ms: number;
}

export interface CompletionTimelineTurnAttributionTrace {
    request_file_seq: number;
    request_epoch: number;
    queue_outcome: string;
    turn_wait_outcome?: string;
    queue_capacity: number;
    queue_depth_before_enqueue: number;
    queue_depth_after_enqueue: number;
    queued_completion_ahead_count: number;
    did_change_ahead_count: number;
    active_completion_count: number;
    dropped_completion_file_seq: number[];
    active_holder?: CompletionTimelineTurnHolderTrace;
    queued_completion_ahead?: CompletionTimelineTurnHolderTrace;
}

export interface CompletionTimelinePrepareDetailsTrace {
    wait_budget_ms?: number;
    guard_outcome?: string;
    outcome?: string;
    route?: string | null;
    fail_closed_cause?: string | null;
    min_file_version?: number;
    shadow_version_at_start?: number;
    observed_file_version?: number;
    wait_elapsed_ms?: number;
    snapshot_elapsed_ms?: number;
    apply_age_at_start_ms?: number;
    apply_age_at_terminal_ms?: number;
}

export interface CompletionTimelineTrace {
    trace_id: string;
    request_id?: string;
    uri: string;
    trigger_mode: string;
    outcome: string;
    started_at_ms: number;
    total_duration_ms: number;
    dominant_stage?: string;
    prepare_details?: CompletionTimelinePrepareDetailsTrace;
    turn_attribution?: CompletionTimelineTurnAttributionTrace;
    stages: CompletionTimelineStageTrace[];
}

export interface CompletionTimelineResponse {
    version: number;
    traces: CompletionTimelineTrace[];
}

export type CompletionTimelineFetchResult =
    | { kind: 'ok'; response: CompletionTimelineResponse }
    | { kind: 'unsupported' }
    | { kind: 'error'; message: string };

/**
 * Параметры запроса всех типов
 */
export interface GetAllTypesRequest {
    limit?: number;
    offset?: number;
    category?: string;
}

/**
 * Детальная информация о типе (соответствует TypeDto из Rust)
 */
export interface TypeDto {
    name: string;
    englishName?: string;
    description?: string;
    category: string;
    source: string;  // "Platform" | "Configuration"
    methods: MethodDto[];
    properties: string[];
    tabularSections?: TabularSectionDto[];
}

/**
 * Информация о методе (соответствует MethodDto из Rust)
 */
export interface MethodDto {
    name: string;
    englishName?: string;
    returnType: string;
    parameters: ParameterDto[];
    description?: string;
}

/**
 * Информация о параметре (соответствует ParameterDto из Rust)
 */
export interface ParameterDto {
    name: string;
    typeName: string;
    isOptional: boolean;
    defaultValue?: string;
}

/**
 * Информация о табличной части (соответствует TabularSectionDto из Rust)
 */
export interface TabularSectionDto {
    name: string;
    attributes: string[];
}

/**
 * Информация о категории типов
 */
export interface CategoryDto {
    name: string;
    displayName: string;
    count: number;
}

/**
 * Ответ на запрос всех типов
 */
export interface GetAllTypesResponse {
    types: TypeDto[];
    categories: Record<string, CategoryDto>;
    totalCount: number;
}

/**
 * Получить статистику TypeRepository из LSP Server
 *
 * @returns Статистика или null если LSP недоступен
 */
export async function getTypeRepositoryStats(): Promise<TypeRepositoryStats | null> {
    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        logger.warn('[Type Stats] LSP client not available');
        return null;
    }

    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getTypeRepositoryStats',
            arguments: [{}]
        });
        return result as TypeRepositoryStats || null;
    } catch (error) {
        logger.error('Failed to get type repository stats', error);
        return null;
    }
}

/**
 * Получить статистику workspace через LSP Server
 */
let workspaceStatsUnsupported = false;
let workspaceStatsUnsupportedNotified = false;

export async function getWorkspaceStats(): Promise<WorkspaceStatsResponse | null> {
    if (workspaceStatsUnsupported) {
        return null;
    }

    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        logger.warn('[Workspace Stats] LSP client not available');
        return null;
    }

    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getWorkspaceStats',
            arguments: [{}]
        });
        return result as WorkspaceStatsResponse || null;
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (message.includes('Method not found')) {
            workspaceStatsUnsupported = true;
            if (!workspaceStatsUnsupportedNotified) {
                workspaceStatsUnsupportedNotified = true;
                logger.warn('[Workspace Stats] LSP server does not support getWorkspaceStats yet');
                const vscode = await import('vscode');
                vscode.window.showWarningMessage(
                    'BSL Analyzer: LSP server does not support workspace stats yet. Please обновите бинарник.'
                );
            }
            return null;
        }
        logger.error('Failed to get workspace stats', error);
        return null;
    }
}

/**
 * Получить снимок метрик observability из LSP сервера (для диагностики "затыков").
 */
let observabilityMetricsUnsupported = false;
let observabilityMetricsUnsupportedNotified = false;
const OBSERVABILITY_METRICS_TIMEOUT_MS = 1500;

export async function getObservabilityMetrics(): Promise<ObservabilityMetricsResponse | null> {
    return getObservabilityMetricsWithRequest();
}

export async function getObservabilityMetricsWithRequest(
    request: ObservabilityMetricsRequest = {}
): Promise<ObservabilityMetricsResponse | null> {
    if (observabilityMetricsUnsupported) {
        return null;
    }

    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        logger.warn('[Observability] LSP client not available');
        return null;
    }

    try {
        const args = Object.keys(request).length > 0 ? [request] : [];
        const result = await withRequestTimeout(
            client.sendRequest('workspace/executeCommand', {
                command: 'bsl.getObservabilityMetrics',
                arguments: args
            }),
            OBSERVABILITY_METRICS_TIMEOUT_MS,
            'Observability request'
        );
        return result as ObservabilityMetricsResponse || null;
    } catch (error) {
        if (isMethodNotFoundError(error)) {
            observabilityMetricsUnsupported = true;
            if (!observabilityMetricsUnsupportedNotified) {
                observabilityMetricsUnsupportedNotified = true;
                logger.warn('[Observability] LSP server does not support getObservabilityMetrics yet');
                const vscode = await import('vscode');
                vscode.window.showWarningMessage(
                    'BSL Analyzer: LSP server does not support observability metrics yet. Please обновите бинарник.'
                );
            }
            return null;
        }
        if (isTimeoutError(error)) {
            logger.warn(
                `[Observability] Request timed out after ${OBSERVABILITY_METRICS_TIMEOUT_MS}ms`
            );
            return null;
        }
        logger.error('Failed to get observability metrics', error);
        return null;
    }
}

let completionTimelineUnsupported = false;

/**
 * Сброс кэша совместимости timeline-контракта (используется только в тестах).
 */
export function resetCompletionTimelineSupportCacheForTests(): void {
    completionTimelineUnsupported = false;
}

/**
 * Получить server-driven per-request completion timeline через executeCommand.
 */
export async function getCompletionTimeline(
    request: CompletionTimelineRequest = {}
): Promise<CompletionTimelineFetchResult> {
    if (completionTimelineUnsupported) {
        return { kind: 'unsupported' };
    }

    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        return { kind: 'error', message: 'LSP client not available' };
    }

    const args = Object.keys(request).length > 0 ? [request] : [];
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getCompletionTimeline',
            arguments: args
        });
        if (!result || typeof result !== 'object') {
            return { kind: 'error', message: 'Invalid completion timeline response' };
        }
        return { kind: 'ok', response: result as CompletionTimelineResponse };
    } catch (error) {
        if (isMethodNotFoundError(error)) {
            completionTimelineUnsupported = true;
            logger.warn('[Completion Timeline] LSP server does not support bsl.getCompletionTimeline');
            return { kind: 'unsupported' };
        }
        const message = error instanceof Error ? error.message : String(error);
        logger.error('Failed to get completion timeline', error);
        return { kind: 'error', message };
    }
}

export async function getCacheStats(
    configurationPath: string
): Promise<CacheStatsResponse | null> {
    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        logger.warn('[Cache Stats] LSP client not available');
        return null;
    }

    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.cache.getStats',
            arguments: [{ configurationPath }]
        });
        return result as CacheStatsResponse || null;
    } catch (error) {
        logger.error('Failed to get cache stats', error);
        return null;
    }
}

/**
 * Получить все типы из TypeRepository через LSP Server
 *
 * @param params - Параметры запроса (limit, offset, category)
 * @returns Список типов с метаданными или null если LSP недоступен
 */
export async function getAllTypes(params?: GetAllTypesRequest): Promise<GetAllTypesResponse | null> {
    const client = (await import('./client')).getLanguageClient();
    if (!client) {
        logger.warn('[Get All Types] LSP client not available');
        return null;
    }

    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getAllTypes',
            arguments: params ? [params] : []
        });
        return result as GetAllTypesResponse || null;
    } catch (error) {
        logger.error('Failed to get all types', error);
        return null;
    }
}
