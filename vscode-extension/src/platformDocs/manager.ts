import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import {
    executeBslCommand,
    getPlatformDocsArchive
} from '../utils';
import { 
    startIndexing, 
    updateIndexingProgress, 
    finishIndexing 
} from '../lsp/progress';
import { BslPlatformDocsProvider } from '../providers';

let outputChannel: vscode.OutputChannel;

/**
 * Инициализирует модуль платформенной документации
 */
export function initializePlatformDocs(channel: vscode.OutputChannel) {
    outputChannel = channel;
}

/**
 * Добавляет платформенную документацию
 */
export async function addPlatformDocumentation(provider: BslPlatformDocsProvider): Promise<void> {
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
        let shcntxPath: string | undefined;
        let shlangPath: string | undefined;
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
        } else if (archiveName.includes('shlang')) {
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
        startIndexing(stepsCount);
        
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
                    updateIndexingProgress(currentStep++, 'Processing platform documentation archives...', 25);
                    progress.report({ increment: 25, message: 'Extracting platform types from archives...' });
                    
                    const extractResult = await executeBslCommand('extract_platform_docs', [
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
                    } else if (typesMatch && typesMatch[1]) {
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
                updateIndexingProgress(currentStep++, 'Finalizing...', 95);
                progress.report({ increment: 20, message: 'Finalizing...' });
                
                finishIndexing(true);
                
                // Формируем сообщение о результате
                let message = `✅ Platform documentation added for version ${version}`;
                if (shcntxPath && shlangPath) {
                    message += ` (${totalTypesCount} total types from both archives)`;
                } else if (shcntxPath) {
                    message += ` (${totalTypesCount} types from shcntx)`;
                    if (!shlangPath) {
                        message += '\n⚠️ Note: shlang archive not found - primitive types may be incomplete';
                    }
                } else if (shlangPath) {
                    message += ` (${totalTypesCount} primitive types from shlang)`;
                    if (!shcntxPath) {
                        message += '\n⚠️ Note: shcntx archive not found - object types may be incomplete';
                    }
                }
                
                vscode.window.showInformationMessage(message);
                outputChannel.appendLine(message);
                
                // Обновляем панель
                provider.refresh();
                
            } catch (error) {
                finishIndexing(false);
                vscode.window.showErrorMessage(`Failed to add platform documentation: ${error}`);
                outputChannel.appendLine(`Error adding platform docs: ${error}`);
            }
        });

    } catch (error) {
        vscode.window.showErrorMessage(`Failed to add platform documentation: ${error}`);
        outputChannel.appendLine(`Error: ${error}`);
    }
}

/**
 * Удаляет платформенную документацию
 */
export async function removePlatformDocumentation(version: string, provider: BslPlatformDocsProvider): Promise<void> {
    const choice = await vscode.window.showWarningMessage(
        `Are you sure you want to remove platform documentation for version ${version}?`,
        { modal: true },
        'Remove'
    );

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
                
                vscode.window.showInformationMessage(
                    `✅ Platform documentation for version ${version} has been removed`
                );
            } else {
                outputChannel.appendLine(`⚠️ Cache file not found: ${versionFile}`);
                vscode.window.showWarningMessage(
                    `Platform documentation cache for version ${version} not found`
                );
            }
            
            // Обновляем панель
            provider.refresh();
            
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to remove platform documentation: ${error}`);
            outputChannel.appendLine(`Error removing platform docs: ${error}`);
        }
    }
}

/**
 * Перепарсит платформенную документацию
 */
export async function parsePlatformDocumentation(version: string): Promise<void> {
    startIndexing(3); // 3 этапа для ре-парсинга
    
    await vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: `Re-parsing platform documentation for version ${version}...`,
        cancellable: false
    }, async (progress) => {
        try {
            // Этап 1: Инициализация
            updateIndexingProgress(1, 'Initializing re-parse...', 15);
            progress.report({ increment: 30, message: 'Initializing re-parse...' });
            
            // Этап 2: Построение индекса
            updateIndexingProgress(2, 'Building unified index...', 70);
            progress.report({ increment: 55, message: 'Building unified index...' });
            
            const args = [
                '--platform-version', version,
                '--force-rebuild'
            ];
            
            const platformDocsArchive = getPlatformDocsArchive();
            if (platformDocsArchive) {
                args.push('--platform-docs-archive', platformDocsArchive);
            }
            
            const result = await executeBslCommand('build_unified_index', args);

            // Этап 3: Завершение
            updateIndexingProgress(3, 'Finalizing...', 95);
            progress.report({ increment: 15, message: 'Finalizing...' });
            
            finishIndexing(true);

            vscode.window.showInformationMessage(
                `✅ Platform documentation re-parsed successfully for version ${version}`
            );
            
            outputChannel.appendLine(`Re-parse result: ${result}`);
            
        } catch (error) {
            finishIndexing(false);
            vscode.window.showErrorMessage(`Failed to re-parse platform documentation: ${error}`);
            outputChannel.appendLine(`Error re-parsing platform docs: ${error}`);
        }
    });
}