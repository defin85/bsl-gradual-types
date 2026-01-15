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

### 2.2. FileRef

```json
{
  "path": "src/CommonModules/Foo/Module.bsl",
  "text": "optional full text (unsaved buffer)",
  "version": 12
}
```

Если `text` отсутствует — читаем файл с диска (в пределах roots).

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

### 3.4. `bsl_diagnostics`

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
  "diagnostics": [
    {
      "diagnostic_id": "hex",
      "file": "path",
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

### 3.5. `bsl_symbol_search`

Поиск символов по имени (для навигации LLM).

Input:
```json
{ "session_id": "uuid", "query": "Документы", "limit": 20 }
```

Output:
```json
{
  "symbols": [
    { "symbol_id": "hex", "name": "Документы", "kind": "namespace", "file": "path", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } } }
  ]
}
```

### 3.6. `bsl_type_at_position`

Тип/разрешение выражения в позиции.

Input:
```json
{
  "session_id": "uuid",
  "file": { "path": "path", "text": "optional" },
  "position": { "line": 10, "character": 5 }
}
```

Output:
```json
{
  "type": { "name": "Строка", "certainty": 1.0, "facet": "Object" },
  "node": { "kind": "MemberAccess", "range": { "start": { "line": 10, "character": 0 }, "end": { "line": 10, "character": 20 } } },
  "explain": { "reasons": [] }
}
```

### 3.7. `bsl_members`

Member list (completion-like) для receiver в позиции (например для `expr.`).

Input:
```json
{
  "session_id": "uuid",
  "file": { "path": "path", "text": "optional" },
  "position": { "line": 10, "character": 12 },
  "limit": 200
}
```

Output:
```json
{
  "receiver": { "type": { "name": "ДокументОбъект.ЗаказПокупателя", "facet": "Object" } },
  "members": [
    { "name": "Записать", "kind": "method", "signature": "Записать()", "return_type": "Булево", "deprecated": false }
  ],
  "truncated": false
}
```

### 3.8. `bsl_definition`

Definition по `symbol_id` или по позиции.

Input (вариант A):
```json
{ "session_id": "uuid", "symbol_id": "hex" }
```

Input (вариант B):
```json
{ "session_id": "uuid", "file": { "path": "path", "text": "optional" }, "position": { "line": 10, "character": 5 } }
```

Output:
```json
{
  "location": { "file": "path", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } } },
  "snippet": { "text": "bounded snippet", "truncated": false }
}
```

### 3.9. `bsl_references`

References по `symbol_id`.

Input:
```json
{ "session_id": "uuid", "symbol_id": "hex", "limit": 200, "include_snippets": false }
```

Output:
```json
{
  "count": 12,
  "references": [
    { "file": "path", "range": { "start": { "line": 1, "character": 1 }, "end": { "line": 1, "character": 5 } } }
  ],
  "truncated": false
}
```

### 3.10. `context_pack` (главный)

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
  "pack_id": "hex",
  "text": "LLM-ready pack (bounded)",
  "items": [
    { "item_id": "hex", "kind": "snippet", "file": "path", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } }, "summary": "..." }
  ],
  "truncated": false,
  "completeness": "full",
  "missing_inputs": []
}
```

### 3.11. `context_expand`

Расширить конкретный item из `context_pack`.

Input:
```json
{ "session_id": "uuid", "pack_id": "hex", "item_id": "hex", "budget_tokens": 1200 }
```

Output:
```json
{ "text": "expanded content", "truncated": false }
```

---

## 4) Resources (минимум)

Resources полезны для интерактивных клиентов, но в MVP достаточно минимального набора:

- `bsl://status?session_id=...` — статус и warnings
- `bsl://diagnostics?session_id=...&scope=...` — текущие diagnostics summary (опционально)

---

## 5) Prompts (MVP)

Prompts — “шаблоны” для UI-хостов:

- `analyze-bsl` — анализ проблемы по `context.pack`
- `fix-type-errors` — пошаговое исправление с учётом gradual typing и impact

