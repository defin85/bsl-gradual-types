#!/bin/bash
# ============================================================================
# BSL Gradual Types - Универсальный кросс-платформенный скрипт сборки
# ============================================================================
# Поддерживает: Linux, macOS, Windows (Git Bash, MSYS2, Cygwin, WSL)
# Использование: ./scripts/build-all.sh [--release|--debug] [--skip-tests]
#   Дополнительно:
#     --no-auto-version            не менять версии в package.json/Cargo.toml
#     --force-build-timestamp      принудительно обновлять BUILD_TIMESTAMP (вызывает перекомпиляцию)
#     --clean-target               cargo clean перед сборкой (удалит весь target/)
#     --prune-target-days N        удалить старые артефакты из target/debug (mtime > N дней)
#     --tests <quick|smoke|full>   режим тестов:
#                                - quick: debug + subset lib-тестов (по умолчанию)
#                                - smoke: ./scripts/run-intellisense-tests.sh smoke
#                                - full: --release + все workspace тесты
#     --nextest                    использовать cargo-nextest (если установлен)
#     --no-nextest                 не использовать cargo-nextest (всегда cargo test)
#     (по умолчанию выполняется cargo clean раз в 2 дня)
# ============================================================================

set -e  # Остановка при первой ошибке

# Переходим в корень проекта (скрипт находится в scripts/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

# ============================================================================
# Определение платформы
# ============================================================================

detect_platform() {
    case "$(uname -s)" in
        Linux*)
            if grep -qi microsoft /proc/version 2>/dev/null; then
                PLATFORM="wsl"
            else
                PLATFORM="linux"
            fi
            BINARY_EXT=""
            ;;
        Darwin*)
            PLATFORM="macos"
            BINARY_EXT=""
            ;;
        CYGWIN*|MINGW*|MSYS*)
            PLATFORM="windows"
            BINARY_EXT=".exe"
            ;;
        *)
            # Fallback: проверяем наличие Windows-специфичных переменных
            if [[ -n "$WINDIR" ]] || [[ -n "$SYSTEMROOT" ]]; then
                PLATFORM="windows"
                BINARY_EXT=".exe"
            else
                PLATFORM="unknown"
                BINARY_EXT=""
            fi
            ;;
    esac
}

detect_platform

# ============================================================================
# Цвета для вывода
# ============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# ============================================================================
# Параметры сборки
# ============================================================================

BUILD_MODE="release"
SKIP_TESTS=false
AUTO_VERSION=true
FORCE_BUILD_TIMESTAMP=false
CLEAN_TARGET=false
PRUNE_TARGET_DAYS=""
AUTO_CLEAN_DAYS=2
CLEAN_TARGET_REASON="manual"
USE_NEXTEST="auto" # auto|true|false
TEST_SUITE="quick" # quick|smoke|full

# Если скрипт запущен без аргументов в WSL — включаем мягкую уборку по умолчанию,
# чтобы target/ не раздувался бесконтрольно.
ORIGINAL_ARGC=$#
DEFAULT_WSL_PRUNE_DAYS=14
AUTO_CLEAN_MARKER=""

# Парсинг аргументов
while [[ $# -gt 0 ]]; do
    arg="$1"
    case "$arg" in
        --debug)
            BUILD_MODE="debug"
            ;;
        --release)
            BUILD_MODE="release"
            ;;
        --skip-tests)
            SKIP_TESTS=true
            ;;
        --no-auto-version)
            AUTO_VERSION=false
            ;;
        --force-build-timestamp)
            FORCE_BUILD_TIMESTAMP=true
            ;;
        --clean-target)
            CLEAN_TARGET=true
            ;;
        --prune-target-days)
            shift
            PRUNE_TARGET_DAYS="${1:-}"
            ;;
        --tests)
            shift
            TEST_SUITE="${1:-}"
            ;;
        --nextest)
            USE_NEXTEST="true"
            ;;
        --no-nextest)
            USE_NEXTEST="false"
            ;;
        *)
            echo -e "${RED}❌ Неизвестный аргумент: $arg${NC}"
            echo "Использование: ./build-all.sh [--release|--debug] [--skip-tests] [--no-auto-version] [--force-build-timestamp] [--clean-target] [--prune-target-days N] [--tests quick|smoke|full] [--nextest|--no-nextest]"
            exit 1
            ;;
    esac
    shift
done

# Политика по умолчанию: если аргументов не было, мы в WSL и пользователь явно не просил clean —
# прореживаем старые debug-артефакты.
if [ "$ORIGINAL_ARGC" -eq 0 ] && [ "$PLATFORM" = "wsl" ] && [ "$CLEAN_TARGET" = false ] && [ -z "$PRUNE_TARGET_DAYS" ]; then
    PRUNE_TARGET_DAYS="$DEFAULT_WSL_PRUNE_DAYS"
fi

# ============================================================================
# Автоверсионирование
# ============================================================================

# Получить текущую версию из package.json
get_current_version() {
    grep '"version"' vscode-extension/package.json | head -1 | sed 's/.*"version": *"\([^"]*\)".*/\1/'
}

# Инкрементировать patch версию (0.4.2 -> 0.4.3)
increment_patch_version() {
    local version="$1"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$version"
    patch=$((patch + 1))
    echo "$major.$minor.$patch"
}

# Обновить версию во всех файлах
update_version_in_files() {
    local old_version="$1"
    local new_version="$2"

    echo -e "${CYAN}  📝 Обновление версии: $old_version -> $new_version${NC}" >&2

    # package.json
    sed -i "s/\"version\": \"$old_version\"/\"version\": \"$new_version\"/" vscode-extension/package.json

    # Cargo.toml (workspace)
    sed -i "s/^version = \"$old_version\"/version = \"$new_version\"/" Cargo.toml
}

# Автоверсионирование при наличии изменений
auto_version() {
    echo -e "${CYAN}\n🔢 Проверка версии...${NC}" >&2

    local current_version=$(get_current_version)
    echo -e "${CYAN}  Текущая версия: $current_version${NC}" >&2

    # Проверяем есть ли изменения в tracked файлах (исключая untracked)
    local changes=$(git diff --name-only 2>/dev/null | wc -l)
    local staged=$(git diff --cached --name-only 2>/dev/null | wc -l)

    if [ "$changes" -gt 0 ] || [ "$staged" -gt 0 ]; then
        local new_version=$(increment_patch_version "$current_version")
        echo -e "${YELLOW}  ⚠️  Обнаружены изменения ($changes файлов modified, $staged staged)${NC}" >&2
        update_version_in_files "$current_version" "$new_version"
        echo -e "${GREEN}  ✅ Версия обновлена: $new_version${NC}" >&2
        echo "$new_version"
    else
        echo -e "${GREEN}  ✅ Изменений нет, версия актуальна: $current_version${NC}" >&2
        echo "$current_version"
    fi
}

# ============================================================================
# Функции для логирования
# ============================================================================

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

# ============================================================================
# Авто-очистка target раз в N дней
# ============================================================================

AUTO_CLEAN_MARKER="$PROJECT_ROOT/.build-all.last-clean"
AUTO_CLEAN_INTERVAL_SECONDS=$((AUTO_CLEAN_DAYS * 24 * 60 * 60))

file_mtime_epoch() {
    local file="$1"
    local ts=""

    if ts=$(stat -c %Y "$file" 2>/dev/null); then
        echo "$ts"
        return 0
    fi

    if ts=$(stat -f %m "$file" 2>/dev/null); then
        echo "$ts"
        return 0
    fi

    echo 0
}

auto_clean_target_if_needed() {
    if [ "$CLEAN_TARGET" = true ]; then
        return 0
    fi

    local now
    now=$(date +%s)

    local last_clean=0
    if [ -f "$AUTO_CLEAN_MARKER" ]; then
        last_clean=$(file_mtime_epoch "$AUTO_CLEAN_MARKER")
    fi

    if [ "$last_clean" -eq 0 ] || [ $((now - last_clean)) -ge "$AUTO_CLEAN_INTERVAL_SECONDS" ]; then
        CLEAN_TARGET=true
        CLEAN_TARGET_REASON="auto"
    fi
}

# Проверка состояния upstream
check_git_upstream_ahead() {
    log_info "\n🔍 Проверка коммитов относительно @{u}..."

    if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        log_warning "⚠️  Git репозиторий не найден, проверка @{u} пропущена"
        return 0
    fi

    if ! git rev-parse --abbrev-ref --symbolic-full-name @{u} >/dev/null 2>&1; then
        log_warning "⚠️  Upstream для текущей ветки не настроен, проверка @{u} пропущена"
        return 0
    fi

    local ahead
    ahead=$(git rev-list --count @{u}..HEAD 2>/dev/null || true)

    if [ -z "$ahead" ]; then
        log_warning "⚠️  Не удалось определить количество коммитов впереди @{u}"
        return 0
    fi

    if [ "$ahead" -gt 0 ]; then
        log_warning "⚠️  Локальная ветка опережает @{u} на $ahead коммит(ов)"
    else
        log_success "✅ Локальная ветка синхронизирована с @{u}"
    fi
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
# ЭТАП 0: Очистка target (опционально)
# ============================================================================

clean_or_prune_target() {
    auto_clean_target_if_needed

    if [ "$CLEAN_TARGET" = true ]; then
        log_section "ЭТАП 0: Очистка target/ (cargo clean)"
        if [ "$CLEAN_TARGET_REASON" = "auto" ]; then
            log_warning "⚠️  Авто-очистка: прошло ${AUTO_CLEAN_DAYS} дней с последнего cargo clean"
        else
            log_warning "⚠️  Выполняется cargo clean — удалится весь target/ (сборка начнётся с нуля)"
        fi
        measure_time cargo clean
        touch "$AUTO_CLEAN_MARKER"
        return 0
    fi

    if [ -n "$PRUNE_TARGET_DAYS" ]; then
        if ! [[ "$PRUNE_TARGET_DAYS" =~ ^[0-9]+$ ]]; then
            log_error "❌ Некорректное значение --prune-target-days: '$PRUNE_TARGET_DAYS' (нужно число дней)"
            return 1
        fi

        log_section "ЭТАП 0: Прореживание target/debug (mtime > ${PRUNE_TARGET_DAYS} дней)"
        log_warning "⚠️  Удаляются старые артефакты; следующая сборка может пересобрать зависимости"

        if [ -d "target" ]; then
            # Удаляем только очевидные heavy dirs; остальные пусть остаются.
            find target \
                -type f \
                \( -path "*/debug/deps/*" -o -path "*/debug/incremental/*" \) \
                -mtime +"$PRUNE_TARGET_DAYS" \
                -print -delete 2>/dev/null || true

            find target \
                -type d \
                \( -path "*/debug/deps/*" -o -path "*/debug/incremental/*" \) \
                -empty \
                -print -delete 2>/dev/null || true
        fi
    fi

    return 0
}

# ============================================================================
# ЭТАП 1: Сборка Rust бинарников
# ============================================================================

build_frontend_static() {
    log_section "ЭТАП 0.5: Сборка Web UI (target/site)"

    if ! command -v trunk >/dev/null 2>&1; then
        log_error "❌ trunk не найден в PATH. Установите trunk: cargo install trunk"
        return 1
    fi

    log_info "\n🌐 Сборка фронтенда (trunk) в target/site ..."
    log_info "Команда: (cd frontend && NO_COLOR=true trunk build --release)"

    (
        cd frontend
        NO_COLOR=true trunk build --release
    )

    log_info "\n📦 Проверка target/site:"
    check_file "target/site/index.html" "Web UI (index.html)" || return 1
    return 0
}

build_rust_binaries() {
    log_section "ЭТАП 1: Сборка Rust бинарников ($BUILD_MODE)"

    local cargo_flags=""
    if [ "$BUILD_MODE" = "release" ]; then
        cargo_flags="--release"
    fi

    log_info "\n🦀 Компиляция Rust проекта..."
    log_info "Режим: $BUILD_MODE"
    log_info "Флаги: cargo build $cargo_flags --workspace"

    # ВНИМАНИЕ: принудительное обновление BUILD_TIMESTAMP ломает повторное использование
    # артефактов и быстро раздувает target/debug/deps из-за новых hash-имен.
    if [ "$FORCE_BUILD_TIMESTAMP" = true ]; then
        log_warning "⚠️  Включён --force-build-timestamp: будет выполнен touch backend/build.rs (может сильно раздувать target/)"
        touch backend/build.rs
    fi

    measure_time cargo build $cargo_flags --workspace

    # Для MCP-конфига удобно иметь стабильный путь: target/release/bsl-agent
    # (даже если общий билд был в debug).
    log_info "\n🤖 Сборка bsl-agent для MCP (release)..."
    measure_time cargo build --release -p bsl-agent

    log_success "\n✅ Rust бинарники собраны"

    # Проверка результатов
    log_info "\n📦 Проверка собранных бинарников:"

    local target_dir="target/$BUILD_MODE"
    local all_ok=true

    check_file "$target_dir/bsl-lsp-server${BINARY_EXT}" "LSP Server (bsl-lsp-server${BINARY_EXT})" || all_ok=false
    check_file "target/release/bsl-agent${BINARY_EXT}" "MCP Server (bsl-agent${BINARY_EXT})" || all_ok=false

    # Web Server и CLI - опциональные
    if [ -f "$target_dir/bsl-web-server${BINARY_EXT}" ]; then
        check_file "$target_dir/bsl-web-server${BINARY_EXT}" "Web Server (bsl-web-server${BINARY_EXT})"
    fi

    if [ -f "$target_dir/bsl-cli${BINARY_EXT}" ]; then
        check_file "$target_dir/bsl-cli${BINARY_EXT}" "CLI (bsl-cli${BINARY_EXT})"
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
    local src_file="$source_dir/bsl-lsp-server${BINARY_EXT}"
    local dst_file="$target_dir/lsp-server${BINARY_EXT}"

    log_info "\n📋 Копирование бинарников:"
    log_info "  Источник: $src_file"
    log_info "  Назначение: $dst_file"

    # Создаём директорию если не существует
    mkdir -p "$target_dir"

    # Копируем LSP Server
    log_info "\n🔍 LSP Server:"
    if [ -f "$src_file" ]; then
        cp "$src_file" "$dst_file"
        local size=$(du -h "$dst_file" | cut -f1)
        log_success "  ✅ Скопирован ($size)"
    else
        log_error "  ❌ Файл не найден: $src_file"
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
    check_file "vscode-extension/bin/lsp-server${BINARY_EXT}" "LSP Server binary (lsp-server${BINARY_EXT})" || all_ok=false

    if [ "$all_ok" = false ]; then
        log_error "\n❌ Не все файлы расширения собраны!"
        return 1
    fi

    log_success "\n✅ VSCode Extension собрано успешно"
    return 0
}

# ============================================================================
# ЭТАП 4: Тесты (опционально)
# ============================================================================

has_cargo_nextest() {
    command -v cargo-nextest >/dev/null 2>&1
}

run_tests() {
    if [ "$SKIP_TESTS" = true ]; then
        log_warning "\n⏭️  Тесты пропущены (--skip-tests)"
        return 0
    fi

    case "$TEST_SUITE" in
        quick|smoke|full)
            ;;
        *)
            log_error "❌ Неизвестный режим тестов: $TEST_SUITE (ожидалось: quick|smoke|full)"
            return 1
            ;;
    esac

    local use_nextest="$USE_NEXTEST"
    if [ "$use_nextest" = "auto" ]; then
        if has_cargo_nextest; then
            use_nextest="true"
        else
            use_nextest="false"
        fi
    fi

    if [ "$use_nextest" = "true" ] && ! has_cargo_nextest; then
        log_error "❌ Запрошен --nextest, но cargo-nextest не найден в PATH"
        log_info "Установка: cargo install cargo-nextest --locked"
        return 1
    fi

    if [ "$TEST_SUITE" = "quick" ]; then
        log_section "ЭТАП 4: Быстрые проверки (debug + subset)"
        log_info "\n🧪 Запуск быстрых unit тестов (debug)..."

        # Quick должен быть действительно быстрым:
        # - debug (без --release),
        # - subset пакетов,
        # - только lib-тесты.
        local pkgs=(
            "-p" "bsl-line-index"
            "-p" "bsl-shared"
            "-p" "bsl-syntax"
            "-p" "bsl-diagnostics"
            "-p" "bsl-analysis-v2"
        )

        if [ "$use_nextest" = "true" ]; then
            log_info "  ⚡ Используем cargo-nextest (для отключения: --no-nextest)"
            measure_time cargo nextest run "${pkgs[@]}" --lib
        else
            measure_time cargo test "${pkgs[@]}" --lib --quiet
        fi

        log_success "\n✅ Быстрые тесты пройдены"
        return 0
    fi

    if [ "$TEST_SUITE" = "smoke" ]; then
        log_section "ЭТАП 4: Smoke проверки (IntelliSense)"
        log_info "\n🧪 Запуск smoke набора IntelliSense..."
        measure_time ./scripts/run-intellisense-tests.sh smoke
        log_success "\n✅ Smoke тесты пройдены"
        return 0
    fi

    log_section "ЭТАП 4: Полный прогон тестов (--release)"
    log_info "\n🧪 Запуск полного набора тестов (release)..."
    if [ "$use_nextest" = "true" ]; then
        log_info "  ⚡ Используем cargo-nextest (для отключения: --no-nextest)"
        measure_time cargo nextest run --release --workspace
    else
        measure_time cargo test --release --workspace --quiet
    fi

    log_success "\n✅ Полные тесты пройдены"
    return 0
}

# ============================================================================
# ЭТАП 5: Итоговый отчёт
# ============================================================================

print_summary() {
    log_section "📊 ИТОГОВЫЙ ОТЧЁТ"

    echo ""
    echo -e "${GREEN}🏷️  Версия: $BUILD_VERSION${NC}"
    echo -e "${CYAN}📦 Собранные компоненты:${NC}"
    echo ""

    # Rust бинарники
    local target_dir="target/$BUILD_MODE"
    echo -e "${CYAN}🦀 Rust ($BUILD_MODE):${NC}"
    [ -f "$target_dir/bsl-lsp-server${BINARY_EXT}" ] && echo "  ✅ LSP Server ($(du -h "$target_dir/bsl-lsp-server${BINARY_EXT}" | cut -f1))"
    [ -f "$target_dir/bsl-web-server${BINARY_EXT}" ] && echo "  ✅ Web Server ($(du -h "$target_dir/bsl-web-server${BINARY_EXT}" | cut -f1))"
    [ -f "$target_dir/bsl-cli${BINARY_EXT}" ] && echo "  ✅ CLI ($(du -h "$target_dir/bsl-cli${BINARY_EXT}" | cut -f1))"

    # VSCode Extension
    echo ""
    echo -e "${CYAN}📦 VSCode Extension:${NC}"
    [ -f "vscode-extension/out/extension.js" ] && echo "  ✅ TypeScript ($(du -h vscode-extension/out/extension.js | cut -f1))"
    [ -f "vscode-extension/bin/lsp-server${BINARY_EXT}" ] && echo "  ✅ LSP Server binary ($(du -h "vscode-extension/bin/lsp-server${BINARY_EXT}" | cut -f1))"

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

	    log_info "Платформа: $PLATFORM"
	    log_info "Расширение бинарников: '${BINARY_EXT:-<нет>}'"
	    log_info "Режим сборки: $BUILD_MODE"
	    if [ "$SKIP_TESTS" = true ]; then
	        log_info "Тесты: пропущены"
	    else
	        log_info "Тесты: $TEST_SUITE (nextest=$USE_NEXTEST)"
	    fi

    check_git_upstream_ahead

    local total_start=$SECONDS

    # Автоверсионирование (опционально)
    if [ "$AUTO_VERSION" = true ]; then
        BUILD_VERSION=$(auto_version)
    else
        BUILD_VERSION=$(get_current_version)
        log_warning "⏭️  Автоверсионирование выключено (--no-auto-version), версия: $BUILD_VERSION"
    fi

    # Этап 0: Очистка/прореживание target (опционально)
    if ! clean_or_prune_target; then
        log_error "\n❌ Очистка target провалилась!"
        exit 1
    fi

    # Этап 0.5: Web UI (нужно перед сборкой bsl-agent, т.к. UI вшивается в бинарник)
    if ! build_frontend_static; then
        log_error "\n❌ Сборка Web UI провалилась!"
        exit 1
    fi

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
	    if ! run_tests; then
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
