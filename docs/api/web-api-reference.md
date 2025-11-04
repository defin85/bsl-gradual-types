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

### 6. Semantic Visualization (Milestone 2.16)

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
| `/api/analyze` | POST | ✅ Работает | 2.8, 2.9 |
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
- `GET /api/hover/:file_path` — hover информация (как в LSP)
