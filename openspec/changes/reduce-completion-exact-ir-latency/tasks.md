# Tasks

## 1. Спека и acceptance

- [x] Зафиксировать final contract для shared exact IR flight между request path и background prewarm.
- [x] Зафиксировать latest-only / no-partial-publish semantics для superseded exact compute.

## 2. Shared revision flight

- [x] Убрать direct exact IR bypass у current-revision prewarm и свести prewarm/request path к одному revision-key entrypoint.
- [x] Обеспечить, что request может attach-иться к уже идущему warm flight, а warm path может reuse request-started flight без duplicate compute.

## 3. Cooperative cancellation внутри exact compute

- [x] Протянуть cancellable checker внутрь AST->IR и exact facts build с bounded checkpoints по крупным work batches.
- [x] Гарантировать, что superseded/cancelled exact compute не пишет partial IR/semantic artifacts в shared cache и не публикует stale result как latest.

## 4. Verification

- [x] Добавить focused tests на request/prewarm reuse для одного revision key.
- [x] Добавить tests на superseded/cancelled exact IR unwind без stale publish.
- [x] Добавить representative gate или counter-based evidence, что duplicate exact IR compute для одной revision не запускается параллельно по разным orchestration path.
- [x] Прогнать минимальный релевантный verify set для backend/runtime.
