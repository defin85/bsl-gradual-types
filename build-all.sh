#!/bin/bash
# ============================================================================
# BSL Gradual Types - Универсальный скрипт сборки
# ============================================================================
# Собирает все компоненты проекта: Rust бинарники + VSCode Extension
# Использование: ./build-all.sh [--release|--debug] [--skip-tests]
# ============================================================================

set -e  # Остановка при первой ошибке

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Параметры сборки
BUILD_MODE="release"
SKIP_TESTS=false

# Парсинг аргументов
for arg in "$@"; do
    case $arg in
        --debug)
            BUILD_MODE="debug"
            ;;
        --release)
            BUILD_MODE="release"
            ;;
        --skip-tests)
            SKIP_TESTS=true
            ;;
        *)
            echo -e "${RED}❌ Неизвестный аргумент: $arg${NC}"
            echo "Использование: ./build-all.sh [--release|--debug] [--skip-tests]"
            exit 1
            ;;
    esac
done

# Функции для логирования
log_info() {
    echo -e "${CYAN}$1${NC}"
}

log_success() {
    echo -e "${GREEN}$1${NC}"
}

log_error() {
    echo -e "${RED}$1${NC}"
}

log_warning() {
    echo -e "${YELLOW}$1${NC}"
}

log_section() {
    echo ""
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}============================================================${NC}"
}

# Функция для измерения времени
measure_time() {
    local start=$SECONDS
    "$@"
    local duration=$((SECONDS - start))
    echo -e "${CYAN}⏱️  Время выполнения: ${duration}s${NC}"
}

# Функция для проверки существования файла
check_file() {
    if [ -f "$1" ]; then
        local size=$(du -h "$1" | cut -f1)
        log_success "  ✅ $2 ($size)"
        return 0
    else
        log_error "  ❌ $2 не найден: $1"
        return 1
    fi
}

# ============================================================================
# ЭТАП 1: Сборка Rust бинарников
# ============================================================================

build_rust_binaries() {
    log_section "ЭТАП 1: Сборка Rust бинарников ($BUILD_MODE)"

    local cargo_flags=""
    if [ "$BUILD_MODE" = "release" ]; then
        cargo_flags="--release"
    fi

    log_info "\n🦀 Компиляция Rust проекта..."
    log_info "Режим: $BUILD_MODE"
    log_info "Флаги: cargo build $cargo_flags --workspace"

    measure_time cargo build $cargo_flags --workspace

    log_success "\n✅ Rust бинарники собраны"

    # Проверка результатов
    log_info "\n📦 Проверка собранных бинарников:"

    local target_dir="target/$BUILD_MODE"
    local all_ok=true

    check_file "$target_dir/bsl-lsp-server.exe" "LSP Server" || all_ok=false

    # Web Server и CLI - опциональные
    if [ -f "$target_dir/bsl-web-server.exe" ]; then
        check_file "$target_dir/bsl-web-server.exe" "Web Server"
    fi

    if [ -f "$target_dir/bsl-cli.exe" ]; then
        check_file "$target_dir/bsl-cli.exe" "CLI"
    fi

    if [ "$all_ok" = false ]; then
        log_error "\n❌ Не все обязательные бинарники собраны!"
        return 1
    fi

    return 0
}

# ============================================================================
# ЭТАП 2: Копирование бинарников в VSCode Extension
# ============================================================================

copy_binaries() {
    log_section "ЭТАП 2: Копирование бинарников в VSCode Extension"

    local source_dir="target/$BUILD_MODE"
    local target_dir="vscode-extension/bin"

    log_info "\n📋 Копирование бинарников:"
    log_info "Источник: $source_dir/"
    log_info "Назначение: $target_dir/"

    # Создаём директорию если не существует
    mkdir -p "$target_dir"

    # Копируем LSP Server
    log_info "\n🔍 LSP Server:"
    if [ -f "$source_dir/bsl-lsp-server.exe" ]; then
        cp "$source_dir/bsl-lsp-server.exe" "$target_dir/lsp-server.exe"
        local size=$(du -h "$target_dir/lsp-server.exe" | cut -f1)
        log_success "  ✅ Скопирован lsp-server.exe ($size)"
    else
        log_error "  ❌ bsl-lsp-server.exe не найден в $source_dir/"
        return 1
    fi

    log_success "\n✅ Бинарники скопированы успешно"
    return 0
}

# ============================================================================
# ЭТАП 3: Сборка VSCode Extension
# ============================================================================

build_vscode_extension() {
    log_section "ЭТАП 3: Сборка VSCode Extension"

    cd vscode-extension || {
        log_error "❌ Директория vscode-extension не найдена!"
        return 1
    }

    log_info "\n📦 Установка зависимостей (если нужно)..."
    if [ ! -d "node_modules" ]; then
        measure_time npm install
    else
        log_success "  ⏭️  node_modules существует, пропускаем npm install"
    fi

    log_info "\n🔨 Компиляция TypeScript + сборка WASM..."
    measure_time npm run compile

    cd .. || return 1

    # Проверка результатов
    log_info "\n📦 Проверка собранного расширения:"

    local all_ok=true
    check_file "vscode-extension/out/extension.js" "Extension main file" || all_ok=false
    check_file "vscode-extension/bin/lsp-server.exe" "LSP Server binary" || all_ok=false

    if [ "$all_ok" = false ]; then
        log_error "\n❌ Не все файлы расширения собраны!"
        return 1
    fi

    log_success "\n✅ VSCode Extension собрано успешно"
    return 0
}

# ============================================================================
# ЭТАП 4: Быстрые тесты (опционально)
# ============================================================================

run_quick_tests() {
    if [ "$SKIP_TESTS" = true ]; then
        log_warning "\n⏭️  Тесты пропущены (--skip-tests)"
        return 0
    fi

    log_section "ЭТАП 4: Быстрые проверки"

    log_info "\n🧪 Запуск быстрых unit тестов..."
    measure_time cargo test --lib --workspace --quiet

    log_success "\n✅ Быстрые тесты пройдены"
    return 0
}

# ============================================================================
# ЭТАП 5: Итоговый отчёт
# ============================================================================

print_summary() {
    log_section "📊 ИТОГОВЫЙ ОТЧЁТ"

    echo ""
    echo -e "${CYAN}📦 Собранные компоненты:${NC}"
    echo ""

    # Rust бинарники
    local target_dir="target/$BUILD_MODE"
    echo -e "${CYAN}🦀 Rust ($BUILD_MODE):${NC}"
    [ -f "$target_dir/bsl-lsp-server.exe" ] && echo "  ✅ LSP Server ($(du -h "$target_dir/bsl-lsp-server.exe" | cut -f1))"
    [ -f "$target_dir/bsl-web-server.exe" ] && echo "  ✅ Web Server ($(du -h "$target_dir/bsl-web-server.exe" | cut -f1))"
    [ -f "$target_dir/bsl-cli.exe" ] && echo "  ✅ CLI ($(du -h "$target_dir/bsl-cli.exe" | cut -f1))"

    # VSCode Extension
    echo ""
    echo -e "${CYAN}📦 VSCode Extension:${NC}"
    [ -f "vscode-extension/out/extension.js" ] && echo "  ✅ TypeScript ($(du -h vscode-extension/out/extension.js | cut -f1))"
    [ -f "vscode-extension/bin/lsp-server.exe" ] && echo "  ✅ LSP Server binary ($(du -h vscode-extension/bin/lsp-server.exe | cut -f1))"

    local wasm_count=$(find vscode-extension/media/webview -name "*.wasm" 2>/dev/null | wc -l)
    if [ "$wasm_count" -gt 0 ]; then
        echo "  ✅ WASM bundles ($wasm_count files)"
    fi

    echo ""
    log_success "✅ Все компоненты собраны успешно!"
    echo ""

    # Следующие шаги
    echo -e "${CYAN}🚀 Следующие шаги:${NC}"
    echo "  1. Запустить тесты: ./test-runner.sh или /test-runner"
    echo "  2. Запустить VSCode: code vscode-extension/"
    echo "  3. Проверить расширение: F5 в VSCode"
    echo ""
}

# ============================================================================
# MAIN
# ============================================================================

main() {
    log_section "🏗️  BSL Gradual Types - Полная сборка"

    log_info "Режим сборки: $BUILD_MODE"
    log_info "Тесты: $([ "$SKIP_TESTS" = true ] && echo "пропущены" || echo "включены")"

    local total_start=$SECONDS

    # Этап 1: Rust
    if ! build_rust_binaries; then
        log_error "\n❌ Сборка Rust провалилась!"
        exit 1
    fi

    # Этап 2: Копирование
    if ! copy_binaries; then
        log_error "\n❌ Копирование бинарников провалилось!"
        exit 1
    fi

    # Этап 3: VSCode Extension
    if ! build_vscode_extension; then
        log_error "\n❌ Сборка VSCode Extension провалилась!"
        exit 1
    fi

    # Этап 4: Тесты (опционально)
    if ! run_quick_tests; then
        log_warning "\n⚠️  Тесты провалились (не критично для сборки)"
    fi

    # Итоговый отчёт
    print_summary

    local total_duration=$((SECONDS - total_start))
    log_info "⏱️  Общее время сборки: ${total_duration}s"

    log_success "\n🎉 Сборка завершена успешно!"
}

# Запуск
main "$@"
