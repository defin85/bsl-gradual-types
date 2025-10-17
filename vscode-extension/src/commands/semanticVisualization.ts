import * as vscode from 'vscode';
import { LanguageClient } from 'vscode-languageclient/node';

/**
 * Регистрирует команду для показа семантического дерева BSL модуля
 *
 * Использует LSP custom request `bsl/getSemanticHtml` для получения
 * готового HTML с семантическим деревом, таблицей символов и метриками.
 *
 * @param client - LSP клиент для общения с backend
 * @returns Disposable для cleanup при деактивации extension
 */
export function registerSemanticVisualization(client: LanguageClient): vscode.Disposable {
    return vscode.commands.registerCommand('bsl-gradual-types.showSemanticVisualization', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('Нет открытого BSL файла');
            return;
        }

        const uri = editor.document.uri.toString();
        const fileName = editor.document.fileName.split(/[/\\]/).pop() || 'unknown.bsl';

        // Создаём webview panel справа от редактора
        const panel = vscode.window.createWebviewPanel(
            'bslSemanticVisualization',
            `Семантическое дерево: ${fileName}`,
            vscode.ViewColumn.Two,
            {
                enableScripts: true,
                retainContextWhenHidden: true
            }
        );

        // Показываем индикатор загрузки
        panel.webview.html = getLoadingHtml();

        try {
            // Определяем тему VSCode для корректной цветовой схемы
            const isDark = vscode.window.activeColorTheme.kind === vscode.ColorThemeKind.Dark;
            const theme = isDark ? 'dark' : 'light';

            // Запрашиваем готовый HTML через LSP custom request
            const response = await client.sendRequest<{ html: string }>('bsl/getSemanticHtml', {
                uri: uri,
                theme: theme,
                compact: false
            });

            // Отображаем семантическое дерево
            panel.webview.html = response.html;

        } catch (error) {
            // Показываем ошибку в webview
            panel.webview.html = getErrorHtml(error);
            vscode.window.showErrorMessage(`Ошибка получения семантического дерева: ${error}`);
        }
    });
}

/**
 * HTML индикатор загрузки
 */
function getLoadingHtml(): string {
    return `<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
        }
        .spinner {
            border: 4px solid rgba(0, 0, 0, 0.1);
            border-left-color: var(--vscode-progressBar-background);
            border-radius: 50%;
            width: 40px;
            height: 40px;
            animation: spin 1s linear infinite;
        }
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
    </style>
</head>
<body>
    <div>
        <div class="spinner"></div>
        <p>Загрузка семантического дерева...</p>
    </div>
</body>
</html>`;
}

/**
 * HTML для отображения ошибки
 */
function getErrorHtml(error: any): string {
    const errorMessage = error?.message || String(error);
    return `<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
            padding: 20px;
        }
        h1 {
            color: var(--vscode-errorForeground);
        }
        pre {
            background: var(--vscode-textCodeBlock-background);
            padding: 10px;
            border-radius: 4px;
            overflow-x: auto;
        }
    </style>
</head>
<body>
    <h1>❌ Ошибка</h1>
    <pre>${errorMessage}</pre>
</body>
</html>`;
}
