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
const logger_1 = require("../lsp/logger");
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
            // ✅ Валидация структуры сообщения
            if (!message || !message.type || typeof message.type !== 'string') {
                logger_1.logger.warn('Invalid message format received from Quick Actions webview');
                return;
            }
            // ✅ Whitelist допустимых типов сообщений
            const ALLOWED_MESSAGE_TYPES = ['ready', 'executeAction', 'searchTypes', 'showTypeDetails'];
            if (!ALLOWED_MESSAGE_TYPES.includes(message.type)) {
                logger_1.logger.warn(`Unknown message type from Quick Actions webview: ${message.type}`);
                return;
            }
            switch (message.type) {
                case 'ready':
                    // Quick Actions initialized
                    logger_1.logger.debug('Quick Actions webview ready');
                    break;
                case 'executeAction':
                    await this.handleAction(message.data);
                    break;
                case 'searchTypes':
                    await this.handleSearchTypes(webviewView, message.data);
                    break;
                case 'showTypeDetails':
                    vscode.commands.executeCommand('bslAnalyzer.showTypeDetails', message.data // Type name sent in data field from Leptos
                    );
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
        // 🆕 Find WASM files dynamically (hash changes on rebuild)
        const fs = require('fs');
        const webviewPath = vscode.Uri.joinPath(this.extensionUri, 'media', 'webview').fsPath;
        const files = fs.readdirSync(webviewPath);
        this.outputChannel?.appendLine(`🔍 Quick Actions: Looking for WASM files in ${webviewPath}`);
        this.outputChannel?.appendLine(`   Available files: ${files.join(', ')}`);
        const wasmFile = files.find((f) => f.match(/^bsl-frontend-.*\.wasm$/));
        const jsFile = files.find((f) => f.match(/^bsl-frontend-.*\.js$/));
        // ✅ ИСПРАВЛЕНИЕ: Ищем специфично tailwind CSS для Quick Actions
        const cssFile = files.find((f) => f.match(/^tailwind-.*\.css$/)) ||
            files.find((f) => f.match(/\.css$/));
        this.outputChannel?.appendLine(`   Found: WASM=${wasmFile}, JS=${jsFile}, CSS=${cssFile}`);
        if (!wasmFile || !jsFile) {
            throw new Error('WASM bundle not found. Run: npm run build:wasm');
        }
        const wasmUri = webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', wasmFile));
        const scriptUri = webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', jsFile));
        const styleUri = cssFile ? webview.asWebviewUri(vscode.Uri.joinPath(this.extensionUri, 'media', 'webview', cssFile)) : null;
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
                   style-src ${webview.cspSource};
                   script-src 'nonce-${nonce}' 'wasm-unsafe-eval';
                   img-src ${webview.cspSource} data:;">
    ${styleTag}
    <title>BSL Quick Actions</title>
</head>
<body>
    <div id="root"></div>
    <script type="module" nonce="${nonce}">
        import init, { start_quick_actions_app } from '${scriptUri}';
        
        console.log('[Quick Actions] Initializing WASM...');
        console.log('[Quick Actions] WASM URI:', '${wasmUri}');
        console.log('[Quick Actions] Script URI:', '${scriptUri}');
        
        init('${wasmUri}')
            .then(() => {
                console.log('[Quick Actions] WASM initialized successfully');
                start_quick_actions_app();
                console.log('[Quick Actions] App started');
            })
            .catch(err => {
                console.error('[Quick Actions] Failed to initialize:', err);
                document.getElementById('root').innerHTML = 
                    '<div style="padding: 20px; color: red;">' +
                    '<h3>❌ Failed to load Quick Actions</h3>' +
                    '<pre>' + err.toString() + '</pre>' +
                    '</div>';
            });
    <\/script>
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