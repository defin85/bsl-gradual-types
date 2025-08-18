/**
 * Типы для BSL Analyzer Extension
 */

/**
 * Метрики качества кода
 */
export interface CodeMetrics {
    file: string;
    complexity: number;
    lines: number;
    functions: number;
    errors: number;
    warnings: number;
    score: number;
    internerSymbols?: number;
    internerBytes?: number;
    details?: {
        cyclomaticComplexity?: number;
        cognitiveComplexity?: number;
        duplicateLines?: number;
        codeSmells?: number;
    };
}

/**
 * Параметры запроса информации о типе
 */
export interface TypeInfoParams {
    typeName: string;
    includeInherited?: boolean;
    includePrivate?: boolean;
}

/**
 * Параметры валидации метода
 */
export interface ValidateMethodParams {
    objectType: string;
    methodName: string;
    arguments?: Array<{
        type: string;
        value?: string;
    }>;
}

/**
 * Прогресс индексации
 */
export interface IndexingProgressParams {
    step: number;
    totalSteps: number;
    message: string;
    percentage: number;
}

/**
 * Конфигурационные параметры
 */
export interface ConfigurationParams {
    items: Array<{
        section: string;
        scopeUri?: string;
    }>;
}

/**
 * Обработчик команд
 */
export type CommandHandler = (...args: unknown[]) => unknown | Promise<unknown>;

/**
 * Обработчик уведомлений LSP
 */
export type NotificationHandler = (method: string, params: unknown, next: Function) => unknown;

/**
 * Обработчик конфигурации workspace
 */
export type WorkspaceConfigurationHandler = (params: ConfigurationParams, token: unknown, next: Function) => unknown;