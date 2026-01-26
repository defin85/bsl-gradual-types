import * as vscode from 'vscode';
import { BslEntity, HierarchicalTypeItem } from './typeModels';
/**
 * Построитель дерева типов BSL
 *
 * Загружает типы из кеша и категоризирует их для отображения в TreeView
 */
export declare class TypeTreeBuilder {
    private outputChannel;
    private platformTypes;
    private configTypes;
    private typeCategories;
    constructor(outputChannel?: vscode.OutputChannel);
    /**
     * Загружает и категоризирует все типы через LSP
     */
    loadTypes(): Promise<void>;
    /**
     * Конвертирует TypeDto из LSP в BslEntity для TreeView
     */
    private convertTypeDtoToBslEntity;
    /**
     * Возвращает количество типов платформы
     */
    get platformTypesCount(): number;
    /**
     * Возвращает количество типов конфигурации
     */
    get configTypesCount(): number;
    /**
     * Возвращает количество категорий
     */
    get categoriesCount(): number;
    /**
     * Получает сущность типа по имени
     */
    getEntity(typeName: string): BslEntity | undefined;
    /**
     * Проверяет, есть ли у сущности методы, свойства или события
     */
    hasMembers(entity: BslEntity): boolean;
    /**
     * Извлекает UUID проекта из Configuration.xml
     */
    extractUuidProjectId(configPath: string): string | null;
    /**
     * Возвращает категории платформы для дерева
     */
    getPlatformCategories(): HierarchicalTypeItem[];
    /**
     * Возвращает категории конфигурации для дерева
     */
    getConfigCategories(): HierarchicalTypeItem[];
    /**
     * Возвращает типы для категории
     */
    getCategoryTypes(categoryKey: string): HierarchicalTypeItem[];
    /**
     * Возвращает элементы для отображения членов типа (методы, свойства)
     */
    getTypeMembers(typeName: string): HierarchicalTypeItem[];
    /**
     * Возвращает методы типа
     */
    getTypeMethods(typeName: string): HierarchicalTypeItem[];
    /**
     * Возвращает свойства типа
     */
    getTypeProperties(typeName: string): HierarchicalTypeItem[];
    private categorizeTypes;
    private matchesCategory;
}
//# sourceMappingURL=typeTreeBuilder.d.ts.map