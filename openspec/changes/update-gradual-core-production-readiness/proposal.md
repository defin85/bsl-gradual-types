# Change: update-gradual-core-production-readiness

## Why
Проект уже имеет сильную базу gradual typing: явные уровни certainty, controlled degradation, drift-prevention тесты и правильно сформулированную целевую архитектуру shared resolved path.

Но текущее состояние ядра ещё не дотягивает до production-grade shared truth:
- snapshot-local structural knowledge для typed `Структура` и typed-row ещё не выражено first-class в общем resolved contract;
- часть consumer-путей всё ещё содержит локальные смысловые ветки, особенно в completion;
- acceptance и traceability артефакты могут завышать фактическую готовность change по отношению к MUST-требованиям;
- процесс допускает расхождение между состоянием OpenSpec change и реальным P1 backlog в Beads.

Нужно отдельно зафиксировать future-facing contract, который превратит сильную платформу gradual typing в действительно общую и операционно честную систему.

## What Changes
- Зафиксировать для `bsl-intellisense-v2`, что shared resolved contract MUST first-class нести snapshot-local structural member knowledge.
- Зафиксировать, что semantic consumers и adapter surfaces MUST читать один и тот же resolved path, а локальные ветки допускаются только как thin adapters без собственной semantic truth.
- Зафиксировать более жёсткий acceptance contract: cross-consumer consistency должна доказывать shared semantics, а не только smoke-level отсутствие явного drift.
- Зафиксировать для `dev-workflow` readiness gate против “отчётного самообмана”: change нельзя считать complete, если по его MUST-требованиям ещё открыт критический follow-up backlog.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `dev-workflow`
- Affected code (future expected):
  - `bsl-types/src/types/*`
  - `shared/src/domain/metadata_lookup/*`
  - `analysis-v2/src/type_inference_v2.rs`
  - `bsl-runtime/src/application/type_system/services/*`
  - acceptance / parity / traceability tooling

## Notes
- Этот change intentionally future-facing и фиксирует архитектурную цель, а не немедленную реализацию.
- Он не заменяет текущий epic `bsl-gradual-types-cb6`, а сохраняет более широкий вывод “что ещё отделяет сильную платформу от production-grade ядра”.
