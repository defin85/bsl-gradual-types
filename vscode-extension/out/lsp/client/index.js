"use strict";
/**
 * LSP Client module - модульная структура для управления LSP клиентом
 *
 * Структура:
 * - index.ts (этот файл) - инициализация и re-exports
 * - lifecycle.ts - start, stop, restart, getClient, isClientRunning
 * - server-options.ts - buildServerOptions()
 * - client-options.ts - buildClientOptions()
 * - progress-handler.ts - setupProgressHandler()
 * - health-check.ts - startHealthCheck, stopHealthCheck
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.initializeLspClient = exports.sendCustomNotification = exports.sendCustomRequest = exports.isClientRunning = exports.getServerVersion = exports.getLanguageClient = exports.restartLanguageClient = exports.stopLanguageClient = exports.startLanguageClient = void 0;
const lifecycle_1 = require("./lifecycle");
// Re-export public API from lifecycle
var lifecycle_2 = require("./lifecycle");
Object.defineProperty(exports, "startLanguageClient", { enumerable: true, get: function () { return lifecycle_2.startLanguageClient; } });
Object.defineProperty(exports, "stopLanguageClient", { enumerable: true, get: function () { return lifecycle_2.stopLanguageClient; } });
Object.defineProperty(exports, "restartLanguageClient", { enumerable: true, get: function () { return lifecycle_2.restartLanguageClient; } });
Object.defineProperty(exports, "getLanguageClient", { enumerable: true, get: function () { return lifecycle_2.getLanguageClient; } });
Object.defineProperty(exports, "getServerVersion", { enumerable: true, get: function () { return lifecycle_2.getServerVersion; } });
Object.defineProperty(exports, "isClientRunning", { enumerable: true, get: function () { return lifecycle_2.isClientRunning; } });
Object.defineProperty(exports, "sendCustomRequest", { enumerable: true, get: function () { return lifecycle_2.sendCustomRequest; } });
Object.defineProperty(exports, "sendCustomNotification", { enumerable: true, get: function () { return lifecycle_2.sendCustomNotification; } });
/**
 * Инициализирует модуль LSP клиента
 * @param channel Output channel для логирования
 */
function initializeLspClient(channel) {
    (0, lifecycle_1.initializeLifecycle)(channel);
}
exports.initializeLspClient = initializeLspClient;
//# sourceMappingURL=index.js.map