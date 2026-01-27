"use strict";
/**
 * LSP Custom Requests - заменяют CLI бинарники
 *
 * Все запросы идут через LSP server вместо fork процессов
 */
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
exports.getAllTypes = exports.getCacheStats = exports.getObservabilityMetrics = exports.getWorkspaceStats = exports.getTypeRepositoryStats = exports.searchTypes = exports.extractPlatformDocs = exports.resumeAutoReindex = exports.pauseAutoReindex = exports.incrementalUpdate = exports.analyzeFile = exports.checkTypeCompatibility = exports.validateMethod = exports.buildIndex = exports.queryType = void 0;
const logger_1 = require("./logger");
// ============================================================================
// Helper Functions - прямые вызовы LSP custom requests
// ============================================================================
/**
 * Запрос информации о типе через LSP
 * Заменяет: executeBslCommand('query_type', ...)
 */
async function queryType(typeName) {
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
    if (!client) {
        throw new Error('LSP client not available');
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.queryType',
            arguments: [{
                    type_name: typeName
                }]
        });
        return result;
    }
    catch (error) {
        logger_1.logger.error('Failed to query type via LSP', error);
        throw error;
    }
}
exports.queryType = queryType;
/**
 * Построение индекса типов через LSP
 * Заменяет: executeBslCommand('build_unified_index', ...)
 */
async function buildIndex(params) {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    return await sendCustomRequest('bsl/buildIndex', params);
}
exports.buildIndex = buildIndex;
/**
 * Валидация вызова метода через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
async function validateMethod(objectType, methodName, args) {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    return await sendCustomRequest('bsl/validateMethod', {
        object_type: objectType,
        method_name: methodName,
        arguments: args
    });
}
exports.validateMethod = validateMethod;
/**
 * Проверка совместимости типов через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
async function checkTypeCompatibility(sourceType, targetType) {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    return await sendCustomRequest('bsl/checkTypeCompatibility', {
        source_type: sourceType,
        target_type: targetType
    });
}
exports.checkTypeCompatibility = checkTypeCompatibility;
/**
 * Анализ файла через LSP (уже работает через textDocument/didOpen)
 * Заменяет: executeBslCommand('bsl-analyzer', ...)
 */
async function analyzeFile(filePath) {
    // Файл анализируется автоматически при открытии через LSP
    // Дополнительно можно отправить custom request если нужно
    logger_1.logger.debug(`File ${filePath} will be analyzed via LSP textDocument/didOpen`);
}
exports.analyzeFile = analyzeFile;
/**
 * Инкрементальное обновление индекса через LSP
 * Заменяет: executeBslCommand('incremental_update', ...)
 */
async function incrementalUpdate(configPath, platformVersion, changedPaths, isAuto) {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    const params = {
        config_path: configPath,
        platform_version: platformVersion,
        changed_paths: changedPaths
    };
    if (isAuto !== undefined) {
        params.is_auto = isAuto;
    }
    return await sendCustomRequest('bsl/incrementalUpdate', params);
}
exports.incrementalUpdate = incrementalUpdate;
/**
 * Пауза авто-реиндексации через LSP
 */
async function pauseAutoReindex() {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    return await sendCustomRequest('bsl/pauseAutoReindex', {});
}
exports.pauseAutoReindex = pauseAutoReindex;
/**
 * Возобновление авто-реиндексации через LSP
 */
async function resumeAutoReindex() {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    return await sendCustomRequest('bsl/resumeAutoReindex', {});
}
exports.resumeAutoReindex = resumeAutoReindex;
/**
 * Извлечение платформенной документации через LSP
 * Заменяет: executeBslCommand('extract_platform_docs', ...)
 */
async function extractPlatformDocs(archivePath, platformVersion, force = false) {
    const { sendCustomRequest } = await Promise.resolve().then(() => __importStar(require('./client')));
    return await sendCustomRequest('bsl/extractPlatformDocs', {
        archive_path: archivePath,
        platform_version: platformVersion,
        force
    });
}
exports.extractPlatformDocs = extractPlatformDocs;
/**
 * Поиск типов в TypeRepository через LSP
 * Заменяет: mock данные в Quick Actions Webview
 *
 * @param query - поисковый запрос (partial match, case-insensitive)
 * @param limit - максимум результатов (по умолчанию 15)
 * @returns массив найденных типов
 */
async function searchTypes(query, limit) {
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
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
        return result;
    }
    catch (error) {
        logger_1.logger.error('Failed to search types via LSP', error);
        throw error;
    }
}
exports.searchTypes = searchTypes;
/**
 * Получить статистику TypeRepository из LSP Server
 *
 * @returns Статистика или null если LSP недоступен
 */
async function getTypeRepositoryStats() {
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
    if (!client) {
        logger_1.logger.warn('[Type Stats] LSP client not available');
        return null;
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getTypeRepositoryStats',
            arguments: [{}]
        });
        return result || null;
    }
    catch (error) {
        logger_1.logger.error('Failed to get type repository stats', error);
        return null;
    }
}
exports.getTypeRepositoryStats = getTypeRepositoryStats;
/**
 * Получить статистику workspace через LSP Server
 */
let workspaceStatsUnsupported = false;
let workspaceStatsUnsupportedNotified = false;
async function getWorkspaceStats() {
    if (workspaceStatsUnsupported) {
        return null;
    }
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
    if (!client) {
        logger_1.logger.warn('[Workspace Stats] LSP client not available');
        return null;
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getWorkspaceStats',
            arguments: [{}]
        });
        return result || null;
    }
    catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (message.includes('Method not found')) {
            workspaceStatsUnsupported = true;
            if (!workspaceStatsUnsupportedNotified) {
                workspaceStatsUnsupportedNotified = true;
                logger_1.logger.warn('[Workspace Stats] LSP server does not support getWorkspaceStats yet');
                const vscode = await Promise.resolve().then(() => __importStar(require('vscode')));
                vscode.window.showWarningMessage('BSL Analyzer: LSP server does not support workspace stats yet. Please обновите бинарник.');
            }
            return null;
        }
        logger_1.logger.error('Failed to get workspace stats', error);
        return null;
    }
}
exports.getWorkspaceStats = getWorkspaceStats;
/**
 * Получить снимок метрик observability из LSP сервера (для диагностики "затыков").
 */
let observabilityMetricsUnsupported = false;
let observabilityMetricsUnsupportedNotified = false;
async function getObservabilityMetrics() {
    if (observabilityMetricsUnsupported) {
        return null;
    }
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
    if (!client) {
        logger_1.logger.warn('[Observability] LSP client not available');
        return null;
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getObservabilityMetrics',
            arguments: [{}]
        });
        return result || null;
    }
    catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (message.includes('Method not found')) {
            observabilityMetricsUnsupported = true;
            if (!observabilityMetricsUnsupportedNotified) {
                observabilityMetricsUnsupportedNotified = true;
                logger_1.logger.warn('[Observability] LSP server does not support getObservabilityMetrics yet');
                const vscode = await Promise.resolve().then(() => __importStar(require('vscode')));
                vscode.window.showWarningMessage('BSL Analyzer: LSP server does not support observability metrics yet. Please обновите бинарник.');
            }
            return null;
        }
        logger_1.logger.error('Failed to get observability metrics', error);
        return null;
    }
}
exports.getObservabilityMetrics = getObservabilityMetrics;
async function getCacheStats(configurationPath) {
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
    if (!client) {
        logger_1.logger.warn('[Cache Stats] LSP client not available');
        return null;
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.cache.getStats',
            arguments: [{ configurationPath }]
        });
        return result || null;
    }
    catch (error) {
        logger_1.logger.error('Failed to get cache stats', error);
        return null;
    }
}
exports.getCacheStats = getCacheStats;
/**
 * Получить все типы из TypeRepository через LSP Server
 *
 * @param params - Параметры запроса (limit, offset, category)
 * @returns Список типов с метаданными или null если LSP недоступен
 */
async function getAllTypes(params) {
    const client = (await Promise.resolve().then(() => __importStar(require('./client')))).getLanguageClient();
    if (!client) {
        logger_1.logger.warn('[Get All Types] LSP client not available');
        return null;
    }
    try {
        const result = await client.sendRequest('workspace/executeCommand', {
            command: 'bsl.getAllTypes',
            arguments: params ? [params] : []
        });
        return result || null;
    }
    catch (error) {
        logger_1.logger.error('Failed to get all types', error);
        return null;
    }
}
exports.getAllTypes = getAllTypes;
//# sourceMappingURL=customRequests.js.map