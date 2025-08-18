import * as vscode from 'vscode';
/**
 * WebView провайдер для панели быстрых действий
 */
export declare class BslActionsWebviewProvider implements vscode.WebviewViewProvider {
    private readonly extensionUri;
    constructor(extensionUri: vscode.Uri);
    resolveWebviewView(webviewView: vscode.WebviewView): void;
    private getWebviewContent;
}
//# sourceMappingURL=actionsWebview.d.ts.map