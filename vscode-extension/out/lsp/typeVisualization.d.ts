/**
 * TypeScript wrapper для Rust TypeVisualization через LSP
 *
 * Заменяет legacy TypeScript HtmlRenderer на вызовы Rust версии
 * Milestone 2.5: Унификация визуализации
 */
import { LanguageClient } from 'vscode-languageclient/node';
/**
 * Рендеринг HTML для типа через Rust TypeVisualization
 *
 * @param client LSP клиент
 * @param typeName Имя типа для рендеринга
 * @param theme Опциональная тема (если не указана, auto-detect из VSCode)
 * @returns HTML контент для webview
 */
export declare function renderTypeHtml(client: LanguageClient, typeName: string, theme?: 'light' | 'dark' | 'high-contrast'): Promise<string>;
/**
 * Показать webview с типом, используя Rust TypeVisualization
 *
 * Заменяет showTypeInfoWebview из webviewContent.ts
 */
export declare function showTypeInfoWebview(client: LanguageClient, typeName: string): Promise<void>;
/**
 * Показать webview с методами типа через Rust TypeVisualization
 */
export declare function showMethodInfoWebview(client: LanguageClient, typeName: string, methodName: string): Promise<void>;
//# sourceMappingURL=typeVisualization.d.ts.map