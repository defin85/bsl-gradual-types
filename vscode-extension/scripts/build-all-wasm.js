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
const { ensureDir, readJsonIfExists, writeJson, summarizePaths, getMaxMtimeInDir } = require('./build-cache-utils');

const FRONTEND_DIR = path.resolve(__dirname, '../../frontend');
const WEBVIEW_DIST = path.resolve(__dirname, '../media/webview');
const TEMP_DIR = path.resolve(__dirname, '../.tmp-wasm-build');
const PROJECT_ROOT = path.resolve(__dirname, '../..');
const CACHE_DIR = path.resolve(__dirname, '..', '.cache');
const CACHE_PATH = path.join(CACHE_DIR, 'wasm-build.json');
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
const isRelease = args.includes('--release');
const isForce = args.includes('--force') || process.env.BSL_FORCE_WASM_BUILD === 'true';

console.log('🚀 Building ALL WASM webview bundles...');
console.log(`   Build mode: ${isRelease ? 'RELEASE' : 'DEBUG'}\n`);

function shouldIncludeFrontendFile(relPath) {
    // Keep it simple and stable: track sources that affect Trunk builds.
    // We intentionally do NOT include frontend/target or frontend/dist (generated).
    const normalized = relPath.replace(/\\/g, '/');
    if (normalized.includes('/target/')) {
        return false;
    }
    if (normalized.includes('/dist/')) {
        return false;
    }
    if (normalized.includes('/node_modules/')) {
        return false;
    }

    // Most relevant extensions for trunk/leptos/tailwind.
    if (normalized.endsWith('.rs')) return true;
    if (normalized.endsWith('.toml')) return true;
    if (normalized.endsWith('.lock')) return true;
    if (normalized.endsWith('.html')) return true;
    if (normalized.endsWith('.css')) return true;
    if (normalized.endsWith('.js')) return true;
    if (normalized.endsWith('.ts')) return true;
    if (normalized.endsWith('.json')) return true;
    if (normalized.endsWith('.svg')) return true;
    if (normalized.endsWith('.png')) return true;

    // Also track these build-related files when present.
    const base = path.posix.basename(normalized);
    if (base === 'Trunk.toml') return true;
    if (base === 'tailwind.config.js') return true;

    return false;
}

function getInputsSummary() {
    const excludeDirNames = new Set(['target', 'dist', 'node_modules', '.git']);
    return summarizePaths(
        [
            FRONTEND_DIR,
            path.join(PROJECT_ROOT, 'Cargo.toml'),
            path.join(PROJECT_ROOT, 'Cargo.lock'),
        ],
        {
            projectRoot: PROJECT_ROOT,
            excludeDirNames,
            includeFile: shouldIncludeFrontendFile,
        }
    );
}

function hasWebviewOutputs() {
    if (!fs.existsSync(WEBVIEW_DIST)) {
        return false;
    }
    try {
        const files = fs.readdirSync(WEBVIEW_DIST);
        return files.some(f => f.endsWith('.wasm')) && files.some(f => f.endsWith('.js'));
    } catch {
        return false;
    }
}

function isUpToDate(inputsSummary, cache) {
    if (!hasWebviewOutputs()) {
        return false;
    }

    const outputsMaxMtimeMs = getMaxMtimeInDir(WEBVIEW_DIST);
    if (outputsMaxMtimeMs <= 0) {
        return false;
    }

    // If outputs are older than inputs, it's definitely stale.
    if (outputsMaxMtimeMs < inputsSummary.maxMtimeMs) {
        return false;
    }

    // Strictness:
    // - For release builds we require that the previous build mode was RELEASE (same dir is reused).
    // - For debug builds it's OK to reuse RELEASE outputs (they are compatible for running the extension).
    if (isRelease) {
        if (!cache || cache.mode !== 'release') {
            return false;
        }
    }

    // Fingerprint gives a strong signal; if cache missing but outputs are newer, still accept and write cache.
    if (!cache) {
        return true;
    }
    return cache.fingerprint === inputsSummary.fingerprint;
}

ensureDir(CACHE_DIR);
const inputsSummary = getInputsSummary();
const cache = readJsonIfExists(CACHE_PATH);

if (!isForce && isUpToDate(inputsSummary, cache)) {
    console.log('✅ WASM webview bundles are up-to-date, skipping.\n');
    const resolvedMode = (() => {
        if (isRelease) return 'release';
        if (cache && cache.mode) return cache.mode;
        return 'unknown';
    })();
    writeJson(CACHE_PATH, {
        mode: resolvedMode,
        fingerprint: inputsSummary.fingerprint,
        fileCount: inputsSummary.fileCount,
        inputsMaxMtimeMs: inputsSummary.maxMtimeMs,
        outputsMaxMtimeMs: getMaxMtimeInDir(WEBVIEW_DIST),
        updatedAt: new Date().toISOString(),
    });
    process.exit(0);
}

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
                env: trunkEnv,
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

writeJson(CACHE_PATH, {
    mode: isRelease ? 'release' : 'debug',
    fingerprint: inputsSummary.fingerprint,
    fileCount: inputsSummary.fileCount,
    inputsMaxMtimeMs: inputsSummary.maxMtimeMs,
    outputsMaxMtimeMs: getMaxMtimeInDir(WEBVIEW_DIST),
    builtAt: new Date().toISOString(),
});
