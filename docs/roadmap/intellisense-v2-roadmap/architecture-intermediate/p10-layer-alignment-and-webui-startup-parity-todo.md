# P10: TODO list — Выравнивание слоёв + Startup parity для WebUI (deps/config/index)

**Дата:** 2026-01-12  
**Статус:** 🔴 TODO  
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

- [ ] Ввести структуру `StartupInputs` (system слой), которая описывает все входы, влияющие на deps/config/index.
- [ ] Реализовать `StartupInputs::normalize()`:
  - [ ] одинаковая нормализация путей (absolute/canonical где возможно; одинаковые дефолты);
  - [ ] `configuration_path` нормализуется к root директории (если передан `Configuration.xml` — использовать parent);
  - [ ] `platform_version` приводится к одному формату (тот же формат, что используется при инициализации);
  - [ ] `strict_fingerprint` становится явным параметром (не только через env).
- [ ] Добавить “bridge” функции:
  - [ ] `StartupInputs::from_lsp_settings(...)`
  - [ ] `StartupInputs::from_web_flags(...)`
  - [ ] (опционально) `StartupInputs::to_log_fields()` для одинаковых логов.

### B) Один entrypoint для “startup → deps bundle”

- [ ] В system слое сделать один публичный entrypoint, который принимает `StartupInputs` и возвращает:
  - [ ] готовый `SystemCoordinator` (инициализированный через `start_with_paths(...)`);
  - [ ] `DepsBundleV2` (с заполненным `meta`);
  - [ ] “effective inputs” (после нормализации).
- [ ] Перевести WebUI запуск на этот entrypoint (вместо ручного `start_with_paths + build_deps_bundle_v2`).
- [ ] Перевести LSP путь “инициализация + deps_update_v2 на старте” на тот же entrypoint/те же нормализаторы.

### C) Прозрачность снапшота в WebUI (мета и сравнение)

- [ ] Добавить API endpoint: `GET /api/snapshot/meta` (или аналог) в Web server:
  - [ ] `deps_id`
  - [ ] `index_snapshot_id`
  - [ ] `platform_version`
  - [ ] `platform_fingerprint` / `config_fingerprint`
  - [ ] `strict_fingerprint`
  - [ ] `RepositoryStats` (total/platform/config/user_defined)
  - [ ] нормализованные пути входов (для сопоставления с LSP settings)
- [ ] В UI показать “Snapshot banner” (read-only):
  - [ ] значения meta (копируемые),
  - [ ] кнопку “reload deps” (пересборка bundle) с отображением нового `deps_id/index_snapshot_id`.
- [ ] Документировать ручную проверку parity:
  - [ ] как получить `deps_id/index_snapshot_id` из WebUI,
  - [ ] как получить это же из логов LSP.

### D) Выравнивание слоёв (Application vs Presentation)

Цель: LSP и Web должны быть тонкими адаптерами; бизнес‑логика v2 должна жить в `backend/src/application/...`.

- [ ] Вынести core‑логику signatureHelp (v2) в application слой (новый service/module).
- [ ] Вынести core‑логику go to definition (v2) в application слой (новый service/module).
- [ ] Переподключить LSP handlers на вызовы application функций (без дублирования логики).
- [ ] (Опционально) унифицировать semantic visualization:
  - [ ] один источник DTO (semantic tree),
  - [ ] один рендерер HTML (или один “view”, который умеют рендерить разные frontends).

### E) Тесты (минимум для корректности и воспроизводимости)

- [ ] Unit tests для `StartupInputs::normalize()` (основные кейсы путей + version).
- [ ] Тест на стабильность “по входам”: одинаковые `StartupInputs` → одинаковые `deps_id/index_snapshot_id`.
- [ ] Интеграционный тест: “Web startup” и “LSP startup” используют один нормализатор (проверка не через процессы, а через общий entrypoint/функции).

---

## DoD

- [ ] WebUI и LSP используют один и тот же нормализованный `StartupInputs` и один entrypoint подготовки deps/index на старте.
- [ ] WebUI отображает пользователю `deps_id/index_snapshot_id` и прочую мету снапшота.
- [ ] SignatureHelp + GoToDefinition v2 реализованы в application слое, LSP/Web — только адаптеры.
- [ ] Есть тесты, которые ловят регрессии нормализации/идентификаторов.
- [ ] Есть короткая инструкция “как руками проверить parity” (Web vs LSP).

---

## Верификация (минимум)

```bash
cargo test -p bsl-backend --bin bsl-lsp-server -- --color never
cargo test --workspace --no-run
```

