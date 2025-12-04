#!/usr/bin/env node
/**
 * Скрипт для копирования актуальных Rust бинарников в директорию расширения
 * Запускается автоматически перед компиляцией и публикацией расширения
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

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

// Пути
const PROJECT_ROOT = path.resolve(__dirname, '../..');
const EXTENSION_BIN_DIR = path.join(__dirname, '..', 'bin');
const TARGET_RELEASE_DIR = path.join(PROJECT_ROOT, 'target', 'release');

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

    // Проверяем существование target/release
    if (!fileExists(TARGET_RELEASE_DIR) || force) {
        log('📦 Сборка Rust бинарников в release режиме...', colors.yellow);
        try {
            execSync('cargo build --release --bin bsl-lsp-server', {
                cwd: PROJECT_ROOT,
                stdio: 'inherit'
            });
            log('✅ Rust бинарники успешно собраны', colors.green);
        } catch (error) {
            log(`❌ Ошибка сборки: ${error.message}`, colors.red);
            process.exit(1);
        }
    } else {
        log('✅ Release бинарники найдены', colors.green);
    }
}

/**
 * Основная функция
 */
function main() {
    log('🚀 BSL Gradual Types - Синхронизация бинарников', colors.cyan);
    log('=' .repeat(60), colors.cyan);
    log(`🖥️  Платформа: ${process.platform} (${isWindows ? 'Windows' : 'Linux/macOS'})`, colors.cyan);

    // Создаём директорию bin если не существует
    if (!fileExists(EXTENSION_BIN_DIR)) {
        fs.mkdirSync(EXTENSION_BIN_DIR, { recursive: true });
        log(`📁 Создана директория: ${EXTENSION_BIN_DIR}`, colors.yellow);
    }

    // Проверяем флаг --force
    const forceRebuild = process.argv.includes('--force');
    if (forceRebuild) {
        log('⚡ Режим принудительной пересборки включён', colors.yellow);
    }

    // Собираем бинарники если нужно
    buildRustBinaries(forceRebuild);

    log('\n📋 Копирование бинарников:', colors.cyan);

    let copiedCount = 0;
    let skippedCount = 0;
    let errorCount = 0;

    for (const binary of BINARIES) {
        const sourcePath = path.join(TARGET_RELEASE_DIR, binary.source);
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
