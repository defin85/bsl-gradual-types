import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { BslAnalyzerConfig } from '../config/configHelper';
import { BslEntity, TypeCategory, HierarchicalTypeItem } from './typeModels';
import { getCategoryIcon, getConfigCategoryName, PLATFORM_CATEGORIES, formatMethodTooltip, formatPropertyTooltip } from './typeFormatter';

/**
 * Построитель дерева типов BSL
 *
 * Загружает типы из кеша и категоризирует их для отображения в TreeView
 */
export class TypeTreeBuilder {
    private outputChannel: vscode.OutputChannel | undefined;
    private platformTypes: Map<string, BslEntity> = new Map();
    private configTypes: Map<string, BslEntity> = new Map();
    private typeCategories: Map<string, TypeCategory> = new Map();

    constructor(outputChannel?: vscode.OutputChannel) {
        this.outputChannel = outputChannel;
    }

    /**
     * Загружает и категоризирует все типы
     */
    async loadTypes(): Promise<void> {
        this.platformTypes.clear();
        this.configTypes.clear();
        this.typeCategories.clear();

        // TODO Milestone 2.10: Запрашивать типы через LSP Custom Request (bsl/getAllTypes)
        // ВРЕМЕННО ОТКЛЮЧЕНО (Milestone 2.9): Убираем дублирование кеша типов
        // Теперь единственный источник истины - TypeRepository в LSP Server
        // Extension будет запрашивать типы через LSP вместо прямого чтения JSONL

        // Загружаем типы платформы
        // await this.loadPlatformTypes(); // ВРЕМЕННО ОТКЛЮЧЕНО

        // Загружаем типы конфигурации
        // await this.loadConfigurationTypes(); // ВРЕМЕННО ОТКЛЮЧЕНО

        // Группируем типы по категориям
        // this.categorizeTypes(); // ВРЕМЕННО ОТКЛЮЧЕНО
    }

    /**
     * Возвращает количество типов платформы
     */
    get platformTypesCount(): number {
        return this.platformTypes.size;
    }

    /**
     * Возвращает количество типов конфигурации
     */
    get configTypesCount(): number {
        return this.configTypes.size;
    }

    /**
     * Возвращает количество категорий
     */
    get categoriesCount(): number {
        return this.typeCategories.size;
    }

    /**
     * Получает сущность типа по имени
     */
    getEntity(typeName: string): BslEntity | undefined {
        return this.platformTypes.get(typeName) || this.configTypes.get(typeName);
    }

    /**
     * Проверяет, есть ли у сущности методы, свойства или события
     */
    hasMembers(entity: BslEntity): boolean {
        const hasMethod = entity.interface?.methods && Object.keys(entity.interface.methods).length > 0;
        const hasProps = entity.interface?.properties && Object.keys(entity.interface.properties).length > 0;
        const hasEvents = entity.interface?.events && Object.keys(entity.interface.events).length > 0;
        return !!(hasMethod || hasProps || hasEvents);
    }

    /**
     * Извлекает UUID проекта из Configuration.xml
     */
    extractUuidProjectId(configPath: string): string | null {
        try {
            const configXmlPath = path.join(configPath, 'Configuration.xml');
            if (!fs.existsSync(configXmlPath)) return null;
            const xml = fs.readFileSync(configXmlPath, 'utf-8');
            const m = xml.match(/<Configuration[^>]*uuid="([^"]+)"/i);
            if (m && m[1]) {
                const uuid = m[1].replace(/-/g, '');
                return `${path.basename(configPath)}_${uuid}`;
            }
        } catch (e) {
            this.outputChannel?.appendLine(`Error extracting UUID: ${e}`);
        }
        return null;
    }

    // ============ Методы получения категорий для TreeView ============

    /**
     * Возвращает категории платформы для дерева
     */
    getPlatformCategories(): HierarchicalTypeItem[] {
        const categories: HierarchicalTypeItem[] = [];

        for (const [key, category] of this.typeCategories) {
            if (key.startsWith('platform:')) {
                const categoryItem = new HierarchicalTypeItem(
                    `${category.icon} ${category.name} (${category.types.length})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    category.name,
                    'category',
                    key
                );
                categories.push(categoryItem);
            }
        }

        return categories;
    }

    /**
     * Возвращает категории конфигурации для дерева
     */
    getConfigCategories(): HierarchicalTypeItem[] {
        const categories: HierarchicalTypeItem[] = [];

        for (const [key, category] of this.typeCategories) {
            if (key.startsWith('config:')) {
                const categoryItem = new HierarchicalTypeItem(
                    `${category.icon} ${category.name} (${category.types.length})`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    category.name,
                    'category',
                    key
                );
                categories.push(categoryItem);
            }
        }

        return categories;
    }

    /**
     * Возвращает типы для категории
     */
    getCategoryTypes(categoryKey: string): HierarchicalTypeItem[] {
        const category = this.typeCategories.get(categoryKey);
        if (!category) return [];

        return category.types.slice(0, 100).map(entity => {
            const hasMembers = this.hasMembers(entity);
            return new HierarchicalTypeItem(
                entity.display_name || entity.qualified_name,
                hasMembers ? vscode.TreeItemCollapsibleState.Collapsed : vscode.TreeItemCollapsibleState.None,
                entity.qualified_name,
                'type',
                entity.qualified_name
            );
        });
    }

    /**
     * Возвращает элементы для отображения членов типа (методы, свойства)
     */
    getTypeMembers(typeName: string): HierarchicalTypeItem[] {
        const entity = this.getEntity(typeName);
        if (!entity) return [];

        const items: HierarchicalTypeItem[] = [];

        // Добавляем папку с методами
        const methodCount = entity.interface?.methods ? Object.keys(entity.interface.methods).length : 0;
        if (methodCount > 0) {
            const methodsFolder = new HierarchicalTypeItem(
                `📦 Методы (${methodCount})`,
                vscode.TreeItemCollapsibleState.Collapsed,
                'Методы',
                'methods-folder',
                typeName
            );
            items.push(methodsFolder);
        }

        // Добавляем папку со свойствами
        const propCount = entity.interface?.properties ? Object.keys(entity.interface.properties).length : 0;
        if (propCount > 0) {
            const propsFolder = new HierarchicalTypeItem(
                `📋 Свойства (${propCount})`,
                vscode.TreeItemCollapsibleState.Collapsed,
                'Свойства',
                'properties-folder',
                typeName
            );
            items.push(propsFolder);
        }

        // Добавляем описание, если есть
        if (entity.documentation) {
            const docItem = new HierarchicalTypeItem(
                `📄 ${entity.documentation.substring(0, 100)}...`,
                vscode.TreeItemCollapsibleState.None,
                'Описание',
                'documentation'
            );
            docItem.tooltip = entity.documentation;
            items.push(docItem);
        }

        return items;
    }

    /**
     * Возвращает методы типа
     */
    getTypeMethods(typeName: string): HierarchicalTypeItem[] {
        const entity = this.getEntity(typeName);
        if (!entity || !entity.interface?.methods) return [];

        return Object.entries(entity.interface.methods).slice(0, 50).map(([name, method]) => {
            const item = new HierarchicalTypeItem(
                `⚡ ${name}`,
                vscode.TreeItemCollapsibleState.None,
                name,
                'method'
            );
            item.tooltip = formatMethodTooltip(name, method);
            return item;
        });
    }

    /**
     * Возвращает свойства типа
     */
    getTypeProperties(typeName: string): HierarchicalTypeItem[] {
        const entity = this.getEntity(typeName);
        if (!entity || !entity.interface?.properties) return [];

        return Object.entries(entity.interface.properties).slice(0, 50).map(([name, prop]) => {
            const item = new HierarchicalTypeItem(
                `📌 ${name}`,
                vscode.TreeItemCollapsibleState.None,
                name,
                'property'
            );
            item.tooltip = formatPropertyTooltip(name, prop);
            return item;
        });
    }

    // ============ Приватные методы загрузки и категоризации ============

    private async loadPlatformTypes(): Promise<void> {
        try {
            const homedir = require('os').homedir();
            const platformVersion = BslAnalyzerConfig.platformVersion;
            const platformCachePath = path.join(homedir, '.bsl_analyzer', 'platform_cache', `${platformVersion}.jsonl`);

            if (fs.existsSync(platformCachePath)) {
                const content = fs.readFileSync(platformCachePath, 'utf-8');
                const lines = content.trim().split('\n');

                for (const line of lines) {
                    try {
                        const entity: BslEntity = JSON.parse(line);
                        if (entity.qualified_name) {
                            this.platformTypes.set(entity.qualified_name, entity);
                        }
                    } catch (e) {
                        // Игнорируем ошибки парсинга
                    }
                }

                this.outputChannel?.appendLine(`Loaded ${this.platformTypes.size} platform types`);
            }
        } catch (error) {
            this.outputChannel?.appendLine(`Error loading platform types: ${error}`);
        }
    }

    private async loadConfigurationTypes(): Promise<void> {
        try {
            const configPath = BslAnalyzerConfig.configurationPath;
            this.outputChannel?.appendLine(`Loading config types from: ${configPath || 'not set'}`);
            if (!configPath) {
                this.outputChannel?.appendLine('Configuration path not set, skipping config types');
                return;
            }

            const homedir = require('os').homedir();
            const platformVersion = BslAnalyzerConfig.platformVersion;

            // Extract UUID from Configuration.xml to match Rust's approach
            const projectId: string | null = this.extractUuidProjectId(configPath);
            if (!projectId) {
                this.outputChannel?.appendLine('UUID not found in Configuration.xml; configuration cache will not be located (no fallback by design)');
                return; // прекращаем загрузку типов конфигурации
            }

            const projectCachePath = path.join(
                homedir,
                '.bsl_analyzer',
                'project_indices',
                projectId!,
                platformVersion,
                'config_entities.jsonl'
            );

            this.outputChannel?.appendLine(`Looking for config cache at: ${projectCachePath}`);

            if (fs.existsSync(projectCachePath)) {
                this.outputChannel?.appendLine('Config cache found, loading...');
                const content = fs.readFileSync(projectCachePath, 'utf-8');
                const lines = content.trim().split('\n');

                for (const line of lines) {
                    try {
                        const entity: BslEntity = JSON.parse(line);
                        if (entity.qualified_name) {
                            this.configTypes.set(entity.qualified_name, entity);
                        }
                    } catch (e) {
                        // Игнорируем ошибки парсинга
                    }
                }

                this.outputChannel?.appendLine(`Loaded ${this.configTypes.size} configuration types`);
            } else {
                this.outputChannel?.appendLine('Config cache not found');
            }
        } catch (error) {
            this.outputChannel?.appendLine(`Error loading configuration types: ${error}`);
        }
    }

    private categorizeTypes(): void {
        // Создаем категории для платформенных типов
        for (const [categoryName, typePatterns] of Object.entries(PLATFORM_CATEGORIES)) {
            const category: TypeCategory = {
                name: categoryName,
                icon: getCategoryIcon(categoryName),
                types: []
            };

            for (const [typeName, entity] of this.platformTypes) {
                if (this.matchesCategory(typeName, entity.display_name, typePatterns)) {
                    category.types.push(entity);
                }
            }

            if (category.types.length > 0) {
                this.typeCategories.set(`platform:${categoryName}`, category);
            }
        }

        // Категории для типов конфигурации
        if (this.configTypes.size > 0) {
            const configCategories: Map<string, TypeCategory> = new Map();

            for (const [, entity] of this.configTypes) {
                const categoryName = getConfigCategoryName(entity.entity_kind || 'Other');

                if (!configCategories.has(categoryName)) {
                    configCategories.set(categoryName, {
                        name: categoryName,
                        icon: getCategoryIcon(categoryName),
                        types: []
                    });
                }

                configCategories.get(categoryName)!.types.push(entity);
            }

            for (const [categoryName, category] of configCategories) {
                this.typeCategories.set(`config:${categoryName}`, category);
            }
        }

        // Добавляем категорию "Все остальные" для неклассифицированных типов платформы
        const uncategorized: BslEntity[] = [];
        for (const [, entity] of this.platformTypes) {
            let found = false;
            for (const category of this.typeCategories.values()) {
                if (category.types.includes(entity)) {
                    found = true;
                    break;
                }
            }
            if (!found) {
                uncategorized.push(entity);
            }
        }

        if (uncategorized.length > 0) {
            this.typeCategories.set('platform:Другие', {
                name: 'Другие типы платформы',
                icon: '📦',
                types: uncategorized
            });
        }
    }

    private matchesCategory(typeName: string, displayName: string, patterns: string[]): boolean {
        for (const pattern of patterns) {
            if (typeName.includes(pattern) || displayName?.includes(pattern)) {
                return true;
            }
        }
        return false;
    }
}
