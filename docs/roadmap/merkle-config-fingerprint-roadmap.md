# Roadmap: Merkle-фингерпринт конфигурации (XML + BSL)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Приоритет:** HIGH  
**Цель:** привести фингерпринт конфигурации к Merkle-дереву по артефактам (XML + BSL), чтобы обеспечить per-artifact invalidation и соответствие требованиям `disk-cache-platform-config-parsing-roadmap.md`.

---

## Контекст

Merkle‑фингерпринт реализован и используется в ключах DiskCache для конфигурации:
- `config_fingerprint` (слой A) — Merkle по XML метаданным;
- `config_layer_b_fingerprint` (слой B) — Merkle по XML + *.bsl модулям, извлечённым из метаданных.

Поддерживаются режимы:
- fast (mtime/size) — по умолчанию;
- strict (content hash) — через `BSL_CACHE_STRICT_FINGERPRINT`.

---

## Фактический статус (по коду): ✅

- Merkle‑сборка (leaf/node/root + версия `"merkle-root-v1"`) реализована в `backend/src/system/system_coordinator/config_loader.rs`.
- `config_fingerprint()` и `config_layer_b_fingerprint()` используют Merkle root для `source_fingerprint` (DiskCache keys): `backend/src/system/system_coordinator/config_loader.rs`.
- Тесты Merkle root (стабильность/изменение одного файла/odd-count/порядок/дедуп/XML+BSL) добавлены в `backend/src/system/system_coordinator/config_loader.rs` и проходят: `cargo test -p bsl-backend merkle_`.
- Интеграционная проверка DiskCache reuse проходит: `cargo test -p bsl-backend config_metadata_disk_cache_reuse`.

---

## Требования

### Функциональные
- Фингерпринт строится как Merkle‑дерево:
  - лист = hash(file_path + file_metadata + content_hash?);
  - уровень выше = hash(pair(left, right));
  - root = финальный fingerprint.
- Источники:
  - XML метаданные;
  - BSL модули;
  - расширяемые типы артефактов (макеты и др.).
- Возможность вычислять fingerprint быстро на mtime/size и полно на содержимом (опционально).

### Нефункциональные
- Детеминизм: одинаковые входные наборы → одинаковый Merkle root.
- Производительность: поддержка short‑circuit (если на leaf видно, что файл не менялся).
- Прозрачность: возможность логировать/экспортировать список leaf‑хэшей (для диагностики).

---

## Архитектура (предпочтительный вариант)

1) Собираем список файлов (XML + BSL + доп. артефакты), сортируем канонически.  
2) Для каждого файла строим leaf‑hash (path + size + mtime + content_hash?) и сохраняем в структуру `MerkleArtifact` (kind + path_norm).  
3) Собираем дерево снизу вверх попарным хешированием.  
4) Root используется в `config_fingerprint`/`config_layer_b_fingerprint`.

---

## Milestones

### M1: Спецификация формата Merkle‑листов ✅
**Цель:** утвердить формулу leaf‑hash и каноническую сортировку.
**Критерии успеха:**
- документирован формат leaf;
- определён порядок файлов и алгоритм сборки дерева;
- есть решение для odd‑count узлов (например, дублирование последнего).

#### Спецификация M1

**Формат leaf (v1):**
- leaf_hash (fast) = H( 0x00 || encode(artifact_kind) || 0x00 || encode(path_norm) || 0x00 || encode(size_u64) || 0x00 || encode(mtime_ns_u64) )
- leaf_hash (strict) = H( 0x00 || encode(artifact_kind) || 0x00 || encode(path_norm) || 0x00 || encode(content_hash) )
- H = blake3 (как в текущем проекте).
- content_hash = blake3(content).
- encode(...) = ASCII/UTF-8 bytes без локали; числовые поля в little-endian u64.
- artifact_kind: ASCII идентификатор типа артефакта (например, "xml", "bsl", "layout").

**Формат node:**
- node_hash = H( 0x01 || left_hash || right_hash ).
- left_hash/right_hash = 32 байта.

**Каноническая сортировка файлов:**
1) artifact_kind (лексикографически);
2) path_norm (лексикографически).

**Нормализация пути (path_norm):**
- относительный путь от корня конфигурации;
- разделитель "/" независимо от ОС;
- без "." и "..";
- без ведущего "./";
- без завершающего "/".

**Odd-count на уровне дерева:**
- если на уровне нечётное число узлов, последний дублируется как right.

**Пустой набор файлов:**
- если нет листьев, root_raw = H(0x01 || empty || empty), где empty = 32 нулевых байта.

**Версионирование формата:**
- в расчёт root включать префикс версии: root = H( 0x02 || encode("merkle-root-v1") || 0x00 || root_raw ).

### M2: Реализация Merkle‑фингерпринта ✅
**Цель:** внедрить Merkle‑root в `config_fingerprint` и `config_layer_b_fingerprint`.
**Критерии успеха:**
- отдельный модуль/функции для Merkle‑сборки;
- возможность переключать режим (fast vs strict).

### M3: Интеграция с DiskCache ✅
**Цель:** использовать Merkle‑root в ключах кэша конфигурации.
**Критерии успеха:**
- ключи config cache используют Merkle‑root;
- инвалидация по изменению одного файла влияет только на root.

### M4: Тесты и проверка ✅
**Цель:** доказать корректность и полезность Merkle‑схемы.
**Критерии успеха:**
- unit‑тест на стабильность root;
- тест на изменение одного файла → новый root;
- проверка соответствия требованиям из `disk-cache-platform-config-parsing-roadmap.md`.

---

## Definition of Done

- ✅ Merkle‑root применяется для config fingerprints.
- ✅ Документирован формат листов и дерево.
- ✅ Покрытие тестами (unit + интеграционные): `cargo test -p bsl-backend merkle_`, `cargo test -p bsl-backend config_metadata_disk_cache_reuse`.
- ✅ Обновлён статус и факты (по коду/тестам).
