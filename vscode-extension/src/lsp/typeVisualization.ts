/**
 * TypeScript wrapper для Rust TypeVisualization через LSP
 *
 * Заменяет legacy TypeScript HtmlRenderer на вызовы Rust версии
 * Milestone 2.5: Унификация визуализации
 */

import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

import { logger } from './logger';
/**
 * LSP Request параметры для bsl/renderTypeHtml
 */
interface RenderTypeHtmlParams {
    type_name: string;
    theme?: 'light' | 'dark' | 'high-contrast';
}

/**
 * LSP Response от bsl/renderTypeHtml
 */
interface RenderTypeHtmlResponse {
    html: string;
    success: boolean;
    message?: string;
}

/**
 * Определить текущую тему VSCode
 */
function getCurrentTheme(): 'light' | 'dark' | 'high-contrast' {
    const themeKind = vscode.window.activeColorTheme.kind;

    switch (themeKind) {
        case vscode.ColorThemeKind.Light:
            return 'light';
        case vscode.ColorThemeKind.Dark:
            return 'dark';
        case vscode.ColorThemeKind.HighContrast:
        case vscode.ColorThemeKind.HighContrastLight:
            return 'high-contrast';
        default:
            return 'dark'; // fallback
    }
}

/**
 * Рендеринг HTML для типа через Rust TypeVisualization
 *
 * @param client LSP клиент
 * @param typeName Имя типа для рендеринга
 * @param theme Опциональная тема (если не указана, auto-detect из VSCode)
 * @returns HTML контент для webview
 */
export async function renderTypeHtml(
    client: LanguageClient,
    typeName: string,
    theme?: 'light' | 'dark' | 'high-contrast'
): Promise<string> {
    const resolvedTheme = theme || getCurrentTheme();

    try {
        const response = await client.sendRequest<RenderTypeHtmlResponse>(
            'bsl/renderTypeHtml',
            {
                type_name: typeName,
                theme: resolvedTheme
            } as RenderTypeHtmlParams
        );

        if (!response.success) {
            throw new Error(response.message || 'Failed to render type HTML');
        }

        return response.html;
    } catch (error) {
        logger.error('TypeVisualization error:', error);

        // Fallback HTML на случай ошибки
        return `
            <html>
                <body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 20px;">
                    <h2 style="color: #f44336;">❌ Ошибка рендеринга типа</h2>
                    <p><strong>Тип:</strong> ${typeName}</p>
                    <p><strong>Ошибка:</strong> ${error}</p>
                    <p><em>Проверьте что LSP сервер запущен и поддерживает bsl/renderTypeHtml</em></p>
                </body>
            </html>
        `;
    }
}

/**
 * Показать webview с типом, используя Rust TypeVisualization
 *
 * Заменяет showTypeInfoWebview из webviewContent.ts
 */
export async function showTypeInfoWebview(
    client: LanguageClient,
    typeName: string
): Promise<void> {
    const panel = vscode.window.createWebviewPanel(
        'bslTypeInfo',
        `BSL Type: ${typeName}`,
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    // Показываем loading пока рендерим
    panel.webview.html = '<html><body><h2>🔄 Загрузка...</h2></body></html>';

    // Рендерим через Rust TypeVisualization
    const html = await renderTypeHtml(client, typeName);
    panel.webview.html = html;
}

/**
 * Показать webview с методами типа через Rust TypeVisualization
 */
export async function showMethodInfoWebview(
    client: LanguageClient,
    typeName: string,
    methodName: string
): Promise<void> {
    const panel = vscode.window.createWebviewPanel(
        'bslMethodInfo',
        `BSL Method: ${typeName}.${methodName}`,
        vscode.ViewColumn.Two,
        {
            enableScripts: true,
            retainContextWhenHidden: true
        }
    );

    panel.webview.html = '<html><body><h2>🔄 Загрузка...</h2></body></html>';

    // TODO: Добавить отдельный LSP request для методов или расширить bsl/renderTypeHtml
    // Пока используем тот же рендеринг типа
    const html = await renderTypeHtml(client, typeName);
    panel.webview.html = html;
}
