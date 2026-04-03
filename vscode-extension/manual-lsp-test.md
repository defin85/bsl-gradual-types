# Ручное тестирование LSP сервера

## Тест 1: Базовая работоспособность бинарника

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension/bin
./lsp-server.exe --help
```

**Ожидаемый результат**: Вывод справки или сообщение об использовании

---

## Тест 2: LSP протокол через stdio (симулирует VSCode)

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension

# Отправить LSP initialize запрос (Content-Length ДОЛЖЕН быть 107!)
printf 'Content-Length: 107\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}' | timeout 5s ./bin/lsp-server.exe 2>&1
```

**Ожидаемый результат**: JSON ответ с capabilities:
```json
{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"completionProvider":{"triggerCharacters":["."," "]},...}}}
```

**ВАЖНО**: Если видишь `Parse error` — проблема в Content-Length (должно быть 107, не 141)!

---

## Тест 3: Проверка зависимостей Windows (DLL)

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension/bin
ldd lsp-server.exe | grep "not found"
```

**Ожидаемый результат**: Пустой вывод (все DLL найдены)

---

## Тест 4: Запуск с полным логированием

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension/bin
export RUST_LOG=debug RUST_BACKTRACE=full
./lsp-server.exe
```

**Что делать**:
1. Сервер должен запуститься и ждать ввода
2. Вставь вручную (Ctrl+V):
   ```
   Content-Length: 107

   {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}
   ```
3. Нажми Enter дважды
4. Сервер должен вернуть JSON ответ с логами инициализации

**Для выхода**: Ctrl+C

---

## Тест 5: Node.js симулятор VSCode (уже создан)

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension
node test-lsp-vscode.js
```

**Ожидаемый результат**:
```
Starting LSP server...
Server started, PID: <number>
Sending initialize request...
Response: {"jsonrpc":"2.0",...}
```

---

## Тест 6: Запуск из рабочей директории расширения (как VSCode)

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension
RUST_LOG=info RUST_BACKTRACE=1 ./bin/lsp-server.exe
```

**Что проверяем**: Возможно, сервер ожидает запуска из конкретной директории

---

## Диагностика результатов

### Если Тест 1 не работает
- Проблема: Бинарник поврежден или несовместим
- Действие: Пересобрать `cargo build --release --bin bsl-lsp-server`

### Если Тест 2 не работает
- Проблема: LSP протокол не реализован корректно
- Действие: Проверить код `backend/src/bin/lsp_server.rs`

### Если Тест 3 показывает missing DLLs
- Проблема: Отсутствуют системные зависимости
- Действие: Установить Visual C++ Redistributable

### Если Тест 4 падает с ошибкой
- Проблема: Ошибка инициализации (SystemCoordinator, TypeService)
- Действие: Читать RUST_BACKTRACE для стека

### Если Тест 5 работает, но VSCode крашится
- Проблема: Специфика spawn() в VSCode LanguageClient
- Действие: Проверить рабочую директорию и environment в client.ts

---

## Быстрая проверка (одна команда) ⚡

```bash
cd c:/1CProject/bsl-gradual-types/vscode-extension && \
printf 'Content-Length: 107\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}' | \
timeout 5s ./bin/lsp-server.exe 2>&1
```

**Что это делает**: Отправляет LSP initialize и ждет ответ (5 сек таймаут)

**Если видишь JSON с capabilities** → Сервер работает ✅
**Если `Parse error`** → Content-Length неверный ❌
**Если таймаут/crash** → Сервер не отвечает ❌

---

## Тест 7: Completion Timeline и Client Probe Feed

```bash
cd /path/to/bsl-gradual-types/vscode-extension
npm run compile:fast
BSL_TEST_GREP='Completion Probe (Schema|Recorder|Runtime|Store) Test Suite|Completion Timeline (Clipboard|Drilldown|Model|Webview Provider) Test Suite|Client Options Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found|getObservabilityMetricsFetchResult should preserve unsupported capability until reset|getObservabilityMetricsFetchResult should return unavailable error on timeout' node ./out/test/runTest.js
```

**Ожидаемый результат**:
- проходят focused extension-host тесты для `Completion Timeline` и `Client Probe Feed`;
- тот же smoke slice покрывает `Observability Incident Bundle` export, partial-export semantics и actual command file export path;
- transport hook (`Completion Probe Runtime`) входит в тот же smoke path, что и `run-intellisense-tests.sh smoke`;
- `Server Timeline` на payload `version=21` показывает bounded root-cause attribution с canonical grouped query-body split (`query_bundle_pool_wait`, `query_bundle_deps_and_file_snapshot`, `query_bundle_owner_hint`, `query_bundle_ir_query`, `query_bundle_ir_retry`, `query_bundle_other`), existing ingress/runtime detail (`adapter_read_at_ms`, `adapter_to_dispatch_wait_ms`, legacy `transport_received_at_ms_provenance`, `jsonrpc_dispatch_received_at_ms`, `dispatch_to_request_context_wait_ms`, `service_future_created_at_ms`, `service_future_first_poll_entered_at_ms`, `service_future_to_first_poll_wait_ms`, `service_future_first_poll_outcome`, `service_future_first_wake_scheduled_at_ms`, `first_poll_to_first_wake_wait_ms`, bounded `first_poll_contention_attribution`, `transport_to_service_future_wait_ms`, `service_future_to_scope_wait_ms`, существующие `transport_to_service_scope_wait_ms`, `service_scope_to_method_wait_ms`, bounded `pre_method_attribution_provenance`, `dispatcher_resolution_latency_ms`, `prepare_progress`, `wait_for_file_version_runtime`, `snapshot_with_deps_runtime`, `snapshot_with_deps_timeout_runtime`, `timeout_attribution`, bounded `exact_wait` waiter/task-state, `artifact_poll`, `transport_slot_released_at_ms`, request-bound `client_probe_id`) и новый flush-aware post-handler split (`response_flush_completed_at_ms`, `response_ready_to_flush_wait_ms`);
- `bsl.getCompletionTimeline` остаётся fail-closed и не смешивается с локальными client probes;
- clipboard/webview/incident summary выносят truthful verdict'ы из authoritative stages: `query_bundle_dominant` + leaf verdicts (`query_bundle_pool_wait_dominant`, `query_bundle_deps_and_file_snapshot_dominant`, `query_bundle_owner_hint_dominant`, `query_bundle_ir_query_dominant`, `query_bundle_ir_retry_dominant`, `query_bundle_other_dominant`), а также ingress-only verdicts (`adapter_before_dispatch_dominant`, `server_before_method_entry_dominant`, `client_before_transport_dominant`, `handler_prelude_dominant`) только когда query-body dominance не доказана; fail-closed verdicts `prepare_timeout@prepare_guard` и `exact_deadline@artifact_poll` по-прежнему выводятся без чтения raw JSON;
- synthetic `Averaged` trace явно помечается как synthetic и не выдаёт trustworthy `v8` pre-method attribution provenance, `v9` pre-service-scope split, `v10` dispatch split, `v11` first-poll / first-wake split или `v12` first-poll contention attribution за per-request факт;
- при более старом backend payload `v20`/`v19`/`v18`/`v17`/`v16`/`v15`/`v14`/`v13`/`v12`/`v11`/`v10`/`v9`/`v8`/`v7` extension деградирует явно и не выдумывает отсутствующие `v21`/`v20`/`v19`/`v18`/`v17`/`v16`/`v15`/`v14`/`v13`/`v12`/`v11`/`v10`/`v9`/`v8` поля;
- export bundle сохраняет `summary.md`, `incident.json` и `raw/*` attachments без использования truncated Output dump как источника;
- `incident.json` и `summary.md` выражают request-centric handoff: `capture_scope`, `request_count`, bounded request list, bounded `dispatch` + `service_future_created` split и explicit `correlation=correlated|unavailable|ambiguous` без guessed probe/trace pairs.
