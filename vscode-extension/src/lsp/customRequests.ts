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

export interface SnapshotStatusRequest {
    uri: string;
}

export interface PrimeExactTypeIndexRequest {
    uri: string;
    requestedVersion?: number;
    reason?: string;
}

export type SnapshotReadinessState =
    | 'idle'
    | 'building'
    | 'ready'
    | 'stale'
    | 'shadow_only'
    | 'failed';

export type SnapshotTaskState =
    | 'absent'
    | 'in_flight_same_revision'
    | 'in_flight_other_revision'
    | 'ready_same_revision'
    | 'ready_stale_revision'
    | 'not_applicable';

export type SnapshotPhase = 'waiting' | 'parsing' | 'materializing';

export type SnapshotTrigger =
    | 'did_open'
    | 'did_change'
    | 'did_save'
    | 'current_context'
    | 'documents_set'
    | 'job';

export interface SnapshotStatusResponse {
    schemaVersion: number;
    uri?: string;
    path?: string;
    sessionId?: string;
    requestedVersion?: number;
    readyVersion?: number;
    analysisRevision?: number;
    state: SnapshotReadinessState;
    exact: boolean;
    taskState: SnapshotTaskState;
    phase?: SnapshotPhase;
    trigger?: SnapshotTrigger;
    updatedAtMs: number;
    fallbackReason?: string;
}

export type SnapshotStatusFetchResult =
    | { kind: 'ok'; response: SnapshotStatusResponse }
    | { kind: 'unsupported' }
    | { kind: 'error'; message: string };

export interface PrimeExactTypeIndexResponse {
    accepted: boolean;
    alreadyReady: boolean;
    observedVersion?: number;
    action: string;
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
    const client = (await import('./client/index')).getLanguageClient();
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
    const { sendCustomRequest } = await import('./client/index');
    return await sendCustomRequest<BuildIndexResponse>('bsl/buildIndex', params);
}

/**
 * Текущее состояние full-index на стороне LSP (server-driven source of truth)
 */
export async function getIndexState(
    params: GetIndexStateParams = {}
): Promise<GetIndexStateResponse> {
    const { sendCustomRequest } = await import('./client/index');
    return await sendCustomRequest<GetIndexStateResponse>('bsl/getIndexState', params);
}

let snapshotStatusUnsupported = false;
let primeExactTypeIndexUnsupported = false;

export function resetSnapshotStatusCapabilityCacheForTests(): void {
    snapshotStatusUnsupported = false;
}

export function resetPrimeExactTypeIndexCapabilityCacheForTests(): void {
    primeExactTypeIndexUnsupported = false;
}

export async function getSnapshotStatusFetchResult(
    request: SnapshotStatusRequest
): Promise<SnapshotStatusFetchResult> {
    if (snapshotStatusUnsupported) {
        return { kind: 'unsupported' };
    }

    const { sendCustomRequest } = await import('./client/index');
    try {
        const response = await sendCustomRequest<SnapshotStatusResponse>(
            'bsl/getSnapshotStatus',
            request
        );
        return { kind: 'ok', response };
    } catch (error) {
        if (isMethodNotFoundError(error)) {
            snapshotStatusUnsupported = true;
            logger.warn('[Snapshot Status] LSP server does not support bsl/getSnapshotStatus');
            return { kind: 'unsupported' };
        }
        const message = error instanceof Error ? error.message : String(error);
        logger.error('Failed to get snapshot status', error);
        return { kind: 'error', message };
    }
}

export async function getSnapshotStatus(
    request: SnapshotStatusRequest
): Promise<SnapshotStatusResponse | null> {
    const result = await getSnapshotStatusFetchResult(request);
    return result.kind === 'ok' ? result.response : null;
}

export async function primeExactTypeIndex(
    request: PrimeExactTypeIndexRequest
): Promise<PrimeExactTypeIndexResponse | null> {
    if (primeExactTypeIndexUnsupported) {
        return null;
    }

    const { sendCustomRequest } = await import('./client/index');
    try {
        return await sendCustomRequest<PrimeExactTypeIndexResponse>(
            'bsl/primeExactTypeIndex',
            request
        );
    } catch (error) {
        if (isMethodNotFoundError(error)) {
            primeExactTypeIndexUnsupported = true;
            logger.warn('[Exact Warmup] LSP server does not support bsl/primeExactTypeIndex');
            return null;
        }
        logger.error('Failed to prime exact type index', error);
        return null;
    }
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
    const { sendCustomRequest } = await import('./client/index');
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
    const { sendCustomRequest } = await import('./client/index');
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
    const { sendCustomRequest } = await import('./client/index');
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
    const { sendCustomRequest } = await import('./client/index');
    return await sendCustomRequest<AutoReindexStateResponse>('bsl/pauseAutoReindex', {});
}

/**
 * Возобновление авто-реиндексации через LSP
 */
export async function resumeAutoReindex(): Promise<AutoReindexStateResponse> {
    const { sendCustomRequest } = await import('./client/index');
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
    const { sendCustomRequest } = await import('./client/index');
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
    const client = (await import('./client/index')).getLanguageClient();
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
    didChangeParseSnapshotEvidence?: DidChangeParseSnapshotEvidenceResponse;
}

export interface DidChangeParseSnapshotEvidenceTrace {
    evidenceId: string;
    uri: string;
    requestedVersion: number;
    startedAtMs: number;
    parseMode: string;
    baseTextSource: string;
    changeShape: string;
    contentChangesCount: number;
    replayOrder: string;
    baseDocumentVersion?: number;
    changedRangesCount: number;
    fallbackReason?: string;
    parserBaseRootCause?: string;
    shadowDocumentVersion?: number;
    latestReadyDocumentVersion?: number;
    matchingReadySnapshotForShadowState?: boolean;
    readySnapshotPrimeAttempted?: boolean;
    treeCacheMatchesShadowTextAfterPrime?: boolean;
}

export interface DidChangeParseSnapshotEvidenceResponse {
    version: number;
    entries: DidChangeParseSnapshotEvidenceTrace[];
}

export interface ObservabilityMetricsRequest {
    shape?: 'full' | 'sidebar';
}

export type ObservabilityMetricsFetchResult =
    | { kind: 'ok'; response: ObservabilityMetricsResponse }
    | { kind: 'unsupported' }
    | { kind: 'error'; message: string };

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
    dispatcher_resolution_latency_ms?: number;
    turn_wait_entered_at_ms?: number;
    turn_wait_resolved_at_ms?: number;
    wake_after_turn_resolution_at_ms?: number;
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

export interface CompletionTimelinePrepareProgressTrace {
    phase?: string;
    phase_started_offset_ms?: number;
    wait_completed_offset_ms?: number;
    snapshot_completed_offset_ms?: number;
}

export interface CompletionTimelinePrepareRuntimeTrace {
    queue_wait_ms?: number;
    exec_ms?: number;
    wake_wait_ms?: number;
    resolution?: string;
}

export interface CompletionTimelinePrepareTimeoutAttributionTrace {
    source: string;
    phase: string;
    budget_ms: number;
    elapsed_ms: number;
    overshoot_ms: number;
}

export type CompletionTimelinePreMethodAttributionProvenance =
    | 'same_request_authoritative'
    | 'best_effort_fallback'
    | 'unavailable';

export type CompletionTimelineTransportReceivedAtMsProvenance =
    | 'request_context_call_entry'
    | 'jsonrpc_dispatch_received';

export type CompletionTimelineServiceFutureFirstPollOutcome =
    | 'ready'
    | 'pending';

export type CompletionTimelineFirstPollContentionClass =
    | 'document_sync'
    | 'completion'
    | 'other_request'
    | 'other_notification'
    | 'mixed'
    | 'none_visible'
    | 'unavailable';

export type CompletionTimelineFirstPollContentionUriScope =
    | 'same_uri'
    | 'other_uri'
    | 'mixed'
    | 'unavailable';

export interface CompletionTimelineFirstPollContentionAttributionTrace {
    contender_class: CompletionTimelineFirstPollContentionClass;
    uri_scope: CompletionTimelineFirstPollContentionUriScope;
    inflight_count: number;
    oldest_inflight_age_ms?: number;
    concurrency_level: number;
}

export type CompletionTimelineInflightRequestClass =
    | 'document_sync'
    | 'completion'
    | 'other_request'
    | 'other_notification';

export interface CompletionTimelineFirstPollContentionContenderTrace {
    request_class: CompletionTimelineInflightRequestClass;
    method: string;
    command?: string;
    phase?: string;
    uri?: string;
    age_ms: number;
}

export interface CompletionTimelineExactArtifactPollTrace {
    poll_count: number;
    poll_elapsed_ms: number;
    observed_file_version?: number;
    head_ready?: boolean;
    exact_ready?: boolean;
}

export interface CompletionTimelineExactWaitDetailsTrace {
    head_ready_before_wait?: boolean;
    exact_ready_before_wait?: boolean;
    current_revision_head_owner_hints_ready?: boolean;
    artifact_wait_outcome?: string;
    type_index_wait_outcome?: string;
    type_index_waiter_action?: string;
    matching_task_state?: string;
    task_phase?: string;
    artifact_poll?: CompletionTimelineExactArtifactPollTrace;
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
    progress?: CompletionTimelinePrepareProgressTrace;
    wait_for_file_version_runtime?: CompletionTimelinePrepareRuntimeTrace;
    snapshot_with_deps_runtime?: CompletionTimelinePrepareRuntimeTrace;
    snapshot_with_deps_timeout_runtime?: CompletionTimelinePrepareRuntimeTrace;
    timeout_attribution?: CompletionTimelinePrepareTimeoutAttributionTrace;
    exact_wait?: CompletionTimelineExactWaitDetailsTrace;
}

export interface CompletionTimelineServerEdgeDetailsTrace {
    adapter_read_started_at_ms?: number;
    adapter_read_at_ms?: number;
    adapter_parse_completed_at_ms?: number;
    transport_received_at_ms: number;
    transport_received_at_ms_provenance?: CompletionTimelineTransportReceivedAtMsProvenance;
    jsonrpc_dispatch_received_at_ms?: number;
    transport_slot_released_at_ms?: number;
    service_future_created_at_ms?: number;
    service_future_first_poll_entered_at_ms?: number;
    service_future_first_poll_outcome?: CompletionTimelineServiceFutureFirstPollOutcome;
    service_future_first_wake_scheduled_at_ms?: number;
    first_poll_contention_attribution?: CompletionTimelineFirstPollContentionAttributionTrace;
    first_poll_contention_contenders?: CompletionTimelineFirstPollContentionContenderTrace[];
    pre_method_attribution_provenance?: CompletionTimelinePreMethodAttributionProvenance;
    service_scope_entered_at_ms?: number;
    method_entered_at_ms?: number;
    handler_entered_at_ms: number;
    response_sent_at_ms: number;
    response_output_handoff_started_at_ms?: number;
    response_output_handoff_enqueued_at_ms?: number;
    response_output_enqueue_completed_at_ms?: number;
    response_output_encode_started_at_ms?: number;
    response_output_write_started_at_ms?: number;
    response_output_encode_completed_at_ms?: number;
    response_flush_completed_at_ms?: number;
    cancel_observed_at_ms?: number;
    read_loop_wait_reason?: string;
    read_loop_wait_ms?: number;
    pending_completion_spillover_depth?: number;
    pending_general_request_staged?: boolean;
    admission_try_enqueue_at_ms?: number;
    admission_lane?: string;
    admission_lane_depth_before?: number;
    admission_lane_depth_after?: number;
    admission_enqueue_outcome?: string;
    admission_spillover_outcome?: string;
    admission_enqueued_at_ms?: number;
    scheduler_woke_at_ms?: number;
    scheduler_poll_ready_entered_at_ms?: number;
    scheduler_poll_ready_resolved_at_ms?: number;
    scheduler_dequeued_at_ms?: number;
    completion_barrier_active_at_dequeue?: boolean;
    completion_barrier_generation?: number;
    completion_barrier_owner_method?: string;
    completion_barrier_owner_uri?: string;
    completion_barrier_owner_version?: number;
    completion_barrier_wait_ms?: number;
    doc_sync_first_poll_exec_ms?: number;
    doc_sync_first_poll_outcome?: string;
    doc_sync_first_poll_method?: string;
    doc_sync_first_poll_uri?: string;
    doc_sync_first_poll_version?: number;
    same_file_ingress_token_required_version?: number;
    same_file_ingress_token_published_at_ms?: number;
    same_file_ingress_token_source?: string;
    same_file_ingress_token_wait_ms?: number;
    scheduler_service_call_started_at_ms?: number;
    scheduler_service_call_returned_at_ms?: number;
    dispatch_to_request_context_wait_ms?: number;
    adapter_to_dispatch_wait_ms?: number;
    admission_queue_wait_ms?: number;
    scheduler_poll_ready_wait_ms?: number;
    scheduler_service_call_sync_exec_ms?: number;
    scheduler_ready_to_dispatch_wait_ms?: number;
    transport_to_slot_release_wait_ms?: number;
    transport_to_service_future_wait_ms?: number;
    service_future_to_scope_wait_ms?: number;
    service_future_to_first_poll_wait_ms?: number;
    first_poll_to_first_wake_wait_ms?: number;
    transport_to_service_scope_wait_ms?: number;
    service_scope_to_method_wait_ms?: number;
    transport_to_method_wait_ms?: number;
    method_prelude_exec_ms?: number;
    slot_release_to_handler_wait_ms?: number;
    slot_release_to_response_wait_ms?: number;
    transport_to_handler_wait_ms: number;
    server_handler_exec_ms: number;
    response_ready_to_output_handoff_wait_ms?: number;
    response_output_handoff_send_wait_ms?: number;
    response_output_handoff_to_writer_wait_ms?: number;
    response_ready_to_output_enqueue_wait_ms?: number;
    response_output_queue_wait_ms?: number;
    response_output_encode_exec_ms?: number;
    response_output_write_and_flush_exec_ms?: number;
    response_ready_to_flush_wait_ms?: number;
    cancel_observed_after_handler_enter_ms?: number;
}

export interface CompletionTimelineTrace {
    trace_id: string;
    request_id?: string;
    client_probe_id?: string;
    uri: string;
    trigger_mode: string;
    outcome: string;
    started_at_ms: number;
    total_duration_ms: number;
    dominant_stage?: string;
    prepare_details?: CompletionTimelinePrepareDetailsTrace;
    server_edge_details?: CompletionTimelineServerEdgeDetailsTrace;
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

export interface CurrentContextTimelineRequest {
    limit?: number;
    uri?: string;
}

export interface CurrentContextTimelineTrace {
    trace_id: string;
    uri: string;
    line: number;
    character: number;
    started_at_ms: number;
    requested_version?: number;
    editor_session_id?: string;
    request_generation?: number;
    route?: 'ready_snapshot' | 'broker_leader' | 'broker_follower';
    broker_role?: 'leader' | 'follower';
    readiness_wait_result?:
        | 'immediate'
        | 'ready'
        | 'superseded'
        | 'budget_exhausted'
        | 'no_matching_task'
        | 'no_shadow_state';
    ready_snapshot_wait_ms?: number;
    ready_snapshot_wait_budget_ms?: number;
    broker_wait_result?: 'leader' | 'resolved' | 'superseded' | 'budget_exhausted';
    broker_wait_ms?: number;
    broker_wait_budget_ms?: number;
    parse_source?: 'ready_snapshot' | 'parser_coordinator' | 'syntax_fallback' | 'parse_unavailable';
    parse_ms?: number;
    wall_ms: number;
    supersession_outcome: 'none' | 'superseded' | 'budget_exhausted';
    final_status: 'resolved' | 'parse_unavailable' | 'superseded' | 'budget_exhausted';
}

export interface CurrentContextTimelineResponse {
    version: number;
    traces: CurrentContextTimelineTrace[];
}

export type CurrentContextTimelineFetchResult =
    | { kind: 'ok'; response: CurrentContextTimelineResponse }
    | { kind: 'unsupported' }
    | { kind: 'error'; message: string };

export interface DiagnosticsSaveTimelineRequest {
    limit?: number;
}

export interface DiagnosticsSaveTimelinePublishTrace {
    profile: string;
    publish_kind: string;
    outcome: string;
    elapsed_ms: number;
    syntax_work_mode?: 'reused' | 'recomputed';
    semantic_path?: 'ready_artifacts' | 'detached_ready_artifacts' | 'shadow_state' | 'generic_pipeline';
    semantic_parse_source?: 'snapshot' | 'salsa';
    semantic_ir_source?: 'exact_cache' | 'snapshot_build' | 'salsa';
    runtime_queue_wait_ms?: number;
    apply_lag_ms?: number;
    blocking_queue_wait_ms?: number;
    wait_for_file_version_ms?: number;
    snapshot_with_deps_ms?: number;
    syntax_diagnostics_query_ms?: number;
    semantic_diagnostics_query_ms?: number;
    publish_wait_ms?: number;
}

export interface DiagnosticsSaveTimelineTrace {
    trace_id: string;
    uri: string;
    requested_version: number;
    save_cycle_sequence: number;
    diagnostics_generation: number;
    trigger: string;
    started_at_ms: number;
    first_publish?: DiagnosticsSaveTimelinePublishTrace;
    followup_publish?: DiagnosticsSaveTimelinePublishTrace;
    save_fastlane_outcome?: string;
    idle_heavy_outcome?: string;
    followup_syntax_work_mode?: 'reused' | 'recomputed';
    followup_semantic_path?: 'ready_artifacts' | 'detached_ready_artifacts' | 'shadow_state' | 'generic_pipeline';
    followup_semantic_parse_source?: 'snapshot' | 'salsa';
    followup_semantic_ir_source?: 'exact_cache' | 'snapshot_build' | 'salsa';
    followup_ready_snapshot_zero_probe?:
        | 'ready'
        | 'not_ready'
        | 'generation_mismatch'
        | 'version_mismatch'
        | 'timeout'
        | 'cancelled'
        | 'superseded';
    followup_ready_snapshot_wait_probe?:
        | 'ready'
        | 'not_ready'
        | 'generation_mismatch'
        | 'version_mismatch'
        | 'timeout'
        | 'cancelled'
        | 'superseded';
    followup_ready_snapshot_task_state?:
        | 'absent'
        | 'in_flight_same_version'
        | 'in_flight_other_version'
        | 'ready_same_version';
    followup_ready_snapshot_timeout_phase?:
        | 'waiting'
        | 'parse_exec'
        | 'post_parse_pre_materialization'
        | 'ready_install'
        | 'document_symbol_side_work';
    followup_ready_snapshot_timeout_phase_elapsed_ms?: number;
    followup_ready_snapshot_timeout_leaf?:
        | 'waiting'
        | 'before_first_parse_exec_subphase'
        | 'before_first_core_build_checkpoint'
        | 'pre_parse_setup'
        | 'parser_base_recovery'
        | 'parser_tree_build'
        | 'exact_ready_snapshot_assembly'
        | 'before_first_exact_ready_snapshot_assembly_checkpoint'
        | 'program_lowering'
        | 'publishable_artifact_packaging'
        | 'syntax_error_collection'
        | 'tree_cache_install'
        | 'optional_cache_enrichment'
        | 'post_parse_pre_materialization'
        | 'ready_install'
        | 'document_symbol_side_work';
    followup_ready_snapshot_timeout_leaf_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_ms?: number;
    followup_ready_snapshot_parse_exec_timeout_subphase?:
        | 'core_parse_build'
        | 'optional_cache_enrichment';
    followup_ready_snapshot_parse_exec_timeout_subphase_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_core_parse_build_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_pre_parse_setup_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint?:
        | 'pre_parse_setup'
        | 'parser_base_recovery'
        | 'parser_tree_build'
        | 'exact_ready_snapshot_assembly'
        | 'tree_cache_install';
    followup_ready_snapshot_parse_exec_core_build_timeout_checkpoint_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_parser_base_recovery_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_parser_tree_build_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint?:
        | 'program_lowering'
        | 'publishable_artifact_packaging'
        | 'syntax_error_collection';
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_timeout_checkpoint_elapsed_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_conversion_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_outcome?: string;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_lowering_units?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_lowering_units?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reused_window_count?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_rebuilt_window_count?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_build_source?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_source?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_candidate_count?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_seed_eviction_reason?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_failure_reason?: string | null;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_take_if_unique_hit?: boolean;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_program_lowering_reuse_plan_borrowed_cache_hit?: boolean;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_publishable_artifact_packaging_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_syntax_error_collection_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint?:
        | 'program_lowering'
        | 'publishable_artifact_packaging'
        | 'syntax_error_collection';
    followup_ready_snapshot_parse_exec_core_build_exact_ready_snapshot_assembly_dominant_checkpoint_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_tree_cache_install_ms?: number;
    followup_ready_snapshot_parse_exec_optional_cache_enrichment_ms?: number;
    followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint?:
        | 'pre_parse_setup'
        | 'parser_base_recovery'
        | 'parser_tree_build'
        | 'exact_ready_snapshot_assembly'
        | 'tree_cache_install';
    followup_ready_snapshot_parse_exec_core_build_dominant_checkpoint_ms?: number;
    followup_ready_snapshot_parse_exec_dominant_subphase?:
        | 'core_parse_build'
        | 'optional_cache_enrichment';
    followup_ready_snapshot_parse_exec_dominant_subphase_ms?: number;
    followup_ready_snapshot_post_parse_pre_materialization_ms?: number;
    followup_ready_snapshot_ready_install_ms?: number;
    followup_ready_snapshot_document_symbol_side_work_ms?: number;
    followup_ready_snapshot_dominant_phase?:
        | 'parse_exec'
        | 'post_parse_pre_materialization'
        | 'ready_install'
        | 'document_symbol_side_work';
    followup_ready_snapshot_dominant_phase_ms?: number;
    followup_ready_snapshot_relief_valve_outcome?:
        | 'engaged_helped'
        | 'engaged_timed_out'
        | 'engaged_version_mismatch'
        | 'engaged_generation_mismatch'
        | 'engaged_cancelled'
        | 'engaged_superseded'
        | 'skipped_not_exact_still_current'
        | 'skipped_runtime_queue_wait'
        | 'skipped_apply_lag'
        | 'skipped_timeout_phase_unavailable'
        | 'skipped_timeout_phase_waiting';
    followup_ready_snapshot_relief_valve_budget_ms?: number;
    followup_ready_snapshot_relief_valve_elapsed_ms?: number;
    followup_shadow_state_available?: boolean;
    followup_wait_reason?: 'apply_lag' | 'runtime_queue_wait' | 'semantic_work' | 'pending_publish' | 'superseded';
    followup_blocker_reason?: 'apply_lag' | 'post_ready_publish_gate';
    followup_runtime_queue_wait_ms?: number;
    followup_apply_lag_ms?: number;
    followup_wait_for_file_version_ms?: number;
    followup_snapshot_with_deps_ms?: number;
    followup_readiness_blocker_bucket?:
        | 'wait_for_file_version'
        | 'snapshot_with_deps'
        | 'runtime_queue_wait'
        | 'apply_lag'
        | 'post_ready_publish_gate'
        | 'program_lowering_tail'
        | 'ready_snapshot_task'
        | 'unclassified_readiness_residual';
    followup_unclassified_readiness_residual_ms?: number;
    terminal_outcome?: string;
}

export interface DiagnosticsSaveTimelineResponse {
    version: number;
    traces: DiagnosticsSaveTimelineTrace[];
}

export type DiagnosticsSaveTimelineFetchResult =
    | { kind: 'ok'; response: DiagnosticsSaveTimelineResponse }
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
    const client = (await import('./client/index')).getLanguageClient();
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

    const client = (await import('./client/index')).getLanguageClient();
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

export function resetObservabilityCapabilityCaches(): void {
    snapshotStatusUnsupported = false;
    primeExactTypeIndexUnsupported = false;
    observabilityMetricsUnsupported = false;
    observabilityMetricsUnsupportedNotified = false;
    completionTimelineUnsupported = false;
    currentContextTimelineUnsupported = false;
    diagnosticsSaveTimelineUnsupported = false;
}

export async function getObservabilityMetrics(): Promise<ObservabilityMetricsResponse | null> {
    return getObservabilityMetricsWithRequest({ shape: 'full' });
}

function shouldWarnOnObservabilityTimeout(request: ObservabilityMetricsRequest): boolean {
    return request.shape !== 'sidebar';
}

export async function getObservabilityMetricsFetchResult(
    request: ObservabilityMetricsRequest = {}
): Promise<ObservabilityMetricsFetchResult> {
    if (observabilityMetricsUnsupported) {
        return { kind: 'unsupported' };
    }

    const client = (await import('./client/index')).getLanguageClient();
    if (!client) {
        logger.warn('[Observability] LSP client not available');
        return { kind: 'error', message: 'LSP client not available' };
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
        return {
            kind: 'ok',
            response: result as ObservabilityMetricsResponse || { metrics: null },
        };
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
            return { kind: 'unsupported' };
        }
        if (isTimeoutError(error)) {
            const message = `Observability request timed out after ${OBSERVABILITY_METRICS_TIMEOUT_MS}ms`;
            if (shouldWarnOnObservabilityTimeout(request)) {
                logger.warn(`[Observability] Request timed out after ${OBSERVABILITY_METRICS_TIMEOUT_MS}ms`);
            }
            return { kind: 'error', message };
        }
        logger.error('Failed to get observability metrics', error);
        return {
            kind: 'error',
            message: error instanceof Error ? error.message : String(error),
        };
    }
}

export async function getObservabilityMetricsWithRequest(
    request: ObservabilityMetricsRequest = {}
): Promise<ObservabilityMetricsResponse | null> {
    const result = await getObservabilityMetricsFetchResult(request);
    return result.kind === 'ok' ? result.response : null;
}

let completionTimelineUnsupported = false;
let currentContextTimelineUnsupported = false;
let diagnosticsSaveTimelineUnsupported = false;

/**
 * Сброс кэша совместимости timeline-контракта (используется только в тестах).
 */
export function resetCompletionTimelineSupportCacheForTests(): void {
    resetObservabilityCapabilityCaches();
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

    const client = (await import('./client/index')).getLanguageClient();
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

export async function getCurrentContextTimeline(
    request: CurrentContextTimelineRequest = {}
): Promise<CurrentContextTimelineFetchResult> {
    if (currentContextTimelineUnsupported) {
        return { kind: 'unsupported' };
    }

    const client = (await import('./client/index')).getLanguageClient();
    if (!client) {
        return { kind: 'error', message: 'LSP client not available' };
    }

    const args = Object.keys(request).length > 0 ? [request] : [];
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getCurrentContextTimeline',
            arguments: args
        });
        if (!result || typeof result !== 'object') {
            return { kind: 'error', message: 'Invalid current context timeline response' };
        }
        return { kind: 'ok', response: result as CurrentContextTimelineResponse };
    } catch (error) {
        if (isMethodNotFoundError(error)) {
            currentContextTimelineUnsupported = true;
            logger.warn('[Current Context Timeline] LSP server does not support bsl.getCurrentContextTimeline');
            return { kind: 'unsupported' };
        }
        const message = error instanceof Error ? error.message : String(error);
        logger.error('Failed to get current context timeline', error);
        return { kind: 'error', message };
    }
}

export async function getDiagnosticsSaveTimeline(
    request: DiagnosticsSaveTimelineRequest = {}
): Promise<DiagnosticsSaveTimelineFetchResult> {
    if (diagnosticsSaveTimelineUnsupported) {
        return { kind: 'unsupported' };
    }

    const client = (await import('./client/index')).getLanguageClient();
    if (!client) {
        return { kind: 'error', message: 'LSP client not available' };
    }

    const args = Object.keys(request).length > 0 ? [request] : [];
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getDiagnosticsSaveTimeline',
            arguments: args
        });
        if (!result || typeof result !== 'object') {
            return { kind: 'error', message: 'Invalid diagnostics save timeline response' };
        }
        return { kind: 'ok', response: result as DiagnosticsSaveTimelineResponse };
    } catch (error) {
        if (isMethodNotFoundError(error)) {
            diagnosticsSaveTimelineUnsupported = true;
            logger.warn('[Diagnostics Save Timeline] LSP server does not support bsl.getDiagnosticsSaveTimeline');
            return { kind: 'unsupported' };
        }
        const message = error instanceof Error ? error.message : String(error);
        logger.error('Failed to get diagnostics save timeline', error);
        return { kind: 'error', message };
    }
}

export async function getCacheStats(
    configurationPath: string
): Promise<CacheStatsResponse | null> {
    const client = (await import('./client/index')).getLanguageClient();
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
    const client = (await import('./client/index')).getLanguageClient();
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
