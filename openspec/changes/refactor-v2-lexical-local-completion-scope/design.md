## Context
v2 completion уже использует единый snapshot для типовых операций, но non-member локальные кандидаты берутся из файлового symbol index. Это даёт завышенную область видимости и снижает точность подсказок.

В проекте уже есть необходимые строительные блоки:
- `SemanticProgram` и иерархия scopes,
- byte offset привязка к позиции курсора,
- узлы `VariableDeclaration`/`Assignment`/параметры, пригодные для вычисления локальной видимости.

## Goals / Non-Goals
- Goals:
  - давать локальные completion candidates, строго релевантные позиции курсора;
  - сделать правила видимости детерминированными и тестируемыми;
  - не ухудшить latency completion в hot path.
- Non-Goals:
  - не переписывать member-access completion;
  - не менять внешний LSP-контракт completion/resolve.

## Decisions
- Decision: использовать IR-first сбор локалов (on-demand на запрос completion) как источник local candidates.
  - Why:
    - IR уже отражает scope-и и позицию, что устраняет рассинхронизацию с индексом;
    - минимальные изменения в архитектуре ranking/dedup.
- Decision: оставить module/global/meta/keywords в существующем pipeline индексов.
  - Why:
    - это снижает риск регрессий вне локальной видимости;
    - локализует изменение только в local-слое.
- Decision: ввести явный алгоритм разрешения конфликтов имён:
  - nearest scope wins;
  - при равной области — latest declaration before cursor wins.
- Decision: не вводить runtime toggle совместимости для legacy file-wide локалов.
  - Why:
    - stricter behavior принимается как целевое поведение v2;
    - отсутствие toggle снижает сложность runtime и тестовой матрицы;
    - уменьшает риск долгоживущего legacy-режима и расхождения поведения.

## Alternatives Considered
- Option A: расширить файловый symbol index блоковыми диапазонами и фильтровать там.
  - Минусы: дублирование semantic-логики scopes, высокий риск рассинхронизации с IR.
- Option B (chosen): IR-first локальный сбор на запрос completion.
  - Плюсы: единый источник истины, меньше дублирования, проще корректность.
- Option C: отдельный precomputed lexical index.
  - Минусы: усложнение инкрементальности и invalidation.

## Risks / Trade-offs
- Риск latency при линейном сканировании IR на каждый completion.
  - Mitigation: фильтр только по видимым scope + `span.start <= cursor`, ранний выход, профилирование.
- Риск поведенческой регрессии (меньше подсказок, чем раньше).
  - Mitigation: regression tests и явная спецификация expected behavior.
- Риск divergence между hover/definition и completion.
  - Mitigation: reuse одного helper определения текущего scope/offset.

## Migration Plan
1. Добавить helper позиция→scope и локальный сборщик кандидатов.
2. Интегрировать в non-member ветку completion.
3. Добавить тесты на блоки/позицию/затенение.
4. Сверить снапшоты completion и зафиксировать изменения как intentional.

## Open Questions
- Нет открытых вопросов.
