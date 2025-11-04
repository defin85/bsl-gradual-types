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
    platformVersion: string
): Promise<IncrementalUpdateResponse> {
    const { sendCustomRequest } = await import('./client');
    return await sendCustomRequest<IncrementalUpdateResponse>('bsl/incrementalUpdate', {
        config_path: configPath,
        platform_version: platformVersion
    });
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
// MILESTONE 2.20.4: Type Repository Statistics
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
