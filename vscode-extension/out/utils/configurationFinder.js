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
exports.autoDetectConfiguration = exports.selectConfiguration = exports.findMainConfiguration = exports.findConfigurations = void 0;
const fs = __importStar(require("fs"));
const path = __importStar(require("path"));
const vscode = __importStar(require("vscode"));
const logger_1 = require("../lsp/logger");
/**
 * Находит все конфигурации 1С в директории
 */
async function findConfigurations(rootPath) {
    const configurations = [];
    try {
        const entries = await fs.promises.readdir(rootPath, { withFileTypes: true });
        for (const entry of entries) {
            if (entry.isDirectory()) {
                const configPath = path.join(rootPath, entry.name);
                const configXmlPath = path.join(configPath, 'Configuration.xml');
                if (fs.existsSync(configXmlPath)) {
                    const configInfo = await analyzeConfiguration(configXmlPath);
                    if (configInfo) {
                        configurations.push({
                            ...configInfo,
                            path: configPath,
                            name: entry.name
                        });
                    }
                }
            }
        }
    }
    catch (error) {
        logger_1.logger.error(`Error scanning for configurations: ${error}`);
    }
    return configurations;
}
exports.findConfigurations = findConfigurations;
/**
 * Анализирует Configuration.xml и определяет тип конфигурации
 */
async function analyzeConfiguration(xmlPath) {
    try {
        const content = await fs.promises.readFile(xmlPath, 'utf-8');
        // Проверяем, является ли это расширением
        const isExtension = content.includes('<ConfigurationExtensionPurpose>');
        // Извлекаем UUID конфигурации
        const uuidMatch = content.match(/<Configuration[^>]*uuid="([^"]+)"/);
        const uuid = uuidMatch ? uuidMatch[1] : undefined;
        return { isExtension, uuid };
    }
    catch (error) {
        logger_1.logger.error(`Error analyzing configuration: ${error}`);
        return null;
    }
}
/**
 * Находит основную конфигурацию в workspace
 */
async function findMainConfiguration() {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return null;
    }
    for (const folder of workspaceFolders) {
        // Ищем конфигурации в корне workspace
        let configurations = await findConfigurations(folder.uri.fsPath);
        // Если не нашли в корне, ищем в стандартных директориях
        if (configurations.length === 0) {
            const standardDirs = ['conf', 'src', 'configuration', 'Конфигурация'];
            for (const dir of standardDirs) {
                const dirPath = path.join(folder.uri.fsPath, dir);
                if (fs.existsSync(dirPath)) {
                    configurations = await findConfigurations(dirPath);
                    if (configurations.length > 0)
                        break;
                }
            }
        }
        // Фильтруем только основные конфигурации (не расширения)
        const mainConfigs = configurations.filter(c => !c.isExtension);
        if (mainConfigs.length > 0) {
            // Если несколько основных конфигураций, берем первую
            // В будущем можно добавить диалог выбора
            return mainConfigs[0];
        }
    }
    return null;
}
exports.findMainConfiguration = findMainConfiguration;
/**
 * Показывает диалог выбора конфигурации
 */
async function selectConfiguration(configurations) {
    const items = configurations.map(config => ({
        label: config.name,
        description: config.isExtension ? '📦 Расширение' : '🏢 Основная конфигурация',
        detail: config.path,
        config
    }));
    const selected = await vscode.window.showQuickPick(items, {
        placeHolder: 'Выберите конфигурацию для индексации',
        title: 'BSL Analyzer: Выбор конфигурации'
    });
    return selected ? selected.config : null;
}
exports.selectConfiguration = selectConfiguration;
/**
 * Автоматически определяет и устанавливает путь к конфигурации
 */
async function autoDetectConfiguration(outputChannel) {
    outputChannel?.appendLine('🔍 Searching for 1C configuration in workspace...');
    const isTestMode = process.env.NODE_ENV === 'test' ||
        process.env.VSCODE_TEST_MODE === '1' ||
        vscode.extensions.getExtension('vscode.vscode-api-tests') !== undefined;
    const mainConfig = await findMainConfiguration();
    if (mainConfig) {
        outputChannel?.appendLine(`✅ Found main configuration: ${mainConfig.name} at ${mainConfig.path}`);
        // Сохраняем в настройках
        const config = vscode.workspace.getConfiguration('bslAnalyzer');
        await config.update('configurationPath', mainConfig.path, vscode.ConfigurationTarget.Workspace);
        return mainConfig.path;
    }
    else {
        outputChannel?.appendLine('❌ No 1C configuration found in workspace');
        // В тестовом режиме НЕ показываем UI-диалоги (они приводят к Cancel и ломают активацию/тесты).
        if (isTestMode) {
            outputChannel?.appendLine('ℹ️ [Test Mode] Skipping configuration selection UI');
            return null;
        }
        // Предлагаем выбрать вручную
        const result = await vscode.window.showInformationMessage('Конфигурация 1С не найдена автоматически', 'Выбрать папку', 'Пропустить');
        if (result === 'Выбрать папку') {
            const uri = await vscode.window.showOpenDialog({
                canSelectFolders: true,
                canSelectFiles: false,
                canSelectMany: false,
                openLabel: 'Выбрать конфигурацию',
                title: 'Выберите папку с конфигурацией 1С'
            });
            if (uri && uri.length > 0) {
                const configPath = uri[0].fsPath;
                const config = vscode.workspace.getConfiguration('bslAnalyzer');
                await config.update('configurationPath', configPath, vscode.ConfigurationTarget.Workspace);
                return configPath;
            }
        }
    }
    return null;
}
exports.autoDetectConfiguration = autoDetectConfiguration;
//# sourceMappingURL=configurationFinder.js.map