---
name: start-lsp-api
description: Запускает BSL Web Server для тестирования LSP функций через HTTP API
---

# Start LSP Web API Skill

Запускает BSL Web Server для автоматизированного тестирования LSP функций через HTTP API.

## 🎯 Назначение

Позволяет Claude автоматически тестировать LSP функции без запуска VSCode Extension:
- ✅ Semantic Diagnostics (`POST /api/validate`)
- ✅ Type search (`GET /api/search`)
- ✅ Type information (`GET /api/types`)

## 🚀 Процесс запуска

### 1. Сборка Web Server (если нужно)

```bash
# Проверить существование бинарника
ls -lh target/release/bsl-web-server.exe

# Если нет - собрать
cargo build --release --bin bsl-web-server
```

### 2. Запуск сервера

**Базовый режим (только platform types):**
```bash
./target/release/bsl-web-server.exe \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper &
```

**С конфигурацией 1С (полный режим - рекомендуется):**
```bash
./target/release/bsl-web-server.exe \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper \
  --project-path /c/1CProject/conf &
```

**Важно:** `--project-path` загружает типы из конфигурации 1С (справочники, документы и т.д.), что даёт 100% confidence в hover и полную информацию о свойствах/методах.

```bash
# Сохранить PID для остановки
echo $! > .web-server.pid
```

**Альтернатива (без фона):**
```bash
cargo run --release --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper \
  --project-path /c/1CProject/conf
```

### 3. Проверка health

```bash
# Проверить что сервер запущен
curl -s http://localhost:3002/api/health | jq

# Ожидается:
# {
#   "status": "healthy",
#   "version": "0.4.0",
#   "types_loaded": 13320
# }
```

### 4. Остановка сервера

```bash
# Если запущен в фоне
kill $(cat .web-server.pid)
rm .web-server.pid

# Или найти и убить процесс
pkill -f bsl-web-server
```

---

## 🧪 Использование для тестирования

### Semantic Diagnostics

**Тестовый код с потенциальной ошибкой:**
```bash
curl -X POST http://localhost:3002/api/validate \
  -H "Content-Type: application/json" \
  -d '{
    "code": "Функция Тест()\n    ТаблицаТип = Новый ТаблицаЗначений;\n    Кол = ТаблицаТип.Количество();\nКонецФункции"
  }' | jq
```

**Ожидаемый результат (БЕЗ ошибок):**
```json
{
  "diagnostics": []
}
```

**Если есть ошибка:**
```json
{
  "diagnostics": [
    {
      "severity": "Error",
      "message": "Метод 'Количество' не существует для типа 'ТаблицаЗначений'",
      "line": 3,
      "column": 17
    }
  ]
}
```

---

### Type Search

```bash
# Поиск типа
curl -s "http://localhost:3002/api/search?q=ТаблицаЗначений" | jq

# URL-encoding для кириллицы (если нужно)
QUERY=$(python3 -c "import urllib.parse; print(urllib.parse.quote('ТаблицаЗначений'))")
curl -s "http://localhost:3002/api/search?q=$QUERY" | jq
```

---

## 📊 Claude Workflow

### Итеративная разработка с автоматическим тестированием:

1. **Claude изменяет код** (например, TypeResolver)
2. **Пересобирает:** `cargo build --release --bin bsl-web-server`
3. **Перезапускает сервер:** `pkill bsl-web-server && ./target/release/... &`
4. **Тестирует через WebFetch:**
   ```
   WebFetch("http://localhost:3002/api/validate", {
     method: "POST",
     headers: {"Content-Type": "application/json"},
     body: JSON.stringify({code: "..."})
   })
   ```
5. **Видит результаты немедленно** - есть ли false positives
6. **Итерирует** до правильного поведения
7. **Финальная проверка** - просит пользователя протестировать в VSCode

---

## ⚙️ Конфигурация

### Порт (по умолчанию: 3002)

Можно изменить через флаг `--port`:
```bash
./target/release/bsl-web-server.exe --port 8080 ...
```

### Platform Types (обязательно)

```bash
--syntax-helper-path examples/syntax_helper
```

Загружает базовые типы платформы 1С (ТаблицаЗначений, Массив и т.д.). Без этого TypeRepository будет пустым.

### Project Configuration (рекомендуется)

```bash
--project-path /c/1CProject/conf
```

Загружает типы из конфигурации 1С:
- Справочники (СправочникМенеджер, СправочникОбъект, СправочникСсылка)
- Документы (ДокументМенеджер, ДокументОбъект, ДокументСсылка)
- Регистры и другие прикладные объекты

**Без `--project-path`:** hover показывает только platform types с 50% confidence.
**С `--project-path`:** hover показывает полную информацию с 100% confidence.

### CORS

Для тестирования из браузера/Postman:
```bash
--enable-cors true
```

---

## 🔗 Связанные навыки

- **api-tester** - тестирование всех Web API endpoints
- **build** - сборка Web Server
- **test-runner** - integration тесты

---

## 📚 Документация

- **[docs/api/web-api-reference.md](../../docs/api/web-api-reference.md)** - все endpoints
- **[docs/guides/claude-lsp-testing.md](../../docs/guides/claude-lsp-testing.md)** - workflow для Claude (создать!)

---

## ⚠️ Особенности

### GitBash на Windows

Все команды используют Unix-style:
```bash
✅ ./target/release/bsl-web-server.exe &
❌ start target\release\bsl-web-server.exe
```

### Background процесс

Проверка что сервер запущен:
```bash
ps aux | grep bsl-web-server
# Или
curl http://localhost:3002/api/health
```

---

## 🎯 Использование

```
/start-lsp-api
```

Claude автоматически:
1. Соберёт Web Server (если нужно)
2. Запустит в фоне
3. Проверит health
4. Будет готов к тестированию через WebFetch
