# Semantic Tree API Tests

Тесты для нового endpoint `/api/semantic-tree` (Milestone 5.3).

## Интеграционные тесты

### Запуск всех тестов

```bash
cargo test -p bsl-backend --test api_semantic_tree_test
```

### Запуск конкретного теста

```bash
cargo test -p bsl-backend --test api_semantic_tree_test test_simple_procedure -- --nocapture
```

### Список тестов

1. **test_simple_procedure_creates_function_declaration_node** - простая процедура создаёт узел Procedure
2. **test_function_with_return_type** - функция с возвратом создаёт узел Function
3. **test_multiple_procedures_and_functions** - несколько процедур и функций
4. **test_variables_in_procedure_body** - переменные в теле процедуры
5. **test_empty_code_returns_empty_tree** - пустой код возвращает пустое дерево
6. **test_code_with_whitespace_only** - код только с пробелами
7. **test_procedure_with_parameters** - процедура с параметрами (проверка атрибутов)
8. **test_nested_structures** - вложенные структуры (if/for)
9. **test_metrics_analysis_time_is_set** - метрики времени анализа
10. **test_export_function_in_attributes** - экспортная функция
11. **test_response_structure** - структура ответа (все поля присутствуют)
12. **test_cyrillic_code_support** - поддержка кириллицы

## Ручное тестирование через curl

### Простой bash скрипт (для ASCII кода)

```bash
./backend/tests/test_semantic_tree_api.sh
```

**Требования:**
- Запущенный Web Server: `cargo run -p bsl-backend --bin bsl-web-server`
- Установленный `jq` для форматирования JSON

### Python скрипт (для кириллицы)

```bash
./backend/tests/test_semantic_tree_api_python.sh
```

**Требования:**
- Запущенный Web Server: `cargo run -p bsl-backend --bin bsl-web-server`
- Python 3 (стандартная библиотека)

**Особенности:**
- Правильная работа с кириллицей через `ensure_ascii=False`
- UTF-8 encoding для запросов
- Обработка HTTP ошибок

## Структура ответа

### Request (POST /api/semantic-tree)

```json
{
  "code": "Процедура Тест()\n    Сообщить(\"Привет\");\nКонецПроцедуры",
  "file_path": "test.bsl",
  "includeFlowSensitive": false
}
```

Параметры:
- `includeFlowSensitive` (bool, optional, default: `false`) — включает flow-sensitive вычисления (если нужны в дереве/метриках).
- Legacy `include_flow_sensitive` (snake_case) **не поддерживается** и возвращает `400 Bad Request` (breaking change).

### Response (200 OK)

```json
{
  "file_path": "test.bsl",
  "root_nodes": [
    {
      "kind": "Procedure",
      "name": "Тест",
      "location": {
        "line": 1,
        "column": 0
      },
      "range": {
        "start": {"line": 1, "column": 0},
        "end": {"line": 3, "column": 15}
      },
      "children": [...],
      "attributes": {
        "parameter_count": "0"
      }
    }
  ],
  "symbol_table": {},
  "metrics": {
    "node_count": 2,
    "procedure_count": 1,
    "function_count": 0,
    "variable_count": 0,
    "known_types": 0,
    "inferred_types": 0,
    "unknown_types": 0,
    "analysis_time_ms": 0,
    "request_duration_ms": 5
  }
}
```

## Типы узлов

- **Procedure** - процедура
- **Function** - функция
- **Variable** - переменная
- **Assignment** - присваивание
- **IfStatement** - условие If
- **ForLoop** - цикл For
- **MethodCall** - вызов метода

## Метрики

- `node_count` - общее количество узлов
- `procedure_count` - количество процедур
- `function_count` - количество функций
- `variable_count` - количество переменных
- `parameter_count` - количество параметров
- `known_types` - количество известных типов
- `inferred_types` - количество выведенных типов
- `unknown_types` - количество неизвестных типов
- `average_certainty` - средняя уверенность типизации (0.0 - 1.0)
- `analysis_time_ms` - время анализа (мс)
- `tree_depth` - глубина дерева
- `call_count` - количество вызовов функций

## Debug тест

Для отладки структуры ответа:

```bash
cargo test -p bsl-backend --test debug_semantic_tree_output -- --ignored --nocapture
```

Выводит полную структуру ответа с детальной информацией о каждом узле.

## Примеры curl запросов

### Простая процедура

```bash
curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d '{"code": "Процедура Тест()\nКонецПроцедуры", "file_path": "test.bsl", "includeFlowSensitive": false}' \
  | jq '.'
```

### Только метрики

```bash
curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d '{"code": "Процедура Тест()\nКонецПроцедуры", "file_path": "test.bsl", "includeFlowSensitive": false}' \
  | jq '.metrics'
```

### Проверка узлов

```bash
curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d '{"code": "Процедура Тест()\nКонецПроцедуры", "file_path": "test.bsl", "includeFlowSensitive": false}' \
  | jq '.root_nodes[] | {kind, name, location}'
```

## Проблемы с кириллицей

**ВАЖНО:** На Windows/GitBash используй Python скрипт для работы с кириллицей.

Обычный curl может некорректно работать с кириллицей в GitBash:

```bash
# ❌ НЕ работает в GitBash
curl -d '{"code": "Процедура Тест()"}' ...

# ✅ Работает через Python
python3 -c "import json; ..."
```

См. `test_semantic_tree_api_python.sh` для примеров.

## CI/CD

Интеграционные тесты запускаются автоматически в CI pipeline:

```bash
cargo test --workspace
```

Включает `api_semantic_tree_test` автоматически.

## Связанные файлы

- **Тесты:** `backend/tests/api_semantic_tree_test.rs`
- **Handler:** `backend/src/presentation/web/handlers.rs` (функция `get_semantic_tree`)
- **Router:** `backend/src/presentation/web/router.rs` (маршрут `/api/semantic-tree`)
- **DTO:** `shared/src/api/semantic_dtos.rs` (`SemanticTreeDto`, `SemanticNodeDto`, etc.)
- **Service:** `backend/src/application/type_system_service.rs` (метод `get_semantic_tree`)
- **IR Conversion:** `shared/src/ir/mod.rs` (метод `to_dto`)

## Документация API

См. `docs/api/web-api-reference.md` для полной документации Web API.
