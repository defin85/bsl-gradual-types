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
exports.parsePlatformDocumentation = exports.removePlatformDocumentation = exports.addPlatformDocumentation = exports.initializePlatformDocs = void 0;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
const os = __importStar(require("os"));
const utils_1 = require("../utils");
const progress_1 = require("../lsp/progress");
let outputChannel;
/**
 * Инициализирует модуль платформенной документации
 */
function initializePlatformDocs(channel) {
    outputChannel = channel;
}
exports.initializePlatformDocs = initializePlatformDocs;
/**
 * Добавляет платформенную документацию
 */
async function addPlatformDocumentation(provider) {
    try {
        // 1. Спросим у пользователя версию платформы
        const version = await vscode.window.showInputBox({
            prompt: 'Enter platform version (e.g., 8.3.25)',
            placeHolder: '8.3.25',
            value: '8.3.25'
        });
        if (!version) {
            return;
        }
        // 2. Выберем архив с документацией
        const archiveFiles = await vscode.window.showOpenDialog({
            canSelectFiles: true,
            canSelectMany: false,
            filters: {
                'Help Archives': ['zip']
            },
            openLabel: 'Select Platform Documentation Archive (shcntx or shlang)'
        });
        if (!archiveFiles || archiveFiles.length === 0) {
            return;
        }
        const firstFile = archiveFiles[0];
        if (!firstFile) {
            return;
        }
        const archivePath = firstFile.fsPath;
        const archiveDir = path.dirname(archivePath);
        const archiveName = path.basename(archivePath);
        // Определяем тип архива и ищем companion архив
        let shcntxPath;
        let shlangPath;
        let totalTypesCount = 0;
        if (archiveName.includes('shcntx')) {
            shcntxPath = archivePath;
            // Ищем shlang архив в той же папке
            const possibleShlangFiles = [
                'rebuilt.shlang_ru.zip',
                'shlang_ru.zip',
                archiveName.replace('shcntx', 'shlang')
            ];
            for (const shlangFile of possibleShlangFiles) {
                const shlangFullPath = path.join(archiveDir, shlangFile);
                if (fs.existsSync(shlangFullPath)) {
                    shlangPath = shlangFullPath;
                    outputChannel.appendLine(`📂 Found companion archive: ${shlangFile}`);
                    break;
                }
            }
        }
        else if (archiveName.includes('shlang')) {
            shlangPath = archivePath;
            // Ищем shcntx архив в той же папке
            const possibleShcntxFiles = [
                'rebuilt.shcntx_ru.zip',
                'shcntx_ru.zip',
                archiveName.replace('shlang', 'shcntx')
            ];
            for (const shcntxFile of possibleShcntxFiles) {
                const shcntxFullPath = path.join(archiveDir, shcntxFile);
                if (fs.existsSync(shcntxFullPath)) {
                    shcntxPath = shcntxFullPath;
                    outputChannel.appendLine(`📂 Found companion archive: ${shcntxFile}`);
                    break;
                }
            }
        }
        // 3. Выполним парсинг через бинарь с прогрессом
        const stepsCount = (shcntxPath && shlangPath) ? 5 : 3; // Больше шагов если есть оба архива
        (0, progress_1.startIndexing)(stepsCount);
        outputChannel.appendLine('ℹ️ Using force mode to replace existing documentation if present');
        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: `Adding/updating platform documentation for version ${version}...`,
            cancellable: false
        }, async (progress) => {
            try {
                let currentStep = 1;
                // extract_platform_docs автоматически находит второй архив в той же директории
                // Поэтому достаточно вызвать один раз с любым из архивов
                const primaryArchive = shcntxPath || shlangPath;
                if (primaryArchive) {
                    (0, progress_1.updateIndexingProgress)(currentStep++, 'Processing platform documentation archives...', 25);
                    progress.report({ increment: 25, message: 'Extracting platform types from archives...' });
                    const extractResult = await (0, utils_1.executeBslCommand)('extract_platform_docs', [
                        '--archive', primaryArchive,
                        '--platform-version', version,
                        '--force' // Всегда форсируем при ручном добавлении документации
                    ]);
                    // Ищем количество типов в выводе
                    // extract_platform_docs обрабатывает оба архива и выводит общее количество
                    const typesMatch = extractResult.match(/(\d+)\s+types/i) ||
                        extractResult.match(/(\d+)\s+entities/i) ||
                        extractResult.match(/Objects\s+│\s+(\d+)/i);
                    const savedMatch = extractResult.match(/Saved\s+(\d+)\s+platform\s+types/i);
                    if (savedMatch && savedMatch[1]) {
                        totalTypesCount = parseInt(savedMatch[1]);
                    }
                    else if (typesMatch && typesMatch[1]) {
                        totalTypesCount = parseInt(typesMatch[1]);
                    }
                    // Проверяем, были ли обработаны оба архива
                    const hasAutoDetected = extractResult.includes('Auto-detected');
                    if (hasAutoDetected) {
                        outputChannel.appendLine(`✅ Both archives processed automatically`);
                    }
                    outputChannel.appendLine(`✅ Platform documentation extracted: ${totalTypesCount} types`);
                }
                // Финализация
                (0, progress_1.updateIndexingProgress)(currentStep++, 'Finalizing...', 95);
                progress.report({ increment: 20, message: 'Finalizing...' });
                (0, progress_1.finishIndexing)(true);
                // Формируем сообщение о результате
                let message = `✅ Platform documentation added for version ${version}`;
                if (shcntxPath && shlangPath) {
                    message += ` (${totalTypesCount} total types from both archives)`;
                }
                else if (shcntxPath) {
                    message += ` (${totalTypesCount} types from shcntx)`;
                    if (!shlangPath) {
                        message += '\n⚠️ Note: shlang archive not found - primitive types may be incomplete';
                    }
                }
                else if (shlangPath) {
                    message += ` (${totalTypesCount} primitive types from shlang)`;
                    if (!shcntxPath) {
                        message += '\n⚠️ Note: shcntx archive not found - object types may be incomplete';
                    }
                }
                vscode.window.showInformationMessage(message);
                outputChannel.appendLine(message);
                // Обновляем панель
                provider.refresh();
            }
            catch (error) {
                (0, progress_1.finishIndexing)(false);
                vscode.window.showErrorMessage(`Failed to add platform documentation: ${error}`);
                outputChannel.appendLine(`Error adding platform docs: ${error}`);
            }
        });
    }
    catch (error) {
        vscode.window.showErrorMessage(`Failed to add platform documentation: ${error}`);
        outputChannel.appendLine(`Error: ${error}`);
    }
}
exports.addPlatformDocumentation = addPlatformDocumentation;
/**
 * Удаляет платформенную документацию
 */
async function removePlatformDocumentation(version, provider) {
    const choice = await vscode.window.showWarningMessage(`Are you sure you want to remove platform documentation for version ${version}?`, { modal: true }, 'Remove');
    if (choice === 'Remove') {
        try {
            // Определяем пути к кешу
            const homeDir = os.homedir();
            const cacheBasePath = path.join(homeDir, '.bsl_analyzer', 'platform_cache');
            const versionFile = path.join(cacheBasePath, `v${version}.jsonl`);
            outputChannel.appendLine(`Removing platform cache for version ${version}`);
            outputChannel.appendLine(`Cache file: ${versionFile}`);
            // Проверяем существование файла
            if (fs.existsSync(versionFile)) {
                // Удаляем файл кеша
                fs.unlinkSync(versionFile);
                outputChannel.appendLine(`✅ Successfully removed cache file: ${versionFile}`);
                // Также удаляем связанные индексы проектов для этой версии
                const projectIndicesPath = path.join(homeDir, '.bsl_analyzer', 'project_indices');
                if (fs.existsSync(projectIndicesPath)) {
                    const projects = fs.readdirSync(projectIndicesPath);
                    for (const project of projects) {
                        const versionPath = path.join(projectIndicesPath, project, `v${version}`);
                        if (fs.existsSync(versionPath)) {
                            // Рекурсивно удаляем директорию версии
                            fs.rmSync(versionPath, { recursive: true, force: true });
                            outputChannel.appendLine(`✅ Removed project index: ${versionPath}`);
                        }
                    }
                }
                vscode.window.showInformationMessage(`✅ Platform documentation for version ${version} has been removed`);
            }
            else {
                outputChannel.appendLine(`⚠️ Cache file not found: ${versionFile}`);
                vscode.window.showWarningMessage(`Platform documentation cache for version ${version} not found`);
            }
            // Обновляем панель
            provider.refresh();
        }
        catch (error) {
            vscode.window.showErrorMessage(`Failed to remove platform documentation: ${error}`);
            outputChannel.appendLine(`Error removing platform docs: ${error}`);
        }
    }
}
exports.removePlatformDocumentation = removePlatformDocumentation;
/**
 * Перепарсит платформенную документацию
 */
async function parsePlatformDocumentation(version) {
    (0, progress_1.startIndexing)(3); // 3 этапа для ре-парсинга
    await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `Re-parsing platform documentation for version ${version}...`,
        cancellable: false
    }, async (progress) => {
        try {
            // Этап 1: Инициализация
            (0, progress_1.updateIndexingProgress)(1, 'Initializing re-parse...', 15);
            progress.report({ increment: 30, message: 'Initializing re-parse...' });
            // Этап 2: Построение индекса
            (0, progress_1.updateIndexingProgress)(2, 'Building unified index...', 70);
            progress.report({ increment: 55, message: 'Building unified index...' });
            const args = [
                '--platform-version', version,
                '--force-rebuild'
            ];
            const platformDocsArchive = (0, utils_1.getPlatformDocsArchive)();
            if (platformDocsArchive) {
                args.push('--platform-docs-archive', platformDocsArchive);
            }
            const result = await (0, utils_1.executeBslCommand)('build_unified_index', args);
            // Этап 3: Завершение
            (0, progress_1.updateIndexingProgress)(3, 'Finalizing...', 95);
            progress.report({ increment: 15, message: 'Finalizing...' });
            (0, progress_1.finishIndexing)(true);
            vscode.window.showInformationMessage(`✅ Platform documentation re-parsed successfully for version ${version}`);
            outputChannel.appendLine(`Re-parse result: ${result}`);
        }
        catch (error) {
            (0, progress_1.finishIndexing)(false);
            vscode.window.showErrorMessage(`Failed to re-parse platform documentation: ${error}`);
            outputChannel.appendLine(`Error re-parsing platform docs: ${error}`);
        }
    });
}
exports.parsePlatformDocumentation = parsePlatformDocumentation;
//# sourceMappingURL=manager.js.map