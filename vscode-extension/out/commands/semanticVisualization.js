"use strict";
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
exports.registerSemanticVisualization = void 0;
const vscode = __importStar(require("vscode"));
/**
 * Регистрирует команду для показа семантического дерева BSL модуля
 *
 * Использует LSP custom request `bsl/getSemanticHtml` для получения
 * готового HTML с семантическим деревом, таблицей символов и метриками.
 *
 * @param client - LSP клиент для общения с backend
 * @returns Disposable для cleanup при деактивации extension
 */
function registerSemanticVisualization(client) {
    return vscode.commands.registerCommand('bsl-gradual-types.showSemanticVisualization', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('Нет открытого BSL файла');
            return;
        }
        const uri = editor.document.uri.toString();
        const fileName = editor.document.fileName.split(/[/\\]/).pop() || 'unknown.bsl';
        // Создаём webview panel справа от редактора
        const panel = vscode.window.createWebviewPanel('bslSemanticVisualization', `Семантическое дерево: ${fileName}`, vscode.ViewColumn.Two, {
            enableScripts: true,
            retainContextWhenHidden: true
        });
        // Показываем индикатор загрузки
        panel.webview.html = getLoadingHtml();
        try {
            // Определяем тему VSCode для корректной цветовой схемы
            const isDark = vscode.window.activeColorTheme.kind === vscode.ColorThemeKind.Dark;
            const theme = isDark ? 'dark' : 'light';
            // Запрашиваем готовый HTML через LSP custom request
            const response = await client.sendRequest('bsl/getSemanticHtml', {
                uri: uri,
                theme: theme,
                compact: false
            });
            // Отображаем семантическое дерево
            panel.webview.html = response.html;
        }
        catch (error) {
            // Показываем ошибку в webview
            panel.webview.html = getErrorHtml(error);
            vscode.window.showErrorMessage(`Ошибка получения семантического дерева: ${error}`);
        }
    });
}
exports.registerSemanticVisualization = registerSemanticVisualization;
/**
 * HTML индикатор загрузки
 */
function getLoadingHtml() {
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
function getErrorHtml(error) {
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
//# sourceMappingURL=semanticVisualization.js.map