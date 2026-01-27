# Tasks: fix-conf-big-intellisense-regressions

## 1. Spec delta + validate
- [x] Добавить delta к `openspec/changes/fix-conf-big-intellisense-regressions/specs/bsl-intellisense-ide-grade/spec.md`.
- [x] `openspec validate fix-conf-big-intellisense-regressions --strict --no-interactive`

## 2. Formatting: убрать `-32600` на save
- [x] Привести gating capabilities к реальному состоянию: если форматирование выключено — не заявлять `documentFormattingProvider`/`documentRangeFormattingProvider`.
- [x] Если клиент всё равно вызвал formatting при выключенном режиме — отвечать предсказуемо (без `INVALID_REQUEST`).
- [x] Регресс‑тест/интеграционный тест LSP: форматирование выключено + запрос → нет `-32600`.

## 3. Локальные функции/процедуры: вызов до объявления
- [x] Обеспечить, что локальные функции/процедуры модуля доступны для резолвинга как callable до анализа тела (hoisting).
- [x] Регресс‑тест: вызов `СписокВидимыхТабличныхЧастей()` до объявления не создаёт диагностику `UndeclaredVariable`.

## 4. Definition по common modules конфигурации
- [x] Добавить резолвинг `CommonModules.<Name>` и `CommonModules.<Name>.<ExportProc>` в `textDocument/definition`.
- [x] Регресс‑тест: `РеализацияТоваровУслугФормы.ПриСозданииНаСервере` → `.../CommonModules/РеализацияТоваровУслугФормы/Ext/Module.bsl`.

## 5. Формы: реквизиты и элементы
- [x] Подмешать `Form.xml` attributes в тип формы/контекст модуля формы (пример: `СчетФактура` имеет тип `cfg:DocumentRef.*`).
- [x] Расширить маппинг элементов формы по kind (пример: `UsualGroup` → `ГруппаФормы`) и включать реальные элементы в `ЭлементыФормы.*`.
- [x] Регресс‑тесты:
  - `Элементы.СчетФактураПросмотр` не даёт “property not exists” для формы `ФормаДокументаОбщая`;
  - hover по `СчетФактура` не остаётся “unknown”.

## 6. Quality gates (apply‑стадия)
- [x] `cargo fmt`
- [x] `cargo clippy -- -D warnings`
- [x] `cargo test`
