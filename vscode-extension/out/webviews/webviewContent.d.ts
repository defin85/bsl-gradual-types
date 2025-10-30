/**
 * Webview Content Generation - MIGRATED TO Rust TypeVisualization
 *
 * Milestone 2.5: Унификация визуализации типов
 * Использует Rust HtmlRenderer через LSP вместо TypeScript legacy
 */
import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';
import { MethodCallInfo } from '../utils';
import { CodeMetrics } from '../types';
/**
 * Показать webview с информацией о типе
 * ✅ MIGRATED: Использует Rust TypeVisualization через LSP
 */
export declare function showTypeInfoWebview(clientOrContext: LanguageClient | vscode.ExtensionContext, typeName: string, _result?: string): Promise<void>;
/**
 * Показать webview с информацией о методе
 * ✅ MIGRATED: Использует Rust TypeVisualization через LSP
 */
export declare function showMethodInfoWebview(clientOrContext: LanguageClient | vscode.ExtensionContext, typeName: string, methodName: string, _result?: string): Promise<void>;
/**
 * Показать webview с обозревателем типов
 * ✅ MIGRATED: Использует Rust TypeVisualization
 */
export declare function showTypeExplorerWebview(clientOrContext: LanguageClient | vscode.ExtensionContext, typeName: string, _result?: string): Promise<void>;
/**
 * Показать webview со статистикой индекса
 * ⚠️ TODO: Создать отдельный LSP request для метрик или использовать JsonRenderer
 */
export declare function showIndexStatsWebview(_context: vscode.ExtensionContext, result: string): void;
/**
 * Показать webview с валидацией метода
 */
export declare function showMethodValidationWebview(_context: vscode.ExtensionContext, methodCall: MethodCallInfo, result: string): void;
/**
 * Показать webview с проверкой совместимости типов
 */
export declare function showTypeCompatibilityWebview(_context: vscode.ExtensionContext, fromType: string, toType: string, result: string): void;
/**
 * Показать webview с метриками
 */
export declare function showMetricsWebview(_context: vscode.ExtensionContext, metrics: CodeMetrics): void;
//# sourceMappingURL=webviewContent.d.ts.map