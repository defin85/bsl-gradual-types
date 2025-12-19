import * as assert from 'assert';
import * as sinon from 'sinon';
import * as vscode from 'vscode';

import { setupProgressHandler } from '../../lsp/client/progress-handler';

suite('LSP $/progress Handler (multi-token)', () => {
    let outputChannel: vscode.OutputChannel;

    setup(() => {
        outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub()
        } as any;
    });

    teardown(() => {
        sinon.restore();
    });

    test('end for one token does not resolve other token', async () => {
        let notificationHandler: ((params: any) => void) | undefined;

        const client: any = {
            onNotification: (method: string, cb: (params: any) => void) => {
                if (method === '$/progress') {
                    notificationHandler = cb;
                }
            }
        };

        const promises: Promise<void>[] = [];
        const resolved: Record<string, boolean> = {};

        sinon.stub(vscode.window, 'withProgress').callsFake((_opts: any, task: any) => {
            const fakeProgress = { report: sinon.stub() };
            const p = task(fakeProgress);
            promises.push(p);
            return p;
        });

        setupProgressHandler(client as any, outputChannel);
        assert.ok(notificationHandler, '$/progress handler should be registered');

        notificationHandler!({
            token: 't1',
            value: { kind: 'begin', title: 'A', message: 'init' }
        });
        notificationHandler!({
            token: 't2',
            value: { kind: 'begin', title: 'B', message: 'init' }
        });

        // Hook resolution flags after begin (withProgress already produced promises).
        promises[0].then(() => { resolved.t1 = true; });
        promises[1].then(() => { resolved.t2 = true; });

        notificationHandler!({
            token: 't1',
            value: { kind: 'end', message: 'done' }
        });

        await Promise.resolve();
        assert.strictEqual(resolved.t1, true, 't1 should resolve on end');
        assert.notStrictEqual(resolved.t2, true, 't2 should stay active');

        notificationHandler!({
            token: 't2',
            value: { kind: 'end', message: 'done' }
        });

        await Promise.resolve();
        assert.strictEqual(resolved.t2, true, 't2 should resolve on end');
    });
});

