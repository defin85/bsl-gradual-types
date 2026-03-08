# Change: update-gradual-core-production-readiness

## Why
Change появился после production-readiness review, который выявил четыре класса gap:
- snapshot-local structural knowledge для typed `Структура` и typed-row ещё не было first-class shared truth со stable identity;
- completion сохранял риск consumer-local reconstruction;
- exact acceptance и traceability ещё не доказывали same member identity и hidden-hint fail-closed behaviour;
- OpenSpec change можно было optimistic трактовать как `complete` без machine-readable readiness gate и без active default workflow.

Нужен был change, который сначала формализует этот contract, а затем доведёт его до прямого `Requirement -> Code -> Test` evidence.

## What Changes
- Доставлен first-class shared structural contract для typed `Структура` и typed-row, включая stable member identity.
- Completion / hover / type-at-position / diagnostics / `LSP` / `MCP` / Web adapters сведены к одному resolved truth.
- Bootstrap-only implicit module-context fallback в completion owner resolution удалён; этот scope теперь strict shared-hint-driven.
- Exact acceptance доказывает same member identity, same known/unknown policy, hidden-hint fail-closed behaviour и snapshot revision isolation across interfaces.
- Для `dev-workflow` введён change-specific readiness gate с machine-readable governance artefacts и active default workflow `.github/workflows/ci.yml`, который блокирует optimistic `complete` при конфликте backlog/evidence.

## Impact
- Affected specs:
  - `bsl-intellisense-v2`
  - `dev-workflow`
- Affected code:
  - `bsl-types/src/types/*`
  - `analysis-v2/src/type_inference_v2.rs`
  - `bsl-runtime/src/application/type_system/services/*`
  - `backend/src/bin/lsp_server/**`
  - `bsl-agent/src/session/**`
  - `.github/workflows/ci.yml`
  - change-local governance / validation artefacts under `openspec/changes/update-gradual-core-production-readiness/`

## Delivery Status

Delivered:
- shared structural member identity and lifecycle semantics;
- exact cross-consumer acceptance for typed `Структура`, typed-row and implicit module-context owner resolution;
- fail-closed hidden-hint regressions in direct handler paths;
- default LSP-path evidence for `FormModule.Объект.`;
- active default governance workflow and refreshed closure evidence.

Current verdict:
- `complete` for this change;
- direct evidence is recorded in `traceability.md`, `residual-risk-review.md`, `validation/final-closure-checklist.md`, and `governance/readiness_status.json`.
