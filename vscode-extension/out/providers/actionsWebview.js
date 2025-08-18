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
/**
 * WebView провайдер для панели быстрых действий
 */
class BslActionsWebviewProvider {
    constructor(extensionUri) {
        this.extensionUri = extensionUri;
    }
    resolveWebviewView(webviewView) {
        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [this.extensionUri]
        };
        webviewView.webview.html = this.getWebviewContent();
        // Handle messages from webview
        webviewView.webview.onDidReceiveMessage(async (message) => {
            switch (message.command) {
                case 'analyzeCurrentFile':
                    vscode.commands.executeCommand('bslAnalyzer.analyzeFile');
                    break;
                case 'buildIndex':
                    vscode.commands.executeCommand('bslAnalyzer.buildIndex');
                    break;
                case 'showMetrics':
                    vscode.commands.executeCommand('bslAnalyzer.showMetrics');
                    break;
                case 'openSettings':
                    vscode.commands.executeCommand('workbench.action.openSettings', 'bslAnalyzer');
                    break;
            }
        });
    }
    getWebviewContent() {
        return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>BSL Analyzer Actions</title>
            <style>
                body {
                    font-family: var(--vscode-font-family);
                    font-size: var(--vscode-font-size);
                    color: var(--vscode-foreground);
                    background-color: var(--vscode-editor-background);
                    margin: 0;
                    padding: 16px;
                }
                .action-button {
                    display: block;
                    width: 100%;
                    padding: 8px 12px;
                    margin: 8px 0;
                    background-color: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                    text-align: left;
                    font-size: 13px;
                }
                .action-button:hover {
                    background-color: var(--vscode-button-hoverBackground);
                }
                .action-button:active {
                    transform: translateY(1px);
                }
                .section-title {
                    font-weight: bold;
                    margin: 16px 0 8px 0;
                    color: var(--vscode-descriptionForeground);
                    font-size: 11px;
                    text-transform: uppercase;
                }
                .icon {
                    margin-right: 8px;
                }
            </style>
        </head>
        <body>
            <div class="section-title">Analysis</div>
            <button class="action-button" onclick="sendMessage('analyzeCurrentFile')">
                <span class="icon">🔍</span>Analyze Current File
            </button>
            <button class="action-button" onclick="sendMessage('buildIndex')">
                <span class="icon">📋</span>Build Type Index
            </button>
            <button class="action-button" onclick="sendMessage('showMetrics')">
                <span class="icon">📊</span>Show Metrics
            </button>
            
            <div class="section-title">Configuration</div>
            <button class="action-button" onclick="sendMessage('openSettings')">
                <span class="icon">⚙️</span>Open Settings
            </button>

            <script>
                const vscode = acquireVsCodeApi();
                
                function sendMessage(command) {
                    vscode.postMessage({ command: command });
                }
            </script>
        </body>
        </html>
        `;
    }
}
exports.BslActionsWebviewProvider = BslActionsWebviewProvider;
//# sourceMappingURL=actionsWebview.js.map