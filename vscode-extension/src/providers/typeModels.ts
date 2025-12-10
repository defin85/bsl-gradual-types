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
export class HierarchicalTypeItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly typeName: string,
        public readonly typeContext: string,
        public readonly itemData?: string
    ) {
        super(label, collapsibleState);
        this.contextValue = typeContext;
        this.tooltip = typeName;
    }
}
