import * as vscode from 'vscode';
import { MethodCallInfo } from '../utils';
import { CodeMetrics } from '../types';
/**
 * Показать webview с информацией о типе
 */
export declare function showTypeInfoWebview(_context: vscode.ExtensionContext, typeName: string, result: string): void;
/**
 * Показать webview с информацией о методе
 */
export declare function showMethodInfoWebview(_context: vscode.ExtensionContext, typeName: string, methodName: string, result: string): void;
/**
 * Показать webview с обозревателем типов
 */
export declare function showTypeExplorerWebview(_context: vscode.ExtensionContext, typeName: string, result: string): void;
/**
 * Показать webview со статистикой индекса
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