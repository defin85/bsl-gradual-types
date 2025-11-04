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
/**
 * MILESTONE 2.20.2.4: Work Done Progress notification types (LSP Standard)
 */
/**
 * Параметры $/progress notification
 */
export interface ProgressParams {
    /**
     * Уникальный токен прогресса
     */
    token: string | number;
    /**
     * Значение прогресса (begin/report/end)
     */
    value: WorkDoneProgressBegin | WorkDoneProgressReport | WorkDoneProgressEnd;
}
/**
 * Начало Work Done Progress
 */
export interface WorkDoneProgressBegin {
    kind: 'begin';
    title: string;
    message?: string;
    percentage?: number;
    cancellable?: boolean;
}
/**
 * Обновление Work Done Progress
 */
export interface WorkDoneProgressReport {
    kind: 'report';
    message?: string;
    percentage?: number;
    cancellable?: boolean;
}
/**
 * Завершение Work Done Progress
 */
export interface WorkDoneProgressEnd {
    kind: 'end';
    message?: string;
}
/**
 * Результат парсинга прогресса из message string
 */
export interface ParsedProgressMessage {
    /**
     * Текущий номер элемента (для "Тип 150/3927")
     */
    currentItem?: number;
    /**
     * Всего элементов (для "Тип 150/3927")
     */
    totalItems?: number;
    /**
     * Название текущего элемента (для "Справочники.Контрагенты")
     */
    itemName?: string;
    /**
     * ETA в секундах (для "ETA: 42s")
     */
    eta?: number;
    /**
     * Исходное сообщение
     */
    originalMessage: string;
}
/**
 * MILESTONE 2.20.3: Server Status notification (rust-analyzer approach)
 * Custom bsl/serverStatus notification для управления status bar icon
 */
export interface ServerStatusParams {
    /**
     * Загружается ли LSP server (парсинг типов платформы)
     * true = показывать $(loading~spin) icon
     * false = обычный status bar
     */
    loading: boolean;
    /**
     * Опциональное сообщение о текущей операции
     */
    message?: string;
}
//# sourceMappingURL=index.d.ts.map