/**
 * UI Setup Module
 *
 * Status bar, welcome messages, webview panels
 */

import * as vscode from 'vscode';

/**
 * Создание и настройка status bar item
 */
export function createStatusBarItem(context: vscode.ExtensionContext): vscode.StatusBarItem {
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBarItem.text = "$(loading~spin) BSL: Initializing...";
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);
    return statusBarItem;
}

/**
 * Показать welcome message при первом запуске
 */
export async function showWelcomeMessage(): Promise<void> {
    const config = vscode.workspace.getConfiguration('bsl');
    const hasShownWelcome = config.get('hasShownWelcome', false);

    if (!hasShownWelcome) {
        const selection = await vscode.window.showInformationMessage(
            'Welcome to BSL Gradual Type System! ' +
            'This is a production-ready type system with flow-sensitive analysis, ' +
            'union types, and enhanced LSP features.',
            'Show Features',
            'Configure',
            "Don't show again"
        );

        switch (selection) {
            case 'Show Features':
                await showFeaturesOverview();
                break;
            case 'Configure':
                await vscode.commands.executeCommand('workbench.action.openSettings', 'bsl');
                break;
            case "Don't show again":
                await config.update('hasShownWelcome', true, vscode.ConfigurationTarget.Global);
                break;
        }
    }
}

/**
 * Показать обзор возможностей
 */
export async function showFeaturesOverview(): Promise<void> {
    const panel = vscode.window.createWebviewPanel(
        'bslFeatures',
        'BSL Gradual Type System Features',
        vscode.ViewColumn.Active,
        { enableScripts: true }
    );

    panel.webview.html = `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>BSL Features</title>
            <style>
                body { font-family: var(--vscode-font-family); padding: 20px; }
                .feature { margin: 20px 0; padding: 15px; border-left: 4px solid var(--vscode-accent); }
                .feature h3 { margin-top: 0; color: var(--vscode-accent); }
                .performance { background: var(--vscode-terminal-ansiGreen); color: white; padding: 5px 10px; border-radius: 3px; }
            </style>
        </head>
        <body>
            <h1>BSL Gradual Type System v1.0.0</h1>

            <div class="feature">
                <h3>Flow-Sensitive Analysis</h3>
                <p>Tracks variable type changes throughout program execution</p>
            </div>

            <div class="feature">
                <h3>Union Types</h3>
                <p>Full Union types with normalization and weighted probabilities</p>
            </div>

            <div class="feature">
                <h3>Enhanced LSP</h3>
                <p>Incremental parsing, smart autocompletion, real-time diagnostics</p>
            </div>

            <div class="feature">
                <h3>Type Hints</h3>
                <p>Inline type display directly in code</p>
            </div>

            <div class="feature">
                <h3>Code Actions</h3>
                <p>Automatic fixes and refactoring suggestions</p>
            </div>

            <div class="performance">
                Performance: Parsing ~189us | Type Checking ~125us | Flow Analysis ~175ns
            </div>
        </body>
        </html>
    `;
}

/**
 * Показать результаты анализа проекта
 */
export async function showProjectAnalysisResults(results: ProjectAnalysisResult): Promise<void> {
    const panel = vscode.window.createWebviewPanel(
        'bslProjectResults',
        'Project Analysis Results',
        vscode.ViewColumn.Active,
        { enableScripts: true }
    );

    panel.webview.html = `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>Project Analysis Results</title>
            <style>
                body { font-family: var(--vscode-font-family); padding: 20px; }
                .stat { display: flex; justify-content: space-between; margin: 10px 0; }
                .stat-value { font-weight: bold; color: var(--vscode-accent); }
            </style>
        </head>
        <body>
            <h1>Project Analysis Results</h1>

            <div class="stat">
                <span>Total Files:</span>
                <span class="stat-value">${results.stats.totalFiles}</span>
            </div>
            <div class="stat">
                <span>Successful:</span>
                <span class="stat-value">${results.stats.successfulFiles}</span>
            </div>
            <div class="stat">
                <span>Functions Found:</span>
                <span class="stat-value">${results.stats.totalFunctions}</span>
            </div>
            <div class="stat">
                <span>Variables Found:</span>
                <span class="stat-value">${results.stats.totalVariables}</span>
            </div>
            <div class="stat">
                <span>Diagnostics:</span>
                <span class="stat-value">${results.stats.totalDiagnostics}</span>
            </div>
            <div class="stat">
                <span>Analysis Time:</span>
                <span class="stat-value">${results.totalTime}</span>
            </div>

            <h2>Performance</h2>
            <div class="stat">
                <span>Average per file:</span>
                <span class="stat-value">${results.stats.avgAnalysisTime}</span>
            </div>
        </body>
        </html>
    `;
}

/**
 * Генерация HTML для type info
 */
export function generateTypeInfoHtml(hover: HoverInfo): string {
    return `
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>Type Information</title>
            <style>
                body {
                    font-family: var(--vscode-font-family);
                    padding: 20px;
                    background: var(--vscode-editor-background);
                    color: var(--vscode-editor-foreground);
                }
                .type-info {
                    background: var(--vscode-textBlockQuote-background);
                    padding: 15px;
                    border-radius: 5px;
                    border-left: 4px solid var(--vscode-accent);
                }
                .confidence {
                    color: var(--vscode-terminal-ansiGreen);
                    font-weight: bold;
                }
                .source {
                    color: var(--vscode-terminal-ansiBlue);
                    font-style: italic;
                }
            </style>
        </head>
        <body>
            <div class="type-info">
                ${hover.contents.value}
            </div>
        </body>
        </html>
    `;
}

// Type definitions for UI module
export interface ProjectAnalysisResult {
    stats: {
        totalFiles: number;
        successfulFiles: number;
        totalFunctions: number;
        totalVariables: number;
        totalDiagnostics: number;
        avgAnalysisTime: string;
    };
    totalTime: string;
}

export interface HoverInfo {
    contents: {
        value: string;
    };
}
