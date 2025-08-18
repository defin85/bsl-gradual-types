"use strict";
/**
 * Экспорт всех LSP модулей
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.sendCustomNotification = exports.sendCustomRequest = exports.isClientRunning = exports.getLanguageClient = exports.restartLanguageClient = exports.stopLanguageClient = exports.startLanguageClient = exports.initializeLspClient = exports.getCurrentProgress = exports.updateStatusBar = exports.finishIndexing = exports.updateIndexingProgress = exports.startIndexing = exports.initializeProgress = exports.progressEmitter = void 0;
// Модуль управления прогрессом
var progress_1 = require("./progress");
Object.defineProperty(exports, "progressEmitter", { enumerable: true, get: function () { return progress_1.progressEmitter; } });
Object.defineProperty(exports, "initializeProgress", { enumerable: true, get: function () { return progress_1.initializeProgress; } });
Object.defineProperty(exports, "startIndexing", { enumerable: true, get: function () { return progress_1.startIndexing; } });
Object.defineProperty(exports, "updateIndexingProgress", { enumerable: true, get: function () { return progress_1.updateIndexingProgress; } });
Object.defineProperty(exports, "finishIndexing", { enumerable: true, get: function () { return progress_1.finishIndexing; } });
Object.defineProperty(exports, "updateStatusBar", { enumerable: true, get: function () { return progress_1.updateStatusBar; } });
Object.defineProperty(exports, "getCurrentProgress", { enumerable: true, get: function () { return progress_1.getCurrentProgress; } });
// Модуль LSP клиента
var client_1 = require("./client");
Object.defineProperty(exports, "initializeLspClient", { enumerable: true, get: function () { return client_1.initializeLspClient; } });
Object.defineProperty(exports, "startLanguageClient", { enumerable: true, get: function () { return client_1.startLanguageClient; } });
Object.defineProperty(exports, "stopLanguageClient", { enumerable: true, get: function () { return client_1.stopLanguageClient; } });
Object.defineProperty(exports, "restartLanguageClient", { enumerable: true, get: function () { return client_1.restartLanguageClient; } });
Object.defineProperty(exports, "getLanguageClient", { enumerable: true, get: function () { return client_1.getLanguageClient; } });
Object.defineProperty(exports, "isClientRunning", { enumerable: true, get: function () { return client_1.isClientRunning; } });
Object.defineProperty(exports, "sendCustomRequest", { enumerable: true, get: function () { return client_1.sendCustomRequest; } });
Object.defineProperty(exports, "sendCustomNotification", { enumerable: true, get: function () { return client_1.sendCustomNotification; } });
//# sourceMappingURL=index.js.map