# Design: update-gradual-core-production-readiness

## Context
Change больше не является purely future-facing.

После remediation и final-hardening work delivered state выглядит так:
- `ResolutionMetadata.structural_members` остаётся canonical carrier для snapshot-local member knowledge;
- structural member entry несёт explicit `member_id`;
- completion ranking и adapter payloads не теряют structural identity;
- exact acceptance закрывает same-identity, hidden-hint fail-closed и revision-switch leakage;
- implicit module-context owner resolution больше не держит bootstrap-only fallback;
- governance gate wired by default через `.github/workflows/ci.yml`.

## Goals
- Зафиксировать delivered `Requirement -> Code -> Test` contract для shared structural semantics и delivery honesty.
- Убрать stale future-facing wording и bounded-exception prose там, где change уже backed by code/tests/workflow evidence.
- Сохранить архитектурный вывод как canonical record для archive и follow-up work.

## Non-Goals
- Не переписывать runtime на новый carrier поверх уже доставленного `TypeResolution`-centric contract.
- Не возвращать historical wide CI bundle; активный workflow покрывает именно default governance/readiness path.
- Не дублировать весь remediation epic task-by-task внутри design artefact.

## Current Code Signals
- `bsl-types/src/types/certainty.rs` хранит snapshot-local structural members внутри `ResolutionMetadata.structural_members`.
- `bsl-types/src/types/structural_members.rs` задаёт shared carrier для `member_id`, `canonical_name`, `member_type`, `source_span`, `certainty`.
- `analysis-v2/src/type_inference_v2/tests.rs` покрывает alias/update/merge lifecycle для typed `Структура` и typed-row.
- `bsl-runtime/src/application/type_system/services/completion_service/member_resolution.rs` теперь читает только `member_access_owner_type_hint`; completion-local bootstrap owner path удалён.
- `backend/src/bin/lsp_server/server/core/tests.rs` содержит exact cross-consumer acceptance, revision-switch regressions и default-LSP acceptance для `FormModule.Объект.`.
- `.github/workflows/ci.yml` является default fail-closed entrypoint для governance gate по затронутым `openspec/changes/<id>`.

## Decisions

### 1. Shared structural knowledge MUST be first-class
Typed `Структура` и typed-row не считаются fully shared semantics, пока member knowledge не живёт в общем контракте как first-class данные.

Минимальный structural-member payload:
- canonical member name;
- stable identity;
- member type;
- certainty;
- source span / source location.

Preferred shape для change: `TypeResolution` + `ResolutionMetadata.structural_members`.

Почему:
- это уже совпадает с текущим ядром типов и не создаёт второй carrier для той же semantic truth;
- hover / type-at-position / diagnostics / completion читают один resolved result;
- equality, cloning, serialization и snapshot lifecycle уже привязаны к `TypeResolution`.

### 2. Semantic consumers MUST use one resolved path
`completion`, `hover`, `type-at-position`, `semantic diagnostics`, а также adapter surfaces (`LSP`, `MCP`, Web) читают owner/type из одного semantic contract.

Допустимы только thin adapters:
- formatting / transport mapping;
- ranking / snippet generation;
- response-shape differences без новой semantic truth.

Недопустимый end-state:
- completion-only schema/effect truth;
- hidden local owner reconstruction;
- acceptance, зависящий от consumer-specific hints, не представимых другим consumers.

Delivered state для этого change:
- direct handler path fail-closed без shared owner hint;
- actual `textDocument/completion` path для `FormModule.Объект.` проходит через shared owner-hint producer, а не через runtime fallback;
- bootstrap-only implicit module-context exception удалена.

### 3. Acceptance MUST prove shared semantics
Smoke/parity проверки полезны, но недостаточны. Production-grade acceptance для этого change должна доказывать:
- одинаковый owner resolution результат;
- одинаковую member identity;
- одинаковую known/unknown policy;
- hidden-hint fail-closed behaviour;
- default runtime wiring для LSP-path.

Delivered evidence classes:
- `analysis-v2` unit tests for typed `Структура` / typed-row materialization and snapshot isolation;
- backend exact cross-consumer acceptance tests for completion + hover + type-at-position + diagnostics;
- direct-handler fail-closed checks без shared hint;
- core-level default-LSP acceptance для `FormModule.Объект.`.

### 4. Delivery readiness MUST stay honest relative to MUST backlog
Readiness gate обязан сверять:
- OpenSpec status / checklist;
- traceability matrix;
- review artifact;
- critical Beads backlog;
- default operational entrypoint для gate.

Delivered governance path:
1. OpenSpec requirements задают MUST truth.
2. Traceability фиксирует `Requirement -> Code -> Test`.
3. Review artefacts и machine-readable status запрещают optimistic `complete`.
4. Critical backlog закрывается перед final verdict.
5. Active workflow `.github/workflows/ci.yml` делает этот gate default fail-closed path для touched changes.

### 5. This change is delivered through two linked follow-up lines
Historical delivery path:
- `6mx.*` закрыли shared structural contract, identity, exact acceptance и первые governance artefacts.

Final hardening path:
- epic `bsl-gradual-types-b6q` wired active default governance workflow (`b6q.1`, `b6q.2`);
- retired bootstrap-only implicit module-context fallback и добавил strict acceptance (`b6q.3`, `b6q.4`);
- refreshed closure artefacts (`b6q.5`).

## Alternatives Considered

### Keep the conclusion only in review notes
Rejected.
Такой вывод быстро теряется и не становится частью change governance.

### Deliver only product-spec without dev-workflow enforcement
Rejected.
Тогда теряется ключевой вывод про расхождение между declared completion и реальной readiness.

### Keep the bootstrap fallback as a permanent bounded exception
Rejected.
Это оставляло бы вторую semantic ветку именно в той области, где review требовал strict shared-path proof.

## Risks / Trade-offs
- Change объединяет архитектурную и процессную тему.
  - Mitigation: scope ограничен readiness contract и не уходит в unrelated refactoring.
- Active CI workflow intentionally narrow и покрывает только default governance path.
  - Mitigation: README/CONTRIBUTING честно описывают это; локальные Rust quality gates перед PR остаются обязательными.

## Migration Plan
1. Применить change и архивировать его только после финального OpenSpec workflow.
2. Сохранять `TypeResolution`-centric structural contract как canonical path для следующих gradual-typing changes.
3. Любые будущие exceptions оформлять отдельным approved change с explicit replacement path и automated evidence.

## Open Questions
- Нет. Для этого change readiness gate автоматизирован tooling-скриптами и wired by default через `.github/workflows/ci.yml`.
