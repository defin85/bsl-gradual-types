#!/bin/bash
# scripts/clear_cache.sh
#
# Скрипт для очистки Windows File System Cache (Standby List)
# Используется для тестирования прогресса парсинга типов платформы 1С
#
# Использование:
#   ./scripts/clear_cache.sh
#
# Требования:
#   - Windows 10/11
#   - Утилита EmptyStandbyList.exe в tools/
#   - Права администратора

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOL_PATH="$PROJECT_ROOT/tools/EmptyStandbyList.exe"

echo "═══════════════════════════════════════════════════════════"
echo "🧹 Очистка Windows File System Cache"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Проверяем наличие утилиты
POWERSHELL_SCRIPT="$PROJECT_ROOT/tools/ClearFileCache.ps1"

if [ ! -f "$TOOL_PATH" ]; then
    echo "⚠️ Утилита EmptyStandbyList.exe не найдена"
    echo ""

    # Проверяем наличие PowerShell скрипта как альтернативы
    if [ -f "$POWERSHELL_SCRIPT" ]; then
        echo "✅ Используем альтернативный метод: PowerShell скрипт"
        echo ""

        # Конвертируем путь для PowerShell
        WIN_PS_PATH=$(cygpath -w "$POWERSHELL_SCRIPT" 2>/dev/null || echo "$POWERSHELL_SCRIPT" | sed 's|^/c/|C:/|')

        echo "🔄 Очистка File System Cache..."
        echo "   (требует прав администратора - появится UAC prompt)"
        echo ""

        # Запускаем PowerShell скрипт от имени администратора
        powershell -Command "Start-Process powershell -ArgumentList '-ExecutionPolicy Bypass -File \"$WIN_PS_PATH\"' -Verb RunAs -Wait" 2>&1

        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 0 ]; then
            echo ""
            echo "═══════════════════════════════════════════════════════════"
            exit 0
        else
            echo ""
            echo "❌ Не удалось очистить кэш через PowerShell (код: $EXIT_CODE)"
            exit $EXIT_CODE
        fi
    else
        echo "❌ Ни одна утилита не найдена!"
        echo ""
        echo "📥 Вариант 1: Скачай EmptyStandbyList.exe"
        echo "   cd tools"
        echo "   curl -L https://wj32.org/wp/download/releases/empty-standby-list/EmptyStandbyList.exe -o EmptyStandbyList.exe"
        echo ""
        echo "📥 Вариант 2: Используй PowerShell скрипт (уже есть в tools/ClearFileCache.ps1)"
        echo "   powershell -ExecutionPolicy Bypass -File tools/ClearFileCache.ps1"
        echo ""
        echo "Подробнее: tools/README.md"
        exit 1
    fi
fi

# Проверяем размер файла (должен быть ~6-10 KB)
FILE_SIZE=$(stat -c%s "$TOOL_PATH" 2>/dev/null || stat -f%z "$TOOL_PATH" 2>/dev/null || echo "0")
if [ "$FILE_SIZE" -lt 5000 ] || [ "$FILE_SIZE" -gt 50000 ]; then
    echo "⚠️ Размер EmptyStandbyList.exe подозрительный: $FILE_SIZE байт"
    echo "   Ожидается: 6-10 KB"
    echo ""
    read -p "Продолжить? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "📍 Утилита: $TOOL_PATH"
echo "📏 Размер: $(du -h "$TOOL_PATH" | cut -f1)"
echo ""

# Конвертируем путь в Windows формат для PowerShell
WIN_TOOL_PATH=$(cygpath -w "$TOOL_PATH" 2>/dev/null || echo "$TOOL_PATH" | sed 's|^/c/|C:/|')

echo "🔄 Очистка Standby List..."
echo "   (требует прав администратора - появится UAC prompt)"
echo ""

# Запускаем утилиту от имени администратора
powershell -Command "Start-Process '$WIN_TOOL_PATH' -ArgumentList 'standbylist' -Verb RunAs -Wait" 2>&1

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "✅ Файловый кэш Windows очищен!"
    echo ""
    echo "📊 Теперь можешь тестировать прогресс парсинга:"
    echo "   1. Открой VSCode Extension (F5)"
    echo "   2. Открой конфигурацию 1С"
    echo "   3. Наблюдай прогресс в статус-баре (1-2 минуты)"
    echo ""
else
    echo ""
    echo "❌ Не удалось очистить кэш (код: $EXIT_CODE)"
    echo ""
    echo "Возможные причины:"
    echo "   - Отменил UAC prompt (нажал 'Нет')"
    echo "   - Недостаточно прав администратора"
    echo "   - Антивирус заблокировал утилиту"
    echo ""
    exit $EXIT_CODE
fi

echo "═══════════════════════════════════════════════════════════"
