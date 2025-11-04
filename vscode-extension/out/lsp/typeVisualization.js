"use strict";
/**
 * TypeScript wrapper для Rust TypeVisualization через LSP
 *
 * Заменяет legacy TypeScript HtmlRenderer на вызовы Rust версии
 * Milestone 2.5: Унификация визуализации
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.showMethodInfoWebview = exports.showTypeInfoWebview = exports.renderTypeHtml = void 0;
const vscode = __importStar(require("vscode"));
const logger_1 = require("./logger");
/**
 * Определить текущую тему VSCode
 */
function getCurrentTheme() {
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
async function renderTypeHtml(client, typeName, theme) {
    const resolvedTheme = theme || getCurrentTheme();
    try {
        const response = await client.sendRequest('bsl/renderTypeHtml', {
            type_name: typeName,
            theme: resolvedTheme
        });
        if (!response.success) {
            throw new Error(response.message || 'Failed to render type HTML');
        }
        return response.html;
    }
    catch (error) {
        logger_1.logger.error('TypeVisualization error:', error);
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
exports.renderTypeHtml = renderTypeHtml;
/**
 * Показать webview с типом, используя Rust TypeVisualization
 *
 * Заменяет showTypeInfoWebview из webviewContent.ts
 */
async function showTypeInfoWebview(client, typeName) {
    const panel = vscode.window.createWebviewPanel('bslTypeInfo', `BSL Type: ${typeName}`, vscode.ViewColumn.Two, {
        enableScripts: true,
        retainContextWhenHidden: true
    });
    // Показываем loading пока рендерим
    panel.webview.html = '<html><body><h2>🔄 Загрузка...</h2></body></html>';
    // Рендерим через Rust TypeVisualization
    const html = await renderTypeHtml(client, typeName);
    panel.webview.html = html;
}
exports.showTypeInfoWebview = showTypeInfoWebview;
/**
 * Показать webview с методами типа через Rust TypeVisualization
 */
async function showMethodInfoWebview(client, typeName, methodName) {
    const panel = vscode.window.createWebviewPanel('bslMethodInfo', `BSL Method: ${typeName}.${methodName}`, vscode.ViewColumn.Two, {
        enableScripts: true,
        retainContextWhenHidden: true
    });
    panel.webview.html = '<html><body><h2>🔄 Загрузка...</h2></body></html>';
    // TODO: Добавить отдельный LSP request для методов или расширить bsl/renderTypeHtml
    // Пока используем тот же рендеринг типа
    const html = await renderTypeHtml(client, typeName);
    panel.webview.html = html;
}
exports.showMethodInfoWebview = showMethodInfoWebview;
//# sourceMappingURL=typeVisualization.js.map