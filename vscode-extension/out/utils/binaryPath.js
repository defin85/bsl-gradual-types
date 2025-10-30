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
exports.getBinaryPath = exports.setOutputChannel = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const configHelper_1 = require("../config/configHelper");
let outputChannel;
function setOutputChannel(channel) {
    outputChannel = channel;
}
exports.setOutputChannel = setOutputChannel;
/**
 * Получает путь к бинарному файлу BSL Analyzer
 * @param binaryName Имя бинарного файла (без расширения)
 * @param extensionContext Контекст расширения
 * @returns Полный путь к бинарному файлу
 */
function getBinaryPath(binaryName, extensionContext) {
    const useBundled = configHelper_1.BslAnalyzerConfig.useBundledBinaries;
    // Если явно указано использовать встроенные бинарники
    if (useBundled) {
        // Сначала пробуем глобальный контекст (для development режима)
        if (extensionContext) {
            const contextBinPath = path.join(extensionContext.extensionPath, 'bin', `${binaryName}.exe`);
            if (fs.existsSync(contextBinPath)) {
                outputChannel?.appendLine(`✅ Using bundled binary from context: ${contextBinPath}`);
                return contextBinPath;
            }
        }
        // Затем пробуем найти установленное расширение
        const extensionPath = vscode.extensions.getExtension('bsl-analyzer-team.bsl-type-safety-analyzer')?.extensionPath;
        if (extensionPath) {
            const bundledBinPath = path.join(extensionPath, 'bin', `${binaryName}.exe`);
            if (fs.existsSync(bundledBinPath)) {
                outputChannel?.appendLine(`✅ Using bundled binary: ${bundledBinPath}`);
                return bundledBinPath;
            }
        }
        // Fallback на vscode-extension/bin для development
        const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (workspacePath) {
            const devBinPath = path.join(workspacePath, 'vscode-extension', 'bin', `${binaryName}.exe`);
            if (fs.existsSync(devBinPath)) {
                outputChannel?.appendLine(`✅ Using development binary: ${devBinPath}`);
                return devBinPath;
            }
        }
    }
    // Если указан внешний путь к бинарникам
    const binaryPath = configHelper_1.BslAnalyzerConfig.binaryPath;
    if (binaryPath) {
        const externalBinPath = path.join(binaryPath, `${binaryName}.exe`);
        if (fs.existsSync(externalBinPath)) {
            outputChannel?.appendLine(`✅ Using external binary: ${externalBinPath}`);
            return externalBinPath;
        }
        outputChannel?.appendLine(`❌ Binary not found in specified path: ${externalBinPath}`);
    }
    // Последняя попытка - проверить в PATH
    const pathBinary = `${binaryName}.exe`;
    outputChannel?.appendLine(`⚠️ Attempting to use binary from PATH: ${pathBinary}`);
    return pathBinary;
}
exports.getBinaryPath = getBinaryPath;
//# sourceMappingURL=binaryPath.js.map