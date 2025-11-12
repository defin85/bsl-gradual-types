#!/usr/bin/env node
/**
 * Симуляция запуска LSP сервера как это делает VSCode
 */

const { spawn } = require('child_process');
const path = require('path');

const serverPath = path.join(__dirname, 'bin', 'lsp-server.exe');

console.log('🧪 Тестирование LSP сервера (VSCode симуляция)');
console.log('📍 Путь:', serverPath);
console.log('🚀 Запуск процесса...\n');

const serverProcess = spawn(serverPath, [], {
    stdio: ['pipe', 'pipe', 'pipe'],
    env: {
        ...process.env,
        RUST_LOG: 'info',
        RUST_BACKTRACE: '1'
    }
});

// LSP initialize request
const initRequest = JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
        processId: process.pid,
        rootUri: null,
        capabilities: {}
    }
});

const message = `Content-Length: ${initRequest.length}\r\n\r\n${initRequest}`;

console.log('📤 Отправка initialize request...');
serverProcess.stdin.write(message);

let buffer = '';

serverProcess.stdout.on('data', (data) => {
    buffer += data.toString();
    console.log('📥 Получен ответ от сервера:');
    console.log(buffer);
});

serverProcess.stderr.on('data', (data) => {
    console.log('⚠️ STDERR:', data.toString());
});

serverProcess.on('error', (error) => {
    console.error('❌ Ошибка запуска процесса:', error);
    process.exit(1);
});

serverProcess.on('exit', (code, signal) => {
    console.log(`\n🛑 Процесс завершён: code=${code}, signal=${signal}`);
    if (code !== 0 && code !== null) {
        console.error('❌ Сервер завершился с ошибкой');
        process.exit(1);
    }
});

// Graceful shutdown после 5 секунд
setTimeout(() => {
    console.log('\n⏰ Timeout - завершение теста');
    serverProcess.kill();
    process.exit(0);
}, 5000);
