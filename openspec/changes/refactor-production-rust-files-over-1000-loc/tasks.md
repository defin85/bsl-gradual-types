## 1. Baseline And Guardrails
- [ ] 1.1 Зафиксировать inventory production `.rs` файлов `>1000 LOC` в change-артефакте и подтвердить scope исключений (`third_party`, `tests`, generated).
- [ ] 1.2 Добавить автоматическую проверку size-gate для production Rust файлов (`LOC <= 1000`) в локальный workflow и CI.
- [ ] 1.3 Зафиксировать parity validation matrix (команды/наборы тестов) для behavior-preserving refactor по подсистемам.

## 2. Refactor Batch A: LSP/Web Layer
- [ ] 2.1 Декомпозировать `backend/src/bin/lsp_server/server/core.rs` до `<=1000 LOC`.
- [ ] 2.2 Декомпозировать `backend/src/bin/lsp_server/server/language_server.rs` до `<=1000 LOC`.
- [ ] 2.3 Декомпозировать `backend/src/bin/lsp_server/server/completion_dispatcher.rs` до `<=1000 LOC`.
- [ ] 2.4 Декомпозировать `backend/src/bin/lsp_server/handlers/references_and_rename.rs` и `backend/src/bin/lsp_server/handlers/completion.rs` до `<=1000 LOC`.
- [ ] 2.5 Декомпозировать `backend/src/bin/lsp_server/commands/configuration.rs` и `backend/src/presentation/web/handlers.rs` до `<=1000 LOC`.
- [ ] 2.6 Прогнать parity matrix для Batch A и зафиксировать результаты.

## 3. Refactor Batch B: Runtime Services/Observability
- [ ] 3.1 Декомпозировать `bsl-runtime/src/system/basic_observability.rs` до `<=1000 LOC`.
- [ ] 3.2 Декомпозировать `bsl-runtime/src/application/type_system/services/completion_service.rs` и `completion_ranking.rs` до `<=1000 LOC`.
- [ ] 3.3 Декомпозировать `bsl-runtime/src/application/intellisense_v2/facade.rs` и `policy.rs` до `<=1000 LOC`.
- [ ] 3.4 Прогнать parity matrix для Batch B и зафиксировать результаты.

## 4. Refactor Batch C: Runtime Coordinator/Loaders
- [ ] 4.1 Декомпозировать `bsl-runtime/src/system/system_coordinator/config_loader.rs`, `lifecycle.rs`, `coordinator.rs` до `<=1000 LOC`.
- [ ] 4.2 Декомпозировать `bsl-runtime/src/system/disk_cache.rs`, `runtime_config.rs`, `parser_coordinator.rs` до `<=1000 LOC`.
- [ ] 4.3 Декомпозировать `bsl-runtime/src/data/loaders/config_metadata_parser/discovery.rs` и `converter.rs` до `<=1000 LOC`.
- [ ] 4.4 Прогнать parity matrix для Batch C и зафиксировать результаты.

## 5. Refactor Batch D: Analysis/Semantic/Repository
- [ ] 5.1 Декомпозировать `analysis-v2/src/lib.rs` и `analysis-v2/src/type_inference_v2.rs` до `<=1000 LOC`.
- [ ] 5.2 Декомпозировать `semantic-diagnostics/src/visitor.rs` и `bsl-repository/src/repository.rs` до `<=1000 LOC`.
- [ ] 5.3 Прогнать parity matrix для Batch D и зафиксировать результаты.

## 6. Refactor Batch E: Agent Layer
- [ ] 6.1 Декомпозировать `bsl-agent/src/session/mod.rs` и `bsl-agent/src/server/mod.rs` до `<=1000 LOC`.
- [ ] 6.2 Прогнать parity matrix для Batch E и зафиксировать результаты.

## 7. Final Validation
- [ ] 7.1 Подтвердить, что в production scope отсутствуют `.rs` файлы `>1000 LOC`.
- [ ] 7.2 Запустить финальный verification set (`fmt`, `clippy`, `test`, релевантные интеграционные/контрактные проверки).
- [ ] 7.3 `openspec validate refactor-production-rust-files-over-1000-loc --strict --no-interactive`.

## Dependencies / Parallelism
- [ ] D1 Пункты 1.1-1.3 блокируют старты batch’ей 2-6.
- [ ] D2 Batch’и 3, 4, 5, 6 могут выполняться параллельно после завершения Batch 2 (или при отсутствии конфликтов по файлам).
- [ ] D3 Пункт 7.1 блокирует 7.2.
