"use strict";
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
exports.getPlatformDocsArchive = exports.getPlatformVersion = exports.getConfigurationPath = exports.setOutputChannel = void 0;
const config_1 = require("../config");
const fs = __importStar(require("fs"));
let outputChannel;
function setOutputChannel(channel) {
    outputChannel = channel;
}
exports.setOutputChannel = setOutputChannel;
/**
 * Получить путь к конфигурации
 */
function getConfigurationPath() {
    return config_1.BslAnalyzerConfig.configurationPath;
}
exports.getConfigurationPath = getConfigurationPath;
/**
 * Получить версию платформы
 */
function getPlatformVersion() {
    return config_1.BslAnalyzerConfig.platformVersion;
}
exports.getPlatformVersion = getPlatformVersion;
/**
 * Получить путь к архиву документации платформы
 */
function getPlatformDocsArchive() {
    const userArchive = config_1.BslAnalyzerConfig.platformDocsArchive;
    if (userArchive && fs.existsSync(userArchive)) {
        outputChannel?.appendLine(`📚 Using user-specified platform documentation: ${userArchive}`);
        return userArchive;
    }
    if (!userArchive) {
        outputChannel?.appendLine(`⚠️ Platform documentation not configured. Some features may be limited.`);
        outputChannel?.appendLine(`💡 Specify path to rebuilt.shcntx_ru.zip or rebuilt.shlang_ru.zip in settings.`);
    }
    else {
        outputChannel?.appendLine(`❌ Platform documentation not found at: ${userArchive}`);
    }
    return '';
}
exports.getPlatformDocsArchive = getPlatformDocsArchive;
//# sourceMappingURL=config.js.map