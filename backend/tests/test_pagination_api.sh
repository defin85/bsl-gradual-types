#!/bin/bash
# Скрипт для ручного тестирования API Pagination (Phase 3)
# Запуск: bash backend/tests/test_pagination_api.sh

BASE_URL="http://localhost:3002"
PASSED=0
FAILED=0

echo "========================================="
echo "API Pagination Integration Tests (Phase 3)"
echo "========================================="
echo ""

# Функция для проверки JSON поля (поддержка camelCase)
check_field() {
    local json="$1"
    local field="$2"
    local expected="$3"
    local test_name="$4"

    # Конвертация snake_case → camelCase для совместимости
    case "$field" in
        current_page) field="currentPage" ;;
        page_size) field="pageSize" ;;
        total_items) field="totalItems" ;;
        total_pages) field="totalPages" ;;
        has_prev) field="hasPrev" ;;
        has_next) field="hasNext" ;;
    esac

    actual=$(echo "$json" | jq -r ".pagination.$field")

    if [ "$actual" == "$expected" ]; then
        echo "✅ PASS: $test_name"
        ((PASSED++))
    else
        echo "❌ FAIL: $test_name (expected: $expected, got: $actual)"
        ((FAILED++))
    fi
}

# TEST 1: Базовый запрос с page=1
echo "[TEST 1] Базовый запрос page=1, limit=10"
response=$(curl -s "$BASE_URL/api/types?page=1&limit=10")
check_field "$response" "current_page" "1" "current_page=1"
check_field "$response" "page_size" "10" "page_size=10"
echo ""

# TEST 2: Запрос с page=2
echo "[TEST 2] Запрос page=2, limit=50"
response=$(curl -s "$BASE_URL/api/types?page=2&limit=50")
check_field "$response" "current_page" "2" "current_page=2"
check_field "$response" "page_size" "50" "page_size=50"
echo ""

# TEST 3: Граничный случай page=0 (должен стать page=1)
echo "[TEST 3] Граничный случай page=0 → должен стать page=1"
response=$(curl -s "$BASE_URL/api/types?page=0&limit=20")
check_field "$response" "current_page" "1" "page=0 исправлено на page=1"
echo ""

# TEST 4: Граничный случай limit=0 (должен стать limit=1)
echo "[TEST 4] Граничный случай limit=0 → должен стать limit=1"
response=$(curl -s "$BASE_URL/api/types?page=1&limit=0")
check_field "$response" "page_size" "1" "limit=0 исправлено на limit=1"
echo ""

# TEST 5: Граничный случай limit=5000 (должен стать limit=1000)
echo "[TEST 5] Граничный случай limit=5000 → должен стать limit=1000"
response=$(curl -s "$BASE_URL/api/types?page=1&limit=5000")
check_field "$response" "page_size" "1000" "limit=5000 обрезано до limit=1000"
echo ""

# TEST 6: Проверка has_prev для первой страницы
echo "[TEST 6] Проверка has_prev=false для page=1"
response=$(curl -s "$BASE_URL/api/types?page=1&limit=10")
check_field "$response" "has_prev" "false" "has_prev=false для первой страницы"
echo ""

# TEST 7: Проверка has_prev для второй страницы
echo "[TEST 7] Проверка has_prev=true для page=2"
response=$(curl -s "$BASE_URL/api/types?page=2&limit=10")
check_field "$response" "has_prev" "true" "has_prev=true для второй страницы"
echo ""

# TEST 8: Проверка, что offset параметр игнорируется
echo "[TEST 8] Проверка, что offset игнорируется (используется page=1 по умолчанию)"
response=$(curl -s "$BASE_URL/api/types?offset=100&limit=10")
check_field "$response" "current_page" "1" "offset игнорируется, page=1"
echo ""

# TEST 9: Проверка структуры PaginationDto
echo "[TEST 9] Проверка структуры PaginationDto (все обязательные поля присутствуют)"
response=$(curl -s "$BASE_URL/api/types?page=1&limit=10")
fields=("currentPage" "pageSize" "totalItems" "totalPages" "hasPrev" "hasNext")
all_present=true

for field in "${fields[@]}"; do
    value=$(echo "$response" | jq -r ".pagination.$field")
    if [ "$value" == "null" ]; then
        echo "❌ FAIL: Отсутствует поле pagination.$field"
        all_present=false
        ((FAILED++))
    fi
done

if [ "$all_present" = true ]; then
    echo "✅ PASS: Все обязательные поля PaginationDto присутствуют"
    ((PASSED++))
fi
echo ""

# TEST 10: Regression - поиск типов работает
echo "[TEST 10] Regression: поиск типов работает"
response=$(curl -s "$BASE_URL/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2")
items_count=$(echo "$response" | jq '.types | length')

if [ "$items_count" -gt 0 ]; then
    echo "✅ PASS: Поиск типов работает (найдено $items_count элементов)"
    ((PASSED++))
else
    echo "❌ FAIL: Поиск типов не вернул результатов"
    ((FAILED++))
fi
echo ""

# TEST 11: Regression - фильтры по категориям работают
echo "[TEST 11] Regression: фильтр category=Platform работает"
response=$(curl -s "$BASE_URL/api/types?category=Platform&page=1&limit=10")
first_item_category=$(echo "$response" | jq -r '.types[0].category')

if [ "$first_item_category" == "Platform" ]; then
    echo "✅ PASS: Фильтр category=Platform работает"
    ((PASSED++))
else
    echo "❌ FAIL: Фильтр category=Platform не работает (получено: $first_item_category)"
    ((FAILED++))
fi
echo ""

# TEST 12: Граничный случай - пустой результат поиска
echo "[TEST 12] Граничный случай: пустой результат поиска"
response=$(curl -s "$BASE_URL/api/search?q=%D0%9D%D0%B5%D1%81%D1%83%D1%89%D0%B5%D1%81%D1%82%D0%B2%D1%83%D1%8E%D1%89%D0%B8%D0%B9%D0%A2%D0%B8%D0%BF")
check_field "$response" "total_items" "0" "total_items=0 для несуществующего типа"
check_field "$response" "total_pages" "0" "total_pages=0 для пустого результата"
echo ""

# TEST 13: Проверка различных limit
echo "[TEST 13] Проверка различных limit (1, 50, 100, 500, 1000)"
for limit in 1 50 100 500 1000; do
    response=$(curl -s "$BASE_URL/api/types?page=1&limit=$limit")
    actual_limit=$(echo "$response" | jq -r ".pagination.page_size")

    if [ "$actual_limit" == "$limit" ]; then
        echo "✅ PASS: limit=$limit корректен"
        ((PASSED++))
    else
        echo "❌ FAIL: limit=$limit (получено: $actual_limit)"
        ((FAILED++))
    fi
done
echo ""

# ИТОГОВЫЙ ОТЧЁТ
echo "========================================="
echo "ИТОГИ ТЕСТИРОВАНИЯ"
echo "========================================="
echo "✅ Пройдено: $PASSED"
echo "❌ Провалено: $FAILED"
echo "========================================="

if [ $FAILED -eq 0 ]; then
    echo "🎉 Все тесты успешно пройдены!"
    exit 0
else
    echo "⚠️ Обнаружены ошибки в $FAILED тестах"
    exit 1
fi
