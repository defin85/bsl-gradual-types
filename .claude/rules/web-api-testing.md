# Тестирование LSP через Web API

## ВАЖНОЕ ПРАВИЛО

**Claude НЕ должен самостоятельно:**
- Собирать LSP/Web сервер (`cargo build --bin bsl-lsp-server/bsl-web-server`)
- Запускать LSP/Web сервер (ни в фоне, ни в foreground)
- Останавливать серверы (`pkill`)

Это относится и к задачам отладки/исследования **резолвинга типов**: сервер запускает пользователь, Claude использует уже запущенный Web API через `curl`.

**Claude МОЖЕТ:**
- Тестировать через `curl` когда сервер уже запущен пользователем
- Запускать `cargo test` для unit/integration тестов
- Использовать `/build` skill для полной сборки проекта

## Как тестировать

1. **Пользователь запускает сервер** в отдельном терминале:
   ```bash
   # С конфигурацией (рекомендуется для полного тестирования):
   ./scripts/start-web-api.sh

   # Только platform types (без конфигурации):
   ./scripts/start-web-api.sh --no-config

   # С пересборкой (после изменений кода):
   ./scripts/start-web-api.sh --build
   ```

2. **Claude тестирует через curl:**
   ```bash
   # Проверить что сервер работает
   curl -s http://localhost:3002/api/health
   ```

## Координаты (ВАЖНО)

Во всех endpoints, где передаются `line`/`column` (например, `POST /api/hover/enhanced`):

- `line` — **0-based** (первая строка файла = `0`)
- `column` — **0-based** (первый символ строки = `0`)

Если ты смотришь строку в редакторе/через `nl -ba` (обычно 1-based), то для Web API нужно делать:
- `api_line = file_line - 1`

## Шаблон для кириллицы (ОБЯЗАТЕЛЬНО)

**ВАЖНО:** Используй `python3` + локальный файл `test_api.json`:

```bash
# 1. Диагностика кода
python3 -c "
import json, codecs
code = '''Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Добавить();
КонецПроцедуры'''
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/diagnostics \
  -H "Content-Type: application/json" -d @test_api.json | python3 -m json.tool

# 2. Hover на позиции
python3 -c "
import json, codecs
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': 'Процедура Тест()\\n    x = Объект;\\nКонецПроцедуры', 'line': 1, 'column': 8, 'filePath': 'Documents/Док1/Ext/ObjectModule.bsl'}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/hover/enhanced \
  -H "Content-Type: application/json" -d @test_api.json | python3 -m json.tool

# 3. Семантическое дерево (для анализа узлов)
python3 -c "
import json, codecs
code = '''Процедура Тест()
    Ссылка = Документы.ЗаказНаряды.НайтиПоНомеру(\"001\");
    Ссылка.Работы.Выгрузить();
КонецПроцедуры'''
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code, 'file_path': 'test.bsl'}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/semantic-tree \
  -H "Content-Type: application/json" -d @test_api.json | python3 -m json.tool
```

**Ключевые моменты:**
- Файл `test_api.json` в корне проекта (НЕ в /tmp — разные файловые системы)
- `ensure_ascii=False` обязательно
- Используй `python3 -m json.tool` для форматирования вывода

## Доступные endpoints

### Основные (для тестирования)

| Endpoint | Метод | Описание |
|----------|-------|----------|
| `/api/health` | GET | Проверка работоспособности + версия сборки |
| `/api/diagnostics` | POST | Синтаксические + семантические ошибки |
| `/api/hover/enhanced` | POST | Детальная информация hover с фасетами |
| `/api/semantic-tree` | POST | **Семантическое дерево** (узлы, символы, типы) |

### Debug endpoints

| Endpoint | Метод | Описание |
|----------|-------|----------|
| `/api/debug/ast` | POST | AST дерево (только синтаксис, без типов) |
| `/api/diagnostics/debug` | POST | Диагностика + debug info |

### Справочные

| Endpoint | Метод | Описание |
|----------|-------|----------|
| `/api/types` | GET | Список всех типов (с пагинацией) |
| `/api/search?q=...` | GET | Поиск типов по имени |
| `/api/version` | GET | Информация о версии |

## Примеры request body

### /api/diagnostics
```json
{"code": "Процедура Тест()\n    Объект.НесуществующийМетод();\nКонецПроцедуры", "filePath": "Documents/Док1/Ext/ObjectModule.bsl"}
```

### /api/hover/enhanced
```json
{"code": "Процедура Тест()\n    x = Объект;\nКонецПроцедуры", "line": 1, "column": 8, "filePath": "Documents/Док1/Ext/ObjectModule.bsl"}
```

### /api/semantic-tree
```json
{"code": "Процедура Тест()\n    x = 1;\nКонецПроцедуры", "file_path": "test.bsl"}
```

## Различия endpoints

| Что нужно | Используй |
|-----------|-----------|
| Проверить ошибки в коде | `/api/diagnostics` |
| Узнать тип переменной/выражения | `/api/hover/enhanced` |
| Увидеть все узлы семантического дерева | `/api/semantic-tree` |
| Отладить парсинг (только AST) | `/api/debug/ast` |

**Тестовая конфигурация:** `examples/conf` (WSL) или `C:\1CProject\conf` (Windows)

**Полная документация:** [docs/api/web-api-reference.md](docs/api/web-api-reference.md)
