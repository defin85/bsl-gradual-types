# Scripts Directory

Вспомогательные скрипты для разработки и тестирования BSL Gradual Types проекта.

## 🚫 Политика артефактов VSCode extension

Генерируемые файлы `vscode-extension/out/**` и `vscode-extension/*.vsix` не должны попадать в git.

Проверка:
```bash
./scripts/check-vscode-artifacts-policy.sh
```

Локальный pre-commit hook (опционально):
```bash
# Не перезаписывай существующий hook без проверки:
test -e .git/hooks/pre-commit || ln -s ../../scripts/git-hooks/pre-commit .git/hooks/pre-commit
```

## 🔒 Политика Cargo.lock

`Cargo.lock` должен быть отслеживаемым файлом в git (для воспроизводимых сборок).

Проверка:
```bash
./scripts/check-cargo-lockfile-policy.sh
```

## 📜 Доступные скрипты

### `add-defender-exclusions.ps1` - Настройка Windows Defender (НОВОЕ!)

**Назначение:** Автоматическое добавление проекта в исключения Windows Defender для ускорения HBK Recovery и компиляции.

**Использование:**
```powershell
# В PowerShell с правами администратора:
.\scripts\add-defender-exclusions.ps1
```

**Что делает:**
1. Добавляет весь проект в исключения Defender
2. Добавляет `examples\syntax_helper` (HBK Recovery - 52K файлов!)
3. Добавляет `target\` (Rust компиляция)
4. Добавляет LSP процессы в исключения
5. (Опционально) Отключает сканирование ZIP архивов

**Эффект:**
- HBK Recovery: **180s → 122s** (выигрыш ~60 секунд!)
- Компиляция: меньше overhead от Defender
- CPU: MsMpEng.exe <5% вместо 15-25%

**Требования:** Windows 10/11, права администратора

---

### `clear_cache.sh` - Очистка Windows File System Cache

**Назначение:** Очистить Windows File System Cache (Standby List) для тестирования прогресса парсинга.

**Когда использовать:**
- Нужно увидеть прогресс парсинга типов платформы 1С в реальном времени
- После первого парсинга всё кэшируется и повторный парсинг занимает ~1 секунду

**Использование:**
```bash
./scripts/clear_cache.sh
```

**Требования:**
- Windows 10/11
- Утилита `tools/EmptyStandbyList.exe` (скачивается отдельно)
- Права администратора (появится UAC prompt)

**Что делает:**
1. Проверяет наличие `EmptyStandbyList.exe`
2. Запускает утилиту от имени администратора
3. Очищает Standby List (файловый кэш Windows)

---

### `test_progress.sh` - Комплексное тестирование прогресса парсинга

**Назначение:** Автоматизировать подготовку к тестированию прогресса парсинга.

**Использование:**
```bash
./scripts/test_progress.sh
```

**Что делает:**
1. Вызывает `clear_cache.sh` для очистки кэша
2. Проверяет наличие собранного LSP сервера
3. Предлагает скопировать LSP сервер в `vscode-extension/bin/`
4. Даёт инструкции по запуску VSCode Extension

**Требования:**
- Те же, что и для `clear_cache.sh`
- Собранный LSP сервер (`cargo build --release --bin bsl-lsp-server`)

---

## 🎯 Типичный workflow

### Тестирование прогресса парсинга (первый раз):

```bash
# 1. Скачай утилиту EmptyStandbyList.exe (один раз)
cd tools
curl -L https://wj32.org/wp/download/releases/empty-standby-list/EmptyStandbyList.exe -o EmptyStandbyList.exe

# 2. Собери LSP сервер
cd ..
cargo build --release --bin bsl-lsp-server

# 3. Запусти комплексное тестирование
./scripts/test_progress.sh
```

### Тестирование прогресса парсинга (повторно):

```bash
# Просто очисти кэш и запусти VSCode Extension
./scripts/clear_cache.sh

# Затем:
# - Открой vscode-extension в VSCode
# - Нажми F5
# - Открой конфигурацию 1С
# - Наблюдай прогресс!
```

---

### `run-intellisense-perf.sh` - Perf suite для IntelliSense

**Назначение:** authoritative регрессионные замеры canonical semantic perf-gate
(operation-aware latency + resource budgets + anti-rescue guardrails) на профилях
`small`, `large`, `churn`.

**Использование:**
```bash
./scripts/run-intellisense-perf.sh
```

**Обновление baseline:**
```bash
UPDATE_BASELINE=1 ./scripts/run-intellisense-perf.sh
```

**Blocking mode (fail-closed) после фиксации baseline budgets:**
```bash
BSL_V2_PERF_GATE_BLOCKING=1 ./scripts/run-intellisense-perf.sh
```

Authoritative checked-in run использует script defaults:
- `PERF_WARMUP=20`
- `PERF_ITERATIONS=200`

**Небольшой smoke/debug sample, не для cutover verdict:**
```bash
PERF_WARMUP=1 PERF_ITERATIONS=5 THRESHOLD_P95=50 THRESHOLD_P99=50 THRESHOLD_RESOURCE=50 PERF_PROFILES="small large churn" ./scripts/run-intellisense-perf.sh
```

Representative matrix:
- operations: `completion`, `hover`, `definition`, `type_at_position`, `members`
- fixture families: `steady_member_chain`, `post_did_change_current_revision`,
  `object_module_explicit_context`, `recordset_module_explicit_context`,
  `incomplete_syntax_member_access`

`intellisense_churn` использует детерминированный `didChange`-churn (`every=1`), поэтому профиль устойчиво отличается от `large` по latency/resource.
`target_case` задаёт границу, перед которой churn применяется к `post_did_change_current_revision` fixture, не загрязняя `steady_*` measurements.
Для low-millisecond latency и near-zero lock wait blocking gate использует checked-in `relative_ratio_baseline_floors`, чтобы отличать настоящий regression от measurement jitter; blocking relative-ratio policy оценивает `p95`, а `p99` остаётся в отчёте и под absolute ceilings. Для `snapshot_preparation_ms` authoritative gate использует floor `5ms`, чтобы churn-tail jitter не выглядел как regression canonical fast path.

**Вывод:**
- Baselines: `backend/tests/perf/baselines/`
- Отчёты: `backend/tests/perf/reports/`

При `BSL_V2_PERF_GATE_BLOCKING=1` и валидном `CHANGE_ID`/`OPENSPEC_CHANGE_ID`
этот script является checked-in authoritative perf gate для cutover acceptance.

---

### `validate-v2-completion-gates.sh` - Readiness gates для `refactor-ir-canonical-semantic-pipeline`

**Назначение:** воспроизводимый fail-fast прогон checked-in readiness gates
для `refactor-ir-canonical-semantic-pipeline`:
- contract-sync guards (`scripts/test-intellisense-smoke-gate.py`,
  `scripts/test-intellisense-readiness-assets.py`) проверяют, что
  `quality-gates.json`, CI и default smoke path описывают один и тот же shipped
  selector set;
- shipped cross-adapter smoke: LSP/runtime/web/MCP/CLI/module-context slices, включая current-revision stale proofs beyond completion и regression на removal of bare-identifier fallback;
- authoritative representative-matrix perf gate через `./scripts/run-intellisense-perf.sh`
  в blocking mode для `small` / `large` / `churn`;
- fail-closed budget enforcement для mandatory операций `completion`, `hover`,
  `definition`, `type_at_position`, `members`;
- strict-валидация change через OpenSpec.

Скрипт формирует change-specific aggregate summary поверх authoritative отчётов:
- `backend/tests/perf/reports/intellisense_small.json`
- `backend/tests/perf/reports/intellisense_large.json`
- `backend/tests/perf/reports/intellisense_churn.json`

Скрипт не пересобирает `openspec/.../validation/acceptance-report.json`,
`quality-gates.json` или `execution-matrix.md`; эти checked-in acceptance assets
поддерживаются отдельно и должны синхронизироваться вручную с реально shipped
tests/contracts/docs.

**Использование:**
```bash
./scripts/validate-v2-completion-gates.sh
```

**Важно:** скрипт не зависит от `.github/workflows/*` и предназначен для локального запуска или внешнего CI (например, Jenkins/GitLab Runner).

**Артефакты:**
- `backend/tests/perf/reports/refactor-ir-canonical-semantic-pipeline-readiness-gate.json`
- `backend/tests/perf/reports/refactor-ir-canonical-semantic-pipeline-readiness-gate.md`
- `backend/tests/perf/reports/refactor-ir-canonical-semantic-pipeline-openspec-validate.log`

---

### `check-contract-compatibility-diff.py` - Manual compatibility-diff gate для `contracts/**`

**Назначение:** сравнить baseline и candidate versioned contracts и определить
`non_breaking`/`breaking` изменения с policy enforcement:
- breaking без major bump → fail (`breaking_without_major_bump`);
- major bump без migration note → fail (`missing_migration_note`).

**Использование:**
```bash
python3 scripts/check-contract-compatibility-diff.py \
  --baseline-ref master \
  --candidate-root contracts \
  --report artifacts/contracts-compatibility-diff-report.json
```

**Regression fixtures/tests (2 breaking + 2 non-breaking):**
```bash
python3 scripts/test-contract-compatibility-diff.py
```

Фикстуры лежат в `scripts/fixtures/contracts-compatibility-diff/`.

---

### `check-openspec-change-governance.py` - Fail-closed governance gate для OpenSpec change

**Назначение:** проверить machine-readable governance артефакты change:
- `change_criticality` (обязателен);
- `test_first_evidence` (обязателен для `behavioral|architectural|perf_critical`);
- ADR/doc-first минимум для `architectural|perf_critical`;
- обязательную acceptance matrix с pass/fail критериями для `architectural|perf_critical`;
- `bootstrap_policy` (`sample_size_min>=5`, `aggregation_rule=median`) для `perf_critical`;
- `dependency_checks` (D-связи из `tasks.md`) для `architectural|perf_critical`;
- `ownership_signoff` с role-based approvals для `architectural|perf_critical`;
- существование `failing_ref` / `passing_ref` (для файловых ссылок).

**Использование:**
```bash
python3 scripts/check-openspec-change-governance.py \
  --change-id add-performance-first-ai-engineering-guardrails
```

**Default automation path:** workflow `../.github/workflows/ci.yml` остаётся active readiness workflow для репозитория: он определяет затронутые `openspec/changes/<id>` и запускает governance gate fail-closed, а при релевантных source/runtime/contracts/docs изменениях дополнительно прогоняет shipped `./scripts/run-intellisense-tests.sh smoke` и active perf gate.

---

### `check-protected-assets-gate.py` - Fail-closed protected assets gate

**Назначение:** блокировать ad-hoc правки protected acceptance assets без явного
approved override артефакта в change governance. Для диапазона diff используется:
- явный `--base-ref`, если передан;
- иначе auto-resolve через `merge-base HEAD <base-branch>` (по умолчанию `origin/master`) с fallback на `HEAD~1`.

**Использование:**
```bash
python3 scripts/check-protected-assets-gate.py \
  --change-id add-performance-first-ai-engineering-guardrails \
  --manifest openspec/changes/add-performance-first-ai-engineering-guardrails/governance/protected_assets_manifest.txt \
  --override openspec/changes/add-performance-first-ai-engineering-guardrails/governance/protected_assets_override.json \
  --base-ref origin/master
```

---

### `check-perf-gate-architecture.py` - Option B architecture boundary check

**Назначение:** fail-closed валидация, что perf-verdict логика не размазана inline и
используется dedicated evaluator module.

**Использование:**
```bash
python3 scripts/check-perf-gate-architecture.py
```

---

### `check-rust-file-llm-budget.py` - LLM-friendly gate для production Rust файлов

**Назначение:** fail-closed проверка large-file budget для кампании
`refactor-production-rust-files-over-1000-loc`:
- hard policy для production scope: `LOC <= 1000`;
- LLM-friendly budget для target files: `LOC <= 800`, `bytes <= 80 KiB`,
  `tokens <= 12000` (`o200k_base`).
- policy на перенос тестов: inline test modules (`mod tests { ... }`,
  `mod *_tests { ... }`) в production `.rs` запрещены.

**Важно:** скрипт требует `tiktoken`.

**Использование (рекомендуется):**
```bash
uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py
```

**С JSON-отчётом:**
```bash
uv run --with tiktoken python3 scripts/check-rust-file-llm-budget.py --report artifacts/rust-llm-budget-report.json --json
```

В JSON-отчёте inline-test policy отражается в:
- `counts.inline_test_module_violations`
- `violations.inline_test_modules[]`

---

### `run-intellisense-tests.sh` - Smoke/full тесты IntelliSense

**Назначение:** стабильный набор тестов M8 для локального полного прогона; подходит для CI при подключении.

#### Политика уровней тестов (smoke vs manual/heavy)

В репозитории используется двухуровневая модель:

- **smoke**:
  - MUST проходить на чистом checkout репозитория;
  - MUST не требовать внешних фикстур/скачиваний/подготовки окружения (в т.ч. больших корпоративных конфигураций);
  - SHOULD выполняться быстро (ориентир: минуты, а не десятки минут);
  - цель: ежедневная проверка, что IntelliSense v2 не регрессирует по базовым сценариям.
- **manual/heavy**:
  - MAY требовать тяжёлые или внешние данные (например, Syntax Helper, большие конфигурации, специальные «битые» фикстуры);
  - MAY требовать запущенный сервер/особый сценарий запуска;
  - цель: расширенная проверка и расследование регрессий.

Инвентаризация `#[ignore]` и `tests/disabled/*` с причинами: `scripts/test-skip-inventory.md`.

**Использование:**
```bash
# Быстрый smoke (использует репозиторные фикстуры, не требует внешних данных)
./scripts/run-intellisense-tests.sh smoke

# Полный прогон (smoke + fixture конфигурации)
./scripts/run-intellisense-tests.sh full
```

**Состав:**
- `smoke`: completion/golden baseline + cross-adapter default-path selectors для LSP/runtime/web/MCP/CLI, включая anti-rescue invariants (no polluted-search backfill, no MCP parse-result semantic bypass), current-revision stale proofs для `hover` / `definition` / `type_at_position`, exact-index representative slices для `hover` / `signatureHelp` / `definition`, path-aware module-context/facet slices для Web и CLI, отдельный authoritative `Completion Timeline v9` drilldown contract slice (`service_future_created_at_ms`, `transport_to_service_future_wait_ms`, `service_future_to_scope_wait_ms`, существующие `transport_to_service_scope_wait_ms`, `service_scope_to_method_wait_ms`, bounded `pre_method_attribution_provenance`, request-id-safe overlap handling с отдельной fail-closed provenance regression, `wait_for_file_version_runtime`, `snapshot_with_deps_runtime`, `snapshot_with_deps_timeout_runtime`, bounded `timeout_attribution`, bounded `artifact_poll`, `dispatcher_resolution_latency_ms`, fail-open bounded serialization) и focused extension-host slice для `Completion Timeline` / `Client Probe Feed` / `Observability Incident Bundle` (`Completion Probe*`, включая `Completion Probe Runtime` transport hook/selection observer, `Completion Timeline*`, `Client Options`, `Observability Incident Bundle Test Suite`, `Observability Commands Test Suite`, `getCompletionTimeline` fail-closed/executeCommand, truthful observability-metrics capability semantics), который также проверяет `response.version=9` root-cause attribution, human-readable verdict projection (`server_before_method_entry_dominant` только при `same_request_authoritative`, `client_before_transport_dominant`, `handler_prelude_dominant`, `prepare_timeout@prepare_guard`, `exact_deadline@artifact_poll`), synthetic average trace notice для `v8` provenance и `v9` pre-service-scope split, request-centric incident handoff (`capture_scope`, `request_count`, bounded request list, deterministic probe correlation), явную деградацию для `v8`/`v7` payload и actual file export path для incident bundle.
- `full`: smoke + интеграционные тесты, которые загружают fixture конфигурации (например, `examples/conf/conf_test`).

---

## 🔧 Разрешение проблем

### Ошибка: "EmptyStandbyList.exe не найдена"

**Решение:**
```bash
cd tools
curl -L https://wj32.org/wp/download/releases/empty-standby-list/EmptyStandbyList.exe -o EmptyStandbyList.exe
```

Или скачай вручную: https://wj32.org/processhacker/forums/viewtopic.php?t=1569

См. `tools/DOWNLOAD_EmptyStandbyList.txt` для подробных инструкций.

### Ошибка: "Не удалось очистить кэш"

**Возможные причины:**
1. Отменил UAC prompt (нажал "Нет")
   - **Решение:** Запусти снова и нажми "Да" в UAC prompt
2. Антивирус заблокировал утилиту
   - **Решение:** Добавь `tools/EmptyStandbyList.exe` в исключения антивируса
3. Недостаточно прав администратора
   - **Решение:** Запусти GitBash от имени администратора

### Не вижу прогресс парсинга в VSCode

**Проверь:**
1. Кэш действительно очищен? (запусти `clear_cache.sh` снова)
2. LSP сервер обновлён? (`cp target/release/bsl-lsp-server.exe vscode-extension/bin/`)
3. VSCode Extension перезапущен? (закрой debug окно VSCode и запусти F5 снова)

---

## 📚 Дополнительная информация

- **Windows File System Cache:** Windows кэширует прочитанные файлы в RAM (Standby List). После парсинга 24979 файлов документации 1С всё кэшируется в памяти, и повторный парсинг занимает ~1 секунду вместо 1-2 минут.

- **EmptyStandbyList.exe:** Официальная утилита от разработчика Process Hacker (wj32). Очищает Standby List без перезагрузки системы.

- **Альтернативы:** RAMMap от Microsoft Sysinternals (GUI утилита) или PowerShell скрипты.

---

## ⚡ Быстрый прогон тестов: cargo-nextest

`scripts/build-all.sh` на этапе тестов использует `cargo nextest` (если `cargo-nextest` установлен) — это обычно быстрее и даёт более удобный вывод.

Установка:
```bash
cargo install cargo-nextest --locked
```

Управление:
- режим тестов: `./scripts/build-all.sh --tests quick|smoke|full`
  - `quick` (по умолчанию): debug + subset lib-тестов
  - `smoke`: `./scripts/run-intellisense-tests.sh smoke`
  - `full`: `cargo nextest run --release --workspace` (или `cargo test --release --workspace` если nextest выключен)
- после успешной полной сборки скрипт удаляет старые `vscode-extension/*.vsix` и создаёт новый пакет `vscode-extension/<name>-<version>.vsix`
- в режиме `--fast` упаковка `.vsix` пропускается: fast-профиль не пересобирает WASM-артефакты
- принудительно включить nextest: `./scripts/build-all.sh --nextest`
- выключить nextest: `./scripts/build-all.sh --no-nextest`

---

## 🛡️ Безопасность

Все скрипты:
- ✅ Используют официальные утилиты
- ✅ Требуют явного подтверждения UAC
- ✅ Не модифицируют системные файлы
- ✅ Только очищают кэш RAM (обратимая операция)

EmptyStandbyList.exe:
- ✅ Open Source (исходный код доступен)
- ✅ Подписана цифровой подписью
- ✅ Широко используется разработчиками

---

**См. также:**
- `tools/README.md` - Подробнее об утилитах
- `tools/DOWNLOAD_EmptyStandbyList.txt` - Инструкции по скачиванию
