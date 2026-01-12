# P10: TODO list — Выравнивание слоёв + Startup parity для WebUI (deps/config/index)

**Дата:** 2026-01-12  
**Актуализировано:** 2026-01-12  
**Статус:** 🟢 Выполнено  
**Контекст:** WebUI используется для отладки, но должен показывать тот же “фактический репозиторий типов”, что и LSP **при запуске** (одинаковые deps/config/index при одинаковых входах).

---

## Цель

Сделать так, чтобы при одинаковых входных данных:

- `syntax_helper_path` (platform docs),
- `configuration_path` (root конфигурации),
- `platform_version`,
- `cache_enabled`,
- `strict_fingerprint`,

и LSP, и WebUI:

- строили одинаковый `DepsBundleV2` (`deps_id` + `semantic_deps`),
- использовали один и тот же `IndexSnapshot` (`index_snapshot_id`),
- могли “доказуемо” показать пользователю мета‑информацию снапшота (что именно загружено).

Важно: это **не** про live‑совпадение с уже запущенным LSP (разные процессы). Это про совпадение “по входам” на старте.

---

## Не-цели (в P10)

- Live state WebUI = state LSP (IPC/daemon не делаем).
- Инкрементальная синхронизация открытых документов между процессами.
- Глубокая оптимизация latency (сначала корректность/воспроизводимость).

---

## Work Items

### A) Единый формат входов запуска (`StartupInputs`)

- [x] Ввести структуру `StartupInputs` (system слой), которая описывает все входы, влияющие на deps/config/index.
- [x] Реализовать `StartupInputs::normalize()`:
  - [x] одинаковая нормализация путей (absolute/canonical где возможно; одинаковые дефолты);
  - [x] `configuration_path` нормализуется к root директории (если передан `Configuration.xml` — использовать parent);
  - [x] `platform_version` приводится к одному формату (тот же формат, что используется при инициализации);
  - [x] `strict_fingerprint` становится явным параметром (не только через env).
- [x] Добавить “bridge” функции:
  - [x] `StartupInputs::from_lsp_settings(...)`
  - [x] `StartupInputs::from_web_flags(...)`
  - [ ] (опционально) `StartupInputs::to_log_fields()` для одинаковых логов.

### B) Один entrypoint для “startup → deps bundle”

- [x] В system слое сделать один публичный entrypoint, который принимает `StartupInputs` и возвращает:
  - [x] готовый `SystemCoordinator` (инициализированный через `start_with_paths(...)`);
  - [x] `DepsBundleV2` (с заполненным `meta`);
  - [x] “effective inputs” (после нормализации).
- [x] Перевести WebUI запуск на этот entrypoint (вместо ручного `start_with_paths + build_deps_bundle_v2`).
- [x] Перевести LSP путь “инициализация + deps_update_v2 на старте” на тот же entrypoint/те же нормализаторы.

### C) Прозрачность снапшота в WebUI (мета и сравнение)

- [x] Добавить API endpoint: `GET /api/snapshot/meta` (или аналог) в Web server:
  - [x] `deps_id`
  - [x] `index_snapshot_id`
  - [x] `platform_version`
  - [x] `platform_fingerprint` / `config_fingerprint`
  - [x] `strict_fingerprint`
  - [x] `RepositoryStats` (total/platform/config/user_defined)
  - [x] нормализованные пути входов (для сопоставления с LSP settings)
- [x] В UI показать “Snapshot banner” (read-only):
  - [x] значения meta (копируемые),
  - [x] кнопку “reload deps” (пересборка bundle) с отображением нового `deps_id/index_snapshot_id`.
- [x] Документировать ручную проверку parity:
  - [x] как получить `deps_id/index_snapshot_id` из WebUI,
  - [x] как получить это же из логов LSP.

### D) Выравнивание слоёв (Application vs Presentation)

Цель: LSP и Web должны быть тонкими адаптерами; бизнес‑логика v2 должна жить в `backend/src/application/...`.

- [x] Вынести core‑логику signatureHelp (v2) в application слой (новый service/module).
- [x] Вынести core‑логику go to definition (v2) в application слой (новый service/module).
- [x] Переподключить LSP handlers на вызовы application функций (без дублирования логики).
- [ ] (Опционально) унифицировать semantic visualization:
  - [ ] один источник DTO (semantic tree),
  - [ ] один рендерер HTML (или один “view”, который умеют рендерить разные frontends).

### E) Тесты (минимум для корректности и воспроизводимости)

- [x] Unit tests для `StartupInputs::normalize()` (основные кейсы путей + version).
- [x] Тест на стабильность “по входам”: одинаковые `StartupInputs` → одинаковые `deps_id/index_snapshot_id`.
- [x] Интеграционный тест: “Web startup” и “LSP startup” используют один нормализатор (проверка не через процессы, а через общий entrypoint/функции).

---

## DoD

- [x] WebUI и LSP используют один и тот же нормализованный `StartupInputs` и один entrypoint подготовки deps/index на старте.
- [x] WebUI отображает пользователю `deps_id/index_snapshot_id` и прочую мету снапшота.
- [x] SignatureHelp + GoToDefinition v2 реализованы в application слое, LSP/Web — только адаптеры.
- [x] Есть тесты, которые ловят регрессии нормализации/идентификаторов.
- [x] Есть короткая инструкция “как руками проверить parity” (Web vs LSP).

---

## Верификация (repo-wide)

### 1) Тесты / компиляция

```bash
cargo test -p bsl-backend startup_inputs_normalize -- --color never
cargo test -p bsl-backend same_startup_inputs_produce_stable_deps_and_index_ids -- --color never
cargo test -p bsl-backend --bin bsl-lsp-server -- --color never
cargo test --workspace --no-run
```

### 2) Ручная проверка parity (Web vs LSP)

**Web** (сервер должен быть запущен с теми же входами, что и LSP):

```bash
curl -s http://localhost:8080/api/snapshot/meta
curl -s -X POST http://localhost:8080/api/snapshot/reload
```

В UI то же самое видно в “Snapshot banner” (header).

**LSP**: в логах найти строку вида:

```text
deps_update_v2 applied: reason=..., deps_id=..., index_snapshot_id=..., platform_version=..., platform_fp=..., config_fp=..., strict_fingerprint=...
```

Ожидается: при одинаковых входах `deps_id` и `index_snapshot_id` совпадают между Web и LSP.

### Факты (2026-01-12)

- `cargo test -p bsl-backend startup_inputs_normalize -- --color never`:
  ```text
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 241 filtered out; finished in 0.00s
  ```
- `cargo test -p bsl-backend same_startup_inputs_produce_stable_deps_and_index_ids -- --color never`:
  ```text
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 243 filtered out; finished in 0.00s
  ```
- `cargo test -p bsl-backend --bin bsl-lsp-server -- --color never`:
  ```text
  test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
  ```
- `cargo test --workspace --no-run`:
  ```text
  Finished `test` profile [unoptimized + debuginfo] target(s) in 33.39s
  ```
