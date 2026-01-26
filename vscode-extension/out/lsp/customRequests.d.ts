/**
 * LSP Custom Requests - заменяют CLI бинарники
 *
 * Все запросы идут через LSP server вместо fork процессов
 */
export interface QueryTypeParams {
    type_name: string;
}
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
    typeName: string;
    found: boolean;
    certainty?: string;
    facet?: string;
    description?: string;
    methods?: MethodInfo[];
    properties?: PropertyInfo[];
    facets?: string[];
    details?: string;
}
export interface BuildIndexParams {
    workspace_path: string;
}
export interface BuildIndexResponse {
    success: boolean;
    types_count: number;
    message: string;
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
    ir?: IrStats;
}
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
/**
 * Запрос информации о типе через LSP
 * Заменяет: executeBslCommand('query_type', ...)
 */
export declare function queryType(typeName: string): Promise<QueryTypeResponse>;
/**
 * Построение индекса типов через LSP
 * Заменяет: executeBslCommand('build_unified_index', ...)
 */
export declare function buildIndex(params: BuildIndexParams): Promise<BuildIndexResponse>;
/**
 * Валидация вызова метода через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
export declare function validateMethod(objectType: string, methodName: string, args: string[]): Promise<ValidateMethodResponse>;
/**
 * Проверка совместимости типов через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
export declare function checkTypeCompatibility(sourceType: string, targetType: string): Promise<CheckCompatibilityResponse>;
/**
 * Анализ файла через LSP (уже работает через textDocument/didOpen)
 * Заменяет: executeBslCommand('bsl-analyzer', ...)
 */
export declare function analyzeFile(filePath: string): Promise<void>;
/**
 * Инкрементальное обновление индекса через LSP
 * Заменяет: executeBslCommand('incremental_update', ...)
 */
export declare function incrementalUpdate(configPath: string, platformVersion: string, changedPaths?: string[], isAuto?: boolean): Promise<IncrementalUpdateResponse>;
/**
 * Пауза авто-реиндексации через LSP
 */
export declare function pauseAutoReindex(): Promise<AutoReindexStateResponse>;
/**
 * Возобновление авто-реиндексации через LSP
 */
export declare function resumeAutoReindex(): Promise<AutoReindexStateResponse>;
/**
 * Извлечение платформенной документации через LSP
 * Заменяет: executeBslCommand('extract_platform_docs', ...)
 */
export declare function extractPlatformDocs(archivePath: string, platformVersion: string, force?: boolean): Promise<ExtractPlatformDocsResponse>;
/**
 * Поиск типов в TypeRepository через LSP
 * Заменяет: mock данные в Quick Actions Webview
 *
 * @param query - поисковый запрос (partial match, case-insensitive)
 * @param limit - максимум результатов (по умолчанию 15)
 * @returns массив найденных типов
 */
export declare function searchTypes(query: string, limit?: number): Promise<SearchTypesResponse>;
/**
 * Статистика TypeRepository
 */
export interface TypeRepositoryStats {
    totalTypes: number;
    platformTypes: number;
    configurationTypes: number;
    lastUpdateTime?: string;
}
/**
 * Статистика workspace (файлы и диагностика)
 */
export interface WorkspaceStatsResponse {
    bslFiles: number;
    diagnostics: number;
}
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
    source: string;
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
export declare function getTypeRepositoryStats(): Promise<TypeRepositoryStats | null>;
export declare function getWorkspaceStats(): Promise<WorkspaceStatsResponse | null>;
export declare function getCacheStats(configurationPath: string): Promise<CacheStatsResponse | null>;
/**
 * Получить все типы из TypeRepository через LSP Server
 *
 * @param params - Параметры запроса (limit, offset, category)
 * @returns Список типов с метаданными или null если LSP недоступен
 */
export declare function getAllTypes(params?: GetAllTypesRequest): Promise<GetAllTypesResponse | null>;
//# sourceMappingURL=customRequests.d.ts.map