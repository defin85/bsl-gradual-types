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
}
export interface IncrementalUpdateResponse {
    success: boolean;
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
export declare function incrementalUpdate(configPath: string, platformVersion: string): Promise<IncrementalUpdateResponse>;
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
//# sourceMappingURL=customRequests.d.ts.map