## 1. Proposal / Design
- [x] 1.1 Уточнить модель spans в Syntax AST: какие поля добавляем (header/body/branch spans) и как сохраняем обратную совместимость (serde defaults).
- [x] 1.2 Зафиксировать правила для всех `CfgNodeKind::Conditional` (включая `TryExcept`): какие marker-узлы создаём и какие EdgeKind используем для веток.

## 2. Tree-sitter spans в Syntax (syntax/)
- [x] 2.1 Расширить AST statements `If`/`While`/`For`/`ForEach`/`Try` дополнительными spans для header/body/branch (Option + serde default).
- [x] 2.2 В tree_sitter_adapter вычислять spans тела веток/циклов/try-except по позициям keyword-нодов (`THEN/ELSE/ENDIF`, `DO/ENDDO`, `TRY/EXCEPT/ENDTRY` и русские аналоги).
- [x] 2.3 Обновить fallback-конструктор AST (если используется) так, чтобы новые spans были `None` или best-effort, не ломая существующее поведение.

## 3. CFG builder v2 uses structural spans (analysis-v2/)
- [x] 3.1 Удалить строковые эвристики вычисления spans (`compute_if_spans`/`compute_loop_spans`) и перейти на spans из Syntax AST.
- [x] 3.2 Для `If`/циклов: header span на `Conditional`/`LoopHeader`, body spans на marker-узлы тела/веток, marker-узлы создаются всегда (включая пустые ветки).
- [x] 3.3 Для `TryExcept`: представить конструкцию как `CfgNodeKind::Conditional` + marker-узлы `try`/`except` с корректными body spans, чтобы контракт “все Conditional” выполнялся.
- [x] 3.4 Для условий: извлекать текст условия из span выражения condition (tree-sitter), чтобы type guard `ТипЗнч(...) = Тип("...")` извлекался без header-эвристик.

## 4. LSP integration test (backend/)
- [x] 4.1 Добавить интеграционный тест completion: `Если ТипЗнч(x)=Тип("ТаблицаЗначений") Тогда x.` → в выдаче есть `Колонки` (и проверка стабильности на позиции после `.`).
- [x] 4.2 Добавить вариант теста с вложенным `Если/Иначе` (nested else), чтобы зафиксировать отсутствие регрессии в spans/ветвлении.

## 5. Validation
- [x] 5.1 `openspec validate refactor-flow-sensitive-cfg-spans-tree-sitter --strict --no-interactive`.
- [x] 5.2 `cargo test -p bsl-syntax -p bsl-analysis-v2 -p bsl-backend` (и при необходимости `cargo test --workspace`).
