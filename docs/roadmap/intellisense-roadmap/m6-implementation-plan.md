# План реализации M6: Импорты/Using и auto-insert

**Статус:** ⚪️ НЕ ТРЕБУЕТСЯ  
**Цель:** автоматическая вставка импортов/using при выборе completion item.

**Примечание:** в BSL нет импортов/using в привычном смысле; этап помечен как не требующий реализации.

---

## Область работ

- Определение отсутствующих импортов/using для типов/модулей
- Формирование `additionalTextEdits` при completion/resolve
- Конфиг/флаг авто‑импорта (on/off, режимы)
- Устойчивость к форматированию и структуре модулей

---

## Пошаговый план

### Шаг 1: Контракт auto‑import и capabilities 🟡
- Зафиксировать формат `additionalTextEdits` и правила вставки.
- Учесть `completionItem.resolveSupport.additionalTextEdits` в client capabilities.
- Определить точку вставки (верх модуля, блок директив).

**Выход:** контракт auto‑import + включение по capabilities.

---

### Шаг 2: Определение missing import 🟡
- Определить карту: **тип/модуль → required import/using**.
- Проверка “уже импортировано” (по тексту/AST).
- Различать варианты: platform types / config types / modules.

**Выход:** resolver “нужен ли импорт” + набор правил.

---

### Шаг 3: Генерация TextEdit 🟡
- Построить `TextEdit` для вставки import/using.
- Учитывать форматирование (пустые строки, сортировка, блок директив).
- Объединять несколько импортов в один edit при необходимости.

**Выход:** корректные `additionalTextEdits` без побочных эффектов.

---

### Шаг 4: Интеграция в completion/resolve 🟡
- В базовом completion отдавать минимальный payload.
- В resolve добавлять `additionalTextEdits` и (при необходимости) `command`.
- Не выполнять I/O в hot path.

**Выход:** auto‑insert работает через resolve и не влияет на latency.

---

### Шаг 5: Тесты и регрессии ⏳
- Unit: распознавание существующих импортов и точек вставки.
- Integration: completion + resolve → корректные edits.
- Regression: разные структуры модулей (пустой файл, директивы, комментарии).

**Выход:** набор тестов M6.

---

## Критерии завершения

- Авто‑import/using корректно вставляется в типовой структуре модулей.
- `additionalTextEdits` формируются только при необходимости.
- Есть конфиг включения/выключения auto‑insert.
- Нет побочных эффектов при отмене completion.

---

## Фактический статус (по коду)

- Реализация auto‑import не обнаружена.
- `completionItem/resolve` уже используется для detail/documentation/snippets.
- Метка для `additionalTextEdits` и capabilities не реализована.

---

## Чек-лист задач для завершения M6

- Добавить учет `resolveSupport.additionalTextEdits` в LSP capabilities.
- Реализовать `ImportResolver` (missing import + точка вставки).
- Формировать `additionalTextEdits` через resolve.
- Добавить конфиг auto‑import (on/off).
- Добавить unit/integration тесты.

---

## Задачи (тикеты) по M6

### T1: Контракт и capabilities ⏳
**Цель:** определить правила включения auto‑import.  
**Где:** LSP init/config и completion resolve.  
**DoD:**
- учитывается `resolveSupport.additionalTextEdits`;
- задокументирован формат вставки;
- fallback при отсутствии поддержки клиента.

### T2: Import resolver ⏳
**Цель:** определить, нужен ли импорт и где его вставлять.  
**Где:** application/service слой (новый модуль).  
**DoD:**
- определены правила для platform/config/modules;
- есть проверка “уже импортировано”;
- unit‑тесты на базовые кейсы.

### T3: Генерация TextEdit ⏳
**Цель:** корректно сформировать вставку.  
**Где:** new helper (edit builder).  
**DoD:**
- корректные позиции вставки;
- учитывается форматирование/пустые строки;
- регрессионные тесты.

### T4: Интеграция в resolve ⏳
**Цель:** отдавать `additionalTextEdits` при completion resolve.  
**Где:** `backend/src/bin/lsp_server/handlers/completion.rs` + service layer.  
**DoD:**
- edits возвращаются только при необходимости;
- нет I/O в hot path completion;
- конфиг auto‑insert учитывается.

### T5: Тесты M6 ⏳
**Цель:** закрепить поведение auto‑import.  
**Где:** `backend/tests/...`.  
**DoD:**
- интеграционные tests на resolve + edits;
- golden snapshots для сложных структур;
- покрытие отмены/повторного completion.
