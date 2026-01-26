import * as vscode from 'vscode';
/**
 * Сущность BSL типа (платформы или конфигурации)
 */
export interface BslEntity {
    id: string;
    qualified_name: string;
    display_name: string;
    entity_type: 'Platform' | 'Configuration';
    entity_kind: string;
    interface?: {
        methods?: Record<string, MethodInfo>;
        properties?: Record<string, PropertyInfo>;
        events?: Record<string, unknown>;
    };
    documentation?: string;
}
/**
 * Информация о методе
 */
export interface MethodInfo {
    parameters?: unknown;
    returns?: string;
    documentation?: string;
}
/**
 * Информация о свойстве
 */
export interface PropertyInfo {
    type?: string;
    readonly?: boolean;
    documentation?: string;
}
/**
 * Категория типов для группировки в дереве
 */
export interface TypeCategory {
    name: string;
    icon: string;
    types: BslEntity[];
}
/**
 * Элемент иерархического дерева типов
 */
export declare class HierarchicalTypeItem extends vscode.TreeItem {
    readonly label: string;
    readonly collapsibleState: vscode.TreeItemCollapsibleState;
    readonly typeName: string;
    readonly typeContext: string;
    readonly itemData?: string | undefined;
    constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState, typeName: string, typeContext: string, itemData?: string | undefined);
}
//# sourceMappingURL=typeModels.d.ts.map