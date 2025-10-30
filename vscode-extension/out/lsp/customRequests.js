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
exports.searchTypes = exports.extractPlatformDocs = exports.incrementalUpdate = exports.analyzeFile = exports.checkTypeCompatibility = exports.validateMethod = exports.buildIndex = exports.queryType = void 0;
const client_1 = require("./client");
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
        console.error('Failed to query type via LSP:', error);
        throw error;
    }
}
exports.queryType = queryType;
/**
 * Построение индекса типов через LSP
 * Заменяет: executeBslCommand('build_unified_index', ...)
 */
async function buildIndex(params) {
    return await (0, client_1.sendCustomRequest)('bsl/buildIndex', params);
}
exports.buildIndex = buildIndex;
/**
 * Валидация вызова метода через LSP
 * Заменяет: executeBslCommand('check_type_compatibility', ...)
 */
async function validateMethod(objectType, methodName, args) {
    return await (0, client_1.sendCustomRequest)('bsl/validateMethod', {
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
    return await (0, client_1.sendCustomRequest)('bsl/checkTypeCompatibility', {
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
    console.log(`File ${filePath} will be analyzed via LSP textDocument/didOpen`);
}
exports.analyzeFile = analyzeFile;
/**
 * Инкрементальное обновление индекса через LSP
 * Заменяет: executeBslCommand('incremental_update', ...)
 */
async function incrementalUpdate(configPath, platformVersion) {
    return await (0, client_1.sendCustomRequest)('bsl/incrementalUpdate', {
        config_path: configPath,
        platform_version: platformVersion
    });
}
exports.incrementalUpdate = incrementalUpdate;
/**
 * Извлечение платформенной документации через LSP
 * Заменяет: executeBslCommand('extract_platform_docs', ...)
 */
async function extractPlatformDocs(archivePath, platformVersion, force = false) {
    return await (0, client_1.sendCustomRequest)('bsl/extractPlatformDocs', {
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
        console.error('Failed to search types via LSP:', error);
        throw error;
    }
}
exports.searchTypes = searchTypes;
//# sourceMappingURL=customRequests.js.map