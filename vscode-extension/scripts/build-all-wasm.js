#!/usr/bin/env node

/**
 * Build ALL WASM webview bundles sequentially
 *
 * This script builds both type_details and quick_actions webviews,
 * merging their outputs into a single dist directory.
 * 
 * The challenge: Trunk cleans the dist directory before each build,
 * so we need to build to temp directories and then merge.
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const FRONTEND_DIR = path.resolve(__dirname, '../../frontend');
const WEBVIEW_DIST = path.resolve(__dirname, '../media/webview');
const TEMP_DIR = path.resolve(__dirname, '../.tmp-wasm-build');

// Parse arguments
const args = process.argv.slice(2);
const isRelease = args.includes('--release');

console.log('🚀 Building ALL WASM webview bundles...');
console.log(`   Build mode: ${isRelease ? 'RELEASE' : 'DEBUG'}\n`);

// Clean dist and temp directories
console.log('🧹 Cleaning directories...');
if (fs.existsSync(WEBVIEW_DIST)) {
    fs.rmSync(WEBVIEW_DIST, { recursive: true, force: true });
}
if (fs.existsSync(TEMP_DIR)) {
    fs.rmSync(TEMP_DIR, { recursive: true, force: true });
}
fs.mkdirSync(WEBVIEW_DIST, { recursive: true });
fs.mkdirSync(TEMP_DIR, { recursive: true });

const webviews = ['type_details', 'quick_actions'];

for (const webview of webviews) {
    console.log(`\n📦 Building ${webview}...`);
    const tempDist = path.join(TEMP_DIR, webview);
    fs.mkdirSync(tempDist, { recursive: true });

    const releaseArg = isRelease ? '--release' : '';

    try {
        execSync(
            `trunk build ${releaseArg} --dist ${tempDist} --features vscode ${webview}.html`,
            {
                cwd: FRONTEND_DIR,
                stdio: 'inherit',
                shell: true
            }
        );

        // Copy files to main dist (except index.html - we don't need it in webview)
        // ✅ ИСПРАВЛЕНИЕ: Копируем только если файл ещё не существует (чтобы сохранить CSS от обоих)
        console.log(`   Copying ${webview} files to dist...`);
        const files = fs.readdirSync(tempDist);
        for (const file of files) {
            if (file !== 'index.html') {  // Skip index.html
                const src = path.join(tempDist, file);
                const dest = path.join(WEBVIEW_DIST, file);

                // Check if it's a directory (e.g., snippets from inline_js)
                const stats = fs.statSync(src);
                if (stats.isDirectory()) {
                    // Recursively copy directory
                    if (!fs.existsSync(dest)) {
                        fs.cpSync(src, dest, { recursive: true });
                        console.log(`   ✅ ${file}/ (directory)`);
                    } else {
                        console.log(`   ⏭️  ${file}/ (directory exists, skipping)`);
                    }
                    continue;
                }

                // Копируем только если файл не существует ИЛИ это CSS файл (CSS уникальны)
                const isCss = file.endsWith('.css');
                const exists = fs.existsSync(dest);

                if (!exists || isCss) {
                    fs.copyFileSync(src, dest);
                    console.log(`   ✅ ${file}${exists && isCss ? ' (CSS updated)' : ''}`);
                } else {
                    console.log(`   ⏭️  ${file} (already exists, skipping)`);
                }
            }
        }
    } catch (error) {
        console.error(`❌ Failed to build ${webview}:`, error.message);
        process.exit(1);
    }
}

// ✅ Проверка перед удалением temp директории
console.log('\n🔍 Verifying copied files...');
const copiedFiles = fs.readdirSync(WEBVIEW_DIST);
console.log('   Files in dist:', copiedFiles);

// Clean up temp directory
console.log('\n🧹 Cleaning up temp files...');
fs.rmSync(TEMP_DIR, { recursive: true, force: true });

// Show final bundle sizes
console.log('\n📊 Final bundle sizes:');
const files = fs.readdirSync(WEBVIEW_DIST);
files.forEach(file => {
    const filePath = path.join(WEBVIEW_DIST, file);
    const stats = fs.statSync(filePath);
    const sizeKB = (stats.size / 1024).toFixed(2);
    console.log(`   ${file}: ${sizeKB} KB`);
});

console.log('\n✅ All WASM bundles built successfully!');
