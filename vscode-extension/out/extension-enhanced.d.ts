/**
 * BSL Gradual Type System - Enhanced VSCode Extension
 *
 * Интегрирует VSCode с enhanced LSP сервером, предоставляя:
 * - Flow-sensitive type analysis
 * - Union types с инкрементальным парсингом
 * - Code actions и type hints
 * - Performance profiling integration
 */
import * as vscode from 'vscode';
/**
 * Активация расширения
 */
export declare function activate(context: vscode.ExtensionContext): Promise<void>;
/**
 * Деактивация расширения
 */
export declare function deactivate(): Promise<void>;
/**
 * Обновление package.json с enhanced функциональностью
 */
export declare function getEnhancedPackageContributions(): {
    commands: {
        command: string;
        title: string;
        category: string;
    }[];
    configuration: {
        type: string;
        title: string;
        properties: {
            "bsl.typeHints.showVariableTypes": {
                type: string;
                default: boolean;
                description: string;
            };
            "bsl.typeHints.showReturnTypes": {
                type: string;
                default: boolean;
                description: string;
            };
            "bsl.typeHints.showUnionDetails": {
                type: string;
                default: boolean;
                description: string;
            };
            "bsl.typeHints.minCertainty": {
                type: string;
                default: number;
                minimum: number;
                maximum: number;
                description: string;
            };
            "bsl.performance.enableProfiling": {
                type: string;
                default: boolean;
                description: string;
            };
            "bsl.analysis.useParallelProcessing": {
                type: string;
                default: boolean;
                description: string;
            };
            "bsl.analysis.enableCaching": {
                type: string;
                default: boolean;
                description: string;
            };
        };
    };
};
//# sourceMappingURL=extension-enhanced.d.ts.map