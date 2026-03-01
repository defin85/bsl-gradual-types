# Change: Декомпозиция production Rust файлов >1000 LOC (behavior-preserving)

## Why
В репозитории сейчас 26 production Rust файлов (без `third_party/**`, `target/**`, `node_modules/**`, `tests/benches/examples`), которые превышают 1000 строк.

Это создаёт системные риски:
- рост связности и смешение ответственностей;
- усложнение безопасных изменений и code review;
- локальные правки в hot paths становятся дороже и менее предсказуемыми.

Запрошен независимый change со строгим требованием: декомпозировать все такие файлы без изменения поведения.

## What Changes
- **ADDED (dev-workflow)**: policy, что production Rust файлы MUST быть `<=1000 LOC` (с явными исключениями только для generated/vendor paths вне production scope).
- **ADDED (dev-workflow)**: behavior-preserving refactor contract для кампаний декомпозиции крупных файлов.
- **ADDED (dev-workflow)**: обязательный inventory + parity validation matrix для декомпозиции крупных файлов.
- **PLANNED REFACTOR SCOPE**: поэтапная декомпозиция всех текущих production `.rs` файлов >1000 LOC.

Текущий target inventory (`LOC > 1000`):
- `backend/src/bin/lsp_server/server/core.rs`
- `backend/src/bin/lsp_server/server/language_server.rs`
- `bsl-agent/src/session/mod.rs`
- `bsl-runtime/src/application/type_system/services/completion_service.rs`
- `bsl-runtime/src/system/basic_observability.rs`
- `analysis-v2/src/lib.rs`
- `bsl-runtime/src/application/intellisense_v2/facade.rs`
- `analysis-v2/src/type_inference_v2.rs`
- `bsl-runtime/src/system/system_coordinator/config_loader.rs`
- `backend/src/bin/lsp_server/handlers/references_and_rename.rs`
- `bsl-runtime/src/system/disk_cache.rs`
- `bsl-runtime/src/application/intellisense_v2/policy.rs`
- `bsl-agent/src/server/mod.rs`
- `backend/src/bin/lsp_server/handlers/completion.rs`
- `bsl-runtime/src/system/runtime_config.rs`
- `bsl-runtime/src/system/parser_coordinator.rs`
- `backend/src/bin/lsp_server/server/completion_dispatcher.rs`
- `bsl-runtime/src/system/system_coordinator/lifecycle.rs`
- `semantic-diagnostics/src/visitor.rs`
- `bsl-repository/src/repository.rs`
- `backend/src/bin/lsp_server/commands/configuration.rs`
- `bsl-runtime/src/application/type_system/services/completion_ranking.rs`
- `bsl-runtime/src/system/system_coordinator/coordinator.rs`
- `backend/src/presentation/web/handlers.rs`
- `bsl-runtime/src/data/loaders/config_metadata_parser/discovery.rs`
- `bsl-runtime/src/data/loaders/config_metadata_parser/converter.rs`

## Impact
- Affected specs:
  - `dev-workflow`
- Affected code (implementation follow-up):
  - `backend/src/bin/lsp_server/**`
  - `bsl-runtime/src/**`
  - `analysis-v2/src/**`
  - `bsl-agent/src/**`
  - `semantic-diagnostics/src/**`
  - `bsl-repository/src/**`
- Affected tooling:
  - size-gate для production Rust файлов (`LOC <= 1000`)
  - parity validation matrix для behavior-preserving refactor

## Non-Goals
- Изменение пользовательского поведения LSP/Web/MCP/CLI.
- Введение новых фич или изменение внешних контрактов API/LSP.
- Рефакторинг generated/vendor/test-only путей.
