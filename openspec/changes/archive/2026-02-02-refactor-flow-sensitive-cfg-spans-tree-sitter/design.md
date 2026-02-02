## Контекст
CFG v2 используется как опорный слой для flow-sensitive (narrowing / null-safety) и для устойчивого выбора контекста по позиции (completion/hover/diagnostics).
Предыдущий change стабилизировал:
- наличие CFG в snapshot;
- единый API mapping `byte offset → CFG node` с bias;
- учёт loop header в null-safety.

Но качество mapping всё ещё зависит от корректности spans CFG узлов. Сейчас spans для `If`/циклов вычисляются в `analysis-v2` эвристически (по строкам), что:
- ломается на вложенных `Иначе/ИначеЕсли` и нестандартном форматировании;
- не покрывает `TryExcept`, хотя он тоже представлен как `CfgNodeKind::Conditional`.

## Цели
1) Spans CFG узлов должны быть структурно корректными и воспроизводимыми из дерева разбора.
2) Контракт header/body должен распространяться на все `CfgNodeKind::Conditional`, включая `TryExcept`.
3) Нужен интеграционный LSP тест, который проверяет “реальный” сценарий: completion на `x.` в then-ветке после `ТипЗнч`-guard использует narrowed type.

## Решение: вычислять body/branch spans на уровне tree-sitter adapter (syntax/)
### Почему так лучше, чем “перепарсить” в analysis-v2
- `analysis-v2` получает уже сконвертированный Syntax AST (`bsl_syntax::ast::Program`) и строку исходника.
- В текущей архитектуре дерево tree-sitter не прокидывается в v2 pipeline; перепарсить текст в `analysis-v2` ради spans — это лишняя работа на hot-path.
- tree_sitter_adapter уже проходит по Node-дереву и видит keyword-ноды (`THEN/ELSE/ENDIF`, `DO/ENDDO`, `TRY/EXCEPT/ENDTRY`) и их byte-диапазоны.

### Модель данных
Минимально расширить нужные `Statement` варианты в `syntax/src/ast.rs`:
- `If`: добавить `header_span`, `then_span`, `else_span` (Option для else).
- `While`/`For`/`ForEach`: добавить `header_span`, `body_span`.
- `Try`: добавить `try_span`, `except_span` (и при необходимости `header_span` как span keyword `TRY`).

Технические требования:
- новые поля должны быть `Option<Span>` и иметь `#[serde(default)]`, чтобы не ломать существующие сериализации/десериализации;
- для источников AST не из tree-sitter (fallback) допустимы `None` (понижение качества spans); основной контракт — для v2 pipeline через tree-sitter.

## CFG builder: убрать строковые эвристики и использовать структурные spans
### If / loops
- `Conditional`/`LoopHeader` получают span header (до `Тогда/Цикл`).
- Отдельные marker-узлы получают body span (между keyword’ами).
- Marker-узлы создаются всегда, даже если список statement’ов пустой, чтобы mapping в “пустое тело” не деградировал.

### Try/Except как Conditional
Чтобы выполнить “все Conditional”:
- представляем `TryExcept` как `CfgNodeKind::Conditional { condition: "exception" }` (как сейчас),
  но добавляем marker-узлы `try_body` и `except_body` со spans тел и делаем ветвление через `ConditionalTrue/ConditionalFalse`.
- `merge` остаётся как join-узел.

Это не требует нового `CfgNodeKind`, остаётся в текущей модели, но делает spans/ветки “похожими” на `If`.

## Условие (condition text): извлекать из span выражения
Для type guards (в т.ч. `ТипЗнч(x)=Тип("...")`) лучше извлекать текст условия из `Expression.span` в Syntax AST, а не из header-строки.
Это:
- уменьшает зависимость от форматирования (`Если`/`Тогда`/переводы строк);
- делает извлечение type guards более точным.

## Тестовая фиксация: интеграционный LSP completion тест
Добавить тест (backend) в стиле существующих M8 incremental completion тестов:
- код содержит `Если ТипЗнч(x) = Тип("ТаблицаЗначений") Тогда x.` внутри процедуры;
- completion вызывается на позиции сразу после `.`;
- ожидание: среди label есть `Колонки` (платформенный member для `ТаблицаЗначений`);
- добавить вариант с вложенным `Если/Иначе` внутри else-ветки, чтобы зафиксировать корректность spans на nested else.

## Риски и компромиссы
- Изменение Syntax AST затронет match’и в нескольких местах (analysis-v2, backend handlers, runtime, тесты). Это приемлемо, так как изменения локальны и несут явную пользу.
- При `None` spans (fallback) качество mapping может быть ниже. В v2 pipeline (tree-sitter) должно быть `Some` для новых spans.
- Если потребуется, можно расширить `node_at_byte_offset` индексами по интервалам, но это не цель данного change.

