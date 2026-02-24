## Context
Стадия syntax diagnostics в текущем v2 пути зависит от полного parse на новую ревизию текста. На больших модулях это становится доминирующим алгоритмическим фактором latency.

Цель изменения: уменьшить вычислительную стоимость syntax diagnostics на последовательных ревизиях одного файла без потери корректности и детерминированности.

## Goals / Non-Goals
- Goals:
  - Перейти на incremental parse для последовательных `didChange` ревизий.
  - Сохранить эквивалентность user-facing diagnostics относительно full parse.
  - Сделать root-cause наблюдаемым через hit/miss/fallback метрики.
- Non-Goals:
  - Изменять grammar языка BSL.
  - Изменять существующую message policy синтаксических diagnostics.

## Decisions
- Decision 1: Использовать incremental tree reuse как primary path
  - Для последовательных ревизий одного файла система хранит предыдущее parse tree.
  - На `didChange` применяется edit mapping и запускается incremental parse от предыдущего дерева.
  - Результат становится новым canonical tree текущей ревизии.

  Alternatives considered:
  - Только увеличить debounce heavy diagnostics.
    - Отклонено: уменьшает частоту запусков, но не снижает алгоритмическую стоимость одного запуска.
  - Полная замена parser стеком другого типа.
    - Отклонено: высокий migration risk и большой объем несвязанных изменений.

- Decision 2: Fallback-first correctness policy
  - Если edit mapping некорректен или incremental parse невалиден, система MUST выполнять full parse текущей ревизии.
  - Fallback MUST быть детерминирован и прозрачно наблюдаем через метрики причин.

- Decision 3: Каноническая эквивалентность diagnostics
  - User-facing diagnostics после пост-обработки MUST оставаться эквивалентными full parse контракту для той же ревизии текста.
  - Любые различия допустимы только как внутренние performance детали, но не как изменение semantic/diagnostic контракта.

## Risks / Trade-offs
- Риск: ошибки в edit mapping могут давать непредсказуемое дерево.
  - Mitigation: строгая валидация mapping + немедленный fallback на full parse.
- Риск: хранение деревьев увеличит память.
  - Mitigation: ограничение кэша на открытые документы и очистка при close/removal.
- Риск: смешанный путь (incremental/full) усложнит отладку.
  - Mitigation: observability hit/miss/fallback причины и тесты эквивалентности.

## Migration / Rollout
1. Включить incremental path под feature/runtime flag в report-only режиме.
2. Снять метрики hit/miss/fallback и сравнить с full parse baseline.
3. Перевести incremental path в default после подтвержденной эквивалентности diagnostics.

## Open Questions
- Нужен ли отдельный лимит на максимальный диапазон edit для incremental path перед принудительным full parse.
- Нужно ли хранить промежуточные trees для rollback между несколькими конкурентными версиями документа.
