# Change: add-bsl-lsp-inlay-hints-and-code-actions

## Why
Сейчас VS Code extension сознательно не регистрирует custom providers для inlay hints и code actions (чтобы не было заглушек), и полагается на стандартный LSP pipeline.
Однако LSP сервер пока не объявляет `inlayHintProvider` / `codeActionProvider` и не реализует `textDocument/inlayHint` / `textDocument/codeAction`, поэтому эти IDE-grade возможности фактически недоступны.

## What Changes
- Добавить в LSP сервер поддержку:
  - `textDocument/inlayHint` (inlay hints по типам),
  - `textDocument/codeAction` (MVP code actions),
  - соответствующие capabilities `inlayHintProvider` / `codeActionProvider`.
- Сделать поддержку конфигурируемой и предсказуемой:
  - capability объявляется только когда фича включена и реально реализована,
  - иначе сервер не заявляет поддержку (или возвращает предсказуемый отказ, если клиент всё равно вызвал метод).
- Зафиксировать MVP-границы: что именно считается “осмысленным результатом” для hints/actions.

## Impact
- Спецификации:
  - `openspec/specs/bsl-intellisense/spec.md` (delta: inlay hints + code actions).
  - Согласуется с целевыми ожиданиями в `openspec/specs/bsl-intellisense-ide-grade/spec.md`.
- Код:
  - `backend/src/bin/lsp_server/server/language_server.rs` (capabilities + handlers),
  - новые/расширенные модули обработчиков и настройки (конфигурация `bsl.typeHints.*` и `bsl.codeActions.*`).
- Тесты:
  - интеграционные тесты LSP на корректность capabilities и содержимого ответов.

## Assumptions (to be confirmed)
- Inlay hints MVP: типы для локальных переменных (VarDeclaration) и (опционально) return type для функций.
- Code actions MVP: минимум 1 refactor action (например, extract variable) и минимум 1 quick fix, который можно детерминированно вычислить по данным LSP (без парсинга “сообщения ошибки” регулярками).
- Настройки `bsl.typeHints.*` уже существуют в extension; сервер будет читать их из `didChangeConfiguration` (секция `bsl`) и/или из `initializationOptions`.

## Non-goals (MVP)
- Полный набор quick fixes для всех диагностик.
- “Умные” refactors с глубоким анализом cross-file.
- Обещание фичи в IDE при отсутствии соответствующей server capability.

