import * as vscode from 'vscode';

/**
 * Уровни логирования
 */
export enum LogLevel {
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
class Logger {
    private outputChannel?: vscode.OutputChannel;
    private level: LogLevel = LogLevel.Info;

    /**
     * Инициализация логгера с Output Channel
     */
    initialize(channel: vscode.OutputChannel, level: LogLevel = LogLevel.Info) {
        this.outputChannel = channel;
        this.level = level;
    }

    /**
     * Debug сообщения (детальная отладочная информация)
     */
    debug(message: string) {
        if (this.level <= LogLevel.Debug) {
            this.log('🔍 [DEBUG]', message);
        }
    }

    /**
     * Info сообщения (общая информация)
     */
    info(message: string) {
        if (this.level <= LogLevel.Info) {
            this.log('ℹ️ [INFO]', message);
        }
    }

    /**
     * Warning сообщения (предупреждения)
     */
    warn(message: string) {
        if (this.level <= LogLevel.Warn) {
            this.log('⚠️ [WARN]', message);
        }
    }

    /**
     * Error сообщения (ошибки)
     */
    error(message: string, error?: unknown) {
        if (this.level <= LogLevel.Error) {
            const errorStr = error instanceof Error
                ? (error.stack || error.message)
                : String(error);

            const fullMessage = error
                ? `${message}\n${errorStr}`
                : message;

            this.log('❌ [ERROR]', fullMessage);
        }
    }

    /**
     * Внутренний метод для записи логов
     */
    private log(prefix: string, message: string) {
        if (this.outputChannel) {
            const timestamp = new Date().toISOString();
            this.outputChannel.appendLine(`[${timestamp}] ${prefix} ${message}`);
        } else {
            // Fallback на console, если outputChannel не инициализирован
            console.log(`${prefix} ${message}`);
        }
    }
}

/**
 * Singleton instance логгера
 */
export const logger = new Logger();
