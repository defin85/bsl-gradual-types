---
name: api-tester
description: Тестирование BSL Web API с поддержкой URL-encoding для кириллицы
---

# API Tester Skill

Автоматическое тестирование BSL Web API с поддержкой URL-encoding для кириллицы.

## 🎯 Назначение

Проверка всех endpoints BSL Web API:
- ✅ Health checks
- ✅ Type search (латиница и кириллица)
- ✅ Code analysis
- ✅ Semantic visualization
- ✅ JSON schema валидация

## 🚀 Подготовка

### Запуск Web сервера

```bash
# Базовый запуск (только примитивные типы)
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true

# С полными типами платформы (Syntax Helper)
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper
```

**Ожидаемый вывод:**
```
🚀 BSL Web Server запущен на http://127.0.0.1:3002
📦 Загружено 3927 типов платформы
✅ CORS включен
```

## 🔍 Тестовые сценарии

### 1. Health Check

```bash
# Проверка доступности сервера
curl -s "http://127.0.0.1:3002/api/health" | jq '.'
```

**Ожидаемый результат:**
```json
{
  "status": "ok",
  "version": "0.4.0",
  "types_loaded": 3927,
  "uptime_seconds": 145
}
```

**Проверка:**
- ✅ HTTP 200 OK
- ✅ JSON валидный
- ✅ `status == "ok"`
- ✅ `types_loaded > 0`

---

### 2. Type Search (латиница)

```bash
# Поиск типа "Array"
curl -s "http://127.0.0.1:3002/api/types?search=Array" | jq '.'
```

**Ожидаемый результат:**
```json
{
  "results": [
    {
      "name": "Массив",
      "english_name": "Array",
      "category": "Примитивные типы",
      "description": "Коллекция элементов с индексным доступом"
    }
  ],
  "total": 1
}
```

**Проверка:**
- ✅ HTTP 200 OK
- ✅ `results` массив не пустой
- ✅ `results[0].english_name == "Array"`

---

### 3. Type Search (кириллица с URL-encoding)

**⚠️ ВАЖНО:** GitBash на Windows требует URL-encoding для кириллицы!

```bash
# Конвертация кириллицы в URL-encoded формат
# "Массив" → "%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"

# Способ 1: Использовать готовую строку
curl -s "http://127.0.0.1:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2" | jq '.'

# Способ 2: Python для конвертации
python3 -c "import urllib.parse; print(urllib.parse.quote('Массив'))"
# Вывод: %D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2

# Затем использовать полученную строку
curl -s "http://127.0.0.1:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2" | jq '.'
```

**Ожидаемый результат:**
```json
{
  "query": "Массив",
  "results": [
    {
      "name": "Массив",
      "type": "PrimitiveType",
      "certainty": "Known",
      "methods": [
        { "name": "Добавить", "description": "Добавляет элемент в конец массива" },
        { "name": "Количество", "description": "Возвращает количество элементов" },
        { "name": "Очистить", "description": "Удаляет все элементы" }
      ]
    }
  ],
  "total": 1
}
```

**Проверка:**
- ✅ HTTP 200 OK
- ✅ `query == "Массив"` (декодирована на сервере)
- ✅ `results[0].methods.length > 0`

---

### 4. Configuration Types (Справочники, Документы)

```bash
# Справочники.Контрагенты (URL-encoded)
# "Справочники.Контрагенты" → "%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8.%D0%9A%D0%BE%D0%BD%D1%82%D1%80%D0%B0%D0%B3%D0%B5%D0%BD%D1%82%D1%8B"

curl -s "http://127.0.0.1:3002/api/search?q=%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8" | jq '.'
```

**Ожидаемый результат (без загруженной конфигурации):**
```json
{
  "query": "Справочники",
  "results": [
    {
      "name": "Справочники",
      "type": "ConfigurationType",
      "certainty": "Inferred(0.5)",
      "warning": "⚠️ Детали типа недоступны. Требуется загрузка метаданных конфигурации."
    }
  ],
  "total": 1
}
```

**Проверка:**
- ✅ HTTP 200 OK
- ✅ `certainty == "Inferred(0.5)"` (честная оценка!)
- ✅ Присутствует предупреждение о недоступности метаданных

---

### 5. Code Analysis

```bash
# POST запрос с BSL кодом
curl -X POST "http://127.0.0.1:3002/api/analyze" \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    МассивДанных = Новый Массив();\n    МассивДанных.Добавить(42);\n    Возврат МассивДанных;\nКонецФункции"
  }' | jq '.'
```

**Ожидаемый результат:**
```json
{
  "analysis": {
    "variables": [
      {
        "name": "МассивДанных",
        "type": "Массив",
        "certainty": "Known",
        "line": 2
      }
    ],
    "functions": [
      {
        "name": "Тест",
        "return_type": "Массив",
        "certainty": "Known"
      }
    ],
    "errors": [],
    "warnings": []
  }
}
```

**Проверка:**
- ✅ HTTP 200 OK
- ✅ `variables.length > 0`
- ✅ `variables[0].type == "Массив"`
- ✅ `errors` массив пустой

---

### 6. Semantic Visualization (Milestone 2.16)

```bash
# JSON формат
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=json" | jq '.'

# HTML формат с темной темой
curl -s "http://127.0.0.1:3002/api/semantic/test.bsl?format=html&theme=dark" > semantic_tree.html

# Открыть в браузере
start semantic_tree.html
```

**Ожидаемый результат (JSON):**
```json
{
  "file_path": "test.bsl",
  "semantic_tree": {
    "nodes": [
      {
        "type": "Function",
        "name": "Тест",
        "span": { "start_line": 1, "start_column": 0, "end_line": 5, "end_column": 0 },
        "children": [
          {
            "type": "Variable",
            "name": "МассивДанных",
            "type_hint": "Массив",
            "span": { "start_line": 2, "start_column": 4 }
          }
        ]
      }
    ]
  }
}
```

**Проверка (JSON):**
- ✅ HTTP 200 OK
- ✅ `semantic_tree.nodes` не пустой
- ✅ Span координаты корректные

**Проверка (HTML):**
- ✅ HTTP 200 OK
- ✅ Content-Type: text/html
- ✅ HTML валидный (открывается в браузере)

---

## 📊 Формат отчёта

```markdown
# 🌐 Отчёт о тестировании BSL Web API

**Дата:** 2025-11-03
**Сервер:** http://127.0.0.1:3002
**Версия:** 0.4.0

---

## ✅ 1. Health Check

**Endpoint:** `GET /api/health`
**Результат:** ✅ Успешно

```json
{
  "status": "ok",
  "types_loaded": 3927
}
```

---

## ✅ 2. Type Search (латиница)

**Endpoint:** `GET /api/types?search=Array`
**Результат:** ✅ Успешно (1 тип найден)

---

## ✅ 3. Type Search (кириллица)

**Endpoint:** `GET /api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2`
**Декодировано:** "Массив"
**Результат:** ✅ Успешно (1 тип найден, 15 методов)

---

## ✅ 4. Configuration Types

**Endpoint:** `GET /api/search?q=%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8`
**Декодировано:** "Справочники"
**Результат:** ✅ Успешно

**Certainty:** 🟡 Inferred (50%)
**Причина:** Метаданные конфигурации не загружены (ожидаемое поведение)

---

## ✅ 5. Code Analysis

**Endpoint:** `POST /api/analyze`
**Результат:** ✅ Успешно

**Анализ:**
- Найдено переменных: 1
- Найдено функций: 1
- Ошибок: 0
- Предупреждений: 0

---

## ✅ 6. Semantic Visualization

**Endpoint:** `GET /api/semantic/test.bsl?format=json`
**Результат:** ✅ Успешно

**Semantic Tree:**
- Узлов: 2 (Function + Variable)
- Span координаты: корректные

**HTML Visualization:**
**Endpoint:** `GET /api/semantic/test.bsl?format=html&theme=dark`
**Результат:** ✅ HTML корректно сгенерирован

---

## 📊 Общий итог

| Endpoint | Метод | Статус | Время (ms) |
|----------|-------|--------|------------|
| /api/health | GET | ✅ | 5 |
| /api/types | GET | ✅ | 12 |
| /api/search (кириллица) | GET | ✅ | 18 |
| /api/search (конфиг) | GET | ✅ | 15 |
| /api/analyze | POST | ✅ | 85 |
| /api/semantic (json) | GET | ✅ | 45 |
| /api/semantic (html) | GET | ✅ | 52 |

**Общая оценка:** ✅ **Все endpoints работают корректно**

**Проверено:**
- ✅ HTTP статус коды
- ✅ JSON schema валидация
- ✅ URL-encoding для кириллицы
- ✅ Честная оценка certainty
- ✅ HTML генерация

---

**Время выполнения:** 8 секунд
**Следующая проверка:** После изменений в Web API
```

## ❌ Обработка ошибок

### Сервер не запущен

```markdown
## ❌ Health Check

**Endpoint:** `GET /api/health`
**Результат:** ❌ Провалено

**Ошибка:**
```
curl: (7) Failed to connect to 127.0.0.1 port 3002: Connection refused
```

**Причина:** Web сервер не запущен

**Решение:**
```bash
cargo run -p bsl-backend --bin bsl-web-server -- --port 3002 --enable-cors true
```
```

### Некорректный JSON response

```markdown
## ❌ Type Search

**Endpoint:** `GET /api/search?q=Array`
**Результат:** ❌ Провалено

**Ошибка:**
```
parse error: Invalid numeric literal at line 1, column 10
```

**Полученный ответ:**
```
Internal Server Error
```

**Причина:** Сервер вернул HTML вместо JSON (internal error)

**Рекомендация:** Проверить логи `rust_lsp_server.log`
```

## 🎯 Использование

Запусти этот навык когда:
- После изменений в Web API
- Перед релизом новой версии
- При добавлении новых endpoints
- Для проверки URL-encoding кириллицы
- Валидация JSON schema

**Команда:**
```
/api-tester
```

**Или:**
```
Протестируй Web API
```

## 🔧 Вспомогательные скрипты

### URL Encoding Helper

```bash
# Функция для конвертации кириллицы в URL-encoded
function urlencode() {
  python3 -c "import urllib.parse; print(urllib.parse.quote('$1'))"
}

# Использование
urlencode "Справочники.Контрагенты"
# Вывод: %D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8.%D0%9A%D0%BE%D0%BD%D1%82%D1%80%D0%B0%D0%B3%D0%B5%D0%BD%D1%82%D1%8B
```

### Готовые URL-encoded строки

Для частых запросов:

| Оригинал | URL-encoded |
|----------|-------------|
| Массив | `%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2` |
| Строка | `%D0%A1%D1%82%D1%80%D0%BE%D0%BA%D0%B0` |
| Число | `%D0%A7%D0%B8%D1%81%D0%BB%D0%BE` |
| Справочники | `%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BE%D1%87%D0%BD%D0%B8%D0%BA%D0%B8` |
| Документы | `%D0%94%D0%BE%D0%BA%D1%83%D0%BC%D0%B5%D0%BD%D1%82%D1%8B` |

## ⚠️ Важные замечания

### GitBash на Windows

```bash
# ✅ Работает - URL-encoded кириллица
curl "http://localhost:3002/api/search?q=%D0%9C%D0%B0%D1%81%D1%81%D0%B8%D0%B2"

# ❌ НЕ работает - кириллица напрямую
curl "http://localhost:3002/api/search?q=Массив"
```

### Проверка CORS

```bash
# Проверить CORS заголовки
curl -I "http://127.0.0.1:3002/api/health"

# Ожидаемый заголовок:
# Access-Control-Allow-Origin: *
```

### Порт занят

Если порт 3002 занят:

```bash
# Использовать другой порт
cargo run -p bsl-backend --bin bsl-web-server -- --port 3003 --enable-cors true

# Обновить URL в тестах
curl "http://127.0.0.1:3003/api/health"
```
