#!/bin/bash
# Скрипт для тестирования API валидации кода

echo "🧪 Тестирование API валидации кода BSL Gradual Types"
echo "===================================================="
echo ""

# URL сервера
BASE_URL="http://127.0.0.1:3004"

# Цвета для вывода
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Функция для тестирования
test_validation() {
    local code="$1"
    local description="$2"
    local expected_valid="$3"

    echo -e "${YELLOW}Тест:${NC} $description"
    echo "Код: $code"

    # Отправляем запрос
    response=$(curl -s -X POST "$BASE_URL/api/validate" \
        -H "Content-Type: application/json" \
        -d "{\"code\": \"$code\"}")

    # Проверяем результат
    is_valid=$(echo "$response" | jq -r '.isValid')
    errors_count=$(echo "$response" | jq -r '.errors | length')

    if [ "$is_valid" = "$expected_valid" ]; then
        echo -e "${GREEN}✅ PASSED${NC}"
    else
        echo -e "${RED}❌ FAILED${NC}"
        echo "Ожидалось: isValid = $expected_valid, получено: $is_valid"
    fi

    if [ "$is_valid" = "false" ]; then
        echo "Ошибки ($errors_count):"
        echo "$response" | jq -r '.errors[] | "  - [\(.errorType)] \(.message)"'
    fi

    echo ""
}

# Проверка доступности сервера
echo "Проверка доступности сервера..."
if ! curl -s "$BASE_URL/api/health" > /dev/null; then
    echo -e "${RED}❌ Сервер недоступен на $BASE_URL${NC}"
    echo "Запустите сервер командой:"
    echo "  cargo run -p bsl-backend --bin bsl-web-server -- --port 3004 --enable-cors true --syntax-helper-path examples/syntax_helper"
    exit 1
fi
echo -e "${GREEN}✅ Сервер доступен${NC}"
echo ""

# Тестовые кейсы

echo "════════════════════════════════════════════════════"
echo "ТЕСТ 1: Валидные методы"
echo "════════════════════════════════════════════════════"
test_validation "Массив.Добавить()" "Валидный метод массива" "true"
test_validation "Массив.Количество()" "Валидный метод массива (Количество)" "true"
test_validation "ТаблицаЗначений.Вставить()" "Валидный метод таблицы значений" "true"

echo "════════════════════════════════════════════════════"
echo "ТЕСТ 2: Несуществующие методы"
echo "════════════════════════════════════════════════════"
test_validation "Массив.НесуществующийМетод()" "Несуществующий метод массива" "false"
test_validation "ТаблицаЗначений.ВыдуманныйМетод()" "Несуществующий метод таблицы" "false"

echo "════════════════════════════════════════════════════"
echo "ТЕСТ 3: Валидные свойства"
echo "════════════════════════════════════════════════════"
test_validation "ТаблицаЗначений.Колонки" "Валидное свойство таблицы значений" "true"
test_validation "ТаблицаЗначений.Индексы" "Валидное свойство (Индексы)" "true"

echo "════════════════════════════════════════════════════"
echo "ТЕСТ 4: Несуществующие свойства"
echo "════════════════════════════════════════════════════"
test_validation "ТаблицаЗначений.НесуществующееСвойство" "Несуществующее свойство" "false"
test_validation "Массив.КакоеТоСвойство" "Свойство у типа без свойств" "false"

echo "════════════════════════════════════════════════════"
echo "ТЕСТ 5: Case-insensitive поиск"
echo "════════════════════════════════════════════════════"
test_validation "Массив.добавить()" "Метод в нижнем регистре" "true"
test_validation "Массив.ДОБАВИТЬ()" "Метод в верхнем регистре" "true"
test_validation "Массив.Add()" "Метод на английском" "true"

echo "════════════════════════════════════════════════════"
echo "✅ Тестирование завершено!"
echo "════════════════════════════════════════════════════"
