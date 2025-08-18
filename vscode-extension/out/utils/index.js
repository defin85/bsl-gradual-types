"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.autoDetectConfiguration = exports.selectConfiguration = exports.findMainConfiguration = exports.findConfigurations = exports.initializeUtils = exports.setConfigOutputChannel = exports.getPlatformDocsArchive = exports.getPlatformVersion = exports.getConfigurationPath = exports.extractTypeName = exports.parseMethodCall = exports.setExecutorOutputChannel = exports.executeBslCommand = exports.setBinaryPathOutputChannel = exports.getBinaryPath = void 0;
/**
 * Экспорт всех утилит из одного места
 */
var binaryPath_1 = require("./binaryPath");
Object.defineProperty(exports, "getBinaryPath", { enumerable: true, get: function () { return binaryPath_1.getBinaryPath; } });
Object.defineProperty(exports, "setBinaryPathOutputChannel", { enumerable: true, get: function () { return binaryPath_1.setOutputChannel; } });
var executor_1 = require("./executor");
Object.defineProperty(exports, "executeBslCommand", { enumerable: true, get: function () { return executor_1.executeBslCommand; } });
Object.defineProperty(exports, "setExecutorOutputChannel", { enumerable: true, get: function () { return executor_1.setOutputChannel; } });
var parser_1 = require("./parser");
Object.defineProperty(exports, "parseMethodCall", { enumerable: true, get: function () { return parser_1.parseMethodCall; } });
Object.defineProperty(exports, "extractTypeName", { enumerable: true, get: function () { return parser_1.extractTypeName; } });
var config_1 = require("./config");
Object.defineProperty(exports, "getConfigurationPath", { enumerable: true, get: function () { return config_1.getConfigurationPath; } });
Object.defineProperty(exports, "getPlatformVersion", { enumerable: true, get: function () { return config_1.getPlatformVersion; } });
Object.defineProperty(exports, "getPlatformDocsArchive", { enumerable: true, get: function () { return config_1.getPlatformDocsArchive; } });
Object.defineProperty(exports, "setConfigOutputChannel", { enumerable: true, get: function () { return config_1.setOutputChannel; } });
/**
 * Инициализирует output channel для всех утилит
 */
function initializeUtils(outputChannel) {
    require('./binaryPath').setOutputChannel(outputChannel);
    require('./executor').setOutputChannel(outputChannel);
    require('./config').setOutputChannel(outputChannel);
}
exports.initializeUtils = initializeUtils;
// Export configuration finder utilities
var configurationFinder_1 = require("./configurationFinder");
Object.defineProperty(exports, "findConfigurations", { enumerable: true, get: function () { return configurationFinder_1.findConfigurations; } });
Object.defineProperty(exports, "findMainConfiguration", { enumerable: true, get: function () { return configurationFinder_1.findMainConfiguration; } });
Object.defineProperty(exports, "selectConfiguration", { enumerable: true, get: function () { return configurationFinder_1.selectConfiguration; } });
Object.defineProperty(exports, "autoDetectConfiguration", { enumerable: true, get: function () { return configurationFinder_1.autoDetectConfiguration; } });
//# sourceMappingURL=index.js.map