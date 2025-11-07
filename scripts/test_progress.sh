#!/bin/bash
# scripts/test_progress.sh
#
# Комбинированный скрипт для тестирования прогресса парсинга типов платформы
#
# Что делает:
#   1. Очищает Windows File System Cache
#   2. Запускает LSP сервер с подробным логированием
#   3. Позволяет увидеть прогресс парсинга в реальном времени
#
# Использование:
#   ./scripts/test_progress.sh
#
# Требования:
#   - Windows 10/11
#   - Утилита EmptyStandbyList.exe в tools/
#   - Права администратора

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "═══════════════════════════════════════════════════════════"
echo "🧪 Тестирование прогресса парсинга типов платформы 1С"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Шаг 1: Очистка кэша
echo "📋 Шаг 1/3: Очистка Windows File System Cache"
echo "─────────────────────────────────────────────────────────"
"$SCRIPT_DIR/clear_cache.sh"

if [ $? -ne 0 ]; then
    echo ""
    echo "❌ Не удалось очистить кэш. Прерываю тестирование."
    exit 1
fi

echo ""
echo "📋 Шаг 2/3: Сборка LSP сервера"
echo "─────────────────────────────────────────────────────────"
cd "$PROJECT_ROOT"

if [ ! -f "target/release/bsl-lsp-server.exe" ]; then
    echo "🔨 LSP сервер не собран. Собираю..."
    cargo build --release --bin bsl-lsp-server
else
    echo "✅ LSP сервер уже собран"
    echo "   (для пересборки: cargo build --release --bin bsl-lsp-server)"
fi

echo ""
echo "📋 Шаг 3/3: Запуск LSP сервера для тестирования"
echo "─────────────────────────────────────────────────────────"
echo ""
echo "💡 Инструкция:"
echo "   1. Скопируй собранный LSP сервер в vscode-extension/bin/"
echo "   2. Открой VSCode Extension (F5 в vscode-extension/)"
echo "   3. В новом окне VSCode открой конфигурацию 1С"
echo "   4. Наблюдай прогресс в статус-баре!"
echo ""
echo "📊 Ожидаемый прогресс (1-2 минуты):"
echo "   • Сканирование файлов: 0/24979 (0%)"
echo "   • Сканирование файлов: 5000/24979 (20%)"
echo "   • Тип 100/24979 - ETA: 884s (10%)"
echo "   • Тип 500/24979 - ETA: 700s (15%)"
echo "   • ..."
echo "   • ✅ Типы платформы загружены (100%)"
echo ""

read -p "📦 Скопировать LSP сервер в vscode-extension/bin/? (Y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
    echo "📦 Копирование..."
    cp -v "$PROJECT_ROOT/target/release/bsl-lsp-server.exe" "$PROJECT_ROOT/vscode-extension/bin/"
    echo "✅ LSP сервер скопирован"
    echo ""
    echo "🚀 Теперь:"
    echo "   cd vscode-extension"
    echo "   code ."
    echo "   # Нажми F5 в VSCode"
fi

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "✅ Подготовка к тестированию завершена!"
echo "═══════════════════════════════════════════════════════════"
