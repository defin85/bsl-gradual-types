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
import { renderTypeHtml, showTypeInfoWebview as showTypeInfoWebviewLsp, showMethodInfoWebview as showMethodInfoWebviewLsp } from '../lsp/typeVisualization';

/**
 * Показать webview с информацией о типе
 * ✅ MIGRATED: Использует Rust TypeVisualization через LSP
 */
export async function showTypeInfoWebview(
    clientOrContext: LanguageClient | vscode.ExtensionContext,
    typeName: string,
    _result?: string
): Promise<void> {
    // Check if it's a LanguageClient (has sendRequest method)
    if ('sendRequest' in clientOrContext) {
        await showTypeInfoWebviewLsp(clientOrContext as LanguageClient, typeName);
    } else {
        console.warn('showTypeInfoWebview called with ExtensionContext - LSP client required');
    }
}

/**
 * Показать webview с информацией о методе
 * ✅ MIGRATED: Использует Rust TypeVisualization через LSP
 */
export async function showMethodInfoWebview(
    clientOrContext: LanguageClient | vscode.ExtensionContext,
    typeName: string,
    methodName: string,
    _result?: string
): Promise<void> {
    if ('sendRequest' in clientOrContext) {
        await showMethodInfoWebviewLsp(clientOrContext as LanguageClient, typeName, methodName);
    } else {
        console.warn('showMethodInfoWebview called with ExtensionContext - LSP client required');
    }
}

/**
 * Показать webview с обозревателем типов
 * ✅ MIGRATED: Использует Rust TypeVisualization
 */
export async function showTypeExplorerWebview(
    clientOrContext: LanguageClient | vscode.ExtensionContext,
    typeName: string,
    _result?: string
): Promise<void> {
    if (!('sendRequest' in clientOrContext)) {
        console.warn('showTypeExplorerWebview called with ExtensionContext - LSP client required');
        return;
    }

    const client = clientOrContext as LanguageClient;

    const panel = vscode.window.createWebviewPanel(
        'bslTypeExplorer',
        `BSL Type Explorer: ${typeName}`,
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    panel.webview.html = '<html><body><h2>🔄 Загрузка...</h2></body></html>';
    const html = await renderTypeHtml(client, typeName);
    panel.webview.html = html;
}

/**
 * Показать webview со статистикой индекса
 * ⚠️ TODO: Создать отдельный LSP request для метрик или использовать JsonRenderer
 */
export function showIndexStatsWebview(_context: vscode.ExtensionContext, result: string) {
    const panel = vscode.window.createWebviewPanel(
        'bslIndexStats',
        'BSL Index Statistics',
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    panel.webview.html = getIndexStatsWebviewContentSimple(result);
}

/**
 * Показать webview с валидацией метода
 */
export function showMethodValidationWebview(_context: vscode.ExtensionContext, methodCall: MethodCallInfo, result: string) {
    const panel = vscode.window.createWebviewPanel(
        'bslMethodValidation',
        'BSL Method Validation',
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    panel.webview.html = getMethodValidationWebviewContent(methodCall, result);
}

/**
 * Показать webview с проверкой совместимости типов
 */
export function showTypeCompatibilityWebview(_context: vscode.ExtensionContext, fromType: string, toType: string, result: string) {
    const panel = vscode.window.createWebviewPanel(
        'bslTypeCompatibility',
        'BSL Type Compatibility',
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    panel.webview.html = getTypeCompatibilityWebviewContent(fromType, toType, result);
}

/**
 * Показать webview с метриками
 */
export function showMetricsWebview(_context: vscode.ExtensionContext, metrics: CodeMetrics) {
    const panel = vscode.window.createWebviewPanel(
        'bslMetrics',
        'BSL Code Quality Metrics',
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    panel.webview.html = getMetricsWebviewContentSimple(metrics);
}

// =============================================================================
// Simple HTML generators (используют VSCode CSS variables)
// =============================================================================

function getIndexStatsWebviewContentSimple(result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Index Statistics</title>
        <style>
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
                padding: 20px;
                background: var(--vscode-editor-background);
                color: var(--vscode-editor-foreground);
            }
            .stats-container {
                background: var(--vscode-sideBar-background);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 20px;
            }
            pre {
                background: var(--vscode-textCodeBlock-background);
                padding: 12px;
                border-radius: 4px;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <h1>📊 Index Statistics</h1>
        <div class="stats-container">
            <pre>${result}</pre>
        </div>
    </body>
    </html>
    `;
}

function getMethodValidationWebviewContent(methodCall: MethodCallInfo, result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <title>BSL Method Validation</title>
        <style>
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
                padding: 20px;
                background: var(--vscode-editor-background);
                color: var(--vscode-editor-foreground);
            }
            .validation-header {
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 16px;
                margin-bottom: 20px;
            }
            .validation-title {
                font-size: 24px;
                font-weight: bold;
                color: var(--vscode-errorForeground);
            }
            .method-call-info {
                background: var(--vscode-textCodeBlock-background);
                padding: 8px 12px;
                border-radius: 4px;
                margin: 8px 0;
                font-family: 'Consolas', 'Monaco', monospace;
            }
            .result-content {
                background: var(--vscode-textCodeBlock-background);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 16px;
                white-space: pre-wrap;
                font-family: 'Consolas', 'Monaco', monospace;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <div class="validation-header">
            <div class="validation-title">✓ Method Validation</div>
            <div class="method-call-info">
                ${methodCall.objectName}.${methodCall.methodName}()
            </div>
        </div>
        <div class="result-content">${result}</div>
    </body>
    </html>
    `;
}

function getTypeCompatibilityWebviewContent(fromType: string, toType: string, result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <title>BSL Type Compatibility</title>
        <style>
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
                padding: 20px;
                background: var(--vscode-editor-background);
                color: var(--vscode-editor-foreground);
            }
            .compatibility-header {
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 16px;
                margin-bottom: 20px;
            }
            .compatibility-title {
                font-size: 24px;
                font-weight: bold;
                color: var(--vscode-editorWarning-foreground);
            }
            .type-comparison {
                background: var(--vscode-textCodeBlock-background);
                padding: 8px 12px;
                border-radius: 4px;
                margin: 8px 0;
                font-family: 'Consolas', 'Monaco', monospace;
                text-align: center;
            }
            .result-content {
                background: var(--vscode-textCodeBlock-background);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 16px;
                white-space: pre-wrap;
                font-family: 'Consolas', 'Monaco', monospace;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <div class="compatibility-header">
            <div class="compatibility-title">↔️ Type Compatibility</div>
            <div class="type-comparison">
                ${fromType} → ${toType}
            </div>
        </div>
        <div class="result-content">${result}</div>
    </body>
    </html>
    `;
}

function getMetricsWebviewContentSimple(metrics: CodeMetrics): string {
    const metricsHtml = Object.entries(metrics)
        .map(([key, value]) => `<tr><td>${key}</td><td>${value}</td></tr>`)
        .join('');

    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <title>BSL Code Quality Metrics</title>
        <style>
            body {
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
                padding: 20px;
                background: var(--vscode-editor-background);
                color: var(--vscode-editor-foreground);
            }
            table {
                width: 100%;
                border-collapse: collapse;
                background: var(--vscode-sideBar-background);
                border-radius: 6px;
                overflow: hidden;
            }
            th, td {
                padding: 12px;
                text-align: left;
                border-bottom: 1px solid var(--vscode-panel-border);
            }
            th {
                background: var(--vscode-textCodeBlock-background);
                font-weight: bold;
            }
        </style>
    </head>
    <body>
        <h1>📈 Code Quality Metrics</h1>
        <table>
            <thead>
                <tr>
                    <th>Metric</th>
                    <th>Value</th>
                </tr>
            </thead>
            <tbody>
                ${metricsHtml}
            </tbody>
        </table>
    </body>
    </html>
    `;
}
