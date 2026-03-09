# Change: refactor-ir-canonical-semantic-pipeline

## Why

Сейчас v2 semantic truth раздвоена:
- exact consumer paths уже используют `SemanticProgram`/IR;
- значимая часть interactive semantic queries и owner-hint/type-at-position fast paths использует отдельный `type_index`/`serve_only` слой, построенный не как projection от IR;
- при miss-сценариях сохраняются degraded/fallback ветки (`stale`, `keyword`, `fallback_unavailable`);
- в `ObjectModule` / `RecordSetModule` остаётся applied-owner bare identifier semantics вне canonical IR path.

Это усложняет доказательство того, что все semantic consumers читают одну и ту же truth, и оставляет архитектуру с несколькими конкурентными источниками semantic knowledge.

Пользователь подтвердил целевой end-state:
- IR становится canonical semantic source of truth;
- `derived semantic index` становится единственным fast query слоем;
- degraded/fallback semantic paths удаляются;
- applied-owner bare identifier fallback удаляется;
- при недоступности canonical артефактов система работает fail-closed, а не через degraded semantics.

## What Changes

- Переопределить архитектуру `bsl-intellisense-v2` как `IR -> derived semantic index -> interactive queries`.
- Зафиксировать, что semantic truth может происходить только из canonical IR snapshot текущей revision.
- Зафиксировать, что `derived semantic index` строится только из IR snapshot и не выполняет самостоятельный semantic inference.
- Удалить как целевой end-state:
  - stale/degraded/keyword semantic fallback paths;
  - non-IR semantic fast paths, которые материализуют truth вне canonical IR;
  - applied-owner bare identifier fallback для `ObjectModule` / `RecordSetModule`.
- Зафиксировать fail-closed contract для `completion`, `hover`, `signatureHelp`, `definition`, `type-at-position`, `members`, `diagnostics` и MCP/Web adapter surfaces.
- Согласовать MCP semantic tools с тем же canonical IR + derived-index runtime contract.

## Impact

- Affected specs:
  - `bsl-intellisense-v2`
  - `mcp-bsl-agent`
- Affected code:
  - `analysis-v2`
  - `bsl-runtime`
  - `backend` (`LSP`, `Web`)
  - `bsl-agent`
  - `contracts/**`
- Breaking changes:
  - user-facing degraded completion/type fallback paths заменяются на explicit unavailable/empty fail-closed behavior;
  - bare identifiers в `ObjectModule` / `RecordSetModule` больше не получают implicit applied-owner resolution вне canonical IR semantics.
- Coordination:
  - pending change `refactor-bsl-agent-index-backed-search` должен быть согласован с новым определением index path как IR-derived, а не как отдельного semantic source.
