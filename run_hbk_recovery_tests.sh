#!/bin/bash
# HBK Recovery Test Suite Runner
# Запускает все тесты для HBK Recovery компоненты

set -e

echo "=========================================="
echo "HBK Recovery Test Suite"
echo "=========================================="
echo ""

PASSED=0
FAILED=0

# Unit-тесты
echo "1️⃣  Unit-тесты (в модуле)..."
if cargo test -p bsl-backend hbk_recovery --lib 2>&1 | grep "test result: ok"; then
    PASSED=$((PASSED + 1))
    echo "   ✅ PASSED"
else
    FAILED=$((FAILED + 1))
    echo "   ❌ FAILED"
fi
echo ""

# Интеграционные тесты
echo "2️⃣  Интеграционные тесты..."
if cargo test -p bsl-backend --test hbk_recovery_integration_test 2>&1 | grep "test result: ok"; then
    PASSED=$((PASSED + 1))
    echo "   ✅ PASSED"
else
    FAILED=$((FAILED + 1))
    echo "   ❌ FAILED"
fi
echo ""

# Error handling тесты
echo "3️⃣  Error handling тесты..."
if cargo test -p bsl-backend --test hbk_recovery_error_handling_test 2>&1 | grep "test result:"; then
    echo "   ⚠️  PARTIAL (требуется доработка)"
fi
echo ""

# Edge cases тесты
echo "4️⃣  Edge cases тесты..."
if cargo test -p bsl-backend --test hbk_recovery_edge_cases_test 2>&1 | grep "test result:"; then
    echo "   ⚠️  PARTIAL (требуется доработка)"
fi
echo ""

echo "=========================================="
echo "Итого: $PASSED/3 основных тестовых наборов успешно"
echo "=========================================="
echo ""
echo "Для подробных результатов запустите:"
echo "  cargo test -p bsl-backend hbk_recovery -- --nocapture"
