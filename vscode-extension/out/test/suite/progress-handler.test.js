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
const assert = __importStar(require("assert"));
const sinon = __importStar(require("sinon"));
const vscode = __importStar(require("vscode"));
const progress_handler_1 = require("../../lsp/client/progress-handler");
suite('LSP $/progress Handler (multi-token)', () => {
    let outputChannel;
    setup(() => {
        outputChannel = {
            appendLine: sinon.stub(),
            show: sinon.stub(),
            dispose: sinon.stub()
        };
    });
    teardown(() => {
        sinon.restore();
    });
    test('end for one token does not resolve other token', async () => {
        const clock = sinon.useFakeTimers();
        try {
            const tick = async (ms) => {
                const tickAsync = clock.tickAsync;
                if (typeof tickAsync === 'function') {
                    await tickAsync.call(clock, ms);
                    return;
                }
                clock.tick(ms);
                await Promise.resolve();
                await Promise.resolve();
            };
            let notificationHandler;
            const client = {
                onNotification: (method, cb) => {
                    if (method === '$/progress') {
                        notificationHandler = cb;
                    }
                }
            };
            const promises = [];
            const resolved = {};
            sinon.stub(vscode.window, 'withProgress').callsFake((_opts, task) => {
                const fakeProgress = { report: sinon.stub() };
                const p = task(fakeProgress);
                promises.push(p);
                return p;
            });
            (0, progress_handler_1.setupProgressHandler)(client, outputChannel);
            assert.ok(notificationHandler, '$/progress handler should be registered');
            notificationHandler({
                token: 't1',
                value: { kind: 'begin', title: 'A', message: 'init' }
            });
            notificationHandler({
                token: 't2',
                value: { kind: 'begin', title: 'B', message: 'init' }
            });
            // Hook resolution flags after begin (withProgress already produced promises).
            promises[0].then(() => { resolved.t1 = true; });
            promises[1].then(() => { resolved.t2 = true; });
            notificationHandler({
                token: 't1',
                value: { kind: 'end', message: 'done' }
            });
            // Progress end is delayed to keep progress visible for a minimum duration
            await tick(800);
            assert.strictEqual(resolved.t1, true, 't1 should resolve on end');
            assert.notStrictEqual(resolved.t2, true, 't2 should stay active');
            notificationHandler({
                token: 't2',
                value: { kind: 'end', message: 'done' }
            });
            await tick(800);
            assert.strictEqual(resolved.t2, true, 't2 should resolve on end');
        }
        finally {
            clock.restore();
        }
    });
});
//# sourceMappingURL=progress-handler.test.js.map