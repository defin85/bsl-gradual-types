#!/bin/bash

echo "========================================"
echo "TEST 1: Return type for Количество()"
echo "========================================"
curl -s -X POST http://localhost:3002/api/hover/enhanced \
  -H "Content-Type: application/json" \
  -d '{"code":"ТЗ = Новый ТаблицаЗначений;\nКол = ТЗ.Количество();","line":2,"column":6}' \
  | grep -o '"type_hint":"[^"]*"' || echo "type_hint not found"

echo ""
echo "========================================"
echo "TEST 2: Return type for ТипЗнч() function"
echo "========================================"
curl -s -X POST http://localhost:3002/api/hover/enhanced \
  -H "Content-Type: application/json" \
  -d '{"code":"ТЗ = Новый ТаблицаЗначений;\nТип = ТипЗнч(ТЗ);","line":2,"column":5}' \
  | grep -o '"type_hint":"[^"]*"' || echo "type_hint not found"

echo ""
echo "========================================"
echo "TEST 3: Void method return type"
echo "========================================"
curl -s -X POST http://localhost:3002/api/hover/enhanced \
  -H "Content-Type: application/json" \
  -d '{"code":"Сообщить(\"Привет\");","line":1,"column":0}' \
  | grep -o '"type_hint":"[^"]*"' || echo "type_hint not found"
