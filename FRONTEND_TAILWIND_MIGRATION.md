# Frontend Tailwind CSS Migration Roadmap

**Статус:** 🔄 В ПРОЦЕССЕ - Week 1 ЗАВЕРШЕНА! ✅
**Milestone F1:** ✅ ЗАВЕРШЁН (tailwind.config.js, smoke test готовы)
**Milestone F2:** 🔄 В ПРОЦЕССЕ (5/14 компонентов мигрировано)
**Общее время:** 4-6 рабочих дней (27-41 час)
**Дата начала:** 2025-01-18
**Дата завершения Week 1:** 2025-11-05

---

## 📊 Executive Summary

### Обзор миграции

**Цель:** Мигрировать 14 компонентов Frontend (Leptos WASM) с custom CSS на Tailwind CSS utilities.

**Подход:** Incremental migration (по 1 компоненту за раз)

**Компоненты для миграции:**
- ✅ **1 компонент** уже на Tailwind (tailwind_smoke_test.rs)
- ✅ **5 простых** компонентов (Week 1: ЗАВЕРШЕНО - 289 строк CSS удалено)
- 🔄 **6 средних** компонентов (Week 2: 14-20 часов)
- 🔄 **2 сложных** компонента (Week 3: 8-12 часов)

**Ожидаемые результаты:**
- ✅ Bundle size: ~38 KB legacy CSS → ~8 KB Tailwind utilities (80% экономия)
- ✅ Maintainability: стили в компонентах, не в отдельном CSS
- ✅ Dark mode: автоматический через `dark:` префикс
- ✅ Developer Experience: IntelliSense для Tailwind классов

---

## 📋 Прогресс выполнения

### Общий прогресс: 6/14 (43%) 🎉

| Week | Компонентов | Статус | Прогресс |
|------|-------------|--------|----------|
| **Week 0** | 1 (smoke test) | ✅ ЗАВЕРШЕНО | 1/1 (100%) |
| **Week 1** | 5 (простые) | ✅ ЗАВЕРШЕНО | 5/5 (100%) |
| **Week 2** | 6 (средние) | 📋 ПЛАНИРУЕТСЯ | 0/6 (0%) |
| **Week 3** | 2 (сложные) | 📋 ПЛАНИРУЕТСЯ | 0/2 (0%) |

### Детальный checklist

#### ✅ Week 0: Подготовка (ЗАВЕРШЕНО)

- [x] **tailwind_smoke_test.rs** ✅ (Milestone F1)
  - Статус: Полностью на Tailwind
  - Commit: 15b6fea
  - Тестирует: 10 категорий утилит

#### ✅ Week 1: Простые компоненты (ЗАВЕРШЕНО - 2025-11-05)

- [x] **1. navigation.rs** ⭐ (30 минут) ✅
  - Сложность: 1/10
  - CSS удалено: ~95 строк
  - Результат: 9.2/10 (Reviewer)
  - Статус: Полностью мигрирован

- [x] **2. metric_card.rs** ⭐ (45 минут) ✅
  - Сложность: 1/10
  - CSS удалено: 58 строк (строки 423-480)
  - Результат: 8.5/10 (Reviewer)
  - Статус: Полностью мигрирован

- [x] **3. view_switcher.rs** ⭐⭐ (2 часа) ✅
  - Сложность: 3/10
  - CSS удалено: 0 строк (не было)
  - Особенность: 4 варианта компонента (ViewSwitcher, ExtendedViewSwitcher, ViewTabs, ViewDropdown)
  - Результат: 8.2/10 (Reviewer)
  - Статус: Полностью мигрирован

- [x] **4. search_bar.rs** ⭐⭐ (1.5 часа) ✅
  - Сложность: 2/10
  - CSS удалено: 18 строк (строки 369-386)
  - Особенность: 3 варианта компонента (SearchBar, SimpleSearchBar, HeaderSearchBar)
  - Результат: 8.6/10 (Reviewer)
  - Статус: Полностью мигрирован

- [x] **5. pagination.rs** ⭐⭐⭐ (2 часа) ✅
  - Сложность: 3/10
  - CSS удалено: 243 строки (строки 1014-1250) - самое большое удаление!
  - Результат: 9.0/10 (Reviewer)
  - Статус: Полностью мигрирован

**Итого Week 1:**
- ✅ Компонентов: 5/5 (100%)
- ✅ Legacy CSS удалено: 289 строк
- ✅ Compilation: PASSED без ошибок
- ✅ Tester: APPROVED FOR PRODUCTION
- ✅ Reviewer: 8.7/10 - APPROVED FOR PRODUCTION
- ✅ Dark mode: 100% coverage
- ✅ Hotfix minor issues: Завершён

#### 📋 Week 2: Средние компоненты (14-20 часов)

- [ ] **6. sidebar.rs** ⭐⭐⭐⭐ (2.5 часа)
  - Сложность: 4/10
  - CSS для удаления: строки 736-809 (74 строки)
  - Особенность: Состояния open/closed
  - Риски: 🟡 Средние

- [ ] **7. graph_view.rs** ⭐⭐⭐ (2 часа)
  - Сложность: 3/10
  - CSS для удаления: НЕТ (большинство классов отсутствует)
  - Риски: 🟢 Низкие

- [ ] **8. dashboard.rs** ⭐⭐⭐⭐ (3 часа)
  - Сложность: 4/10
  - CSS для удаления: строки 416-518 (~103 строки)
  - Особенность: Множество вариантов metric-card
  - Риски: 🟡 Средние

- [ ] **9. type_table.rs** ⭐⭐⭐⭐ (2.5 часа)
  - Сложность: 4/10
  - CSS для удаления: частично строки 614-682 (~30 строк)
  - Риски: 🟡 Средние

- [ ] **10. cards_view.rs** ⭐⭐⭐⭐⭐ (4 часа)
  - Сложность: 5/10
  - CSS для удаления: строки 549-680 (~132 строки)
  - Особенность: Сетка карточек, empty state
  - Риски: 🟡 Средние

- [ ] **11. table_view.rs** ⭐⭐⭐⭐⭐ (4 часа)
  - Сложность: 5/10
  - CSS для удаления: строки 682-735 (~54 строки)
  - Особенность: Сложная таблица с сортировкой
  - Риски: 🟡 Средние

#### 📋 Week 3: Сложные компоненты (8-12 часов)

- [ ] **12. type_card.rs** ⭐⭐⭐⭐⭐⭐⭐⭐ (5 часов)
  - Сложность: 8/10
  - CSS для удаления: строки 555-658 (~104 строки)
  - Особенность: **КРИТИЧЕСКИЙ** — основной компонент приложения
  - Риски: 🟠 ВЫСОКИЕ (hover эффекты, анимации, фасеты)
  - Требует: Тщательное тестирование обеих тем

- [ ] **13. type_details_modal.rs** ⭐⭐⭐⭐⭐⭐⭐ (5 часов)
  - Сложность: 7/10
  - CSS для удаления: строки 1263-1498 (~236 строк)
  - Особенность: Модальное окно, overlay, анимации
  - Риски: 🟠 ВЫСОКИЕ (UX критично)
  - Требует: Проверка ESC/backdrop click, scroll

---

## 🗓️ Детальный Roadmap по неделям

### Week 1: Простые компоненты (День 1-2)

**Цель:** Отработать процесс миграции на простых компонентах

#### **День 1 (2-3 часа)**

**9:00 - 9:30 | navigation.rs** ⭐
```bash
git checkout -b migrate/navigation
# Миграция компонента
trunk serve # Визуальная проверка
git commit -m "migrate: navigation to Tailwind"
git checkout master && git merge migrate/navigation
```

**Миграция:**
```rust
// БЫЛО:
<nav class="main-nav">
  <div class="nav-brand">
    <h1>"BSL Gradual Type System"</h1>
  </div>
</nav>

// СТАЛО:
<nav class="bg-bsl-cream-100 dark:bg-bsl-charcoal-800 border-b border-bsl-brown-600/20 px-6 py-4 flex items-center justify-between">
  <div class="flex flex-col gap-1">
    <h1 class="text-2xl font-bold text-bsl-slate-900 dark:text-bsl-gray-200">
      "BSL Gradual Type System"
    </h1>
  </div>
</nav>
```

**Удаление CSS:** НЕТ (классы не существуют в main.css)

---

**9:30 - 10:15 | metric_card.rs** ⭐

**Миграция:**
```rust
// БЫЛО:
<div class="metric-card">
  <div class="metric-card__icon">{icon}</div>
  <div class="metric-card__content">
    <div class="metric-card__title">{title}</div>
    <div class="metric-card__value">{value}</div>
  </div>
</div>

// СТАЛО:
<div class="
  bg-bsl-cream-100 dark:bg-bsl-charcoal-800
  border border-bsl-brown-600/12 dark:border-bsl-gray-400/15
  rounded-lg p-6
  flex items-center gap-4
  shadow-sm
  transition-all duration-normal ease-smooth
  hover:-translate-y-0.5 hover:shadow-lg
">
  <div class="text-3xl font-bold text-bsl-teal-500">
    {icon}
  </div>
  <div class="flex flex-col gap-1">
    <div class="text-sm text-bsl-text-secondary dark:text-bsl-gray-300/70">
      {title}
    </div>
    <div class="text-2xl font-bold text-bsl-text dark:text-bsl-gray-200">
      {value}
    </div>
  </div>
</div>
```

**Удаление CSS:** main.css строки 423-481 (59 строк)

**Commit pattern:**
```bash
git commit -m "migrate: metric_card to Tailwind

- Replace .metric-card classes with Tailwind utilities
- Add dark mode support (dark: prefix)
- Remove legacy CSS (lines 423-481, 59 lines)
- Test: hover effects ✓, dark mode ✓"
```

---

**10:15 - 11:45 | view_switcher.rs** ⭐⭐

**Особенность:** 4 варианта компонента
1. ViewSwitcher (базовый переключатель)
2. ExtendedViewSwitcher (расширенный с карточками)
3. ViewTabs (табы)
4. ViewDropdown (выпадающий список)

**Миграция ViewSwitcher:**
```rust
// БЫЛО:
<div class="view-switcher">
  <button class="view-btn active">
    <span class="view-icon">"📊"</span>
    <span class="view-label">"Cards"</span>
  </button>
</div>

// СТАЛО:
<div class="flex gap-2 bg-bsl-brown-600/8 dark:bg-bsl-gray-400/10 rounded-lg p-1">
  <button class="
    flex items-center gap-2 px-4 py-2 rounded
    bg-bsl-cream-100 dark:bg-bsl-charcoal-700
    text-bsl-text dark:text-bsl-gray-200
    font-medium text-sm
    shadow-sm
    transition-colors duration-fast
  ">
    <span class="text-lg">"📊"</span>
    <span>"Cards"</span>
  </button>
</div>
```

**Удаление CSS:** НЕТ (классы не существуют)

**Commit:** После реализации всех 4 вариантов

---

#### **День 2 (3-4 часа)**

**9:00 - 10:30 | search_bar.rs** ⭐⭐

**Особенность:** 3 варианта компонента
1. SimpleSearchBar (простой поиск)
2. AdvancedSearchBar (с фильтрами)
3. CompactSearchBar (компактный)

**Миграция form-control:**
```css
/* main.css (строки 369-386) */
.form-control {
  width: 100%;
  padding: var(--space-8) var(--space-12);
  font-size: var(--font-size-md);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface);
  color: var(--color-text);
  transition: all var(--duration-fast) var(--ease-standard);
}
```

**Tailwind эквивалент:**
```rust
class="
  block w-full px-3 py-2 text-md
  text-bsl-text dark:text-bsl-gray-200
  bg-bsl-surface dark:bg-bsl-charcoal-700
  border border-bsl-border dark:border-bsl-gray-400/30
  rounded
  transition-colors duration-fast
  focus:border-bsl-teal-500 focus:outline-2 focus:outline focus:outline-bsl-teal-500
"
```

**Удаление CSS:** main.css строки 369-386, 779-809 (~48 строк)

---

**10:30 - 12:30 | pagination.rs** ⭐⭐⭐

**Много классов, но простая логика:**
- `.pagination` → flex layout
- `.pagination__btn` → button styles
- `.pagination__btn--current` → active state
- `.pagination__select` → dropdown

**Пример миграции:**
```rust
// БЫЛО:
<div class="pagination">
  <div class="pagination__info">
    <span class="pagination__text">"Showing {start}-{end} of {total}"</span>
  </div>
  <div class="pagination__controls">
    <button class="pagination__btn pagination__btn--first">
      "«"
    </button>
    <button class="pagination__btn pagination__btn--current">
      "1"
    </button>
  </div>
</div>

// СТАЛО:
<div class="flex items-center justify-between gap-4 px-4 py-3 bg-bsl-cream-100 dark:bg-bsl-charcoal-800 border-t border-bsl-border dark:border-bsl-gray-400/30">
  <div class="text-sm text-bsl-text-secondary dark:text-bsl-gray-300/70">
    "Showing "{start}"-"{end}" of "{total}
  </div>
  <div class="flex items-center gap-2">
    <button class="
      px-3 py-1.5 text-sm
      bg-bsl-brown-600/12 dark:bg-bsl-gray-400/15
      text-bsl-text dark:text-bsl-gray-200
      rounded
      hover:bg-bsl-teal-500/20
      transition-colors duration-fast
    ">
      "«"
    </button>
    <button class="
      px-3 py-1.5 text-sm
      bg-bsl-teal-500 text-white
      rounded shadow-sm
    ">
      "1"
    </button>
  </div>
</div>
```

**Удаление CSS:** main.css строки 1014-1250 (237 строк)

**Checklist Week 1:**
- [ ] Все 5 компонентов мигрированы
- [ ] Удалено ~344 строки legacy CSS
- [ ] Визуальная проверка в обеих темах
- [ ] 5 commits в master

---

### Week 2: Средние компоненты (День 3-5)

#### **День 3 (4-5 часов)**

**6. sidebar.rs** ⭐⭐⭐⭐ (2.5 часа)

**Особенность:** Состояния open/closed

**Миграция состояний:**
```rust
// БЫЛО:
<div class={if is_open() { "sidebar open" } else { "sidebar closed" }}>

// СТАЛО:
<div class={move || if is_open() {
  "w-64 transition-width duration-normal"
} else {
  "w-16 transition-width duration-normal"
}}>
```

**Удаление CSS:** main.css строки 736-809 (74 строки)

---

**7. graph_view.rs** ⭐⭐⭐ (2 часа)

**Большинство классов отсутствует** → простая миграция

**Удаление CSS:** НЕТ (классы не существуют)

---

#### **День 4 (5-7 часов)**

**8. dashboard.rs** ⭐⭐⭐⭐ (3 часа)

**Множество вариантов metric-card:**
- metric-card--success
- metric-card--warning
- metric-card--danger
- metric-card--info
- metric-card--performance

**Стратегия:** Использовать условные классы вместо modifier классов

```rust
// БЫЛО:
<div class={format!("metric-card metric-card--{}", variant)}>

// СТАЛО:
<div class={move || match variant() {
  Variant::Success => "border-l-4 border-green-500 ...",
  Variant::Warning => "border-l-4 border-yellow-500 ...",
  Variant::Danger => "border-l-4 border-red-500 ...",
  // ...
}}>
```

**Удаление CSS:** main.css строки 416-518 (~103 строки)

---

**9. type_table.rs** ⭐⭐⭐⭐ (2.5 часа)

**Специализированная таблица для типов**

**Удаление CSS:** частично строки 614-682 (~30 строк)

---

#### **День 5 (5-8 часов)**

**10. cards_view.rs** ⭐⭐⭐⭐⭐ (4 часа)

**Сетка карточек + empty state**

**Миграция grid:**
```rust
// БЫЛО:
<div class="cards-grid">

// СТАЛО:
<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 p-4">
```

**Удаление CSS:** main.css строки 549-680 (~132 строки)

---

**11. table_view.rs** ⭐⭐⭐⭐⭐ (4 часа)

**Сложная таблица с сортировкой**

**Удаление CSS:** main.css строки 682-735 (~54 строки)

**Checklist Week 2:**
- [ ] Все 6 компонентов мигрированы
- [ ] Удалено ~393 строки legacy CSS
- [ ] Сортировка работает корректно
- [ ] Grid responsive на разных экранах
- [ ] 6 commits в master

---

### Week 3: Сложные компоненты (День 6)

#### **День 6 (8-12 часов)**

**12. type_card.rs** ⭐⭐⭐⭐⭐⭐⭐⭐ (5 часов)

**⚠️ КРИТИЧЕСКИЙ КОМПОНЕНТ — основной UI элемент**

**Особенности:**
- Hover эффекты (border-color, shadow transitions)
- Facet badges с градиентами (уже в tailwind.css)
- Динамические классы (certainty levels)
- Preview методов/свойств (truncated)

**Стратегия миграции:**
1. Начать с базовой структуры (без hover)
2. Добавить hover эффекты
3. Интегрировать facet badges
4. Тщательно протестировать обе темы

**Риски:**
- 🟠 ВЫСОКИЙ: Основной компонент, видимый всем пользователям
- 🟠 ВЫСОКИЙ: Сложные анимации и transitions

**Тестирование (обязательно):**
- [ ] Hover эффекты работают
- [ ] Facet badges корректно отображаются
- [ ] Dark mode без визуальных багов
- [ ] Certainty badges (Known/Inferred/Unknown)
- [ ] Truncate длинных названий типов

**Удаление CSS:** main.css строки 555-658 (~104 строки)

---

**13. type_details_modal.rs** ⭐⭐⭐⭐⭐⭐⭐ (5 часов)

**Модальное окно — критично для UX**

**Особенности:**
- Overlay с backdrop-blur
- Анимации открытия/закрытия (fade-in, slide-in)
- Scroll для длинного контента
- ESC/backdrop click для закрытия

**Стратегия миграции:**
1. Базовая структура overlay + content
2. Анимации появления/исчезновения
3. Интеграция секций (facets, methods, properties)
4. UX тестирование (ESC, backdrop, scroll)

**Риски:**
- 🟠 ВЫСОКИЙ: UX критично, пользователи часто используют модалку
- 🟡 СРЕДНИЙ: Анимации могут глючить

**Тестирование (обязательно):**
- [ ] Modal открывается/закрывается
- [ ] Overlay backdrop работает (click to close)
- [ ] ESC key закрывает modal
- [ ] Scroll работает для длинного контента
- [ ] Анимации плавные (fade-in, slide-in)
- [ ] Dark mode корректен

**Удаление CSS:** main.css строки 1263-1498 (~236 строк)

**Checklist Week 3:**
- [ ] Оба сложных компонента мигрированы
- [ ] Удалено ~340 строк legacy CSS
- [ ] type_card.rs полностью протестирован
- [ ] type_details_modal UX проверен
- [ ] 2 commits в master

---

## 🎯 Стратегия миграции

### 1. Incremental подход (по 1 компоненту)

```bash
# Workflow для каждого компонента:

# 1. Создать feature branch
git checkout -b migrate/component-name

# 2. Мигрировать компонент на Tailwind
#    - Заменить классы на Tailwind utilities
#    - Добавить dark: префиксы для dark mode
#    - Удалить legacy CSS из main.css

# 3. Локальное тестирование
trunk serve
# Открыть http://localhost:8080
# Проверить:
# - Light mode внешний вид
# - Dark mode внешний вид (переключить в OS settings)
# - Hover эффекты (если есть)
# - Responsive (если есть)

# 4. Commit изменений
git add .
git commit -m "migrate: ComponentName to Tailwind

- Replace .custom-class with Tailwind utilities
- Add dark mode support via dark: prefix
- Remove legacy CSS (main.css lines: XXX-YYY, ZZ lines)
- Test: visual regression ✓, dark mode ✓"

# 5. Merge в master
git checkout master
git merge migrate/component-name

# 6. Push
git push origin master
```

---

### 2. CSS Coexistence Strategy

**Порядок загрузки в index.html:**
```html
<!-- 1. WASM binary -->
<link data-trunk rel="rust" data-wasm-opt="z" />

<!-- 2. Tailwind CSS FIRST (базовые утилиты) -->
<link data-trunk rel="tailwind-css" href="style/tailwind.css" />

<!-- 3. Legacy CSS SECOND (перекрывает для не-мигрированных компонентов) -->
<link data-trunk rel="css" href="style/main.css" />
```

**Почему этот порядок?**
- Tailwind загружается ПЕРВЫМ → устанавливает базовые утилиты
- Legacy CSS ВТОРЫМ → перекрывает Tailwind ТОЛЬКО для не-мигрированных компонентов
- После миграции всех компонентов → удаляем legacy CSS полностью

---

### 3. Постепенное удаление Legacy CSS

**Трекинг удаляемых строк:**

После каждого компонента → удалять соответствующие строки из `frontend/style/main.css`

| Компонент | Удаляемые строки | Количество |
|-----------|------------------|------------|
| navigation.rs | НЕТ | 0 |
| metric_card.rs | 423-481 | 59 |
| view_switcher.rs | НЕТ | 0 |
| search_bar.rs | 369-386, 779-809 | 48 |
| pagination.rs | 1014-1250 | 237 |
| sidebar.rs | 736-809 | 74 |
| graph_view.rs | НЕТ | 0 |
| dashboard.rs | 416-518 | 103 |
| type_table.rs | 614-682 (частично) | ~30 |
| cards_view.rs | 549-680 | 132 |
| table_view.rs | 682-735 | 54 |
| type_card.rs | 555-658 | 104 |
| type_details_modal.rs | 1263-1498 | 236 |
| **ИТОГО** | | **~1077 строк** |

**Финальная чистка (после Week 3):**
- Проверить, что main.css пуст или содержит < 50 строк
- Удалить `<link rel="css" href="style/main.css" />` из index.html
- Полностью удалить `frontend/style/main.css`

---

### 4. Testing Checkpoints

**После каждого компонента:**

```bash
# 1. Визуальная проверка в браузере
trunk serve

# 2. Checklist:
- [ ] Light mode: внешний вид идентичен оригиналу
- [ ] Dark mode: внешний вид корректен
- [ ] Hover эффекты: работают (если есть)
- [ ] Transitions: плавные (если есть)
- [ ] Responsive: корректен на разных экранах (если есть)
- [ ] НЕТ console errors
- [ ] НЕТ визуальных багов

# 3. Cargo check
cargo check -p bsl-frontend

# 4. Если всё OK → commit
git commit -m "migrate: ComponentName to Tailwind ✅"
```

---

## ⚠️ Риски и митигация

### Категории рисков

| Риск | Вероятность | Влияние | Митигация |
|------|-------------|---------|-----------|
| **CSS конфликты (Tailwind vs Legacy)** | 🟡 Средняя | 🟠 Высокое | Порядок загрузки: Tailwind FIRST → Legacy SECOND |
| **Забыли удалить legacy CSS** | 🟢 Низкая | 🟡 Среднее | Checklist удаляемых строк после каждого компонента |
| **Dark mode регрессии** | 🟡 Средняя | 🟠 Высокое | Тестировать обе темы после КАЖДОЙ миграции |
| **Сложные анимации НЕ реализуются** | 🟢 Низкая | 🟡 Среднее | Оставить в tailwind.css как @layer components |
| **Визуальные различия (pixel-perfect)** | 🟡 Средняя | 🟡 Среднее | Скриншоты "до" и "после" для сравнения |
| **Регрессии в type_card.rs** | 🟠 Высокая | 🔴 КРИТИЧЕСКОЕ | Тщательное тестирование: hover, facets, certainty |
| **Modal UX проблемы** | 🟡 Средняя | 🟠 Высокое | Проверить ESC, backdrop, scroll, анимации |

---

### Специфичные риски для сложных компонентов

#### type_card.rs (Week 3)

**Риски:**
- 🟠 **Hover эффекты сломаются** — border-color, shadow transitions
- 🟠 **Facet badges визуально отличаются** — градиенты из tailwind.css могут отличаться
- 🟡 **Certainty badges (Known/Inferred/Unknown)** — условные классы

**Митигация:**
1. Начать с базовой структуры БЕЗ hover
2. Постепенно добавлять hover эффекты
3. Проверить facet badges на соответствие дизайну
4. Тщательно протестировать все 3 certainty levels

---

#### type_details_modal.rs (Week 3)

**Риски:**
- 🟠 **Анимации открытия/закрытия глючат** — fade-in, slide-in
- 🟠 **ESC key не работает** — JavaScript event listener
- 🟡 **Scroll не работает для длинного контента** — overflow issues

**Митигация:**
1. Использовать Tailwind transitions вместо custom keyframes
2. Проверить Leptos event handlers (on:keydown)
3. Тестировать scroll на реальных данных (длинные типы)

---

## 📈 Метрики успеха

### Критерии успеха миграции

✅ **Миграция считается успешной, если:**

1. **Визуальная идентичность:**
   - Нет заметных изменений для пользователя
   - Pixel-perfect соответствие оригиналу (допустимо ±2px)

2. **Performance:**
   - Bundle size уменьшился: 38 KB legacy CSS → ~8 KB Tailwind (80% экономия)
   - Load time НЕ увеличился (допустимо +5%)

3. **Maintainability:**
   - Стили в компонентах (Tailwind классы), не в отдельном CSS
   - 0 дублирования CSS (DRY принцип)
   - IntelliSense работает для Tailwind классов

4. **Dark mode:**
   - Все компоненты корректно работают в тёмной теме
   - Используют `dark:` префикс вместо media queries

5. **Developer Experience:**
   - Легко добавлять новые компоненты (Tailwind utilities)
   - Легко изменять существующие компоненты
   - 0 магических CSS классов

---

### Метрики для отслеживания

**Bundle Size:**
```
BEFORE (main.css):  1667 строк (38 KB минифицированный)
AFTER (Tailwind):   ~8 KB (с PurgeCSS)
ЭКОНОМИЯ:           ~30 KB (79% reduction)
```

**Lines of Code:**
```
BEFORE: 1667 строк CSS в отдельном файле
AFTER:  ~50 строк custom CSS (только facet gradients в tailwind.css)
УДАЛЕНО: ~1617 строк (97% reduction)
```

**Компоненты:**
```
МИГРИРОВАНО: 14/14 компонентов (100%)
LEGACY CSS:  0 строк
TAILWIND:    100% компонентов
```

---

## 📚 Документация и примеры

### Пример миграции: metric_card.rs (полный)

#### БЫЛО (Legacy CSS):

**main.css (строки 423-481):**
```css
.metric-card {
  background: var(--color-surface);
  border: 1px solid var(--color-card-border);
  border-radius: var(--radius-lg);
  padding: var(--space-24);
  display: flex;
  align-items: center;
  gap: var(--space-16);
  box-shadow: var(--shadow-sm);
  transition: all var(--duration-normal) var(--ease-standard);
}

.metric-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
}

.metric-card__icon {
  font-size: var(--font-size-3xl);
  line-height: 1;
  flex-shrink: 0;
}

.metric-card__content {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.metric-card__title {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
  font-weight: var(--font-weight-medium);
  text-transform: uppercase;
  letter-spacing: var(--letter-spacing-tight);
}

.metric-card__value {
  font-size: var(--font-size-3xl);
  font-weight: var(--font-weight-bold);
  color: var(--color-text);
  line-height: var(--line-height-tight);
}
```

**metric_card.rs:**
```rust
view! {
    <div class="metric-card">
        <div
            class="metric-card__icon"
            style=move || format!("color: {};", color.get())
        >
            {icon}
        </div>
        <div class="metric-card__content">
            <div class="metric-card__title">{title}</div>
            <div class="metric-card__value">{value}</div>
        </div>
    </div>
}
```

---

#### СТАЛО (Tailwind):

**metric_card.rs:**
```rust
view! {
    <div class="
        bg-bsl-cream-100 dark:bg-bsl-charcoal-800
        border border-bsl-brown-600/12 dark:border-bsl-gray-400/15
        rounded-lg p-6
        flex items-center gap-4
        shadow-sm
        transition-all duration-normal ease-smooth
        hover:-translate-y-0.5 hover:shadow-lg
    ">
        <div
            class="text-3xl leading-none flex-shrink-0"
            style=move || format!("color: {};", color.get())
        >
            {icon}
        </div>
        <div class="flex flex-col gap-1">
            <div class="text-sm text-bsl-text-secondary dark:text-bsl-gray-300/70 font-medium uppercase tracking-tight">
                {title}
            </div>
            <div class="text-3xl font-bold text-bsl-text dark:text-bsl-gray-200 leading-tight">
                {value}
            </div>
        </div>
    </div>
}
```

**Удалить из main.css:** строки 423-481 (59 строк)

**Commit:**
```bash
git commit -m "migrate: metric_card to Tailwind

- Replace .metric-card CSS classes with Tailwind utilities
- Add dark mode support (dark: prefix for all colors)
- Preserve hover effects (-translate-y-0.5, shadow-lg)
- Remove legacy CSS (main.css lines 423-481, 59 lines)

Testing:
✓ Light mode: визуально идентично
✓ Dark mode: корректные цвета
✓ Hover: transform + shadow работают
✓ НЕТ visual regressions"
```

---

## 🎯 Рекомендации

### 1. Начать с navigation.rs
- Самый простой компонент (30 минут)
- Все классы отсутствуют → нет конфликтов
- Хорошая разминка для процесса миграции

### 2. Коммитить после каждого компонента
- 1 компонент = 1 commit
- Детальное commit message (что изменено, что удалено, что протестировано)
- Легко откатиться, если что-то пошло не так

### 3. Тестировать обе темы ВСЕГДА
- Light mode: основная тема
- Dark mode: критично для UX
- Переключение: `prefers-color-scheme` в OS settings

### 4. Использовать скриншоты для сравнения
```bash
# До миграции
trunk serve
# Сделать скриншот компонента

# После миграции
trunk serve
# Сделать скриншот компонента

# Сравнить визуально
```

### 5. Финальная чистка после Week 3
- Удалить все legacy CSS из main.css
- Удалить `<link rel="css" href="style/main.css" />` из index.html
- Возможно, полностью удалить файл main.css
- Обновить README с Tailwind conventions

---

## 📞 Support и вопросы

**Если возникли проблемы:**

1. **CSS конфликты:** Проверить порядок загрузки в index.html (Tailwind → Legacy)
2. **Dark mode не работает:** Проверить, что используется `dark:` prefix для всех цветов
3. **Hover эффекты не работают:** Проверить `transition-*` классы и `hover:` prefix
4. **Visual regressions:** Сравнить скриншоты "до" и "после"
5. **Compilation errors:** Запустить `cargo check -p bsl-frontend`

---

## 📅 Следующие шаги

**После завершения миграции:**

1. ✅ Обновить TAILWIND_INTEGRATION_ROADMAP.md — отметить Milestone F2 как завершённый
2. ✅ Создать PR с детальным описанием всех изменений
3. ✅ Провести финальный code review (Reviewer)
4. ✅ Обновить документацию (README, Tailwind conventions)
5. 🎉 Праздновать успешную миграцию!

---

**Дата создания roadmap:** 2025-01-18
**Автор:** Architect Agent
**Статус:** 📋 READY TO START
