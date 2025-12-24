#!/usr/bin/env node

/**
 * Build WASM webview bundles using Trunk
 *
 * This script builds Leptos/WASM bundles for VSCode webviews.
 * Usage: node scripts/build-wasm.js [webview_name] [--release] [--optimize]
 *
 * Examples:
 *   npm run build:wasm                    # Build type_details (default)
 *   npm run build:wasm quick_actions      # Build quick_actions
 *   npm run build:wasm -- --release       # Build type_details in release mode
 *   npm run build:wasm quick_actions --release  # Build quick_actions in release mode
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const FRONTEND_DIR = path.resolve(__dirname, '../../frontend');
const WEBVIEW_DIST = path.resolve(__dirname, '../media/webview');
const trunkEnv = (() => {
    const env = { ...process.env };
    if (env.NO_COLOR === '1') {
        env.NO_COLOR = 'true';
    } else if (env.NO_COLOR === '0') {
        env.NO_COLOR = 'false';
    }
    if (env.TRUNK_NO_COLOR === '1') {
        env.TRUNK_NO_COLOR = 'true';
    } else if (env.TRUNK_NO_COLOR === '0') {
        env.TRUNK_NO_COLOR = 'false';
    } else if (env.NO_COLOR && !env.TRUNK_NO_COLOR) {
        env.TRUNK_NO_COLOR = 'true';
    }
    return env;
})();

// Parse arguments
const args = process.argv.slice(2);

// Extract webview name (first non-flag argument)
let webviewName = 'type_details'; // default
for (const arg of args) {
    if (!arg.startsWith('--')) {
        webviewName = arg;
        break;
    }
}

const isRelease = args.includes('--release');
// ✅ Автоматически оптимизировать для release builds
const shouldOptimize = isRelease || args.includes('--optimize');

console.log(`🚀 Building ${webviewName} WASM webview bundle...`);
console.log(`   Build mode: ${isRelease ? 'RELEASE (--release mode)' : 'DEBUG'}`);
console.log(`   Optimize: ${shouldOptimize ? 'YES (trunk default)' : 'NO'}`);

// Ensure dist directory exists
if (!fs.existsSync(WEBVIEW_DIST)) {
    fs.mkdirSync(WEBVIEW_DIST, { recursive: true });
}

// ✅ Build command с использованием --release для оптимального размера
// Note: trunk 0.21.14 не поддерживает --profile флаг, используем --release вместо этого
const releaseArg = isRelease ? '--release' : '';
const htmlFile = `${webviewName}.html`;
// ⚠️ wasm-opt может иметь проблемы с bulk-memory в release mode
// Решение: использовать TRUNK_WATCH_IGNORE для пропуска оптимизации если нужно
const buildCmd = `trunk build ${releaseArg} --dist ${WEBVIEW_DIST} --features vscode ${htmlFile}`;

try {
    console.log(`\n📦 Running: cd ${FRONTEND_DIR} && ${buildCmd}`);
    execSync(buildCmd, {
        cwd: FRONTEND_DIR,
        env: trunkEnv,
        stdio: 'inherit',
        shell: true
    });
    console.log('✅ WASM bundle built successfully');

    // ⚠️ WASM optimization временно отключена из-за bulk-memory проблем
    // Trunk уже применяет базовую оптимизацию в release mode
    if (shouldOptimize) {
        console.log('\n⚠️  WASM optimization disabled due to bulk-memory issues');
        console.log('   Trunk applies basic optimization in release mode');
        console.log('   Bundle size optimizations applied via Cargo profile settings');
    }

    // Show bundle sizes
    console.log('\n📊 Bundle sizes:');
    const files = fs.readdirSync(WEBVIEW_DIST);
    files.forEach(file => {
        const filePath = path.join(WEBVIEW_DIST, file);
        const stats = fs.statSync(filePath);
        const sizeKB = (stats.size / 1024).toFixed(2);
        console.log(`   ${file}: ${sizeKB} KB`);
    });

} catch (error) {
    console.error('❌ Failed to build WASM bundle:', error.message);
    process.exit(1);
}
