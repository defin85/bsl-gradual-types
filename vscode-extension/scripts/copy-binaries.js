#!/usr/bin/env node
/**
 * Скрипт для копирования актуальных Rust бинарников в директорию расширения
 * Запускается автоматически перед компиляцией и публикацией расширения
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const { ensureDir, readJsonIfExists, writeJson, summarizePaths } = require('./build-cache-utils');

// Цвета для консольного вывода
const colors = {
    reset: '\x1b[0m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    red: '\x1b[31m',
    cyan: '\x1b[36m',
};

function log(message, color = colors.reset) {
    console.log(`${color}${message}${colors.reset}`);
}

// Определяем платформу
const isWindows = process.platform === 'win32';
const EXE_EXT = isWindows ? '.exe' : '';

function envFlag(name, defaultValue = false) {
    const raw = process.env[name];
    if (raw == null) {
        return defaultValue;
    }
    const normalized = String(raw).trim().toLowerCase();
    return normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on';
}

// Пути
const PROJECT_ROOT = path.resolve(__dirname, '../..');
const EXTENSION_BIN_DIR = path.join(__dirname, '..', 'bin');
function parseProfileArg() {
    const profileFlag = process.argv.find(arg => arg.startsWith('--profile='));
    if (profileFlag) {
        const value = profileFlag.split('=')[1];
        if (value === 'debug' || value === 'release') {
            return value;
        }
        log(`❌ Некорректный профиль в аргументе ${profileFlag}. Ожидается --profile=debug|release`, colors.red);
        process.exit(1);
    }

    const profileIndex = process.argv.indexOf('--profile');
    if (profileIndex !== -1) {
        const value = process.argv[profileIndex + 1];
        if (value === 'debug' || value === 'release') {
            return value;
        }
        log('❌ Некорректный профиль после --profile. Ожидается debug или release', colors.red);
        process.exit(1);
    }

    return null;
}

const profileArg = parseProfileArg();
const envProfile = String(process.env.BSL_COPY_BINARIES_PROFILE || '').trim().toLowerCase();
const TARGET_PROFILE = profileArg || (envProfile === 'debug' ? 'debug' : 'release');
const TARGET_PROFILE_DIR = path.join(PROJECT_ROOT, 'target', TARGET_PROFILE);
const CACHE_DIR = path.resolve(__dirname, '..', '.cache');
const CACHE_PATH = path.join(CACHE_DIR, `rust-binaries-${TARGET_PROFILE}.json`);

// Конфигурация бинарников (расширение добавляется динамически)
const BINARIES = [
    {
        name: 'LSP Server',
        source: `bsl-lsp-server${EXE_EXT}`,
        target: `lsp-server${EXE_EXT}`,
        description: 'Language Server Protocol server для VSCode'
    }
];

/**
 * Проверяет существование файла
 */
function fileExists(filePath) {
    try {
        return fs.existsSync(filePath);
    } catch {
        return false;
    }
}

/**
 * Получает время последней модификации файла
 */
function getModTime(filePath) {
    try {
        return fs.statSync(filePath).mtime.getTime();
    } catch {
        return 0;
    }
}

/**
 * Получает размер файла в MB
 */
function getFileSizeMB(filePath) {
    try {
        const stats = fs.statSync(filePath);
        return (stats.size / (1024 * 1024)).toFixed(2);
    } catch {
        return 0;
    }
}

/**
 * Проверяет актуальность бинарника
 */
function isBinaryUpToDate(sourcePath, targetPath) {
    if (!fileExists(targetPath)) {
        return false;
    }

    const sourceTime = getModTime(sourcePath);
    const targetTime = getModTime(targetPath);

    return targetTime >= sourceTime;
}

/**
 * Копирует файл с проверкой
 */
function copyBinary(sourcePath, targetPath, binaryName) {
    try {
        fs.copyFileSync(sourcePath, targetPath);
        const size = getFileSizeMB(targetPath);
        log(`  ✅ Скопирован ${binaryName} (${size} MB)`, colors.green);
        return true;
    } catch (error) {
        log(`  ❌ Ошибка копирования ${binaryName}: ${error.message}`, colors.red);
        return false;
    }
}

/**
 * Собирает Rust бинарники в release режиме
 */
function buildRustBinaries(force = false) {
    log('\n🔨 Проверка Rust бинарников...', colors.cyan);

    ensureDir(CACHE_DIR);

    // ВАЖНО: просто наличие target/release недостаточно — бинарник может быть устаревшим после правок.
    // Cargo сам решит, нужно ли пересобирать (по fingerprint'ам), поэтому безопасно вызывать build всегда.
    //
    // NOTE: бинарник `bsl-lsp-server` находится в пакете `bsl-backend`, поэтому указываем `-p bsl-backend`.
    const profileArg = TARGET_PROFILE === 'release' ? '--release' : '';
    const buildCmd = `cargo build -p bsl-backend ${profileArg} --bin bsl-lsp-server`.replace(/\s+/g, ' ').trim();

    const sourceBinaryPath = path.join(TARGET_PROFILE_DIR, `bsl-lsp-server${EXE_EXT}`);

    function shouldIncludeRustInput(relPath) {
        const normalized = relPath.replace(/\\/g, '/');

        if (normalized.includes('/target/')) return false;
        if (normalized.includes('/node_modules/')) return false;
        if (normalized.includes('/dist/')) return false;
        if (normalized.includes('/.git/')) return false;
        if (normalized.includes('/.bsl_cache/')) return false;

        // Tests/benches don't affect release bin (and change often).
        if (normalized.includes('/tests/')) return false;
        if (normalized.includes('/benches/')) return false;

        const base = path.posix.basename(normalized);
        if (base === 'Cargo.toml') return true;
        if (base === 'Cargo.lock') return true;
        if (base === 'build.rs') return true;

        if (normalized.endsWith('.rs')) return true;
        if (normalized.endsWith('.c')) return true;
        if (normalized.endsWith('.h')) return true;

        return false;
    }

    function getInputsSummary() {
        const excludeDirNames = new Set(['target', 'node_modules', 'dist', '.git', '.bsl_cache']);
        return summarizePaths(
            [
                path.join(PROJECT_ROOT, 'Cargo.toml'),
                path.join(PROJECT_ROOT, 'Cargo.lock'),
                path.join(PROJECT_ROOT, 'build.rs'),
                path.join(PROJECT_ROOT, 'backend'),
                path.join(PROJECT_ROOT, 'shared'),
                path.join(PROJECT_ROOT, 'line-index'),
                path.join(PROJECT_ROOT, 'syntax'),
                path.join(PROJECT_ROOT, 'semantic'),
                path.join(PROJECT_ROOT, 'semantic-diagnostics'),
                path.join(PROJECT_ROOT, 'analysis-v2'),
                path.join(PROJECT_ROOT, 'type-visualization'),
                path.join(PROJECT_ROOT, 'third_party', 'tree-sitter-bsl'),
            ],
            {
                projectRoot: PROJECT_ROOT,
                excludeDirNames,
                includeFile: shouldIncludeRustInput,
            }
        );
    }

    const inputsSummary = getInputsSummary();
    const cache = readJsonIfExists(CACHE_PATH);

    if (!force && fileExists(sourceBinaryPath)) {
        const sourceTime = getModTime(sourceBinaryPath);
        const outputsNewerThanInputs = sourceTime >= inputsSummary.maxMtimeMs;

        const fingerprintMatches = cache && cache.fingerprint === inputsSummary.fingerprint;
        const canSkipBuild = outputsNewerThanInputs && (fingerprintMatches || !cache);

        if (canSkipBuild) {
            log('✅ Rust бинарник актуален, сборка пропущена', colors.green);
            writeJson(CACHE_PATH, {
                fingerprint: inputsSummary.fingerprint,
                fileCount: inputsSummary.fileCount,
                inputsMaxMtimeMs: inputsSummary.maxMtimeMs,
                sourceBinaryMtimeMs: sourceTime,
                buildCmd,
                updatedAt: new Date().toISOString(),
            });
            return;
        }
    }

    if (force) {
        log(`⚡ Принудительная пересборка: ${buildCmd}`, colors.yellow);
    } else {
        log(`📦 Проверка актуальности: ${buildCmd}`, colors.cyan);
    }

    try {
        execSync(buildCmd, {
            cwd: PROJECT_ROOT,
            stdio: 'inherit'
        });
        log('✅ Rust бинарники актуальны', colors.green);

        if (fileExists(sourceBinaryPath)) {
            writeJson(CACHE_PATH, {
                fingerprint: inputsSummary.fingerprint,
                fileCount: inputsSummary.fileCount,
                inputsMaxMtimeMs: inputsSummary.maxMtimeMs,
                sourceBinaryMtimeMs: getModTime(sourceBinaryPath),
                buildCmd,
                builtAt: new Date().toISOString(),
            });
        }
    } catch (error) {
        log(`❌ Ошибка сборки: ${error.message}`, colors.red);
        process.exit(1);
    }
}

/**
 * Основная функция
 */
function main() {
    log('🚀 BSL Gradual Types - Синхронизация бинарников', colors.cyan);
    log('=' .repeat(60), colors.cyan);
    log(`🖥️  Платформа: ${process.platform} (${isWindows ? 'Windows' : 'Linux/macOS'})`, colors.cyan);
    log(`🎯 Профиль бинарников: ${TARGET_PROFILE}`, colors.cyan);

    // Создаём директорию bin если не существует
    if (!fileExists(EXTENSION_BIN_DIR)) {
        fs.mkdirSync(EXTENSION_BIN_DIR, { recursive: true });
        log(`📁 Создана директория: ${EXTENSION_BIN_DIR}`, colors.yellow);
    }

    // Проверяем флаг --force
    const forceRebuild = process.argv.includes('--force');
    const skipRustBuild = process.argv.includes('--skip-rust-build') || envFlag('BSL_SKIP_RUST_BUILD', false);
    if (forceRebuild) {
        log('⚡ Режим принудительной пересборки включён', colors.yellow);
    }
    if (skipRustBuild) {
        log('⏭️  Режим skip-rust-build: Rust сборка пропущена', colors.yellow);
    }

    // Собираем бинарники если нужно
    if (!skipRustBuild) {
        buildRustBinaries(forceRebuild);
    }

    log('\n📋 Копирование бинарников:', colors.cyan);

    let copiedCount = 0;
    let skippedCount = 0;
    let errorCount = 0;

    for (const binary of BINARIES) {
        const sourcePath = path.join(TARGET_PROFILE_DIR, binary.source);
        const targetPath = path.join(EXTENSION_BIN_DIR, binary.target);

        log(`\n🔍 ${binary.name}:`, colors.cyan);
        log(`  Источник: ${binary.source}`);
        log(`  Назначение: ${binary.target}`);

        // Проверяем существование источника
        if (!fileExists(sourcePath)) {
            log(`  ❌ Источник не найден: ${sourcePath}`, colors.red);
            errorCount++;
            continue;
        }

        // Проверяем актуальность
        if (!forceRebuild && isBinaryUpToDate(sourcePath, targetPath)) {
            log(`  ⏭️  Пропущен (актуальная версия)`, colors.yellow);
            skippedCount++;
            continue;
        }

        // Копируем
        if (copyBinary(sourcePath, targetPath, binary.name)) {
            copiedCount++;
        } else {
            errorCount++;
        }
    }

    // Итоговая статистика
    log('\n' + '='.repeat(60), colors.cyan);
    log('📊 Статистика:', colors.cyan);
    log(`  ✅ Скопировано: ${copiedCount}`, colors.green);
    log(`  ⏭️  Пропущено: ${skippedCount}`, colors.yellow);
    if (errorCount > 0) {
        log(`  ❌ Ошибок: ${errorCount}`, colors.red);
    }

    // Показываем содержимое bin/
    log('\n📂 Содержимое vscode-extension/bin/:', colors.cyan);
    const binFiles = fs.readdirSync(EXTENSION_BIN_DIR);
    binFiles.forEach(file => {
        const filePath = path.join(EXTENSION_BIN_DIR, file);
        const size = getFileSizeMB(filePath);
        const stats = fs.statSync(filePath);
        const modTime = stats.mtime.toLocaleString('ru-RU');
        log(`  📄 ${file} (${size} MB, изменён: ${modTime})`);
    });

    log('\n' + '='.repeat(60), colors.cyan);

    if (errorCount > 0) {
        log('⚠️  Завершено с ошибками', colors.red);
        process.exit(1);
    } else {
        log('✅ Все бинарники актуальны!', colors.green);
        process.exit(0);
    }
}

// Запуск
main();
