import * as vscode from 'vscode';
export declare class TypeDetailsWebviewProvider {
    private readonly extensionUri;
    private readonly client?;
    private static currentPanel;
    constructor(extensionUri: vscode.Uri, client?: any);
    showTypeDetails(typeName: string): Promise<void>;
    private updateTypeInfo;
    private getWebviewContent;
}
//# sourceMappingURL=typeDetailsWebview.d.ts.map