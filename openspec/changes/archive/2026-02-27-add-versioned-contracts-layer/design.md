## Context

Сейчас внешние интерфейсы (в первую очередь LSP completion и observability labels для completion v2) контролируются через код и тесты, но не имеют явного versioned contract слоя.
Это создаёт риск "тихого" дрейфа публичной поверхности: изменение лейблов, outcome semantics или shape payload может пройти как обычный рефакторинг.

## Goals / Non-Goals

- Goals:
  - Ввести единый versioned contract слой `contracts/**`.
  - Зафиксировать compatibility policy и version bump rules.
  - Снизить риск неявных breaking changes для LSP/observability surface.
- Non-Goals:
  - Полностью покрыть в первом шаге все API/метрики проекта.
  - Заменить существующие интеграционные тесты контрактами.
  - Перепроектировать runtime/LSP архитектуру.

## Decisions

### Decision 1: `contracts/**` становится обязательным источником истины для внешних интерфейсов

Внешняя поверхность, на которую опираются IDE/интеграции/мониторинг, документируется как versioned contracts:
- путь включает поверхность и версию (`contracts/<surface>/vN/...`);
- внутри версии фиксируются schema/examples/changelog.

### Decision 2: Versioning policy явная и проверяемая

- Non-breaking: обновление внутри той же major версии допустимо при обратной совместимости.
- Breaking: обязательно новый major (`vN -> vN+1`) и миграционная заметка.

### Decision 3: Внедрение поэтапное

Первый обязательный baseline:
- completion v2 contract;
- completion v2 observability contract.

Остальные поверхности (Web/MCP/прочие endpoint contracts) подключаются поэтапно отдельными change или follow-up задачами.

### Decision 4: Контракты валидируются в CI

Quality gates должны проверять:
- schema correctness;
- соответствие compatibility policy;
- запрет breaking changes без version bump.

## Risks / Trade-offs

- Риск роста операционной нагрузки (нужно сопровождать contracts alongside code).
  - Митигация: ограничить стартовый scope и стандартизировать шаблон contract пакета.
- Риск рассинхронизации между кодом и contract файлами.
  - Митигация: CI gate + contract-oriented review checklist.
- Риск "бумажных контрактов" без реального enforcement.
  - Митигация: обязательная проверка version policy в PR/CI.

## Migration Plan

1. Специфицировать структуру `contracts/**` и policy.
2. Добавить baseline contracts для completion v2 и observability completion v2.
3. Подключить CI validation.
4. Расширять покрытие поверхностей поэтапно.

## Open Questions

- Нужна ли единая schema технология для всех surfaces (например, JSON Schema), или допускаются разные форматы с единым compatibility gate.
