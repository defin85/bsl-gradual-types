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
exports.BslActionsWebviewProvider = void 0;
const vscode = __importStar(require("vscode"));
const customRequests_1 = require("../lsp/customRequests");
/**
 * WebView провайдер для панели быстрых действий
 */
class BslActionsWebviewProvider {
    constructor(extensionUri, client, outputChannel) {
        this.extensionUri = extensionUri;
        this.client = client;
        this.outputChannel = outputChannel;
    }
    resolveWebviewView(webviewView) {
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
                    vscode.commands.executeCommand('bslAnalyzer.showTypeDetails', message.typeName);
                    break;
            }
        });
    }
    async handleAction(action) {
        const commandMap = {
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
                }
                catch (error) {
                    vscode.window.showWarningMessage(`Не удалось открыть CLAUDE.md. Попробуйте открыть вручную: ${claudeMdPath.fsPath}`);
                }
            }
            else {
                await vscode.commands.executeCommand(command, 'bslAnalyzer');
            }
        }
    }
    async handleSearchTypes(webviewView, query) {
        try {
            // ✅ РЕАЛЬНЫЕ ДАННЫЕ из TypeRepository через LSP
            const response = await (0, customRequests_1.searchTypes)(query, 15);
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
            this.outputChannel?.appendLine(`🔍 Search "${query}" → ${response.total} results from TypeRepository`);
        }
        catch (error) {
            this.outputChannel?.appendLine(`❌ Search failed: ${error}`);
            // Fallback на пустой массив (graceful degradation)
            webviewView.webview.postMessage({
                type: 'searchResults',
                data: [],
            });
        }
    }
    getWebviewContent(webview) {
        const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', 'quickActions.js'));
        const styleUri = webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', 'tailwind.css'));
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
exports.BslActionsWebviewProvider = BslActionsWebviewProvider;
function getNonce() {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
}
//# sourceMappingURL=actionsWebview.js.map