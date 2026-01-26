# Tasks: fix-conf-big-intellisense-regressions

## 1. Spec delta + validate
- [ ] Добавить delta к `openspec/changes/fix-conf-big-intellisense-regressions/specs/bsl-intellisense-ide-grade/spec.md`.
- [ ] `openspec validate fix-conf-big-intellisense-regressions --strict --no-interactive`

## 2. Formatting: убрать `-32600` на save
- [ ] Привести gating capabilities к реальному состоянию: если форматирование выключено — не заявлять `documentFormattingProvider`/`documentRangeFormattingProvider`.
- [ ] Если клиент всё равно вызвал formatting при выключенном режиме — отвечать предсказуемо (без `INVALID_REQUEST`).
- [ ] Регресс‑тест/интеграционный тест LSP: форматирование выключено + запрос → нет `-32600`.

## 3. Локальные функции/процедуры: вызов до объявления
- [ ] Обеспечить, что локальные функции/процедуры модуля доступны для резолвинга как callable до анализа тела (hoisting).
- [ ] Регресс‑тест: вызов `СписокВидимыхТабличныхЧастей()` до объявления не создаёт диагностику `UndeclaredVariable`.

## 4. Definition по common modules конфигурации
- [ ] Добавить резолвинг `CommonModules.<Name>` и `CommonModules.<Name>.<ExportProc>` в `textDocument/definition`.
- [ ] Регресс‑тест: `РеализацияТоваровУслугФормы.ПриСозданииНаСервере` → `.../CommonModules/РеализацияТоваровУслугФормы/Ext/Module.bsl`.

## 5. Формы: реквизиты и элементы
- [ ] Подмешать `Form.xml` attributes в тип формы/контекст модуля формы (пример: `СчетФактура` имеет тип `cfg:DocumentRef.*`).
- [ ] Расширить маппинг элементов формы по kind (пример: `UsualGroup` → `ГруппаФормы`) и включать реальные элементы в `ЭлементыФормы.*`.
- [ ] Регресс‑тесты:
  - `Элементы.СчетФактураПросмотр` не даёт “property not exists” для формы `ФормаДокументаОбщая`;
  - hover по `СчетФактура` не остаётся “unknown”.

## 6. Quality gates (apply‑стадия)
- [ ] `cargo fmt`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`

