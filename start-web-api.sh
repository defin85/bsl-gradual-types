#!/bin/bash
# Скрипт запуска Web API сервера для тестирования LSP функций
# Использование: ./start-web-api.sh [--no-config] [--build] [--no-frontend]
#
# Флаги:
#   --no-config    Запуск без конфигурации (только platform types)
#   --build        Пересобрать backend и frontend перед запуском
#   --no-frontend  Не собирать и не подключать frontend (только API)

set -e

PORT=${PORT:-3002}
SYNTAX_HELPER_PATH="examples/syntax_helper"
PROJECT_PATH="/c/1CProject/conf"
DO_BUILD=false
NO_FRONTEND=false

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
    esac
done

if [[ "$NO_CONFIG" == "true" ]]; then
    echo "🚀 Запуск Web API БЕЗ конфигурации (только platform types)"
    PROJECT_ARG=""
else
    echo "🚀 Запуск Web API С конфигурацией"
    PROJECT_ARG="--project-path $PROJECT_PATH"
fi

# Остановка предыдущего сервера если запущен
pkill -f "bsl-web-server.*--port $PORT" 2>/dev/null || true
sleep 1

# Проверка и сборка backend
BINARY="./target/release/bsl-web-server"
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
    if [[ ! -f "$FRONTEND_INDEX" ]] || [[ "$DO_BUILD" == "true" ]]; then
        echo "🎨 Сборка frontend (WASM)..."
        cd frontend
        trunk build --release
        cd ..
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

./target/release/bsl-web-server \
    --port $PORT \
    --enable-cors true \
    --syntax-helper-path "$SYNTAX_HELPER_PATH" \
    $PROJECT_ARG \
    $STATIC_ARG
