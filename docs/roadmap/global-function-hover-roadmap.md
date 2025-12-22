# Roadmap: Hover глобальных функций и UX вокруг hover/индексации

**Статус:** 🟢 СДЕЛАНО  
**Приоритет:** HIGH  
**Цель:** добавить описания глобальных функций в hover и зафиксировать сопутствующие UX‑задачи (автодополнение, индексация, hover в условиях).

---

## Контекст (текущее состояние)

- Hover для `FunctionCall` показывает IR‑узел, описания глобальных функций не выводятся.
- Глобальные функции берутся из Syntax Helper, но описание теряется на пути до hover.
- Есть несколько UX‑проблем: AutoReindex при сохранении, `Dynamic` в условиях, потеря типа при конкатенации.
- Есть идеи по управлению автоиндексацией и развитию IDE‑фич.

---

## Требования

- Hover для `SemanticNodeKind::FunctionCall` показывает сигнатуру и описание/описание возврата при наличии.
- Поведение hover для переменных/свойств не меняется; fallback сохраняется.
- Обратная совместимость сериализации `MethodSignature` сохраняется.

---

## Scope

- In: `MethodSignature` docs, конвертация глобальных функций, hover форматирование, фиксация UX‑наблюдений.
- Out: изменения Web UI/фильтров и запуск серверов.

---

## План работ (главная задача)

1) ✅ Расширить `MethodSignature` doc‑полями (`description`, `return_description`), обновить `Clone` и `new`.  
2) ✅ В `convert_syntax_helper_global_functions` заполнить описания (и для русских, и для английских алиасов), отсекая пустые строки.  
3) ✅ Добавить форматирование hover для `FunctionCall` через `HoverFormatter` + lookup сигнатуры.  
4) ✅ Добавить тесты (formatter/конвертация).  
5) ✅ Обновить оставшиеся вызовы `MethodSignature::new` под новую сигнатуру.

---

## Фактический прогресс (проверено по репозиторию)

- ✅ `MethodSignature` расширен doc‑полями и обновлены `Clone/new`: `shared/src/domain/signature_index/method.rs:50`, `shared/src/domain/signature_index/method.rs:121`.
- ✅ Конвертация глобальных функций переносит `description`/`return_description` и для английских алиасов: `backend/src/data/adapters/converters.rs:310`, `backend/src/data/adapters/converters.rs:365`.
- ✅ Hover для `FunctionCall` использует lookup сигнатуры и форматирует описания: `backend/src/application/type_system/services/hover_service.rs:265`, `backend/src/helpers/hover_formatter/formatter.rs:186`.
- ✅ Добавлены тесты форматтера/конвертации: `backend/src/helpers/hover_formatter/tests.rs:40`, `backend/src/data/adapters/converters.rs:530`.
- ✅ Обновлены старые вызовы `MethodSignature::new`: `shared/src/domain/repository.rs:513`, `backend/tests/lsp_signature_help_test.rs:453`.

---

## Сопутствующие IDE/UX задачи

### Автодополнение и Code Actions (Milestones 3.1/3.2)

- LSP completion включён (триггеры `.` и пробел), сейчас даёт базовые подсказки (ключевые слова/примитивы/несколько функций).
- VSCode клиент использует LSP completion и собирает метрики.
- CLI `complete` работает по типам из репозитория.
- Web API completion описан как будущий endpoint (в роутинге не реализован).
- Milestone 3.1: Code Intelligence (навигация/Signature Help).
- Milestone 3.2: Code Actions (Quick Fix/Refactor/Generate Code).

### Инкрементальная переиндексация при сохранении (AutoReindex)

- ✅ VSCode собирает изменённые пути (`**/Ext/*.bsl`, `**/*.xml`) и отправляет их в `bsl/incrementalUpdate`: `vscode-extension/src/commands/index-commands.ts:121`, `vscode-extension/src/commands/index-commands.ts:177`, `vscode-extension/src/lsp/customRequests.ts:226`.
- ✅ LSP принимает `changed_paths` и при наличии кэша обновляет только затронутые XML/BSL, иначе делает полный parse: `backend/src/bin/lsp_server/types.rs:94`, `backend/src/bin/lsp_server/commands/configuration.rs:398`, `backend/src/data/loaders/config_bsl_modules/indexing.rs:400`.
- ✅ Кэш автоиндексации хранит только data-only снимки метаданных/сигнатур, пригоден для disk-cache: `backend/src/system/system_coordinator/types.rs:38`, `backend/src/data/loaders/config_bsl_modules/types.rs:14`.
- Связь: `docs/roadmap/disk-cache-platform-config-parsing-roadmap.md` (D4/D5) — остаётся добавить дисковый слой/хеши поверх текущих снапшотов.

### Hover в условиях и циклах показывает Dynamic

- Симптом: hover в `Если/Пока/Для` показывает `Условие: Dynamic`.
- Цель: более информативный hover (ожидаемый тип `Булево` + фактический `TypeResolution` с certainty/причиной).
- ✅ Для бинарных выражений теперь читается оператор и выводится `Булево` для сравнений: `backend/src/system/tree_sitter_adapter/expression_converter.rs:330`, `backend/src/application/ast_to_ir/type_inference.rs:79`.
- ✅ Проверен hover в реальных условиях/циклах и форматирование certainty/причины для `TypeResolution`: `backend/src/application/type_system/formatters/hover_formatters.rs:161`, `backend/tests/condition_loop_hover_test.rs`.

### Самоприсваивание с конкатенацией теряет тип

- Симптом: `ТекстЗаголовка = ТекстЗаголовка + НСтр("ru = ' по командировке'");` — тип не резолвится, хотя ранее был `Строка`.
- Задача: проверить flow‑sensitive обновление типа при бинарных операциях.
- Ожидаемое поведение: после присваивания тип остаётся `Строка`.
- ✅ Тип `Строка` сохраняется для `строка + строка`, а `строка + Known(не-строка)` даёт Unknown с причиной: `backend/src/application/ast_to_ir/type_inference.rs:388`, `shared/src/domain/types/certainty.rs:79`.
- ✅ Диагностика конкатенации становится Error при явном Known и не трогает Unknown: `shared/src/domain/validators/type_validator.rs:220`, `shared/src/domain/validators/error_kinds.rs:95`, `shared/src/domain/validators/error_formatting.rs:176`.
- ✅ Тесты на сохранение типа и ошибку конкатенации: `backend/tests/ast_to_ir_assignment_test.rs:257`, `backend/tests/string_concat_validation_test.rs:13`.

### Управление автоиндексацией в LSP (pause/resume)

- Идея: временно отключать автоиндексацию/диагностику на время интенсивных правок.
- Сценарий: много изменений → отключить индексацию → завершить → включить и проверить.
- Предложение:
  - настройка `bsl.autoReindex.enabled` (true/false);
  - команды `bsl.pauseAutoReindex`, `bsl.resumeAutoReindex`, `bsl.reindexNow`;
  - индикатор в status bar “Indexing paused”.
- Риск: устаревшие hover/completion при выключенной индексации; нужен явный индикатор.

---

## Тестирование и проверка

- `cargo test -p bsl-shared signature_index` — ✅ 56 тестов (0 fail).
- `cargo test -p bsl-backend hover_formatter` — ✅ 23 теста (0 fail).
- Если Web API поднят пользователем: проверить hover для `НСтр` (не проверялось).

---

## Риски и edge cases

- Курсор на аргументах/объекте внутри `FunctionCall` может попадать в fallback.
- `object_type` может быть `None`/`Unknown`, сигнатура не найдётся.
- Английские алиасы без описания дадут пустой hover.

---

## Открытые вопросы

- Показывать ли описание/`return_description` для методов объектов так же, как для глобальных функций?
- Нужно ли выводить `english_name` рядом с русским именем в hover?
