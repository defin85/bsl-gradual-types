## Context
Сейчас LSP уже имеет debounce для diagnostics, но при активном редактировании остаётся проблема:
- старая тяжёлая задача может продолжать CPU-работу для устаревшей версии;
- в логах видны последовательные slow `syntax_diagnostics` для нескольких соседних версий;
- интерактивный путь деградирует из-за конкуренции с фоновыми пересчётами.

В существующем контракте уже есть strict-latest publish и канонический event model, но не зафиксирована tiered-модель запуска проверок на `didChange` vs `didSave/idle`.

## Goals / Non-Goals
- Goals:
  - Ограничить `didChange` дешёвыми шагами.
  - Перенести тяжёлые проверки в `debounced`/`idle`/`didSave` профили.
  - Обеспечить обязательную отмену устаревших задач по версии/поколению до publish.
  - Сохранить единый канонический observability контракт для LSP и MCP.
  - Добиться измеримого эффекта на `bsl-agent` через shared runtime policy.
- Non-Goals:
  - Перевод observability в отдельный внешний backend (Prom/OTel) в рамках этого change.
  - Полный редизайн API LSP/MCP.
  - High-cardinality метрики (URI/путь/символы в ключах).

## Архитектурное решение (рекомендуемое)
Применяется модель `canonical event model + projection`, совместимая с текущим dual-write контрактом.

### 1) Tiered diagnostics pipeline
- `fast` профиль (`didChange`, `didOpen`):
  - только дешёвые инкрементальные шаги;
  - не выполняет heavy-семантику.
- `debounced_full` профиль (`didChange` с задержкой):
  - выполняет полный набор базовых diagnostics;
  - запускается в background CPU class.
- `idle_heavy` профиль (`didSave` и/или `idle`):
  - выполняет дорогие проверки (flow-sensitive/cross-file и т.п.);
  - не активируется на каждый символ.

### 2) Revision-bound supersede/cancellation
- Для файла вводится `diagnostics_generation` (или эквивалентный revision token), растущий при новых релевантных событиях.
- Каждый запуск diagnostics захватывает `(file_version, deps_id, settings_id, generation)` и проверяет актуальность:
  - перед входом в дорогую стадию;
  - перед publish.
- При обнаружении устаревания выполнение MUST завершаться как `superseded`, без публикации.

### 3) Publish gate
- Публикация разрешена только при полном совпадении актуального token:
  - `file_version`;
  - `deps_id`;
  - `settings_id`;
  - `diagnostics_generation`.

### 4) bsl-agent alignment
- `documents_set` обновляет revision и помечает активные batch-job как stale/superseded.
- Интерактивные инструменты (`type_at_position`, `members`, `definition`) используют fast/interactive профиль.
- Batch/scanning операции (`diagnostics project`, `symbol search`, `references`) остаются в background/deferred профиле и не должны блокировать интерактивный прогресс.

### 5) Observability additions (low-cardinality)
- Канонические dimensions дополняются фиксированными значениями:
  - `trigger`: `did_change|did_open|did_save|idle|documents_set|job_start`;
  - `profile`: `fast|debounced_full|idle_heavy`;
  - `reason`: минимум `superseded_version|superseded_generation|cancelled|published`.
- Сохраняется dual-write: drilldown как primary, legacy как deterministic projection.

## Alternatives Considered
### A. Только увеличить debounce
- Плюс: минимальные правки.
- Минус: не решает проблему уже запущенных устаревших тяжёлых задач.

### B. Полная отмена diagnostics на каждый `didChange`
- Плюс: меньше фона.
- Минус: ухудшение качества обратной связи, рост вероятности пустых/устаревших данных.

### C. Рекомендуемая: tiered pipeline + revision-bound supersede
- Плюс: баланс latency и качества; контролируемый rollout; переносимо на MCP через shared runtime.
- Минус: больше orchestration логики и тестов.

## Risks / Trade-offs
- Риск: недопубликация полезных diagnostics при слишком агрессивном supersede.
  - Mitigation: чёткие профили и минимальный гарантированный publish для `debounced_full`.
- Риск: рост сложности планировщика.
  - Mitigation: централизовать policy в shared runtime и покрыть контрактными тестами.
- Риск: divergence LSP и MCP поведения.
  - Mitigation: общий canonical contract + parity tests по observability и mixed-load.

## Rollout и quality gates
1. Включить новую политику за feature-flag/runtime key (по умолчанию безопасный режим).
2. Прогнать regression сценарии `cold/warm` и mixed-load.
3. Проверить SLO и invariants:
   - снижение хвоста `syntax_diagnostics_query` в warm интерактивном сценарии;
   - отсутствие publish stale diagnostics;
   - интерактивные MCP tools не starve под batch-нагрузкой.
