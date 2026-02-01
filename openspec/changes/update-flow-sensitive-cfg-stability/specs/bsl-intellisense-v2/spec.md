## ADDED Requirements

### Requirement: `SemanticProgram.cfg` всегда присутствует в v2 snapshot (MUST)
Система MUST всегда включать CFG в v2 snapshot как `SemanticProgram.cfg = Some(ControlFlowGraph)`, даже если в файле нет исполняемых конструкций.

Минимальный CFG в таком случае MUST содержать:
- узлы `Entry` и `Exit`,
- ребро `Entry -> Exit`.

#### Scenario: CFG присутствует для файлов без исполняемых конструкций
- **GIVEN** пользователь открывает `.bsl` файл, который содержит только объявления (или пустые тела процедур/функций)
- **WHEN** система строит v2 snapshot (IR) для hover/completion/diagnostics
- **THEN** `SemanticProgram.cfg` присутствует и содержит как минимум `Entry -> Exit`
- **AND** flow-sensitive операции (narrowing / null-safety) не требуют fallback на отсутствие CFG

### Requirement: Привязка “позиция → CFG узел” детерминирована и bias-aware (MUST)
Система MUST иметь единый детерминированный алгоритм выбора CFG узла по byte offset, используемый всеми flow-sensitive consumers (hover/completion/diagnostics).

Алгоритм MUST:
- выбирать “самый специфичный” (наиболее узкий) узел, чей span содержит позицию;
- поддерживать bias для случаев, когда позиция находится на границе токена (например, completion на `.` предпочитает выражение слева).

#### Scenario: Completion на `.` выбирает владельца слева
- **GIVEN** пользователь вводит `x.` в теле условной ветки
- **WHEN** IDE запрашивает completion в позиции сразу после `.`
- **THEN** система выбирает CFG узел/контекст, соответствующий выражению слева (`x`), а не произвольный соседний узел
- **AND** результат детерминирован при повторных запросах

### Requirement: Flow-sensitive null-safety учитывает null-check в заголовках циклов (MUST)
Система MUST учитывать null-check условия в заголовках циклов (например, `Пока x <> Null Цикл`) при выполнении flow-sensitive null-safety анализа в v2 pipeline.

#### Scenario: Null-check в `Пока` подавляет warning в теле
- **GIVEN** переменная `x` потенциально nullable
- **WHEN** код содержит цикл `Пока x <> Null Цикл ... x.Method() ... КонецЦикла`
- **THEN** flow-sensitive null-safety не выдаёт предупреждение о возможном Null‑dereference для `x.Method()` внутри тела цикла

