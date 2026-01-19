# MCP API: `bsl-agent` (tools/resources/prompts)

**Статус:** 🔴 ПЛАН  
**Принцип:** read-only, budgeted, deterministic.

---

## 1) Версии и совместимость

- MCP transport: **stdio** (первая версия).
- MCP protocol version: как в `rmcp`/спеке (фиксируется при реализации).
- Версионирование tools: через `serverInfo.version` + явное поле `api_version` в ответах.

---

## 2) Общие типы (DTO)

### 2.1. Position / Range

- `line`: 0-based
- `character`: UTF-16 column (как LSP)

```json
{ "line": 10, "character": 5 }
```

```json
{
  "start": { "line": 10, "character": 5 },
  "end": { "line": 10, "character": 12 }
}
```

### 2.2. RootRef / DocumentRef

`root_id` нужен для корректной работы multi-root workspace и стабильных ID.

```json
{ "root_id": "hex", "path": "/abs/path/to/workspace" }
```

```json
{ "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }
```

### 2.3. FileRef

```json
{
  "doc": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" },
  "text": "optional full text (unsaved buffer)",
  "version": 12
}
```

Если `text` отсутствует — читаем файл с диска (в пределах roots). Если `text` присутствует — он используется как snapshot **только для этого вызова**. Для IDE‑сценариев, где unsaved тексты должны влиять на `context_pack`/`scope=hot`, используется `workspace_documents_set`.

### 2.4. `analysis_revision` и валидность ID

`analysis_revision` — монотонный счётчик изменений effective-состояния документов внутри сессии (overlay и/или изменения на диске, обнаруженные сервером). Все семантические ответы возвращают `analysis_revision`; ID (`symbol_id`, `diagnostic_id`, `pack_id`, `item_id`) считаются валидными только в рамках этого revision.

---

## 3) Tools (MVP)

Нотация: `tools/call` с `name` ниже.

### 3.1. `workspace_open`

Открыть/инициализировать сессию проекта.

Input:

```json
{
  "roots": ["/abs/path/to/workspace"],
  "platform_docs_archive": "/abs/path/to/platform.zip",
  "platform_version": "8.3.24",
  "configuration_path": "/abs/path/to/config_dump",
  "mode": "progressive"
}
```

Output:

```json
{
  "session_id": "uuid",
  "roots": [{ "root_id": "hex", "path": "/abs/path/to/workspace" }],
  "analysis_revision": 0,
  "ready": false,
  "warnings": [],
  "missing_inputs": []
}
```

Примечание: тяжёлая инициализация может быть async; статус получать через `workspace_status`.

### 3.2. `workspace_status`

Получить состояние сессии/прогресс.

Input:
```json
{ "session_id": "uuid" }
```

Output:
```json
{
  "ready": true,
  "analysis_revision": 0,
  "phase": "idle|loading_platform|loading_config|indexing",
  "progress": { "percent": 100 },
  "warnings": [],
  "missing_inputs": []
}
```

### 3.3. `workspace_close`

Закрыть сессию и освободить ресурсы.

Input:
```json
{ "session_id": "uuid" }
```

Output:
```json
{ "ok": true }
```

### 3.4. `workspace_documents_set`

Сохранить unsaved тексты в памяти сессии (overlay). Эти тексты будут использоваться для `scope=hot` и `context_pack`.

Если у `FileRef` не задан `text`, документ просто помечается как “hot” и будет читаться с диска (без overlay).

Input:
```json
{
  "session_id": "uuid",
  "files": [
    { "doc": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "text": "...", "version": 12 }
  ],
  "mark_hot": true
}
```

Output:
```json
{ "ok": true, "analysis_revision": 1 }
```

### 3.5. `workspace_documents_clear`

Удалить overlay для документов (вернуться к чтению с диска).

Input:
```json
{
  "session_id": "uuid",
  "documents": [{ "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }],
  "clear_hot": true
}
```

Output:
```json
{ "ok": true, "analysis_revision": 2 }
```

### 3.6. `bsl_diagnostics`

Диагностики по проекту/файлу.

Input:
```json
{
  "session_id": "uuid",
  "scope": { "kind": "project" },
  "limit": 200,
  "include_impact": true,
  "include_coverage": true
}
```

Output (идея):
```json
{
  "analysis_revision": 2,
  "diagnostics": [
    {
      "diagnostic_id": "hex",
      "file": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" },
      "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 10 } },
      "severity": "error|warning|info",
      "code": "optional",
      "message": "text",
      "impact": { "severity": "low|medium|high|critical", "affected_variables": [], "unchecked_operations": [] },
      "explain": { "reasons": [] }
    }
  ],
  "coverage": { "checked": 0, "unchecked": 0, "percent": 0.0 }
}
```

### 3.7. `bsl_symbol_search`

Поиск символов по имени (для навигации LLM).

Input:
```json
{ "session_id": "uuid", "query": "Документы", "limit": 20 }
```

Output:
```json
{
  "analysis_revision": 2,
  "symbols": [
    { "symbol_id": "hex", "name": "Документы", "kind": "namespace", "file": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } } }
  ]
}
```

### 3.8. `bsl_type_at_position`

Тип/разрешение выражения в позиции.

Input:
```json
{
  "session_id": "uuid",
  "file": { "doc": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "text": "optional" },
  "position": { "line": 10, "character": 5 }
}
```

Output:
```json
{
  "analysis_revision": 2,
  "type": { "name": "Строка", "certainty": 1.0, "facet": "Object" },
  "node": { "kind": "MemberAccess", "range": { "start": { "line": 10, "character": 0 }, "end": { "line": 10, "character": 20 } } },
  "explain": { "reasons": [] }
}
```

### 3.9. `bsl_members`

Member list (completion-like) для receiver в позиции (например для `expr.`).

Input:
```json
{
  "session_id": "uuid",
  "file": { "doc": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "text": "optional" },
  "position": { "line": 10, "character": 12 },
  "limit": 200
}
```

Output:
```json
{
  "analysis_revision": 2,
  "receiver": { "type": { "name": "ДокументОбъект.ЗаказПокупателя", "facet": "Object" } },
  "members": [
    { "name": "Записать", "kind": "method", "signature": "Записать()", "return_type": "Булево", "deprecated": false }
  ],
  "truncated": false
}
```

### 3.10. `bsl_definition`

Definition по `symbol_id` или по позиции.

Input (вариант A):
```json
{ "session_id": "uuid", "symbol_id": "hex" }
```

Input (вариант B):
```json
{ "session_id": "uuid", "file": { "doc": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "text": "optional" }, "position": { "line": 10, "character": 5 } }
```

Output:
```json
{
  "analysis_revision": 2,
  "location": { "file": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } } },
  "snippet": { "text": "bounded snippet", "truncated": false }
}
```

### 3.11. `bsl_references`

References по `symbol_id`.

Input:
```json
{ "session_id": "uuid", "symbol_id": "hex", "limit": 200, "include_snippets": false }
```

Output:
```json
{
  "analysis_revision": 2,
  "count": 12,
  "references": [
    { "file": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "range": { "start": { "line": 1, "character": 1 }, "end": { "line": 1, "character": 5 } } }
  ],
  "truncated": false
}
```

### 3.12. `context_pack` (главный)

Input:
```json
{
  "session_id": "uuid",
  "goal": "Fix the most impactful type errors",
  "focus": { "kind": "diagnostic", "diagnostic_id": "hex" },
  "budget_tokens": 1800,
  "include": { "snippets": true, "types": true, "references": true, "metadata": true, "impact": true, "coverage": true }
}
```

Output (идея):
```json
{
  "analysis_revision": 2,
  "pack_id": "hex",
  "text": "LLM-ready pack (bounded)",
  "items": [
    { "item_id": "hex", "kind": "snippet", "file": { "root_id": "hex", "path": "src/CommonModules/Foo/Module.bsl" }, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } }, "summary": "..." }
  ],
  "truncated": false,
  "completeness": "full",
  "missing_inputs": []
}
```

### 3.13. `context_expand`

Расширить конкретный item из `context_pack`.

Input:
```json
{ "session_id": "uuid", "pack_id": "hex", "item_id": "hex", "budget_tokens": 1200 }
```

Output:
```json
{ "analysis_revision": 2, "text": "expanded content", "truncated": false }
```

---

## 4) Resources (минимум)

Resources полезны для интерактивных клиентов, но в MVP достаточно минимального набора:

- `bsl://status?session_id=...` — статус и warnings
- `bsl://diagnostics?session_id=...&scope=...` — текущие diagnostics summary (опционально)

---

## 5) Prompts (MVP)

Prompts — “шаблоны” для UI-хостов:

- `analyze-bsl` — анализ проблемы по `context_pack`
- `fix-type-errors` — пошаговое исправление с учётом gradual typing и impact
