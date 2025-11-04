"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.logger = exports.LogLevel = void 0;
/**
 * Уровни логирования
 */
var LogLevel;
(function (LogLevel) {
    LogLevel[LogLevel["Debug"] = 0] = "Debug";
    LogLevel[LogLevel["Info"] = 1] = "Info";
    LogLevel[LogLevel["Warn"] = 2] = "Warn";
    LogLevel[LogLevel["Error"] = 3] = "Error";
})(LogLevel = exports.LogLevel || (exports.LogLevel = {}));
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
    constructor() {
        this.level = LogLevel.Info;
    }
    /**
     * Инициализация логгера с Output Channel
     */
    initialize(channel, level = LogLevel.Info) {
        this.outputChannel = channel;
        this.level = level;
    }
    /**
     * Debug сообщения (детальная отладочная информация)
     */
    debug(message) {
        if (this.level <= LogLevel.Debug) {
            this.log('🔍 [DEBUG]', message);
        }
    }
    /**
     * Info сообщения (общая информация)
     */
    info(message) {
        if (this.level <= LogLevel.Info) {
            this.log('ℹ️ [INFO]', message);
        }
    }
    /**
     * Warning сообщения (предупреждения)
     */
    warn(message) {
        if (this.level <= LogLevel.Warn) {
            this.log('⚠️ [WARN]', message);
        }
    }
    /**
     * Error сообщения (ошибки)
     */
    error(message, error) {
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
    log(prefix, message) {
        if (this.outputChannel) {
            const timestamp = new Date().toISOString();
            this.outputChannel.appendLine(`[${timestamp}] ${prefix} ${message}`);
        }
        else {
            // Fallback на console, если outputChannel не инициализирован
            console.log(`${prefix} ${message}`);
        }
    }
}
/**
 * Singleton instance логгера
 */
exports.logger = new Logger();
//# sourceMappingURL=logger.js.map