# Web API Reference

Справочник по BSL Web Server API с примерами curl запросов.

## 🚀 Запуск сервера

### Базовый запуск (только примитивные типы)

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true
```

### С полными типами платформы (Syntax Helper)

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

**База URL:** `http://127.0.0.1:3002`

---

## 📋 Endpoints

### 1. Health Check

**Endpoint:** `GET /api/health`

**Назначение:** Проверка доступности сервера и статус системы

#### Запрос

```bash
curl -s "http://127.0.0.1:3002/api/health" | jq '.'
```

#### Ответ

```json
{
  "status": "ok",
  "version": "0.4.0",
  "types_loaded": 3927,
  "uptime_seconds": 145
}
```

#### Статус коды

- `200 OK` — сервер работает корректно
- `503 Service Unavailable` — сервер не готов

---

### 2. Type Search (латиница)

**Endpoint:** `GET /api/types?search=<query>`

**Назначение:** Поиск типов платформы по английскому названию

#### Запрос

```bash
curl -s "http://127.0.0.1:3002/api/types?search=Array" | jq '.'
```

#### Ответ

```json
{
  "results": [
    {
      "name": "Массив",
      "english_name": "Array",
      "category": "Примитивные типы",
      "description": "Коллекция элементов с индексным доступом",
      "methods_count": 15,
      "properties_count": 0
    }
  ],
  "total": 1
}
```

#### Статус коды

- `200 OK` — результаты найдены (может быть пустой массив)
- `400 Bad Request` — отсутствует параметр `search`

---

### 3. Type Search (кириллица) с URL-encoding

**Endpoint:** `GET /api/search?q=<query>`

**Назначение:** Поиск типов по русскому названию

**Примечание:** В выдачу включаются глобальные функции. Они попадают в категории `Platform`/`Configuration` и помечены описанием вида "Глобальная функция...".

**⚠️ ВАЖНО:** GitBash на Windows требует URL-encoding для кириллицы!

#### Конвертация кириллицы в URL-encoded

```bash
# Способ 1: Python
python3 -c "import urllib.parse; print(urllib.parse.quote('Массив'))"
# Вывод: %D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2

# Способ 2: Использовать готовую таблицу (см. ниже)
```

#### Запрос

```bash
# "Массив" → "%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"
curl -s "http://127.0.0.1:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2" | jq '.'
```

#### Ответ

```json
{
  "query": "Массив",
  "results": [
    {
      "name": "Массив",
      "type": "PrimitiveType",
      "certainty": "Known",
      "methods": [
        {
          "name": "Добавить",
          "params": ["Значение"],
          "description": "Добавляет элемент в конец массива"
        },
        {
          "name": "Количество",
          "returns": "Число",
          "description": "Возвращает количество элементов"
        },
        {
          "name": "Очистить",
          "description": "Удаляет все элементы"
        }
      ],
      "properties": []
    }
  ],
  "total": 1
}
```

#### Статус коды

- `200 OK` — результаты найдены
- `400 Bad Request` — отсутствует параметр `q`

---

### 4. Configuration Types (Справочники, Документы)

**Endpoint:** `GET /api/search?q=<query>`

**Назначение:** Поиск конфигурационных типов (без загруженной конфигурации)

#### Запрос

```bash
# "Справочники" → "%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8"
curl -s "http://127.0.0.1:3002/api/search?q=%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8" | jq '.'
```

#### Ответ (без загруженной конфигурации)

```json
{
  "query": "Справочники",
  "results": [
    {
      "name": "Справочники",
      "type": "ConfigurationType",
      "certainty": "Inferred(0.5)",
      "warning": "⚠️ Детали типа недоступны. Требуется загрузка метаданных конфигурации.",
      "methods": [],
      "properties": []
    }
  ],
  "total": 1
}
```

**Интерпретация:**
- `certainty: "Inferred(0.5)"` — честная оценка (только синтаксис распарсен, метаданных нет)
- `warning` — объяснение причины недоступности деталей

#### Статус коды

- `200 OK` — тип распознан (даже без метаданных)

---

### 5. Code Analysis

**Endpoint:** `POST /api/analyze`

**Назначение:** Статический анализ BSL кода

#### Запрос

```bash
curl -X POST "http://127.0.0.1:3002/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    МассивДанных = Новый Массив();\n    МассивДанных.Добавить(42);\n    Возврат МассивДанных;\nКонецФункции"
  }' | jq '.'
```

#### Ответ

```json
{
  "analysis": {
    "variables": [
      {
        "name": "МассивДанных",
        "type": "Массив",
        "certainty": "Known",
        "line": 2,
        "methods_available": 15
      }
    ],
    "functions": [
      {
        "name": "Тест",
        "return_type": "Массив",
        "certainty": "Known",
        "line": 1
      }
    ],
    "errors": [],
    "warnings": []
  },
  "elapsed_ms": 85
}
```

#### Поля запроса

| Поле | Тип | Обязательно | Описание |
|------|-----|-------------|----------|
| `code` | String | ✅ | BSL код для анализа |

#### Статус коды

- `200 OK` — анализ выполнен успешно
- `400 Bad Request` — некорректный JSON или отсутствует `code`
- `422 Unprocessable Entity` — синтаксическая ошибка в коде

---

### 6. Hover Information

**Endpoint:** `POST /api/hover`

**Назначение:** Получить информацию о типе переменной/выражения в указанной позиции кода (аналог LSP hover)

#### Запрос

```bash
curl -X POST "http://127.0.0.1:3002/api/hover" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    МассивДанных = Новый Массив();\n    Возврат МассивДанных;\nКонецФункции",
    "line": 2,
    "column": 4
  }' | jq '.'
```

#### Ответ

```json
{
  "hover": "Массив\n\nПримитивный тип для работы с коллекциями элементов.\n\nМетоды:\n- Добавить(Значение) — Добавляет элемент в конец массива\n- Количество() → Число — Возвращает количество элементов",
  "line": 2,
  "column": 4,
  "duration_ms": 42
}
```

#### Поля запроса

| Поле | Тип | Обязательно | Описание |
|------|-----|-------------|----------|
| `code` | String | ✅ | BSL код для анализа |
| `line` | Number | ✅ | Строка (0-based) |
| `column` | Number | ✅ | Колонка (0-based) |

#### Статус коды

- `200 OK` — hover информация получена успешно
- `400 Bad Request` — некорректный JSON или отсутствуют требуемые поля
- `500 Internal Server Error` — ошибка парсинга или анализа кода

---

### 7. Semantic Visualization (Milestone 2.16)

**Endpoint:** `GET /api/semantic/:file_path?format=json|html&theme=dark|light&compact=false`

**Назначение:** Получение семантического дерева BSL модуля

**Статус:** ⚠️ MVP stub (заглушка для тестирования API контракта)

#### Параметры

| Параметр | Тип | По умолчанию | Описание |
|----------|-----|--------------|----------|
| `file_path` | Path | - | Путь к BSL файлу (в URL) |
| `format` | Enum | `json` | Формат вывода: `json` или `html` |
| `theme` | Enum | `dark` | Тема для HTML: `dark` или `light` |
| `compact` | Bool | `false` | Упрощённый вывод (меньше деталей) |

#### Запрос (JSON формат)

```bash
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=json" | jq '.'
```

#### Ответ (JSON)

```json
{
  "file_path": "test.bsl",
  "semantic_tree": {
    "nodes": [
      {
        "type": "Function",
        "name": "Тест",
        "span": {
          "start_line": 1,
          "start_column": 0,
          "end_line": 5,
          "end_column": 0
        },
        "children": [
          {
            "type": "Variable",
            "name": "МассивДанных",
            "type_hint": "Массив",
            "span": {
              "start_line": 2,
              "start_column": 4,
              "end_line": 2,
              "end_column": 44
            }
          },
          {
            "type": "MethodCall",
            "method": "Добавить",
            "receiver": "МассивДанных",
            "args": [42],
            "span": {
              "start_line": 3,
              "start_column": 4,
              "end_line": 3,
              "end_column": 32
            }
          }
        ]
      }
    ]
  }
}
```

#### Запрос (HTML формат с темной темой)

```bash
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=html&theme=dark" > semantic_tree.html

# Открыть в браузере
start semantic_tree.html
```

#### Ответ (HTML)

```html
<!DOCTYPE html>
<html>
<head>
  <title>Semantic Tree: test.bsl</title>
  <style>
    body { background: #1e1e1e; color: #d4d4d4; font-family: 'Consolas', monospace; }
    .node { margin-left: 20px; }
    .function { color: #dcdcaa; }
    .variable { color: #9cdcfe; }
    /* ... другие стили ... */
  </style>
</head>
<body>
  <h1>Semantic Tree: test.bsl</h1>
  <div class="tree">
    <div class="node function">
      <span>Function: Тест</span>
      <div class="node variable">
        <span>Variable: МассивДанных (Type: Массив)</span>
      </div>
      <div class="node method-call">
        <span>MethodCall: МассивДанных.Добавить(42)</span>
      </div>
    </div>
  </div>
</body>
</html>
```

#### Статус коды

- `200 OK` — дерево сгенерировано успешно
- `400 Bad Request` — некорректный параметр `format` или `theme`
- `404 Not Found` — файл не найден

---

## 🔑 URL-encoded таблица (кириллица)

Для частых запросов:

| Оригинал | URL-encoded |
|----------|-------------|
| **Примитивные типы** | |
| Массив | `%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2` |
| Строка | `%D0%A1%D1%82%D1%80%D0%BE%D0%BA%D0%B0` |
| Число | `%D0%A7%D0%B8%D1%81%D0%BB%D0%BE` |
| Булево | `%D0%91%D1%83%D0%BB%D0%B5%D0%B2%D0%BE` |
| Дата | `%D0%94%D0%B0%D1%82%D0%B0` |
| Структура | `%D0%A1%D1%82%D1%80%D1%83%D0%BA%D1%82%D1%83%D1%80%D0%B0` |
| Соответствие | `%D0%A1%D0%BE%D0%BE%D1%82%D0%B2%D0%B5%D1%82%D1%81%D1%82%D0%B2%D0%B8%D0%B5` |
| **Конфигурационные типы** | |
| Справочники | `%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8` |
| Документы | `%D0%94%D0%BE%D0%BA%D1%83%D0%BC%D0%B5%D0%BD%D1%82%D1%8B` |
| РегистрыСведений | `%D0%A0%D0%B5%D0%B3%D0%B8%D1%81%D1%82%D1%80%D1%8B%D0%A1%D0%B2%D0%B5%D0%B4%D0%B5%D0%BD%D0%B8%D0%B9` |
| РегистрыНакопления | `%D0%A0%D0%B5%D0%B3%D0%B8%D1%81%D1%82%D1%80%D1%8B%D0%9D%D0%B0%D0%BA%D0%BE%D0%BF%D0%BB%D0%B5%D0%BD%D0%B8%D1%8F` |

---

## 🧪 Тестирование API

### Автоматизация через Skill

```bash
# Автоматическое тестирование всех endpoints
/api-tester
```

**См. также:** [.claude/skills/api-tester.md](../../.claude/skills/api-tester.md)

### Ручное тестирование

```bash
# 1. Health Check
curl -s "http://127.0.0.1:3002/api/health" | jq '.'

# 2. Type Search (латиница)
curl -s "http://127.0.0.1:3002/api/types?search=String" | jq '.'

# 3. Type Search (кириллица)
curl -s "http://127.0.0.1:3002/api/search?q=%D0%A1%D1%82%D1%80%D0%BE%D0%BA%D0%B0" | jq '.'

# 4. Code Analysis
curl -X POST "http://127.0.0.1:3002/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{"code": "Процедура Тест() КонецПроцедуры"}' | jq '.'

# 5. Semantic Visualization (JSON)
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=json" | jq '.'

# 6. Semantic Visualization (HTML)
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=html&theme=dark" > tree.html
```

---

## 🔍 CORS

### Включение CORS

```bash
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true
```

### Проверка CORS заголовков

```bash
curl -I "http://127.0.0.1:3002/api/health"
```

**Ожидаемый заголовок:**
```
Access-Control-Allow-Origin: *
```

---

## ⚠️ Особенности GitBash на Windows

### URL-encoding обязателен для кириллицы

```bash
# ❌ НЕ работает - кириллица напрямую
curl "http://127.0.0.1:3002/api/search?q=Массив"

# ✅ Работает - URL-encoded кириллица
curl "http://127.0.0.1:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"
```

### Конвертация через Python

```bash
# Создать функцию для конвертации
urlencode() {
  python3 -c "import urllib.parse; print(urllib.parse.quote('$1'))"
}

# Использование
urlencode "Справочники.Контрагенты"
# Вывод: %D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8.%D0%9A%D0%BE%D0%BD%D1%82%D1%80%D0%B0%D0%B3%D0%B5%D0%BD%D1%82%D1%8B

# Использовать в curl
curl -s "http://127.0.0.1:3002/api/search?q=$(urlencode "Массив")" | jq '.'
```

---

## 🔗 Связанные документы

- **[Development Workflow](../guides/development-workflow.md)** — команды запуска сервера
- **[Tooling Guide](../guides/tooling-guide.md)** — MCP инструменты для тестирования
- **[Components Detailed](../architecture/components-detailed.md)** — детали Web Server

---

## 📊 Статус endpoints

| Endpoint | Метод | Статус | Milestone |
|----------|-------|--------|-----------|
| `/api/health` | GET | ✅ Работает | - |
| `/api/types` | GET | ✅ Работает | - |
| `/api/search` | GET | ✅ Работает | 2.9, 2.18 |
| `/api/validate` | POST | ✅ Работает | 2.4, 2.18 |
| `/api/hover` | POST | ✅ Работает | 2.5, 2.13 |
| `/api/semantic/:file_path` | GET | ⚠️ MVP stub | 2.16 |

**Легенда:**
- ✅ Полностью работает
- ⚠️ Частично работает (stub/prototype)
- ❌ Не реализовано

---

## 🚀 Будущие endpoints (планируется)

- `GET /api/diagnostics/:file_path` — синтаксические ошибки (Milestone 2.18)
- `POST /api/refactor` — автоматический рефакторинг
- `GET /api/completion/:file_path` — автодополнение кода

---

## 📡 Новые улучшенные endpoints

### 6.1. Enhanced Hover Information (NEW)

**Endpoint:** `POST /api/hover/enhanced`

**Назначение:** Детальная информация о типе переменной для отладки

**Статус:** ✅ Работает (Milestone 2.13)

#### Запрос

```bash
curl -X POST "http://127.0.0.1:3002/api/hover/enhanced" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    ТЗ = Новый ТаблицаЗначений();\nКонецФункции",
    "line": 2,
    "column": 4
  }' | jq '.'
```

#### Ответ

```json
{
  "hoverText": "ТаблицаЗначений\n\nПримитивный тип...",
  "variableName": "ТЗ",
  "variableType": "ТаблицаЗначений",
  "typeHint": "Explicit",
  "foundInScope": true,
  "line": 2,
  "column": 4,
  "durationMs": 15
}
```

---

### 7. Diagnostics with Error Separation (NEW)

**Endpoint:** `POST /api/diagnostics`

**Назначение:** Синтаксические и семантические ошибки раздельно

**Статус:** ✅ Работает (Milestone 2.18)

#### Запрос

```bash
curl -X POST "http://127.0.0.1:3002/api/diagnostics" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    массив.НесуществующийМетод();\nКонецФункции"
  }' | jq '.'
```

#### Ответ

```json
{
  "syntaxErrors": [],
  "semanticErrors": [
    {
      "message": "Метод 'НесуществующийМетод' не найден",
      "line": 2,
      "column": 4,
      "severity": "error"
    }
  ],
  "totalErrors": 1,
  "durationMs": 35
}
```

---

### 8. Debug AST for Parser (NEW)

**Endpoint:** `POST /api/debug/ast`

**Назначение:** AST дерево для отладки парсера

**Статус:** ⚠️ MVP stub (Milestone 2.16)

#### Запрос

```bash
curl -X POST "http://127.0.0.1:3002/api/debug/ast" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    МассивДанных = Новый Массив();\nКонецФункции"
  }' | jq '.'
```

#### Ответ

```json
{
  "nodes": [
    {
      "kind": "Program",
      "startLine": 1,
      "startColumn": 1,
      "endLine": 3,
      "endColumn": 15,
      "text": null
    }
  ],
  "symbolTable": [
    {
      "name": "МассивДанных",
      "typeHint": "Массив",
      "declaredLine": 2,
      "scopeLevel": 0
    }
  ],
  "parseErrors": 0,
  "durationMs": 8
}
```

---

### 9. Semantic Tree Visualization (NEW)

**Endpoint:** `POST /api/semantic-tree`

**Назначение:** Получение семантического дерева BSL модуля с метриками

**Статус:** ✅ Работает (Milestone 2.16)

#### Описание

Анализирует BSL код и возвращает структурированное семантическое дерево с:
- Иерархией узлов (функции, процедуры, переменные, вызовы)
- Таблицей символов
- Метриками анализа (количество узлов, время)

**Отличие от `/api/debug/ast`:** Возвращает семантическую модель (SemanticProgram) вместо низкоуровневого AST

#### Запрос (с файлом .bsl)

**⚠️ ВАЖНО для кириллицы:** Используй Python + локальный файл `test_api.json` на Windows/GitBash

```bash
# 1. Создать JSON через Python (с правильной кодировкой)
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
with codecs.open('examples/bsl/ТестМодуль.bsl', 'r', 'utf-8-sig') as f:
    code = f.read()
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code, 'file_path': 'examples/bsl/ТестМодуль.bsl'}, f, ensure_ascii=False)
"

# 2. Отправить запрос
curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d @test_api.json | jq '.'
```

#### Запрос (inline код)

```bash
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
code = '''Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Добавить();
КонецПроцедуры'''
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code, 'file_path': 'test.bsl'}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d @test_api.json | jq '.'
```

#### Поля запроса

| Поле | Тип | Обязательно | Описание |
|------|-----|-------------|----------|
| `code` | String | ✅ | BSL код для анализа |
| `file_path` | String | ❌ | Путь к файлу (для отображения) |

#### Ответ

```json
{
  "file_path": "test.bsl",
  "root_nodes": [
    {
      "node_type": "Procedure",
      "name": "Тест",
      "span": {
        "start_line": 1,
        "start_column": 0,
        "end_line": 4,
        "end_column": 0
      },
      "children": [
        {
          "node_type": "Assignment",
          "variable": "ТЗ",
          "type_hint": "ТаблицаЗначений",
          "span": {
            "start_line": 2,
            "start_column": 4,
            "end_line": 2,
            "end_column": 35
          }
        },
        {
          "node_type": "MethodCall",
          "receiver": "ТЗ",
          "method": "Добавить",
          "args_count": 0,
          "span": {
            "start_line": 3,
            "start_column": 4,
            "end_line": 3,
            "end_column": 18
          }
        }
      ]
    }
  ],
  "symbol_table": [
    {
      "name": "ТЗ",
      "type_hint": "ТаблицаЗначений",
      "declared_line": 2,
      "scope": "Procedure:Тест"
    }
  ],
  "metrics": {
    "total_nodes": 3,
    "functions_count": 0,
    "procedures_count": 1,
    "variables_count": 1,
    "method_calls_count": 1,
    "parse_duration_ms": 12,
    "analysis_duration_ms": 8
  }
}
```

#### Структура SemanticNode

Каждый узел содержит:

| Поле | Тип | Описание |
|------|-----|----------|
| `node_type` | String | Тип узла: `Function`, `Procedure`, `Assignment`, `MethodCall`, `IfStatement`, `ForLoop` и др. |
| `name` | String | Имя (для функций/процедур/переменных) |
| `span` | Object | Позиция в коде (start_line, start_column, end_line, end_column) |
| `children` | Array | Дочерние узлы |

**Дополнительные поля зависят от типа узла:**

- **Function/Procedure:** `parameters`, `return_type`
- **Assignment:** `variable`, `type_hint`
- **MethodCall:** `receiver`, `method`, `args_count`
- **IfStatement:** `condition`
- **ForLoop:** `iterator`, `collection`

#### Статус коды

- `200 OK` — дерево успешно построено
- `400 Bad Request` — отсутствует поле `code`
- `500 Internal Server Error` — ошибка парсинга или анализа

#### Примеры использования

**1. Анализ файла с кириллицей:**
```bash
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
with codecs.open('examples/bsl/ПримерТипов.bsl', 'r', 'utf-8-sig') as f:
    code = f.read()
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code, 'file_path': 'examples/bsl/ПримерТипов.bsl'}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d @test_api.json | jq '.metrics'
```

**2. Быстрая проверка метрик:**
```bash
# Получить только метрики (без дерева)
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
code = 'Функция Тест() Возврат 42; КонецФункции'
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d @test_api.json | jq '.metrics'
```

**3. Проверка структуры символов:**
```bash
curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" \
  -d @test_api.json | jq '.symbol_table'
```

---

## 📈 Метрики системы (observability)

**Endpoint:** `GET /api/metrics`

**Назначение:** Сводные метрики типов и производительности (completion/signatureHelp/resolve).

#### Запрос

```bash
curl -s "http://127.0.0.1:3002/api/metrics" | jq '.'
```

#### Ответ

```json
{
  "types": {
    "total_types": 420,
    "known_types": 380,
    "inferred_types": 25,
    "unknown_types": 15
  },
  "observability": {
    "counters": {
      "completion_total": 120,
      "completion_incomplete_total": 4,
      "signature_help_total": 32
    },
    "gauges": {
      "analysis_duration_ms": 12.0
    },
    "histograms": {
      "completion_duration_ms": {
        "count": 120,
        "p50": 12.0,
        "p95": 38.0,
        "p99": 49.0
      }
    },
    "rates": {
      "completion_incomplete_rate": 0.0333,
      "signature_help_empty_rate": 0.125
    },
    "uptime_seconds": 3600
  }
}
```

**Примечания:**
- `histograms` содержит агрегаты P50/P95/P99 и количество измерений.
- `rates` вычисляются по счетчикам (например, `completion_incomplete_total / completion_total`).

---

## 📊 Обновлённый статус endpoints

| Endpoint | Метод | Статус | Milestone |
|----------|-------|--------|-----------|
| `/api/health` | GET | ✅ | - |
| `/api/metrics` | GET | ✅ | M7 |
| `/api/types` | GET | ✅ | - |
| `/api/search` | GET | ✅ | 2.9, 2.18 |
| `/api/validate` | POST | ✅ | 2.4, 2.18 |
| `/api/hover` | POST | ✅ | 2.5, 2.13 |
| `/api/hover/enhanced` | POST | ✅ | 2.13 |
| `/api/diagnostics` | POST | ✅ | 2.18 |
| `/api/debug/ast` | POST | ⚠️ MVP | 2.16 |
| `/api/semantic-tree` | POST | ✅ | 2.16 |
