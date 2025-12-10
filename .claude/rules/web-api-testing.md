# Тестирование LSP через Web API

## ВАЖНОЕ ПРАВИЛО

**Claude НЕ должен самостоятельно:**
- Собирать LSP/Web сервер (`cargo build --bin bsl-lsp-server/bsl-web-server`)
- Запускать LSP/Web сервер (ни в фоне, ни в foreground)
- Останавливать серверы (`pkill`)

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
   ```

2. **Claude тестирует через curl:**
   ```bash
   # Проверить что сервер работает
   curl -s http://localhost:3002/api/health
   ```

## Шаблон для кириллицы (ОБЯЗАТЕЛЬНО)

**ВАЖНО:** На Windows/GitBash используй `codecs` + локальный файл `test_api.json`:

```bash
# 1. Тест BSL файла
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
with codecs.open('examples/bsl/ФАЙЛ.bsl', 'r', 'utf-8-sig') as f:
    code = f.read()
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/diagnostics -H "Content-Type: application/json" -d @test_api.json

# 2. Тест inline кода
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
code = '''Процедура Тест()
    ТЗ = Новый ТаблицаЗначений;
    ТЗ.Добавить();
КонецПроцедуры'''
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': code}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/diagnostics -H "Content-Type: application/json" -d @test_api.json

# 3. Hover
cd /c/1CProject/bsl-gradual-types && python -c "
import json, codecs
with codecs.open('test_api.json', 'w', 'utf-8') as f:
    json.dump({'code': 'ТЗ = Новый ТаблицаЗначений;', 'line': 1, 'column': 10}, f, ensure_ascii=False)
" && curl -s -X POST http://localhost:3002/api/hover/enhanced -H "Content-Type: application/json" -d @test_api.json
```

**Ключевые моменты:**
- Используй `codecs.open(..., 'utf-8-sig')` для чтения (убирает BOM)
- Используй `codecs.open(..., 'utf-8')` для записи
- Файл `test_api.json` в корне проекта (НЕ в /tmp — разные файловые системы)
- `ensure_ascii=False` обязательно

## Доступные endpoints

| Endpoint | Метод | Описание |
|----------|-------|----------|
| `/api/health` | GET | Проверка работоспособности |
| `/api/hover/enhanced` | POST | Детальная информация hover |
| `/api/diagnostics` | POST | Синтаксические + семантические ошибки |
| `/api/debug/ast` | POST | AST дерево и symbol table |
| `/api/types` | GET | Список всех типов |

**Тестовая конфигурация:** `C:\1CProject\conf` — содержит документы, справочники и другие объекты метаданных.

**Полная документация:** [docs/api/web-api-reference.md](docs/api/web-api-reference.md)
