# Детализированный план: дисковый кэш парсинга платформы/конфигурации

Связан с `docs/roadmap/disk-cache-platform-config-parsing-roadmap.md`. Цель - детализировать реализацию, артефакты, метрики и управление кэшем, включая AST cache и панель расширения.

## Допущения и договоренности

- Два слоя кэша (A: результаты парсинга/дискавери, B: производные индексы) сохраняются.
- Кэш отключаемый: CLI/env + LSP команда + UI toggle.
- AST кэш требуется в двух уровнях: L1 in-memory + L2 disk snapshot.
- Очистка кэша только per-project/per-config.
- Структуры должны быть сериализуемы в общий формат; `manifest.json` обязателен.

## Ключевые решения (до реализации)

- Формат артефактов: единый формат (например, serde + bincode/CBOR + zstd), без SQLite.
- Идентификаторы:
  - `project_id = hash(root_path)`;
  - `config_id = uuid из Configuration.xml`;
  - `extension_id = uuid из Configuration.xml`;
  - `config_set_id = config_id + sorted(extension_id[])`;
  - `platform_id` (канонизированный путь + версия при наличии).
- Fingerprint: быстрый mtime+size, fallback на blake3.
- Метрики: счетчики hit/miss и тайминги parse/build/load; окно агрегации для UI.

## Структура кэша (предложение)

Базовый путь: `${BSL_CACHE_DIR:-OS_CACHE}/bsl-gradual-types/`.

```
<cache_root>/
  v<schema_version>/
    <source_kind>/                # platform | config | combined | ast
      <project_id>/               # для platform может быть "global"
          <config_id>/              # для platform может быть "none"
          <key_hash>/
            artifact.bin
            manifest.json
```

`manifest.json`:
- schema_version, source_kind, source_identity, source_fingerprint, settings_fingerprint
- created_at, size_bytes, build_time_ms
- optional: stats (hit_count, last_used_at)

## D1: DiskCache API + manifest + locking

Статус: DONE

Задачи:
- Ввести `DiskCache` API: `get_or_build(key, builder) -> artifact`.
- Реализовать file-lock на ключ, запись `tmp + atomic rename`.
- Сериализация/десериализация общего формата.
- Поддержка `BSL_CACHE_DIR` и `BSL_CACHE_DISABLE`.

Артефакты:
- Модуль кэша, тесты на lock/atomic write.
- `manifest.json` и схема ключа.

Тесты:
- Unit: atomic write, lock, schema_version mismatch.

Факты:
- Реализован `DiskCache` + lock + manifest.
- Интеграция в `SystemCoordinator` (инициализация и геттер).
- Запущено `cargo test -p bsl-backend disk_cache`.

## D2: Cache платформы (слой A)

Статус: DONE

Задачи:
- Кэширование результата парсинга `syntax_helper`.
- Инвалидация при изменении пути/версии/содержимого.
- Интеграция с `SyntaxHelperLoader`.

Артефакты:
- `source_kind=platform`, layer A.

Тесты:
- Интеграционный: повторный запуск не парсит полностью.

Факты:
- DiskCache интегрирован в загрузку `syntax_helper` (platform layer A).
- Fingerprint по HTML файлам + настройкам парсера, ключ на blake3.
- Тест: `cargo test -p bsl-backend syntax_helper_disk_cache_reuse`.

## D3: Cache платформы (слой B)

Статус: DONE

Задачи:
- Сериализация `Vec<RawTypeData>` или готовых структур `SignatureIndex`.
- Обеспечить детерминированность сериализации.

Артефакты:
- `source_kind=platform`, layer B.

Тесты:
- Интеграционный: наличие ожидаемых методов (например `Массив.Добавить`).

Факты:
- DiskCache интегрирован в конвертацию `SyntaxHelperDatabase -> Vec<RawTypeData>`.
- Добавлен тест `test_platform_raw_cache_produces_signature_index`.
- Тест: `cargo test -p bsl-backend platform_raw_cache_produces_signature_index`.

## D4: Cache конфигурации (слой A)

Статус: DONE

Задачи:
- Кэш результата discovery/парсинга `Configuration.xml`.
- Инвалидация на изменение XML/метаданных.

Артефакты:
- `source_kind=config`, layer A.

Тесты:
- Интеграционный на примерах `examples/conf*` (если доступны).

Факты:
- DiskCache интегрирован в discovery метаданных конфигураций (layer A).
- Fingerprint по XML файлам + strict режим через `BSL_CACHE_STRICT_FINGERPRINT`.
- Добавлен тест `test_config_metadata_disk_cache_reuse`.
- Тест: `cargo test -p bsl-backend config_metadata_disk_cache_reuse`.

## D5: Cache конфигурации (слой B)

Статус: DONE

Задачи:
- Индекс экспортных методов + `definition_locations`.
- File-level кэш для `.bsl` модулей.
- Merkle-фингерпринт по BSL/XML/другим артефактам.

Артефакты:
- `source_kind=config-layer-b`, layer B.
- `source_kind=config-module-parse`, per-file entries.

Тесты:
- Изменение `.bsl` -> инвалидация только модуля, индекс обновляется.
- Тест: `cargo test -p bsl-backend cached_index_reuses_unchanged_modules`.

Факты:
- DiskCache интегрирован в layer B: `RawTypeData` + индекс экспортных методов.
- Per-file кэш парсинга модулей BSL с повторным использованием.
- Fingerprint по XML + BSL (strict режим через `BSL_CACHE_STRICT_FINGERPRINT`).
- Discovery сохраняет фактический путь CommonModule (для корректной индексации/кэша).

## D6: Combined cache

Статус: DONE

Задачи:
- Комбинированный ключ (platform + config + settings).
- Быстрый "run -> готово" для hover/validation.

Артефакты:
- `source_kind=combined`.

Тесты:
- Интеграционный: `test_combined_cache_roundtrip` (examples).

Факты:
- Добавлен combined cache ключ (platform + config meta) и загрузка перед парсингом конфигурации.
- Реализован сбор combined payload (platform raw + config raw + index) и запись в кэш.
- Тест: `cargo test -p bsl-backend test_combined_cache_roundtrip`.
- Уточнён combined payload без platform raw, прогресс обновляется при hit, meta учитывает быстрый path через config cache.

## D7: Политики обновления и уборка

Задачи:
- Политики: `blocking` vs `stale-while-revalidate` (опционально).
- Cleanup по размеру/TTL.
- Метрики hit/miss, build/load time.

Артефакты:
- Политики, счетчики, логирование.

Тесты:
- Unit на cleanup/TTL.

## AST cache (дополнительный блок)

Цель: ускорить открытие модулей и повторные операции над AST.

L1 in-memory:
- LRU по открытым документам.
- Инвалидация по версии документа.

L2 disk snapshot:
- `source_kind=ast`, ключ включает `project_id/config_id`, `file_path`, `file_hash`, `parser_settings`.
- Сериализация AST + минимальный индекс (если нужен).

Метрики:
- `ast_parse_time_ms`, `open_file_latency_ms`, `ast_cache_hit/miss`.

Тесты:
- Unit на LRU и инвалидацию.
- Интеграционный: открытие большого файла быстрее после прогрева.

## LSP/CLI/Env управление кэшем

CLI/env:
- `BSL_CACHE_DISABLE=1` (no-op режим).
- `BSL_CACHE_DIR=...`.

LSP команды (черновик):
- `bsl.cache.status` -> краткий статус (enabled, size, entries).
- `bsl.cache.metrics` -> счетчики/тайминги.
- `bsl.cache.clear(project|config)` -> очистка per-project/per-config.
- `bsl.cache.toggle` -> включить/выключить (persist в settings).

## Панель расширения (мини-дашборд)

UI блоки:
- Статус кэша (enabled/disabled, policy).
- Метрики (hits, misses, hit_ratio).
- Тайминги (parse/build/load p50/p95 если есть).
- Размер/число ключей.

UI действия:
- Toggle cache.
- Clear per-project/per-config.

## Definition of Done (общие)

- `cargo test --workspace`
- Интеграционные тесты по этапам.
- Проверка фактов через `rg` по точкам интеграции.
