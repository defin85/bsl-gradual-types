#!/usr/bin/env node

/**
 * Build WASM webview bundles using Trunk
 *
 * This script builds Leptos/WASM bundles for VSCode webviews.
 * Usage: node scripts/build-wasm.js [--release] [--optimize]
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const FRONTEND_DIR = path.resolve(__dirname, '../../frontend');
const WEBVIEW_DIST = path.resolve(__dirname, '../media/webview');

// Parse arguments
const args = process.argv.slice(2);
const isRelease = args.includes('--release');
const shouldOptimize = args.includes('--optimize');

console.log('🚀 Building WASM webview bundles...');
console.log(`   Mode: ${isRelease ? 'RELEASE' : 'DEBUG'}`);
console.log(`   Optimize: ${shouldOptimize ? 'YES' : 'NO'}`);

// Ensure dist directory exists
if (!fs.existsSync(WEBVIEW_DIST)) {
    fs.mkdirSync(WEBVIEW_DIST, { recursive: true });
}

// Build command
const releaseArg = isRelease ? '--release' : '';
const buildCmd = `trunk build ${releaseArg} --dist ${WEBVIEW_DIST} type_details.html`;

try {
    console.log(`\n📦 Running: cd ${FRONTEND_DIR} && ${buildCmd}`);
    execSync(buildCmd, {
        cwd: FRONTEND_DIR,
        stdio: 'inherit',
        shell: true
    });
    console.log('✅ WASM bundle built successfully');

    // Optimize WASM if requested
    if (shouldOptimize) {
        // Find WASM file (trunk creates files with hash in name)
        const files = fs.readdirSync(WEBVIEW_DIST);
        const wasmFile = files.find(f => f.endsWith('_bg.wasm'));

        if (wasmFile) {
            const wasmPath = path.join(WEBVIEW_DIST, wasmFile);
            console.log('\n🔧 Optimizing WASM with wasm-opt...');
            try {
                execSync(`wasm-opt -Oz -o ${wasmPath} ${wasmPath}`, {
                    stdio: 'inherit',
                    shell: true
                });
                console.log('✅ WASM optimized successfully');
            } catch (err) {
                console.warn('⚠️  wasm-opt not found, skipping optimization');
                console.warn('   Install it from: https://github.com/WebAssembly/binaryen');
            }
        }
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
