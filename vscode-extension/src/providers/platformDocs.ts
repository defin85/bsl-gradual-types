import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Элемент дерева для документации платформы с расширенными свойствами
 */
export class PlatformDocItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly version: string,
        contextValue?: string,
        public readonly typesCount?: string,
        public readonly archiveName?: string,
        public readonly lastParsed?: string
    ) {
        super(label, collapsibleState);
        if (contextValue) {
            this.contextValue = contextValue;
        }
        
        if (version && contextValue === 'version') {
            this.tooltip = `Platform ${version}: ${typesCount || '?'} types`;
        }
    }
}

/**
 * Provider для отображения документации платформы
 */
export class BslPlatformDocsProvider implements vscode.TreeDataProvider<PlatformDocItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<PlatformDocItem | undefined | null | void> = new vscode.EventEmitter<PlatformDocItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<PlatformDocItem | undefined | null | void> = this._onDidChangeTreeData.event;
    
    private outputChannel: vscode.OutputChannel | undefined;

    constructor(outputChannel?: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: PlatformDocItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: PlatformDocItem): Thenable<PlatformDocItem[]> {
        if (!element) {
            // Показываем доступные версии платформы из кеша
            return this.getAvailablePlatformVersions();
        } else {
            // Показываем детали для конкретной версии
            const details: PlatformDocItem[] = [];
            
            // Показываем количество типов
            details.push(new PlatformDocItem(`📊 Types: ${element.typesCount || 'Unknown'}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            
            // Показываем информацию об архивах
            if (element.archiveName === 'Both archives') {
                details.push(new PlatformDocItem(`✅ Status: Complete (shcntx + shlang)`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`📂 Archive: shcntx_ru.zip`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`📂 Archive: shlang_ru.zip`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            } else if (element.archiveName && element.archiveName.includes('shcntx')) {
                details.push(new PlatformDocItem(`📂 Archive: ${element.archiveName}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`⚠️ Missing: shlang_ru.zip (primitive types)`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            } else if (element.archiveName && element.archiveName.includes('shlang')) {
                details.push(new PlatformDocItem(`📂 Archive: ${element.archiveName}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
                details.push(new PlatformDocItem(`⚠️ Missing: shcntx_ru.zip (object types)`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            } else {
                details.push(new PlatformDocItem(`📦 Archive: ${element.archiveName || 'Unknown'}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            }
            
            // Показываем дату добавления
            details.push(new PlatformDocItem(`🕒 Added: ${element.lastParsed || 'Unknown'}`, vscode.TreeItemCollapsibleState.None, element.version, 'info'));
            
            // Добавляем кнопку удаления
            const removeItem = new PlatformDocItem(`🗑️ Remove this version`, vscode.TreeItemCollapsibleState.None, element.version, 'remove-version');
            removeItem.command = {
                command: 'bslAnalyzer.removePlatformDocs',
                title: 'Remove Platform Documentation',
                arguments: [element]
            };
            details.push(removeItem);
            
            return Promise.resolve(details);
        }
    }

    private async getAvailablePlatformVersions(): Promise<PlatformDocItem[]> {
        const items: PlatformDocItem[] = [];
        
        // Проверяем наличие кеша платформенной документации
        const homedir = require('os').homedir();
        const cacheDir = path.join(homedir, '.bsl_analyzer', 'platform_cache');
        
        if (fs.existsSync(cacheDir)) {
            // Читаем список версий из кеша
            const files = fs.readdirSync(cacheDir);
            // Поддерживаем оба формата: с префиксом "v" и без него
            const versionFiles = files.filter(f => f.match(/^v?[\d.]+\.jsonl$/));
            
            for (const versionFile of versionFiles) {
                const version = versionFile.replace(/^v/, '').replace('.jsonl', '');
                
                // Пытаемся прочитать количество типов из файла
                let typesCount = '?';
                let archiveInfo = 'Unknown';
                try {
                    const filePath = path.join(cacheDir, versionFile);
                    const content = fs.readFileSync(filePath, 'utf-8');
                    const lines = content.trim().split('\n');
                    typesCount = lines.length.toLocaleString();
                    
                    // Анализируем содержимое для определения типа архивов
                    let hasObjectTypes = false;
                    let hasPrimitiveTypes = false;
                    
                    for (const line of lines.slice(0, 100)) { // Проверяем первые 100 строк
                        try {
                            const entity = JSON.parse(line);
                            if (entity.name) {
                                // Проверка на объектные типы (из shcntx)
                                if (entity.name.includes('Массив') || entity.name.includes('Array') ||
                                    entity.name.includes('ТаблицаЗначений') || entity.name.includes('ValueTable')) {
                                    hasObjectTypes = true;
                                }
                                // Проверка на примитивные типы (из shlang)
                                if (entity.name === 'Число' || entity.name === 'Number' ||
                                    entity.name === 'Строка' || entity.name === 'String' ||
                                    entity.name === 'Булево' || entity.name === 'Boolean') {
                                    hasPrimitiveTypes = true;
                                }
                            }
                        } catch (e) {
                            // Игнорируем ошибки парсинга
                        }
                    }
                    
                    if (hasObjectTypes && hasPrimitiveTypes) {
                        archiveInfo = 'Both archives';
                    } else if (hasObjectTypes) {
                        archiveInfo = 'shcntx_ru.zip';
                    } else if (hasPrimitiveTypes) {
                        archiveInfo = 'shlang_ru.zip';
                    }
                    
                } catch (e) {
                    this.outputChannel?.appendLine(`Error reading platform cache: ${e}`);
                }
                
                const lastModified = fs.statSync(path.join(cacheDir, versionFile)).mtime.toLocaleDateString();
                
                // Логируем найденную версию
                this.outputChannel?.appendLine(`Found platform docs: v${version} - ${typesCount} types, archive: ${archiveInfo}`);
                
                items.push(
                    new PlatformDocItem(
                        `📋 Platform ${version} (${typesCount} types)`,
                        vscode.TreeItemCollapsibleState.Expanded,
                        version,
                        'version',
                        typesCount,
                        archiveInfo,
                        lastModified
                    )
                );
            }
        }
        
        // Всегда добавляем кнопку для добавления документации
        const addDocsItem = new PlatformDocItem('➕ Add Platform Documentation...', vscode.TreeItemCollapsibleState.None, '', 'add-docs');
        addDocsItem.command = {
            command: 'bslAnalyzer.addPlatformDocs',
            title: 'Add Platform Documentation',
            arguments: []
        };
        items.push(addDocsItem);
        
        return items;
    }
}