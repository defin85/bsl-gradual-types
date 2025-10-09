/**
 * LSP Custom Requests - заменяют CLI бинарники
 *
 * Все запросы идут через LSP server вместо fork процессов
 */

import { sendCustomRequest } from './client';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface QueryTypeParams {
    type_name: string;
}

export interface QueryTypeResponse {
    type_name: string;
    found: boolean;
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

// ============================================================================
// Helper Functions - прямые вызовы LSP custom requests
// ============================================================================

/**
 * Запрос информации о типе через LSP
 * Заменяет: executeBslCommand('query_type', ...)
 */
export async function queryType(typeName: string): Promise<QueryTypeResponse> {
    return await sendCustomRequest<QueryTypeResponse>('bsl/queryType', {
        type_name: typeName
    });
}

/**
 * Построение индекса типов через LSP
 * Заменяет: executeBslCommand('build_unified_index', ...)
 */
export async function buildIndex(params: BuildIndexParams): Promise<BuildIndexResponse> {
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
    console.log(`File ${filePath} will be analyzed via LSP textDocument/didOpen`);
}

/**
 * Инкрементальное обновление индекса через LSP
 * Заменяет: executeBslCommand('incremental_update', ...)
 */
export async function incrementalUpdate(
    configPath: string,
    platformVersion: string
): Promise<IncrementalUpdateResponse> {
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
    return await sendCustomRequest<ExtractPlatformDocsResponse>('bsl/extractPlatformDocs', {
        archive_path: archivePath,
        platform_version: platformVersion,
        force
    });
}
