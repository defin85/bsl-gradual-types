# Change: update-gradual-core-production-readiness

## Why
Change был создан после production-readiness review, который показал разрыв между сильной архитектурной идеей и фактическим delivery evidence.

Ключевые gaps были такими:
- snapshot-local structural knowledge для typed `Структура` и typed-row ещё не было first-class shared truth с stable identity;
- completion допускал risk consumer-local reconstruction;
- exact acceptance и traceability ещё не доказывали same member identity и fail-closed hidden-hint behaviour;
- OpenSpec change можно было optimistic трактовать как complete без machine-readable readiness gate.

Нужен был отдельный change, который сначала зафиксирует этот contract, а затем будет доведён до прямого `Requirement -> Code -> Test` evidence.

## What Changes
- Зафиксирован и доставлен first-class shared structural contract для typed `Структура` и typed-row, включая stable member identity.
- Completion / hover / type-at-position / diagnostics / `LSP` / `MCP` / Web adapters привязаны к одному resolved truth для reviewed structural scenarios; остающиеся bootstrap-only exceptions явно ограничены design-артефактом.
- Exact acceptance теперь доказывает same member identity, same known/unknown policy, hidden-hint fail-closed behaviour и snapshot revision isolation across interfaces.
- Для `dev-workflow` введён change-specific readiness gate с machine-readable governance artefacts, который блокирует optimistic `complete` при конфликте backlog/evidence.

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
  - change-local governance / validation artefacts under `openspec/changes/update-gradual-core-production-readiness/`

## Delivery Status

Delivered:
- shared structural member identity and lifecycle semantics;
- exact cross-consumer acceptance for typed `Структура` and typed-row;
- fail-closed hidden-hint regressions;
- change-specific readiness governance and honest closure evidence.

Current verdict:
- `complete` for this change;
- direct evidence is recorded in `traceability.md`, `residual-risk-review.md`, `validation/final-closure-checklist.md`, and `governance/readiness_status.json`.
