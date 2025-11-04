import * as vscode from 'vscode';
/**
 * Уровни логирования
 */
export declare enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3
}
/**
 * Централизованный логгер для BSL Analyzer Extension
 *
 * Использование:
 * ```typescript
 * import { logger } from './lsp/logger';
 *
 * logger.info('Server started');
 * logger.error('Failed to connect', error);
 * ```
 */
declare class Logger {
    private outputChannel?;
    private level;
    /**
     * Инициализация логгера с Output Channel
     */
    initialize(channel: vscode.OutputChannel, level?: LogLevel): void;
    /**
     * Debug сообщения (детальная отладочная информация)
     */
    debug(message: string): void;
    /**
     * Info сообщения (общая информация)
     */
    info(message: string): void;
    /**
     * Warning сообщения (предупреждения)
     */
    warn(message: string): void;
    /**
     * Error сообщения (ошибки)
     */
    error(message: string, error?: unknown): void;
    /**
     * Внутренний метод для записи логов
     */
    private log;
}
/**
 * Singleton instance логгера
 */
export declare const logger: Logger;
export {};
//# sourceMappingURL=logger.d.ts.map