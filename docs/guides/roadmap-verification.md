# Roadmap Verification Guide

Правила проверки выполнения задач из ROADMAP_2025.md с честной оценкой прогресса.

## 🚨 КРИТИЧЕСКОЕ ТРЕБОВАНИЕ: Контрольная проверка выполнения

**ПРИ ВЫПОЛНЕНИИ ЛЮБЫХ ЭТАПОВ ИЗ ROADMAP:**

### 1. Обязательная проверка ПЕРЕД отчётом

**ПЕРЕД** заявлением о выполнении задачи — ОБЯЗАТЕЛЬНО провести **контрольную проверку фактического выполнения**.

### 2. Что должна включать проверка

- ✅ **Чтение реального кода** в файлах (Read tool)
- ✅ **Поиск реализованных функций/структур** (Grep/Glob tools)
- ✅ **Запуск тестов** для проверки работоспособности (Bash: `cargo test`)
- ✅ **Проверка компиляции** (Bash: `cargo check/build`)

### 3. Что ЗАПРЕЩЕНО

- ❌ **Утверждать о выполнении без проверки кода**
- ❌ **Отчитываться о готовности всего этапа**, если выполнена только часть задачи
- ❌ **Предполагать наличие кода** без чтения файлов
- ❌ **Заявлять о прохождении тестов** без их реального запуска

---

## 📋 Формат отчёта о выполнении

### Шаблон отчёта

```markdown
## Статус выполнения [Milestone X.Y: Название]

### ✅ Task N: [Название] — ВЫПОЛНЕНО (100%)

**Проверка:**
- ✅ [Файл:строка] — реализация найдена
- ✅ cargo test [test_name] — тесты проходят (X/X passed)
- ✅ cargo check — компиляция успешна

**Код:**
\`\`\`rust
// Доказательство реализации
pub struct ImplementedFeature {
    // ...
}
\`\`\`

---

### ❌ Task M: [Название] — НЕ НАЧАТО (0%)

**Проверка:**
- ❌ grep показывает отсутствие кода
- ❌ файл не существует

**Необходимо:**
1. Создать модуль
2. Реализовать структуры
3. Написать тесты

---

### ⚠️ Task K: [Название] — ЧАСТИЧНО (X%)

**Что есть:**
- ✅ [конкретные файлы и строки]
- ✅ cargo test [partial_test] — частичные тесты проходят (Y/Z passed)

**Что отсутствует:**
- ❌ [конкретные недостающие компоненты]
- ❌ [недостающие тесты]

**Необходимо:**
1. Дореализовать компоненты
2. Добавить недостающие тесты
3. Интегрировать с существующими модулями

---

## 📊 Общий прогресс Milestone X.Y: [процент]% ([формула расчёта])

**Заблокировано:** [если есть блокирующие задачи]
**Следующий шаг:** [конкретное действие]
```

---

## ✅ Примеры ПРАВИЛЬНОЙ проверки

### Пример 1: Проверка Task "Подключить tree-sitter-bsl"

#### Шаг 1: Проверка зависимости в Cargo.toml

```bash
# Поиск зависимости
grep -n "tree-sitter-bsl" Cargo.toml backend/Cargo.toml
```

**Ожидаемый результат:**
```
backend/Cargo.toml:45:tree-sitter-bsl = "0.1.0"
```

**Оценка:** ✅ Зависимость добавлена

#### Шаг 2: Проверка использования в коде

```bash
# Поиск импортов и использования
grep -rn "tree-sitter-bsl\|TreeSitterAdapter" backend/src/
```

**Ожидаемый результат:**
```
backend/src/system/tree_sitter_adapter.rs:1:use tree_sitter_bsl::language;
backend/src/system/tree_sitter_adapter.rs:15:pub struct TreeSitterAdapter {
```

**Оценка:** ✅ Используется в коде

#### Шаг 3: Чтение реализации

```bash
# Чтение файла адаптера
Read backend/src/system/tree_sitter_adapter.rs
```

**Проверка:**
- ✅ Структура `TreeSitterAdapter` определена
- ✅ Метод `parse()` реализован
- ✅ Интеграция с tree-sitter-bsl

**Оценка:** ✅ Реализация найдена

#### Шаг 4: Запуск тестов

```bash
# Запуск тестов tree-sitter
cargo test -p bsl-backend tree_sitter
```

**Ожидаемый результат:**
```
running 5 tests
test tree_sitter_adapter::tests::test_parse_simple ... ok
test tree_sitter_adapter::tests::test_parse_function ... ok
test tree_sitter_adapter::tests::test_parse_error ... ok
test tree_sitter_adapter::tests::test_span_extraction ... ok
test tree_sitter_adapter::tests::test_integration ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured
```

**Оценка:** ✅ Все тесты проходят

#### Итоговая оценка

```markdown
### ✅ Task 1: Подключить tree-sitter-bsl — ВЫПОЛНЕНО (100%)

**Проверка:**
- ✅ backend/Cargo.toml:45 — зависимость tree-sitter-bsl добавлена
- ✅ backend/src/system/tree_sitter_adapter.rs:1-120 — реализация TreeSitterAdapter
- ✅ cargo test tree_sitter — 5/5 тестов проходят
- ✅ cargo check — компиляция успешна

**Код:**
\`\`\`rust
// backend/src/system/tree_sitter_adapter.rs:15
pub struct TreeSitterAdapter {
    parser: Parser,
    language: Language,
}
\`\`\`
```

---

### Пример 2: Проверка Task "TreeSitterAdapter"

#### Шаг 1: Поиск файлов

```bash
# Поиск файлов адаптера
find backend/src -name "*adapter*" | grep tree
```

**Ожидаемый результат:**
```
backend/src/system/tree_sitter_adapter.rs
```

**Оценка:** ✅ Файл существует

#### Шаг 2: Чтение реализации

```bash
Read backend/src/system/tree_sitter_adapter.rs
```

**Проверка структуры:**
- ✅ `TreeSitterAdapter` структура определена
- ✅ `parse()` метод реализован
- ✅ `node_to_span()` метод для Span extraction
- ⚠️ `convert_to_ir()` метод — заглушка (TODO)

**Оценка:** ⚠️ Частично реализовано

#### Шаг 3: Проверка интеграции

```bash
# Поиск использования TreeSitterAdapter
grep -rn "TreeSitterAdapter" backend/src/
```

**Ожидаемый результат:**
```
backend/src/system/tree_sitter_adapter.rs:15:pub struct TreeSitterAdapter
backend/src/system/parser_coordinator.rs:45:use crate::system::TreeSitterAdapter;
backend/src/system/parser_coordinator.rs:120:let adapter = TreeSitterAdapter::new();
```

**Оценка:** ✅ Интегрирован в ParserCoordinator

#### Шаг 4: Запуск тестов

```bash
cargo test -p bsl-backend --test tree_sitter_adapter_test
```

**Результат:**
```
running 3 tests
test test_parse_simple ... ok
test test_span_extraction ... ok
test test_convert_to_ir ... FAILED

failures:
    test_convert_to_ir

test result: FAILED. 2 passed; 1 failed
```

**Оценка:** ⚠️ Часть тестов проваливается

#### Итоговая оценка

```markdown
### ⚠️ Task 2: TreeSitterAdapter — ЧАСТИЧНО (70%)

**Что есть:**
- ✅ backend/src/system/tree_sitter_adapter.rs:15 — структура TreeSitterAdapter
- ✅ backend/src/system/tree_sitter_adapter.rs:45 — метод parse()
- ✅ backend/src/system/tree_sitter_adapter.rs:120 — метод node_to_span()
- ✅ cargo test tree_sitter — 2/3 тестов проходят

**Что отсутствует:**
- ❌ backend/src/system/tree_sitter_adapter.rs:180 — convert_to_ir() не реализован (TODO)
- ❌ test_convert_to_ir проваливается
- ❌ Нет интеграции с AstToIrConverter

**Необходимо:**
1. Реализовать convert_to_ir() метод
2. Исправить провалившийся тест
3. Добавить интеграцию с AstToIrConverter
```

---

### Пример 3: Проверка Task "Flow-sensitive analysis"

#### Шаг 1: Поиск реализации

```bash
# Поиск ключевых структур
grep -rn "FlowSensitive\|CFG\|ControlFlowGraph" backend/src/
```

**Результат:**
```
[пусто]
```

**Оценка:** ❌ Код отсутствует

#### Шаг 2: Проверка файлов

```bash
# Поиск файлов flow analysis
find backend/src -name "*flow*" -o -name "*cfg*"
```

**Результат:**
```
[пусто]
```

**Оценка:** ❌ Файлы не существуют

#### Шаг 3: Проверка тестов

```bash
cargo test -p bsl-backend flow_sensitive
```

**Результат:**
```
no tests to run
```

**Оценка:** ❌ Тесты отсутствуют

#### Итоговая оценка

```markdown
### ❌ Task 4: Flow-sensitive analysis — НЕ НАЧАТО (0%)

**Проверка:**
- ❌ grep показывает отсутствие кода (FlowSensitive, CFG)
- ❌ файлы backend/src/domain/flow_analysis.rs не существуют
- ❌ тесты отсутствуют (no tests to run)

**Необходимо:**
1. Создать модуль backend/src/domain/flow_analysis.rs
2. Реализовать структуры:
   - ControlFlowGraph
   - FlowSensitiveAnalyzer
   - TypeNarrowing
3. Написать unit тесты (минимум 5)
4. Интегрировать с TypeResolver
```

---

## 📊 Расчёт прогресса Milestone

### Формула расчёта

```
Прогресс = (Σ (Task_i * Вес_i)) / Σ Вес_i * 100%

где:
- Task_i ∈ {0.0, 0.1, ..., 1.0} — процент выполнения задачи i
- Вес_i — вес задачи i (по умолчанию 1.0 для всех)
```

### Примеры расчёта

#### Пример 1: Простой случай (все задачи равнозначны)

```
Milestone 2.19:
- Task 1: 100% (✅)
- Task 2: 70% (⚠️)
- Task 3: 0% (❌)

Прогресс = (1.0 + 0.7 + 0.0) / 3 * 100% = 56.7% ≈ 57%
```

#### Пример 2: Взвешенные задачи

```
Milestone 2.20 (веса указаны явно):
- Task 1 (вес 2): 100% (✅)
- Task 2 (вес 1): 50% (⚠️)
- Task 3 (вес 1): 0% (❌)

Прогресс = (2*1.0 + 1*0.5 + 1*0.0) / (2+1+1) * 100% = 2.5/4 * 100% = 62.5%
```

### Округление

- ✅ **Округление вниз:** 56.7% → 56%
- ❌ **НЕ округлять вверх:** 56.7% → 57% (нечестно!)

**Исключение:** Если прогресс 95%+ и все критические задачи выполнены — можно округлить до 100%.

---

## 🎯 Честность в отчётности

### Золотое правило

> **Если сомневаешься в проценте выполнения — округли ВНИЗ, а не ВВЕРХ.**

### Примеры честной оценки

| Ситуация | ❌ Нечестно | ✅ Честно |
|----------|-------------|-----------|
| Структура создана, методы — заглушки | 80% | 20% |
| Код есть, но тесты не проходят | 90% | 50% |
| Реализовано 1 из 3 подзадач | 100% | 33% |
| Прототип работает, но баги | 95% | 60% |
| Документация написана, код нет | 50% | 10% |

### Критерии "выполнено" (100%)

Задача считается выполненной на 100% ТОЛЬКО если:

1. ✅ **Код реализован полностью** (без TODO/FIXME)
2. ✅ **Все тесты проходят** (unit + integration)
3. ✅ **Компиляция успешна** (без warnings, если требуется)
4. ✅ **Интегрировано с существующими компонентами**
5. ✅ **Документация обновлена** (если требуется)

---

## 🔍 Контрольный чек-лист

Перед отчётом о выполнении Milestone пройдись по этому чек-листу:

### ✅ Для каждой задачи

- [ ] Прочитал реальный код через Read
- [ ] Нашёл все ключевые компоненты через Grep
- [ ] Запустил тесты через `cargo test`
- [ ] Проверил компиляцию через `cargo check`
- [ ] Указал точные координаты кода (файл:строка)
- [ ] Честно оценил процент выполнения

### ✅ Для Milestone в целом

- [ ] Все задачи проверены
- [ ] Прогресс рассчитан по формуле
- [ ] Указаны заблокированные задачи (если есть)
- [ ] Предложен следующий шаг
- [ ] Отчёт содержит доказательства (код, выводы тестов)

---

## 🚀 Автоматизация проверки

Для автоматизации используй Claude Skill:

```bash
/roadmap-checker
```

**Что делает:**
1. Читает ROADMAP_2025.md
2. Для каждой задачи:
   - grep поиск кода
   - cargo test запуск
   - Read проверка файлов
3. Генерирует честный отчёт: ✅/⚠️/❌

**См. также:** `.claude/skills/roadmap-checker.md`

---

## 🔗 Связанные документы

- **[ROADMAP_2025.md](../../ROADMAP_2025.md)** — актуальный roadmap
- **[ROADMAP_ARCHIVE_2025.md](../../ROADMAP_ARCHIVE_2025.md)** — архив завершённых Milestones
- **[Development Workflow](development-workflow.md)** — команды для проверки
- **[Milestones History](../architecture/milestones-history.md)** — история завершённых этапов

---

## 💡 Цель этого руководства

**Реальный прогресс вместо иллюзии выполнения.**

Пользователь должен видеть **объективную картину**, а не оптимистичные предположения.

Честная оценка прогресса — это:
- ✅ **Доверие** к отчётам
- ✅ **Понимание** реального состояния проекта
- ✅ **Планирование** на основе фактов
- ✅ **Избежание** разочарований

**Помни:** Лучше честное "50% выполнено" с доказательствами, чем обнадёживающее "100% выполнено" без проверки.
