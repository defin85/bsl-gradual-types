# План реализации M5: Metadata‑first completion (имена/фасеты/табличные части)

**Статус:** ✅ РЕАЛИЗОВАНО  
**Цель:** обеспечить полноту completion по метаданным конфигурации как основному сценарию 1С: имена объектов, фасеты (Manager/Object/Reference/Selection), табличные части, реквизиты и методы, а также разные иконки в редакторе для каждого `MetadataKind`.

---

## Область работ

- Completion по namespace метаданных:
  - `Документы.` → имена документов
  - `Справочники.` → имена справочников
  - и др. коллекции (`РегистрыСведений`, `ПланыСчетов`, ...)
- Completion внутри объекта:
  - `Документы.<Документ>.` → фасеты и members
  - `...<Документ>.ТабличнаяЧасть.` → коллекция строк/строка/реквизиты
- Связка с `TypeResolution` (facet‑aware)
- Отдельные kinds/иконки для каждого `MetadataKind` (клиент LSP должен видеть разные `CompletionItemKind`).

---

## Пошаговый план

### Шаг 1: Контракт метаданных для completion
- Определить, какие сущности и связи нужны в индексе:
  - objects, facets, attributes, tabular sections, commands, формы (если применимо)
- Зафиксировать правила отображения в `CompletionItem` (kind/label/detail).

**Выход:** единый контракт metadata‑слоя для completion.

---

### Шаг 2: Индексация и инвалидация
- Убедиться, что индексы метаданных пересобираются при изменении конфигурации и не дают mixed state.
- Связать с fingerprint/snapshot‑версией (вместе с M2/M7).

**Выход:** актуальные metadata индексы во время редактирования.

---

### Шаг 3: Facet‑aware member lookup
- Реализовать переходы facet‑ов:
  - Manager → Object/Reference/Selection (и обратные, где применимо)
  - TabularSection → Row/Collection типы

**Выход:** completion показывает корректные members для каждой facet‑ветки.

---

### Шаг 4: Тесты на примере конфигурации
- Использовать fixture конфигурации из `examples/` для golden/integration кейсов.

**Выход:** воспроизводимые тесты metadata completion.

---

### Шаг 5: Отдельные kinds и иконки для всех MetadataKind (LSP)
- Расширить доменный `CompletionKind` так, чтобы у каждого `MetadataKind` был свой kind.
- Обновить LSP mapping (`CompletionKind` -> `CompletionItemKind`) так, чтобы каждый `MetadataKind` имел свою иконку в редакторе (использовать только стандартные значения LSP `CompletionItemKind`, без проприетарных/нестандартных enum).
- Класть точный metadata-kind в `CompletionItem.data.kind` (например `metadata.information_register`), чтобы:
  - тесты могли проверять kind независимо от конкретного клиента;
  - клиент мог делать доп. UI поверх стандартных иконок.
- Добавить тесты, которые фиксируют виды (`kind`) хотя бы для нескольких non-Document/Catalog/Enum объектов (из fixture или через синтетический index snapshot).

**Выход:** completion по метаданным показывает разные иконки для каждого metadata‑kind.

---

## Критерии завершения

- `Документы./Справочники./...` дают корректные подсказки имён.
- Внутри объекта подсказки учитывают facet и табличные части.
- Есть тесты по fixture конфигурации.
- Для каждого `MetadataKind` используется отдельный kind + отдельная иконка в редакторе.

---

## Задачи (тикеты) по M5

### T1: Контракт metadata completion ✅
**DoD:**
- определены сущности и связи;
- определены правила отображения.

### T2: Индексация + инвалидация metadata ✅
**DoD:**
- пересборка по изменению конфигурации;
- нет mixed state со snapshot id.

### T3: Facet‑aware lookup ✅
**DoD:**
- корректные переходы facet‑ов;
- тесты на `.Ссылка/.Объект` и табличные части.

### T4: Интеграционный тест completion на fixture конфигурации ✅
**DoD:**
- добавлены кейсы по `examples/conf/...` (интеграция через `AnalysisV2` + `get_completion_with_semantic_program_snapshot_v2`, не LSP golden);
- стабильные результаты.

### T5: Отдельные kinds/иконки для каждого MetadataKind ✅
**DoD:**
- доменный `CompletionKind` покрывает все `MetadataKind` (1:1);
- LSP completion использует разные `CompletionItemKind` для разных metadata‑kinds (уникально для каждого `MetadataKind`);
- `CompletionItem.data.kind` содержит точный metadata-kind (стабильный ключ);
- добавлены тесты на kinds/иконки для non-Document/Catalog/Enum.

---

## Прогресс (факты по коду)

- Контракт metadata completion (kind/detail): `backend/src/application/type_system/services/completion_service.rs` (`add_metadata_items`).
- Facet-aware резолвинг цепочек метаданных (`Документы.<Имя>.` и далее): `backend/src/application/type_system/services/completion_service.rs` (`resolve_receiver_types_from_expression`, `resolve_member_chain_owner_type_sync`).
- Табличные части: `.Работы` резолвится как `ТабличнаяЧасть<Строка...>` на основе конфигурации; `Добавить/Вставить/Получить/Найти` возвращают тип строки табличной части: `backend/src/application/type_system/services/completion_service.rs` (`resolve_property_access_type`, `resolve_method_call_return_type`).
- Доступные фасеты берутся из loaded config (или из fallback mapping по MetadataKind): `shared/src/domain/resolver/member_resolution.rs` (`default_facets_for_kind`).
- T5 (отдельные kinds/иконки): `shared/src/domain/repository.rs` (`CompletionKind::from_metadata_kind`, `CompletionKind::metadata_kind`), `backend/src/bin/lsp_server/handlers/completion.rs` (`map_completion_kind`, `completion_kind_tag` + тесты).
- Snapshot-consistency при загрузке/перезагрузке конфигурации:
  - атомарный reset snapshot id + очистка metadata/type: `backend/src/system/intellisense_index.rs` (`reset_metadata_snapshot`),
  - использование в loader: `backend/src/system/system_coordinator/config_loader.rs`,
  - фиксация combined-cache path (metadata/type индекс не остаётся пустым): `backend/src/system/system_coordinator/lifecycle.rs` (`update_intellisense_index_from_config_raw_types`).
- Интеграционный тест на fixture конфигурации: `backend/tests/metadata_completion_fixture_test.rs`.

## Проверка

- `cargo test -p bsl-backend --test metadata_completion_fixture_test`
- `cargo test -p bsl-backend reset_metadata_snapshot_updates_id_and_clears_indexes`
- `cargo test -p bsl-backend metadata_completion_kinds_have_unique_lsp_kinds`
- `cargo test -p bsl-backend metadata_completion_items_have_granular_kind_in_data`

**Факт (2026-01-13):**
- `cargo test -p bsl-backend --test metadata_completion_fixture_test` — ok (1/1)
- `cargo test -p bsl-backend reset_metadata_snapshot_updates_id_and_clears_indexes` — ok (1/1)
- `cargo test -p bsl-backend metadata_completion_kinds_have_unique_lsp_kinds` — ok (1/1)
- `cargo test -p bsl-backend metadata_completion_items_have_granular_kind_in_data` — ok (1/1)
