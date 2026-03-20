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
- `Server Timeline` на payload `version=8` показывает bounded root-cause attribution (`service_scope_entered` split, `transport_to_service_scope_wait_ms`, `service_scope_to_method_wait_ms`, bounded `pre_method_attribution_provenance`, `dispatcher_resolution_latency_ms`, `prepare_progress`, `wait_for_file_version_runtime`, `snapshot_with_deps_runtime`, `snapshot_with_deps_timeout_runtime`, `timeout_attribution`, bounded `exact_wait` waiter/task-state и `artifact_poll`);
- `bsl.getCompletionTimeline` остаётся fail-closed и не смешивается с локальными client probes;
- clipboard/webview/incident summary выносят truthful verdict'ы вроде `server_before_method_entry_dominant`, `client_before_transport_dominant`, `handler_prelude_dominant`, `prepare_timeout@prepare_guard` и `exact_deadline@artifact_poll` без чтения raw JSON, причём сильный ingress verdict допускается только при `same_request_authoritative`;
- при более старом backend payload `v7` extension деградирует явно и не выдумывает отсутствующий `v8` provenance;
- export bundle сохраняет `summary.md`, `incident.json` и `raw/*` attachments без использования truncated Output dump как источника;
- `incident.json` и `summary.md` выражают request-centric handoff: `capture_scope`, `request_count`, bounded request list и explicit `correlation=correlated|unavailable|ambiguous` без guessed probe/trace pairs.
