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
                        vscode.Uri.joinPath(this.extensionUri, 'media', 'webview')
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
            if (message.type === 'ready') {
                await this.updateTypeInfo(panel, typeName);
            } else if (message.type === 'close') {
                panel.dispose();
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
        const scriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', 'typeDetails.js')
        );
        const styleUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', 'tailwind.css')
        );
        const nonce = getNonce();

        return `<!DOCTYPE html>
<html lang="ru">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src 'nonce-${nonce}';">
    <link href="${styleUri}" rel="stylesheet">
    <title>Type Details</title>
</head>
<body>
    <div id="root"></div>
    <script type="module" nonce="${nonce}" src="${scriptUri}"><\/script>
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
