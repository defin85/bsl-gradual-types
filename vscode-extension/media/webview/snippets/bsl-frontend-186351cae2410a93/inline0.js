
let cachedVsCodeApi = null;

export function getVsCodeApi() {
    if (!cachedVsCodeApi) {
        if (typeof acquireVsCodeApi === 'function') {
            cachedVsCodeApi = acquireVsCodeApi();
            console.log('[VSCode WASM] VSCode API acquired successfully');
        } else {
            console.error('[VSCode WASM] acquireVsCodeApi is not available!');
            return null;
        }
    }
    return cachedVsCodeApi;
}

export function postMessageToVscode(message) {
    const api = getVsCodeApi();
    if (api) {
        api.postMessage(message);
        console.log('[VSCode WASM] Message sent:', message);
        return true;
    }
    console.error('[VSCode WASM] Cannot send message - no VSCode API');
    return false;
}
