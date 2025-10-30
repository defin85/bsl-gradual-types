import * as vscode from 'vscode';
/**
 * WebView провайдер для панели быстрых действий
 */
export declare class BslActionsWebviewProvider implements vscode.WebviewViewProvider {
    private readonly extensionUri;
    private readonly client?;
    private outputChannel?;
    constructor(extensionUri: vscode.Uri, client?: any, outputChannel?: vscode.OutputChannel);
    resolveWebviewView(webviewView: vscode.WebviewView): void;
    private handleAction;
    private handleSearchTypes;
    private getWebviewContent;
}
//# sourceMappingURL=actionsWebview.d.ts.map