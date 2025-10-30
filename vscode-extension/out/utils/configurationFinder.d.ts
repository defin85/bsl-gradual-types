import * as vscode from 'vscode';
export interface ConfigurationInfo {
    path: string;
    name: string;
    isExtension: boolean;
    uuid?: string;
}
/**
 * Находит все конфигурации 1С в директории
 */
export declare function findConfigurations(rootPath: string): Promise<ConfigurationInfo[]>;
/**
 * Находит основную конфигурацию в workspace
 */
export declare function findMainConfiguration(): Promise<ConfigurationInfo | null>;
/**
 * Показывает диалог выбора конфигурации
 */
export declare function selectConfiguration(configurations: ConfigurationInfo[]): Promise<ConfigurationInfo | null>;
/**
 * Автоматически определяет и устанавливает путь к конфигурации
 */
export declare function autoDetectConfiguration(outputChannel?: vscode.OutputChannel): Promise<string | null>;
//# sourceMappingURL=configurationFinder.d.ts.map