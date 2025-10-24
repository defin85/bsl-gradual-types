import * as vscode from 'vscode';
import * as path from 'path';
import { searchTypes } from '../lsp/customRequests';

/**
 * WebView провайдер для панели быстрых действий
 */
export class BslActionsWebviewProvider implements vscode.WebviewViewProvider {
    private outputChannel?: vscode.OutputChannel;

    constructor(
        private readonly extensionUri: vscode.Uri,
        private readonly client?: any,
        outputChannel?: vscode.OutputChannel
    ) {
        this.outputChannel = outputChannel;
    }

    resolveWebviewView(webviewView: vscode.WebviewView): void {
        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                vscode.Uri.joinPath(this.extensionUri, 'media', 'webview')
            ]
        };

        webviewView.webview.html = this.getWebviewContent(webviewView.webview);

        webviewView.webview.onDidReceiveMessage(async (message) => {
            switch (message.type) {
                case 'executeAction':
                    await this.handleAction(message.action);
                    break;

                case 'searchTypes':
                    await this.handleSearchTypes(webviewView, message.query);
                    break;

                case 'showTypeDetails':
                    vscode.commands.executeCommand(
                        'bslAnalyzer.showTypeDetails',
                        message.typeName
                    );
                    break;
            }
        });
    }

    private async handleAction(action: string) {
        const commandMap: Record<string, string> = {
            analyzeProject: 'bslAnalyzer.analyzeWorkspace',
            buildIndex: 'bslAnalyzer.buildIndex',
            openSettings: 'workbench.action.openSettings',
            showDocs: 'markdown.showPreview',
        };

        const command = commandMap[action];
        if (command) {
            if (action === 'showDocs') {
                // Открыть CLAUDE.md из корня проекта расширения (родительская папка)
                const extensionRoot = vscode.Uri.joinPath(this.extensionUri, '..');
                const claudeMdPath = vscode.Uri.joinPath(extensionRoot, 'CLAUDE.md');

                try {
                    const doc = await vscode.workspace.openTextDocument(claudeMdPath);
                    await vscode.window.showTextDocument(doc);
                } catch (error) {
                    vscode.window.showWarningMessage(
                        `Не удалось открыть CLAUDE.md. Попробуйте открыть вручную: ${claudeMdPath.fsPath}`
                    );
                }
            } else {
                await vscode.commands.executeCommand(command, 'bslAnalyzer');
            }
        }
    }

    private async handleSearchTypes(
        webviewView: vscode.WebviewView,
        query: string
    ) {
        try {
            // ✅ РЕАЛЬНЫЕ ДАННЫЕ из TypeRepository через LSP
            const response = await searchTypes(query, 15);

            // Конвертируем в формат для webview (совместим с mock данными)
            const results = response.types.map(t => ({
                name: t.name,
                facet: t.facet,
                certainty: t.certainty,
            }));

            // Отправляем результаты обратно в webview
            webviewView.webview.postMessage({
                type: 'searchResults',
                data: results,
            });

            this.outputChannel?.appendLine(
                `🔍 Search "${query}" → ${response.total} results from TypeRepository`
            );
        } catch (error) {
            this.outputChannel?.appendLine(
                `❌ Search failed: ${error}`
            );

            // Fallback на пустой массив (graceful degradation)
            webviewView.webview.postMessage({
                type: 'searchResults',
                data: [],
            });
        }
    }

    private getWebviewContent(webview: vscode.Webview): string {
        const scriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', 'quickActions.js')
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
    <meta http-equiv="Content-Security-Policy" 
          content="default-src 'none'; 
                   style-src ${webview.cspSource} 'unsafe-inline'; 
                   script-src 'nonce-${nonce}';">
    <link href="${styleUri}" rel="stylesheet">
    <title>BSL Quick Actions</title>
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
