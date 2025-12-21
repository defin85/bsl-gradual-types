# Roadmap: Hover глобальных функций и UX вокруг hover/индексации

**Статус:** 🔴 ПЛАН  
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

1) Расширить `MethodSignature` doc‑полями (`description`, `return_description`), обновить `Clone` и `new`.  
2) В `convert_syntax_helper_global_functions` заполнить описания (и для русских, и для английских алиасов), отсекая пустые строки.  
3) Добавить форматирование hover для `FunctionCall` через `HoverFormatter` + lookup сигнатуры.  
4) Добавить тесты (formatter/конвертация).

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

- При сохранении `*.bsl` триггерится AutoReindex и, судя по прогрессу, идёт почти полная переиндексация.
- Пример логов:
```text
[AutoReindex] Schedule: change: **/Ext/*.bsl
[Progress] BEGIN: bsl-incremental-update-1766311449309 | Incremental index update
[Progress] REPORT: bsl-incremental-update-1766311449309 | Validation OK (10%)
[Progress] REPORT: bsl-incremental-update-1766311449309 | Файл 2335/15192: МенеджерОборудованияВызовСервераПереопределяемый (13%)
[Progress] REPORT: bsl-incremental-update-1766311449309 | Файл 5085/15192: УчетныеЗаписиСинхронизацииФайлов (16%)
[Progress] REPORT: bsl-incremental-update-1766311449309 | Indexed 463/8862: /home/egor/code/bsl-gradual-types/examples/conf_big/InformationRegisters/ОснованияПолномочийОтветственныхЛиц/Ext/ManagerModule.bsl (90%)
[Progress] REPORT: bsl-incremental-update-1766311449309 | Indexed 963/8862: /home/egor/code/bsl-gradual-types/examples/conf_big/Catalogs/ДоверенностиНалогоплательщика/Ext/ObjectModule.bsl (91%)
[Progress] REPORT: bsl-incremental-update-1766311449309 | Indexed 76971 methods, 113 global functions (99%)
[Progress] END: bsl-incremental-update-1766311449309 | Loaded 9371 types
[AutoReindex] Completed
```
- Гипотеза: incremental update пересчитывает весь набор модулей вместо диффа по изменённому файлу.
- Связь: `docs/roadmap/disk-cache-platform-config-parsing-roadmap.md` (кеши и инкрементальность).
- Проверить:
  - источник логов `[AutoReindex]` и `[Progress]` (кто инициирует);
  - почему при единичном изменении берётся полный список `**/Ext/*.bsl`;
  - можно ли сузить пересборку до изменённых модулей + зависимых типов;
  - как использовать дисковый кеш/хеши, чтобы пропускать неизменённые файлы.

### Hover в условиях и циклах показывает Dynamic

- Симптом: hover в `Если/Пока/Для` показывает `Условие: Dynamic`.
- Цель: более информативный hover (ожидаемый тип `Булево` + фактический `TypeResolution` с certainty/причиной).
- Проверить:
  - как формируется `condition_type` в IR;
  - достаточно ли заменить форматирование на более полное;
  - как корректно отображать unknown/union для условий.

### Самоприсваивание с конкатенацией теряет тип

- Симптом: `ТекстЗаголовка = ТекстЗаголовка + НСтр("ru = ' по командировке'");` — тип не резолвится, хотя ранее был `Строка`.
- Задача: проверить flow‑sensitive обновление типа при бинарных операциях.
- Ожидаемое поведение: после присваивания тип остаётся `Строка`.

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

- `cargo test -p bsl_shared signature_index`
- `cargo test -p bsl_backend hover_formatter`
- Если Web API поднят пользователем: проверить hover для `НСтр`.

---

## Риски и edge cases

- Курсор на аргументах/объекте внутри `FunctionCall` может попадать в fallback.
- `object_type` может быть `None`/`Unknown`, сигнатура не найдётся.
- Английские алиасы без описания дадут пустой hover.

---

## Открытые вопросы

- Показывать ли описание/`return_description` для методов объектов так же, как для глобальных функций?
- Нужно ли выводить `english_name` рядом с русским именем в hover?
