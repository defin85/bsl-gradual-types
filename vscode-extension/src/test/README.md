# VSCode Extension Tests

Полный набор тестов для BSL Gradual Types VSCode Extension.

## Структура тестов

### 1. Extension Activation Tests (`extension.test.ts`)
- ✅ Проверка наличия расширения
- ✅ Активация расширения
- ✅ Регистрация всех команд (21 команда)
- ✅ Parser utilities (parseMethodCall, extractTypeName)
- ✅ Configuration tests

**Покрытие:** Extension lifecycle, Commands registration, Parser utils

### 2. LSP Integration Tests (`lsp.test.ts`)
- ✅ Защита от дублирования команд
- ✅ Регистрация команд до запуска LSP
- ✅ Обработка duplicate command registration
- ✅ LSP сервер не регистрирует extension команды
- ✅ Перезапуск сервера без дублирования команд
- ✅ Обработка ошибок инициализации LSP
- ✅ Изоляция команд расширения от LSP
- ✅ Защита от множественной активации

**Покрытие:** LSP lifecycle, Command conflicts prevention, Multiple activation handling

### 3. Commands Tests (`commands.test.ts`)
- ✅ Существование всех команд
- ✅ Refresh команды (4 команды)
- ✅ Обработка отсутствующей конфигурации
- ✅ Валидация формата версии платформы
- ✅ Performance tests (activation < 5s, commands < 1s)
- ✅ TreeDataProvider регистрация
- ✅ Configuration management (8 настроек)
- ✅ Изменение конфигурации

**Покрытие:** Commands execution, Error handling, Performance, Configuration

### 4. Integration Tests (`integration.test.ts`)
- ✅ Полный жизненный цикл расширения
- ✅ Работа с BSL файлами
- ✅ Взаимодействие с Language Server
- ✅ Обработка конфигурации 1С
- ✅ Механизм кеширования
- ✅ Diagnostics provider
- ✅ Output channel
- ✅ WebView команды
- ✅ Совместимость с VSCode >= 1.75.0
- ✅ Доступность всех API

**Покрытие:** End-to-end workflows, File handling, LSP communication, Compatibility

### 5. LSP Custom Requests Tests (`customRequests.test.ts`) ✨ NEW
- ✅ `queryType()` - замена CLI query_type
- ✅ `buildIndex()` - замена CLI build_unified_index
- ✅ `validateMethod()` - замена CLI для методов
- ✅ `checkTypeCompatibility()` - замена CLI check_type_compatibility
- ✅ `incrementalUpdate()` - замена CLI incremental_update
- ✅ `extractPlatformDocs()` - замена CLI extract_platform_docs
- ✅ Обработка ошибок LSP сервера
- ✅ Performance тесты (< 2s)
- ✅ Параллельные вызовы custom requests

**Покрытие:** Task 3 из Milestone 2.2 - все 6 custom requests

### 6. Provider Tests (`providers.test.ts`) ✨ NEW
- ✅ BslOverviewProvider регистрация и refresh
- ✅ BslDiagnosticsProvider регистрация и refresh
- ✅ HierarchicalTypeIndexProvider регистрация и refresh
- ✅ BslPlatformDocsProvider регистрация и CRUD операции
- ✅ BslActionsWebviewProvider регистрация
- ✅ Provider LSP интеграция
- ✅ Обработка перезапуска LSP сервера
- ✅ Performance провайдеров (< 2s)
- ✅ Параллельное обновление провайдеров (< 5s)

**Покрытие:** All Tree Data Providers, WebView providers, LSP integration

## Статистика

- **Всего тестовых файлов:** 6
- **Всего test cases:** 67+
- **Всего test suites:** 20+

### Покрытие по категориям

| Категория | Тесты | Статус |
|-----------|-------|--------|
| Extension Activation | 12 | ✅ |
| LSP Integration | 8 | ✅ |
| Commands | 14 | ✅ |
| Integration | 10 | ✅ |
| Custom Requests | 10 | ✅ NEW |
| Providers | 13 | ✅ NEW |
| **ИТОГО** | **67** | **✅** |

## Запуск тестов

### Обычный режим
```bash
npm test
```

### Фокусный smoke для Completion Timeline / Client Probe Feed
```bash
npm run compile:fast
BSL_TEST_GREP='Completion Probe (Schema|Recorder|Runtime|Store) Test Suite|Completion Timeline (Clipboard|Drilldown|Model|Webview Provider) Test Suite|Client Options Test Suite|Observability Incident Bundle Test Suite|Observability Commands Test Suite|getCompletionTimeline should work via executeCommand|getCompletionTimeline should fail-closed on Method not found|getObservabilityMetricsFetchResult should preserve unsupported capability until reset|getObservabilityMetricsFetchResult should return unavailable error on timeout' node ../scripts/run-vscode-extension-tests.js
```

Этот путь повторяет extension-side slice из `./scripts/run-intellisense-tests.sh smoke` и проверяет:
- bounded/redacted probe schema и eviction;
- runtime transport hook и selection observer для client probes;
- wiring default `LanguageClient` path;
- dual-view `Server Timeline` / `Client Probe Feed`;
- rendering/export для `Server Timeline` с `response.version=21`, bounded root-cause attribution (`adapter_read_at_ms`, `adapter_to_dispatch_wait_ms`, legacy `transport_received_at_ms_provenance`, `jsonrpc_dispatch_received_at_ms`, `dispatch_to_request_context_wait_ms`, `service_future_created_at_ms`, `service_future_first_poll_entered_at_ms`, `service_future_to_first_poll_wait_ms`, `service_future_first_poll_outcome`, `service_future_first_wake_scheduled_at_ms`, `first_poll_to_first_wake_wait_ms`, bounded `first_poll_contention_attribution`, bounded-length `first_poll_contention_contenders` с optional `command` для `workspace/executeCommand` и optional `phase` для inflight completion stage, `transport_to_service_future_wait_ms`, `service_future_to_scope_wait_ms`, существующие `transport_to_service_scope_wait_ms`, `service_scope_to_method_wait_ms`, bounded `pre_method_attribution_provenance`, `dispatcher_resolution_latency_ms`, `turn_wait_entered_at_ms`, `turn_wait_resolved_at_ms`, `wake_after_turn_resolution_at_ms`, `prepare_progress`, `wait_for_file_version_runtime`, `snapshot_with_deps_runtime`, `snapshot_with_deps_timeout_runtime`, `timeout_attribution`, bounded `exact_wait`, `artifact_poll`, `transport_slot_released_at_ms`, request-bound `client_probe_id`, flush-aware `response_flush_completed_at_ms` / `response_ready_to_flush_wait_ms` и canonical grouped query-body stages `query_bundle_pool_wait`, `query_bundle_deps_and_file_snapshot`, `query_bundle_owner_hint`, `query_bundle_ir_query`, `query_bundle_ir_retry`, `query_bundle_other`) и truthful verdict projection (`query_bundle_dominant` + canonical leaf verdicts, ingress-only verdicts только при отсутствии query-body dominance, `prepare_timeout@prepare_guard`, `exact_deadline@artifact_poll`);
- `average` mode остаётся synthetic и поэтому явно помечает trustworthy `v8` pre-method attribution provenance, `v9` pre-service-scope split, `v10` dispatch split, `v11` first-poll / first-wake split, `v12` first-poll contention attribution, `v13` contender snapshot, `v14` executeCommand command detail, `v15` completion phase detail, `v16` turn-wait resolution detail, `v17` transport slot release detail, `v18` request-bound client probe correlation detail, `v19` adapter ingress pre-dispatch split и `v21` flush-aware post-handler egress split как unavailable by design, не inventing strong ingress verdicts;
- graceful degradation для backend payload `v20`/`v19`/`v18`/`v17`/`v16`/`v15`/`v14`/`v13`/`v12`/`v11`/`v10`/`v9`/`v8`/`v7` без выдумывания отсутствующих `v21`/`v20`/`v19`/`v18`/`v17`/`v16`/`v15`/`v14`/`v13`/`v12`/`v11`/`v10`/`v9`/`v8` полей;
- fail-closed/executeCommand поведение `bsl.getCompletionTimeline`;
- truthful `unsupported` vs `unavailable` semantics для observability metrics export;
- actual command export path для `bslAnalyzer.exportObservabilityIncidentBundle`, включая запись `summary.md`, `incident.json` и `raw/*`;
- request-centric handoff для incident bundle: `capture_scope`, `request_count`, bounded request list, bounded `dispatch` + `service_future_created` split и deterministic probe-to-trace correlation только при неамбигуозном сопоставлении;
- reuse текущего Completion Timeline snapshot при экспорте из webview, без принудительного fresh refetch этого источника.

### С coverage (цель: 80%)
```bash
npm run test:coverage
```

Coverage отчёт генерируется в `coverage/index.html`.

### Только линтинг
```bash
npm run lint
```

## Coverage конфигурация

Файл `.c8rc.json`:
- **Lines:** 80%
- **Functions:** 80%
- **Branches:** 70%
- **Statements:** 80%

## Тестируемые компоненты

✅ Extension activation lifecycle
✅ LSP Client initialization
✅ Command registration (21 команда)
✅ LSP Custom Requests (6 requests) — Task 3 Milestone 2.2
✅ Tree Data Providers (5 providers)
✅ Configuration management
✅ Parser utilities
✅ Error handling
✅ Performance constraints
✅ Compatibility with VSCode API

## CI/CD Integration

Тесты готовы для интеграции в CI pipeline:
```yaml
# .github/workflows/test.yml
- name: Run tests
  run: npm test

- name: Check coverage
  run: npm run test:coverage
```

## Milestone 2.2: Task 4 Status ✅

**Цель:** Написать тесты с покрытием 80%

**Выполнено:**
- ✅ 67+ test cases
- ✅ 20+ test suites
- ✅ Покрытие LSP Custom Requests (6/6)
- ✅ Покрытие Providers (5/5)
- ✅ Coverage reporting настроен (c8)
- ✅ Performance constraints проверены
- ✅ Error handling протестирован

**Результат:** Task 4 завершён на 100%
