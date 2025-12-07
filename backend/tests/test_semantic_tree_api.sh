#!/usr/bin/env bash
# Manual testing script for /api/semantic-tree endpoint
#
# Prerequisites:
# 1. Web server must be running: cargo run -p bsl-backend --bin bsl-web-server
# 2. Server should be listening on http://localhost:3002
#
# Usage:
#   chmod +x backend/tests/test_semantic_tree_api.sh
#   ./backend/tests/test_semantic_tree_api.sh

set -e

BASE_URL="http://localhost:3002"
API_ENDPOINT="$BASE_URL/api/semantic-tree"

echo "=========================================="
echo "Testing /api/semantic-tree endpoint"
echo "=========================================="
echo ""

# Test 1: Simple procedure
echo "Test 1: Simple procedure"
echo "------------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Процедура Тест()\n    Сообщить(\"Привет\");\nКонецПроцедуры",
    "file_path": "test.bsl"
  }' | jq '.'
echo ""
echo ""

# Test 2: Function with return
echo "Test 2: Function with return"
echo "----------------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция ПолучитьЧисло()\n    Возврат 42;\nКонецФункции",
    "file_path": "test.bsl"
  }' | jq '.metrics'
echo ""
echo ""

# Test 3: Multiple functions
echo "Test 3: Multiple functions"
echo "--------------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Процедура Первая()\nКонецПроцедуры\n\nФункция Вторая()\n    Возврат 1;\nКонецФункции",
    "file_path": "test.bsl"
  }' | jq '.metrics'
echo ""
echo ""

# Test 4: Empty code
echo "Test 4: Empty code"
echo "------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "",
    "file_path": "empty.bsl"
  }' | jq '.metrics'
echo ""
echo ""

# Test 5: Code with variables
echo "Test 5: Code with variables"
echo "---------------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Процедура Тест()\n    Переменная1 = 10;\n    Переменная2 = \"Строка\";\nКонецПроцедуры",
    "file_path": "test.bsl"
  }' | jq '{
    file_path: .file_path,
    root_nodes_count: (.root_nodes | length),
    symbol_table_size: (.symbol_table | length),
    metrics: .metrics
  }'
echo ""
echo ""

# Test 6: Check root_nodes structure
echo "Test 6: Root nodes structure"
echo "----------------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Процедура Тест()\nКонецПроцедуры",
    "file_path": "test.bsl"
  }' | jq '.root_nodes[] | {kind, name, location}'
echo ""
echo ""

# Test 7: Nested structures
echo "Test 7: Nested structures"
echo "-------------------------"
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Процедура Тест()\n    Если Истина Тогда\n        Сообщить(\"Да\");\n    КонецЕсли;\nКонецПроцедуры",
    "file_path": "test.bsl"
  }' | jq '{
    node_count: .metrics.node_count,
    tree_depth: .metrics.tree_depth
  }'
echo ""
echo ""

# Test 8: Using file from filesystem (if web server supports it)
echo "Test 8: Large code sample"
echo "-------------------------"
cat > /tmp/test_semantic.bsl <<'EOF'
Процедура ПерваяПроцедура()
    Переменная1 = 10;
    Переменная2 = "Строка";
КонецПроцедуры

Функция ПолучитьСумму(А, Б)
    Возврат А + Б;
КонецФункции

Процедура ВтораяПроцедура()
    Результат = ПолучитьСумму(5, 3);
    Сообщить(Результат);
КонецПроцедуры
EOF

CODE=$(cat /tmp/test_semantic.bsl)
curl -s -X POST "$API_ENDPOINT" \
  -H "Content-Type: application/json" \
  -d "{
    \"code\": $(echo "$CODE" | jq -Rs .),
    \"file_path\": \"large_test.bsl\"
  }" | jq '{
    procedures: .metrics.procedure_count,
    functions: .metrics.function_count,
    variables: .metrics.variable_count,
    parameters: .metrics.parameter_count,
    nodes: .metrics.node_count,
    depth: .metrics.tree_depth
  }'
echo ""
echo ""

# Cleanup
rm -f /tmp/test_semantic.bsl

echo "=========================================="
echo "All tests completed!"
echo "=========================================="
