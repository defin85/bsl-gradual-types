import * as vscode from 'vscode';
import { queryType } from '../lsp/customRequests';
import { logger } from '../lsp/logger';

export class TypeDetailsWebviewProvider {
    private static currentPanel: vscode.WebviewPanel | undefined;

    constructor(
        private readonly extensionUri: vscode.Uri,
        private readonly client?: any
    ) {}

    public async showTypeDetails(typeName: string) {
        const column = vscode.window.activeTextEditor?.viewColumn ?? vscode.ViewColumn.One;

        if (TypeDetailsWebviewProvider.currentPanel) {
            TypeDetailsWebviewProvider.currentPanel.reveal(column);
            TypeDetailsWebviewProvider.currentPanel.title = `Тип: ${typeName}`;
        } else {
            TypeDetailsWebviewProvider.currentPanel = vscode.window.createWebviewPanel(
                'bslTypeDetails',
                `Тип: ${typeName}`,
                column,
                {
                    enableScripts: true,
                    localResourceRoots: [
                        vscode.Uri.joinPath(this.extensionUri, 'media', 'webview'),
                        vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', 'snippets')
                    ],
                    retainContextWhenHidden: true,
                }
            );

            const panel = TypeDetailsWebviewProvider.currentPanel;
            panel.webview.html = this.getWebviewContent(panel.webview);

            panel.onDidDispose(() => {
                TypeDetailsWebviewProvider.currentPanel = undefined;
            });
        }

        const panel = TypeDetailsWebviewProvider.currentPanel;
        panel.webview.onDidReceiveMessage(async (message) => {
            // ✅ Валидация структуры сообщения
            if (!message || !message.type || typeof message.type !== 'string') {
                logger.warn('Invalid message format received from webview');
                return;
            }

            // ✅ Whitelist допустимых типов сообщений
            const ALLOWED_MESSAGE_TYPES = ['ready', 'close'];
            if (!ALLOWED_MESSAGE_TYPES.includes(message.type)) {
                logger.warn(`Unknown message type from webview: ${message.type}`);
                return;
            }

            // ✅ Обработка только валидных сообщений
            switch (message.type) {
                case 'ready':
                    await this.updateTypeInfo(panel, typeName);
                    break;
                case 'close':
                    panel.dispose();
                    break;
            }
        });

        await this.updateTypeInfo(panel, typeName);
    }

    private async updateTypeInfo(panel: vscode.WebviewPanel, typeName: string) {
        try {
            // ✅ РЕАЛЬНЫЕ ДАННЫЕ из TypeRepository через LSP
            const typeInfo = await queryType(typeName);

            // 🔍 DEBUG: Логируем полученные данные от LSP
            logger.debug('queryType response: ' + JSON.stringify(typeInfo, null, 2));

            if (!typeInfo.found) {
                // ❌ Тип не найден - показываем placeholder
                logger.warn(`Type '${typeName}' not found in TypeRepository`);
                panel.webview.postMessage({
                    type: 'updateTypeInfo',
                    data: {
                        name: typeName,
                        certainty: 'Unknown',
                        facet: 'Unknown',
                        methods: [],
                        properties: [],
                        documentation: typeInfo.description || 'Информация недоступна'
                    }
                });
                return;
            }

            // ✅ Тип найден - конвертируем в формат для webview
            const modalData = {
                name: typeInfo.typeName,
                certainty: typeInfo.certainty || 'Unknown',
                facet: typeInfo.facet || 'Unknown',
                methods: (typeInfo.methods || []).map(m => ({
                    name: m.name,
                    description: m.description || '',
                    returns: m.returnType || 'void',
                    parameters: (m.params || []).map(p => ({
                        name: p.name,
                        type: p.paramType
                    }))
                })),
                properties: (typeInfo.properties || []).map(p => ({
                    name: p.name,
                    type: p.propType,
                    readonly: p.isReadonly
                })),
                documentation: typeInfo.description || 'Нет описания'
            };

            // 🔍 DEBUG: Логируем данные перед отправкой в webview
            logger.debug(`Sending to webview: ${modalData.methods.length} methods, ${modalData.properties.length} properties`);
            logger.debug('Modal data: ' + JSON.stringify(modalData, null, 2));

            panel.webview.postMessage({ type: 'updateTypeInfo', data: modalData });

        } catch (error) {
            logger.error('Failed to fetch type info', error);
            // Показываем ошибку в webview
            panel.webview.postMessage({
                type: 'error',
                error: `Ошибка загрузки типа: ${error}`
            });
        }
    }

    private getWebviewContent(webview: vscode.Webview): string {
        // 🆕 Find WASM files dynamically (hash changes on rebuild)
        const fs = require('fs');
        const webviewPath = vscode.Uri.joinPath(this.extensionUri, 'media', 'webview').fsPath;
        const files = fs.readdirSync(webviewPath);

        const wasmFile = files.find((f: string) => f.match(/^bsl-frontend-.*\.wasm$/));
        const jsFile = files.find((f: string) => f.match(/^bsl-frontend-.*\.js$/));
        // Use main.css for Type Details (has modal styles)
        const cssFile = files.find((f: string) => f.match(/^main-.*\.css$/)) ||
            files.find((f: string) => f.match(/\.css$/));

        if (!wasmFile || !jsFile) {
            throw new Error('WASM bundle not found. Run: npm run build:wasm');
        }

        const wasmUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', wasmFile)
        );
        const scriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', jsFile)
        );
        const styleUri = cssFile ? webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', cssFile)
        ) : null;

        const nonce = getNonce();
        const styleTag = styleUri ? `<link href="${styleUri}" rel="stylesheet">` : '';

        return `<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Content-Security-Policy"
          content="default-src 'none';
                   connect-src ${webview.cspSource};
                   style-src ${webview.cspSource} 'unsafe-inline';
                   script-src 'nonce-${nonce}' 'wasm-unsafe-eval';
                   img-src ${webview.cspSource} data:;">
    ${styleTag}
    <title>Type Details</title>
    <style nonce="${nonce}">
        /* VSCode Theme - Base */
        :root {
            color-scheme: dark;
        }
        html, body, #root {
            background-color: var(--vscode-editor-background) !important;
            color: var(--vscode-editor-foreground) !important;
            min-height: 100%;
            margin: 0;
            padding: 0;
        }

        /* VSCode Tailwind classes - explicit definitions */
        .bg-vscode-bg { background-color: var(--vscode-editor-background) !important; }
        .text-vscode-fg { color: var(--vscode-editor-foreground) !important; }
        .bg-vscode-input-bg { background-color: var(--vscode-input-background) !important; }
        .text-vscode-input-fg { color: var(--vscode-input-foreground) !important; }
        .bg-vscode-button-bg { background-color: var(--vscode-button-background) !important; }
        .text-vscode-button-fg { color: var(--vscode-button-foreground) !important; }
        .hover\\:bg-vscode-list-hover:hover,
        .bg-vscode-list-hover {
            background-color: var(--vscode-list-hoverBackground) !important;
        }
        .hover\\:bg-vscode-button-hover:hover {
            background-color: var(--vscode-button-hoverBackground) !important;
        }

        /* Border with opacity - VSCode doesn't have RGB version, use panel border */
        .border-vscode-fg\\/20,
        [class*="border-vscode-fg"] {
            border-color: var(--vscode-panel-border) !important;
        }

        /* Scrollbar */
        ::-webkit-scrollbar {
            width: 8px;
        }
        ::-webkit-scrollbar-track {
            background: transparent;
        }
        ::-webkit-scrollbar-thumb {
            background: var(--vscode-scrollbarSlider-background);
            border-radius: 4px;
        }
        ::-webkit-scrollbar-thumb:hover {
            background: var(--vscode-scrollbarSlider-hoverBackground);
        }
    </style>
</head>
<body>
    <div id="root"></div>
    <script type="module" nonce="${nonce}">
        import init, { start_type_details_app } from '${scriptUri}';

        console.log('[Type Details] Initializing WASM...');

        init('${wasmUri}')
            .then(() => {
                console.log('[Type Details] WASM initialized successfully');
                start_type_details_app();
                console.log('[Type Details] App started');
            })
            .catch(err => {
                console.error('[Type Details] Failed to initialize:', err);
                document.getElementById('root').innerHTML =
                    '<div style="padding: 20px; color: red;">' +
                    '<h3>❌ Failed to load Type Details</h3>' +
                    '<pre>' + err.toString() + '</pre>' +
                    '</div>';
            });
    <\/script>
</body>
</html>`;
    }
}

function getNonce() {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}
