import * as vscode from 'vscode';
import { MethodCallInfo } from '../utils';
import { CodeMetrics } from '../types';

/**
 * Показать webview с информацией о типе
 */
export function showTypeInfoWebview(_context: vscode.ExtensionContext, typeName: string, result: string) {
    const panel = vscode.window.createWebviewPanel(
        'bslTypeInfo',
        `BSL Type: ${typeName}`,
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );
    
    panel.webview.html = getTypeInfoWebviewContent(typeName, result);
}

/**
 * Показать webview с информацией о методе
 */
export function showMethodInfoWebview(_context: vscode.ExtensionContext, typeName: string, methodName: string, result: string) {
    const panel = vscode.window.createWebviewPanel(
        'bslMethodInfo',
        `BSL Method: ${typeName}.${methodName}`,
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );
    
    panel.webview.html = getMethodInfoWebviewContent(typeName, methodName, result);
}

/**
 * Показать webview с обозревателем типов
 */
export function showTypeExplorerWebview(_context: vscode.ExtensionContext, typeName: string, result: string) {
    const panel = vscode.window.createWebviewPanel(
        'bslTypeExplorer',
        `BSL Type Explorer: ${typeName}`,
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );
    
    panel.webview.html = getTypeExplorerWebviewContent(typeName, result);
}

/**
 * Показать webview со статистикой индекса
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
    
    panel.webview.html = getIndexStatsWebviewContent(result);
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

    panel.webview.html = getMetricsWebviewContent(metrics);
}

// HTML генераторы для webview
function getTypeInfoWebviewContent(typeName: string, result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Type Information</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            h1 {
                color: var(--vscode-titleBar-activeForeground);
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 10px;
            }
            .type-info {
                background-color: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 4px;
                padding: 15px;
                margin-top: 15px;
            }
            pre {
                background-color: var(--vscode-textBlockQuote-background);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 4px;
                padding: 10px;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <h1>Type: ${typeName}</h1>
        <div class="type-info">
            <pre>${result}</pre>
        </div>
    </body>
    </html>
    `;
}

function getMethodInfoWebviewContent(typeName: string, methodName: string, result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Method Information</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            h1 {
                color: var(--vscode-titleBar-activeForeground);
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 10px;
            }
            .method-info {
                background-color: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 4px;
                padding: 15px;
                margin-top: 15px;
            }
            pre {
                background-color: var(--vscode-textBlockQuote-background);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 4px;
                padding: 10px;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <h1>Method: ${typeName}.${methodName}</h1>
        <div class="method-info">
            <pre>${result}</pre>
        </div>
    </body>
    </html>
    `;
}

function getTypeExplorerWebviewContent(typeName: string, result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Type Explorer</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            .explorer-header {
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 16px;
                margin-bottom: 20px;
            }
            .explorer-title {
                font-size: 24px;
                font-weight: bold;
                color: var(--vscode-charts-blue);
            }
            .result-content {
                background: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 16px;
                white-space: pre-wrap;
                font-family: 'Consolas', 'Monaco', monospace;
                font-size: 14px;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <div class="explorer-header">
            <div class="explorer-title">🧭 Type Explorer: ${typeName}</div>
        </div>
        <div class="result-content">${result}</div>
    </body>
    </html>
    `;
}

function getIndexStatsWebviewContent(result: string): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Index Statistics</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            .stats-header {
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 16px;
                margin-bottom: 20px;
            }
            .stats-title {
                font-size: 24px;
                font-weight: bold;
                color: var(--vscode-charts-orange);
            }
            .result-content {
                background: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 16px;
                white-space: pre-wrap;
                font-family: 'Consolas', 'Monaco', monospace;
                font-size: 14px;
                overflow-x: auto;
            }
        </style>
    </head>
    <body>
        <div class="stats-header">
            <div class="stats-title">📊 Index Statistics</div>
        </div>
        <div class="result-content">${result}</div>
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
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Method Validation</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            .validation-header {
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 16px;
                margin-bottom: 20px;
            }
            .validation-title {
                font-size: 24px;
                font-weight: bold;
                color: var(--vscode-charts-red);
            }
            .method-call-info {
                background: var(--vscode-badge-background);
                color: var(--vscode-badge-foreground);
                padding: 8px 12px;
                border-radius: 4px;
                margin: 8px 0;
                font-family: 'Consolas', 'Monaco', monospace;
            }
            .result-content {
                background: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 16px;
                white-space: pre-wrap;
                font-family: 'Consolas', 'Monaco', monospace;
                font-size: 14px;
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
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Type Compatibility</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            .compatibility-header {
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 16px;
                margin-bottom: 20px;
            }
            .compatibility-title {
                font-size: 24px;
                font-weight: bold;
                color: var(--vscode-charts-yellow);
            }
            .type-comparison {
                background: var(--vscode-badge-background);
                color: var(--vscode-badge-foreground);
                padding: 8px 12px;
                border-radius: 4px;
                margin: 8px 0;
                font-family: 'Consolas', 'Monaco', monospace;
                text-align: center;
            }
            .result-content {
                background: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 16px;
                white-space: pre-wrap;
                font-family: 'Consolas', 'Monaco', monospace;
                font-size: 14px;
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

function getMetricsWebviewContent(metrics: CodeMetrics): string {
    return `
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>BSL Code Quality Metrics</title>
        <style>
            body {
                font-family: var(--vscode-font-family);
                color: var(--vscode-foreground);
                background-color: var(--vscode-editor-background);
                padding: 20px;
            }
            h1 {
                color: var(--vscode-titleBar-activeForeground);
                border-bottom: 2px solid var(--vscode-panel-border);
                padding-bottom: 10px;
            }
            .metrics-grid {
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
                gap: 20px;
                margin-top: 20px;
            }
            .metric-card {
                background-color: var(--vscode-editor-inactiveSelectionBackground);
                border: 1px solid var(--vscode-panel-border);
                border-radius: 6px;
                padding: 15px;
            }
            .metric-title {
                font-weight: bold;
                color: var(--vscode-charts-blue);
                margin-bottom: 10px;
            }
            .metric-value {
                font-size: 24px;
                font-weight: bold;
            }
            .metric-description {
                color: var(--vscode-descriptionForeground);
                font-size: 12px;
                margin-top: 5px;
            }
        </style>
    </head>
    <body>
        <h1>Code Quality Metrics</h1>
        <div class="metrics-grid">
            ${Object.entries(metrics).map(([key, value]) => `
                <div class="metric-card">
                    <div class="metric-title">${key}</div>
                    <div class="metric-value">${value}</div>
                </div>
            `).join('')}
        </div>
    </body>
    </html>
    `;
}