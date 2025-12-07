#!/bin/bash
# ============================================================================
# BSL Gradual Types - Кросс-платформенный скрипт запуска Web API
# ============================================================================
# Поддерживает: Linux, macOS, Windows (Git Bash, MSYS2, Cygwin, WSL)
# Использование: ./scripts/start-web-api.sh [--no-config] [--build] [--no-frontend]
#
# Флаги:
#   --no-config    Запуск без конфигурации (только platform types)
#   --build        Пересобрать backend и frontend перед запуском
#   --no-frontend  Не собирать и не подключать frontend (только API)
# ============================================================================

set -e

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
                # WSL может работать с Windows путями через /mnt/
                DEFAULT_PROJECT_PATH="examples/conf"
            else
                PLATFORM="linux"
                DEFAULT_PROJECT_PATH="examples/conf"
            fi
            BINARY_EXT=""
            ;;
        Darwin*)
            PLATFORM="macos"
            BINARY_EXT=""
            DEFAULT_PROJECT_PATH=""
            ;;
        CYGWIN*|MINGW*|MSYS*)
            PLATFORM="windows"
            BINARY_EXT=".exe"
            DEFAULT_PROJECT_PATH="/c/1CProject/conf"
            ;;
        *)
            if [[ -n "$WINDIR" ]] || [[ -n "$SYSTEMROOT" ]]; then
                PLATFORM="windows"
                BINARY_EXT=".exe"
                DEFAULT_PROJECT_PATH="/c/1CProject/conf"
            else
                PLATFORM="unknown"
                BINARY_EXT=""
                DEFAULT_PROJECT_PATH=""
            fi
            ;;
    esac
}

detect_platform

# ============================================================================
# Параметры
# ============================================================================

PORT=${PORT:-3002}
SYNTAX_HELPER_PATH="examples/syntax_helper"
PROJECT_PATH="${PROJECT_PATH:-$DEFAULT_PROJECT_PATH}"
DO_BUILD=false
NO_FRONTEND=false
NO_CONFIG=false

# Парсинг аргументов
for arg in "$@"; do
    case $arg in
        --no-config)
            NO_CONFIG=true
            ;;
        --build)
            DO_BUILD=true
            ;;
        --no-frontend)
            NO_FRONTEND=true
            ;;
        --help|-h)
            echo "Использование: ./start-web-api.sh [--no-config] [--build] [--no-frontend]"
            echo ""
            echo "Флаги:"
            echo "  --no-config    Запуск без конфигурации (только platform types)"
            echo "  --build        Пересобрать backend и frontend перед запуском"
            echo "  --no-frontend  Не собирать и не подключать frontend (только API)"
            echo ""
            echo "Переменные окружения:"
            echo "  PORT           Порт сервера (по умолчанию: 3002)"
            echo "  PROJECT_PATH   Путь к конфигурации 1С"
            exit 0
            ;;
    esac
done

# ============================================================================
# Логика запуска
# ============================================================================

echo "============================================"
echo "BSL Web API Server"
echo "============================================"
echo "Платформа: $PLATFORM"
echo "Расширение бинарников: '${BINARY_EXT:-<нет>}'"
echo ""

if [[ "$NO_CONFIG" == "true" ]] || [[ -z "$PROJECT_PATH" ]] || [[ ! -d "$PROJECT_PATH" ]]; then
    echo "🚀 Запуск Web API БЕЗ конфигурации (только platform types)"
    PROJECT_ARG=""
else
    echo "🚀 Запуск Web API С конфигурацией"
    echo "   Конфигурация: $PROJECT_PATH"
    PROJECT_ARG="--project-path $PROJECT_PATH"
fi

# Остановка предыдущего сервера если запущен
pkill -f "bsl-web-server.*--port $PORT" 2>/dev/null || true
sleep 1

# Проверка и сборка backend
BINARY="./target/release/bsl-web-server${BINARY_EXT}"
if [[ ! -f "$BINARY" ]] || [[ "$DO_BUILD" == "true" ]]; then
    echo "📦 Сборка backend (release)..."
    cargo build --release -p bsl-backend --bin bsl-web-server
else
    echo "✅ Backend: используем существующую сборку"
fi

# Проверка и сборка frontend
STATIC_ARG=""
if [[ "$NO_FRONTEND" != "true" ]]; then
    FRONTEND_INDEX="./backend/static/index.html"
    NEED_BUILD=false

    # Проверяем нужна ли сборка
    if [[ ! -f "$FRONTEND_INDEX" ]] || [[ "$DO_BUILD" == "true" ]]; then
        NEED_BUILD=true
    else
        # Проверяем что файлы на которые ссылается index.html существуют
        WASM_FILE=$(grep -oP 'bsl-frontend-[a-f0-9]+_bg\.wasm' "$FRONTEND_INDEX" 2>/dev/null | head -1 || true)
        if [[ -n "$WASM_FILE" ]] && [[ ! -f "./backend/static/$WASM_FILE" ]]; then
            echo "⚠️ WASM файл $WASM_FILE не найден, требуется пересборка"
            NEED_BUILD=true
        fi
    fi

    if [[ "$NEED_BUILD" == "true" ]]; then
        echo "🎨 Сборка frontend (WASM)..."
        cd frontend
        trunk build --release
        cd ..

        # Оптимизация WASM с wasm-opt (trunk не передаёт --all-features)
        WASM_FILE=$(ls target/site/bsl-frontend-*_bg.wasm 2>/dev/null | head -1 || true)
        if [[ -n "$WASM_FILE" ]] && command -v wasm-opt &> /dev/null; then
            echo "⚡ Оптимизация WASM с wasm-opt..."
            WASM_SIZE_BEFORE=$(du -h "$WASM_FILE" | cut -f1)
            wasm-opt --all-features -Oz -o "${WASM_FILE}.opt" "$WASM_FILE" 2>/dev/null || true
            if [[ -f "${WASM_FILE}.opt" ]]; then
                mv "${WASM_FILE}.opt" "$WASM_FILE"
                WASM_SIZE_AFTER=$(du -h "$WASM_FILE" | cut -f1)
                echo "   $WASM_SIZE_BEFORE → $WASM_SIZE_AFTER"
            fi
        fi

        # Копируем собранные файлы в backend/static
        echo "📁 Копирование файлов в backend/static..."
        mkdir -p backend/static
        cp -r target/site/* backend/static/
    else
        echo "✅ Frontend: используем существующую сборку"
    fi
    STATIC_ARG="--static-files-path backend/static"
fi

echo ""
echo "🌐 Запуск сервера на порту $PORT..."
echo "   Syntax Helper: $SYNTAX_HELPER_PATH"
if [[ -n "$PROJECT_ARG" ]]; then
    echo "   Project Path: $PROJECT_PATH"
fi
if [[ -n "$STATIC_ARG" ]]; then
    echo "   Static Files: backend/static"
fi
echo ""
echo "📡 Endpoints:"
echo "   Health:      GET  http://localhost:$PORT/api/health"
echo "   Diagnostics: POST http://localhost:$PORT/api/diagnostics"
echo "   Hover:       POST http://localhost:$PORT/api/hover/enhanced"
echo "   Types:       GET  http://localhost:$PORT/api/types"
if [[ -n "$STATIC_ARG" ]]; then
    echo ""
    echo "🖥️  Frontend UI: http://localhost:$PORT/"
fi
echo ""
echo "Press Ctrl+C to stop"
echo "=========================================="

./target/release/bsl-web-server${BINARY_EXT} \
    --port $PORT \
    --enable-cors true \
    --syntax-helper-path "$SYNTAX_HELPER_PATH" \
    $PROJECT_ARG \
    $STATIC_ARG
