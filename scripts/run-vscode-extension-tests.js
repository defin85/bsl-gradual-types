#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const REPO_ROOT = path.resolve(__dirname, '..');
const EXTENSION_DIR = path.join(REPO_ROOT, 'vscode-extension');
const TEST_RUNNER = path.join(EXTENSION_DIR, 'out', 'test', 'runTest.js');
const HEADLESS_ELECTRON_ARGS = [
    '--disable-gpu',
    '--disable-dev-shm-usage',
    '--disable-software-rasterizer',
];
const XVFB_ARGS = [
    '-a',
    '--server-args=-screen 0 1280x960x24',
];

function fileContainsMicrosoft(filePath) {
    try {
        return fs.readFileSync(filePath, 'utf8').toLowerCase().includes('microsoft');
    } catch {
        return false;
    }
}

function isWsl() {
    return Boolean(
        process.env.WSL_DISTRO_NAME ||
            process.env.WSL_INTEROP ||
            fileContainsMicrosoft('/proc/version') ||
            fileContainsMicrosoft('/proc/sys/kernel/osrelease'),
    );
}

function shouldForceHeadless() {
    if (process.platform !== 'linux') {
        return false;
    }

    if (isWsl()) {
        return true;
    }

    return !process.env.DISPLAY && !process.env.WAYLAND_DISPLAY;
}

function hasCommand(command) {
    const probe = spawnSync(command, ['--help'], { stdio: 'ignore' });
    return !probe.error || probe.error.code !== 'ENOENT';
}

function appendHeadlessElectronArgs() {
    const launchArgs = (process.env.BSL_TEST_ELECTRON_LAUNCH_ARGS ?? '')
        .split(/\s+/)
        .filter(Boolean);

    for (const arg of HEADLESS_ELECTRON_ARGS) {
        if (!launchArgs.includes(arg)) {
            launchArgs.push(arg);
        }
    }

    process.env.BSL_TEST_ELECTRON_LAUNCH_ARGS = launchArgs.join(' ');
}

function runCommand(command, args) {
    const result = spawnSync(command, args, {
        cwd: EXTENSION_DIR,
        env: process.env,
        stdio: 'inherit',
    });

    if (result.error) {
        if (result.error.code === 'ENOENT') {
            console.error(`Required command is unavailable: ${command}`);
        } else {
            console.error(result.error.message);
        }
        process.exit(1);
    }

    process.exit(result.status ?? 1);
}

function runTestsHeadless() {
    if (!hasCommand('xvfb-run')) {
        console.error(
            'VS Code integration tests require xvfb-run when DISPLAY/WAYLAND is unavailable or the runner is inside WSL.',
        );
        process.exit(1);
    }

    appendHeadlessElectronArgs();

    const nodeArgs = [TEST_RUNNER, ...process.argv.slice(2)];
    const xvfbCommandArgs = [...XVFB_ARGS, process.execPath, ...nodeArgs];

    if (!process.env.DBUS_SESSION_BUS_ADDRESS && hasCommand('dbus-run-session')) {
        runCommand('dbus-run-session', ['--', 'xvfb-run', ...xvfbCommandArgs]);
        return;
    }

    runCommand('xvfb-run', xvfbCommandArgs);
}

if (shouldForceHeadless()) {
    runTestsHeadless();
} else {
    runCommand(process.execPath, [TEST_RUNNER, ...process.argv.slice(2)]);
}
