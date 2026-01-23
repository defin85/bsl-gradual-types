# Design: audit IntelliSense coverage for BSL (1С)

## 0) Scope
Цель аудита: проверить по исходникам, какие функции IntelliSense реально поддерживаются в текущем репозитории (LSP‑сервер + VS Code extension), и сопоставить это с “современным” чек‑листом IntelliSense для 1С.

**Источник чек‑листа**: перечень функций из текущего диалога (completion/signature/hover/definition/references/rename/diagnostics/code actions/formatting/etc.).

## 1) Методика (evidence-driven)
1) Смотрим, что LSP‑сервер объявляет в `ServerCapabilities` (это контракт с IDE).
2) Проверяем, что соответствующие LSP методы реально реализованы.
3) Для VS Code extension фиксируем: что реально включено, и где есть заглушки (providers, которые зарегистрированы, но возвращают пусто).
4) Для “частичных” фич фиксируем ограничения (например, definition для типов, а не для всех символов).

## 2) Audit‑матрица (текущее состояние)

Легенда статусов:
- ✅ Implemented — есть контракт + реализация (и часто тесты).
- 🟡 Partial — есть часть функциональности/ограничение по scope.
- 🧪 Stub — зарегистрировано/заявлено, но фактически пустая заглушка.
- ❌ Missing — отсутствует (не заявлено/не реализовано).

### 2.1 LSP server (core IntelliSense)

1) **Completion (автодополнение)**
- Статус: ✅ Implemented
- Evidence:
  - `ServerCapabilities.completion_provider` включён: `backend/src/bin/lsp_server/server/language_server.rs:106`
  - handler: `backend/src/bin/lsp_server/handlers/completion.rs:1`
  - поддержка `completionItem/resolve`: `backend/src/bin/lsp_server/server/language_server.rs:874`
  - есть идентификация кандидата (`candidate_id`) в `CompletionItem.data`: `backend/src/bin/lsp_server/handlers/completion.rs:243`
- Notes:
  - триггеры completion: `"."`, `"("` (`backend/src/bin/lsp_server/server/language_server.rs:121`).

2) **Signature Help (подсказки сигнатур)**
- Статус: ✅ Implemented
- Evidence:
  - `ServerCapabilities.signature_help_provider` включён: `backend/src/bin/lsp_server/server/language_server.rs:143`
  - handler: `backend/src/bin/lsp_server/handlers/signature_help.rs:34`

3) **Hover / Quick Info**
- Статус: ✅ Implemented
- Evidence:
  - `ServerCapabilities.hover_provider` включён: `backend/src/bin/lsp_server/server/language_server.rs:119`
  - handler: `backend/src/bin/lsp_server/handlers/hover.rs:19`
- Notes:
  - hover использует platform docs (syntax helper) при наличии `BSL_SYNTAX_HELPER_PATH` или стандартных путей: `backend/src/bin/lsp_server/handlers/hover.rs:28`.

4) **Go to Definition**
- Статус: 🟡 Partial
- Evidence:
  - `ServerCapabilities.definition_provider` включён: `backend/src/bin/lsp_server/server/language_server.rs:120`
  - handler: `backend/src/bin/lsp_server/handlers/definition.rs:1`
- Notes:
  - по комментарию и сигнатурам это “type definitions” (platform/config types), а не универсальная навигация по всем символам (процедуры/функции/локальные переменные).

5) **Diagnostics (ошибки/предупреждения на лету)**
- Статус: ✅ Implemented (push‑diagnostics)
- Evidence:
  - сервер публикует diagnostics через `publish_diagnostics`: `backend/src/bin/lsp_server/server/language_server.rs:585`
  - v2 pipeline обновляет текст и планирует диагностику на `didChange`: `backend/src/bin/lsp_server/server/language_server.rs:558`
- Notes:
  - `diagnostic_provider` в `ServerCapabilities` не объявлен (`None`), но для push‑модели это не обязательно.

6) **Document/Workspace Symbols (outline, “Go to Symbol”, индексация символов)**
- Статус: ❌ Missing
- Evidence:
  - в `ServerCapabilities` нет `document_symbol_provider`/`workspace_symbol_provider`: `backend/src/bin/lsp_server/server/language_server.rs:106`
  - в `backend/src/bin/lsp_server/server/language_server.rs` отсутствуют реализации `document_symbol`/`workspace_symbol`.

7) **Find References**
- Статус: ❌ Missing
- Evidence:
  - в `ServerCapabilities` нет `references_provider`: `backend/src/bin/lsp_server/server/language_server.rs:106`
  - нет handler’ов/методов `references` в `backend/src/bin/lsp_server/`.

8) **Rename**
- Статус: ❌ Missing
- Evidence:
  - в `ServerCapabilities` нет `rename_provider`: `backend/src/bin/lsp_server/server/language_server.rs:106`
  - нет реализации `rename` в `backend/src/bin/lsp_server/`.

9) **Code Actions / Quick Fixes**
- Статус: ❌ Missing (на уровне LSP)
- Evidence:
  - в `ServerCapabilities` нет `code_action_provider`: `backend/src/bin/lsp_server/server/language_server.rs:106`

10) **Formatting**
- Статус: ❌ Missing
- Evidence:
  - нет `document_formatting_provider`/`document_range_formatting_provider` в `ServerCapabilities`: `backend/src/bin/lsp_server/server/language_server.rs:106`
  - нет соответствующих LSP методов в `backend/src/bin/lsp_server/server/language_server.rs`.

### 2.2 VS Code extension (IDE integration)

1) **Запуск и интеграция LSP**
- Статус: ✅ Implemented
- Evidence:
  - запуск LSP клиента при активации: `vscode-extension/src/extension.ts:141`

2) **Enhanced Providers (inlay hints / code actions / enhanced diagnostics)**
- Inlay hints:
  - Статус: 🧪 Stub
  - Evidence: provider зарегистрирован: `vscode-extension/src/setup/providers.ts:27`, но возвращает пусто: `vscode-extension/src/providers/typeHintsProvider.ts:15`.
- Code actions:
  - Статус: 🧪 Stub
  - Evidence: provider зарегистрирован: `vscode-extension/src/setup/providers.ts:41`, но возвращает пусто: `vscode-extension/src/providers/codeActionsProvider.ts:15`.
- Enhanced diagnostics (отдельная коллекция/статистика):
  - Статус: 🧪 Stub/Partial
  - Evidence: класс существует, но `getDiagnosticsStats()` всегда нули: `vscode-extension/src/providers/enhancedDiagnosticsProvider.ts:23`.

3) **Дополнительный UX поверх LSP (custom commands / webviews)**
- Статус: ✅ Implemented (как отдельные команды/UI)
- Evidence:
  - LSP `executeCommand` объявляет команды `bsl.getAllTypes`, `bsl.getSemanticHtml`, `bsl.getSemanticTree`, ...: `backend/src/bin/lsp_server/server/language_server.rs:127`
  - extension вызывает часть из них через `workspace/executeCommand` (см. `vscode-extension/src/lsp/customRequests.ts`).

## 3) Сопоставление с “современным IntelliSense” (gap summary)

В текущем состоянии проект уже покрывает “ядро” IntelliSense:
- completion (+ resolve), hover, signature help, go‑to‑definition (частично), real‑time diagnostics.

Ключевые gaps до IDE‑grade по общему ожиданию:
- find references, rename, symbols (outline/workspace), formatting.
- code actions/quick fixes (сейчас провайдеры‑заглушки в extension).
- inlay hints/type hints (сейчас заглушка).
- отдельная поддержка языка запросов 1С внутри строк (если требуется как фича IDE) — в коде не обнаружен отдельный анализатор для query‑строк.

## 4) Follow-ups (вне scope этого change)
Рекомендуется завести отдельные change‑proposal’ы на:
1) `add-bsl-lsp-symbols` (DocumentSymbol/WorkspaceSymbol).
2) `add-bsl-lsp-references-and-rename` (References/Rename).
3) `add-bsl-vscode-code-actions` (Quick Fix/Refactor actions, убрать заглушки).
4) `add-bsl-formatting` (formatter, хотя бы базовый).
5) (Опционально) `add-1c-query-language-support` (если хотим IDE‑подсказки/диагностику внутри строк запросов).
