---
name: roadmap-checker
description: Автоматическая проверка выполнения задач из ROADMAP с честной оценкой
---

# Roadmap Checker Skill

Автоматическая проверка выполнения задач из ROADMAP_2025.md с честной оценкой прогресса.

## 🎯 Назначение

Этот навык автоматизирует процесс проверки выполнения Milestone задач из roadmap, используя реальные инструменты проверки вместо предположений.

## 🔍 Процесс проверки

### 1. Чтение Roadmap

```bash
# Прочитать текущий roadmap
Read ROADMAP_2025.md

# Извлечь список задач текущего Milestone
# Пример: Milestone 2.19 — Flow-Sensitive Type Analysis
```

### 2. Для каждой задачи выполнить проверку

#### Task 1: Проверка кода

```bash
# Поиск реализации в коде
grep -rn "FlowSensitiveAnalyzer\|ControlFlowGraph" backend/src/

# Проверка существования файлов
find backend/src -name "*flow*" -o -name "*cfg*"

# Чтение конкретных файлов
Read backend/src/domain/flow_analysis.rs
```

#### Task 2: Проверка тестов

```bash
# Поиск тестов
grep -rn "flow_sensitive\|cfg_test" backend/tests/

# Запуск конкретных тестов
cargo test -p bsl-backend flow_sensitive

# Проверка прохождения
cargo test -p bsl-backend --test flow_analysis_test
```

#### Task 3: Проверка компиляции

```bash
# Проверка, что код компилируется
cargo check -p bsl-backend

# Полная сборка
cargo build -p bsl-backend
```

### 3. Генерация отчёта

#### Формат отчёта

```markdown
## Статус выполнения [Milestone X.Y: Название]

### ✅ Task 1: [Название] — ВЫПОЛНЕНО (100%)

**Проверка:**
- ✅ backend/src/domain/flow_analysis.rs:45 — реализация FlowSensitiveAnalyzer найдена
- ✅ cargo test flow_sensitive — 8/8 тестов проходят
- ✅ cargo check — компиляция успешна

**Код:**
\`\`\`rust
pub struct FlowSensitiveAnalyzer {
    cfg: ControlFlowGraph,
    type_states: HashMap<VariableId, TypeState>,
}
\`\`\`

---

### ❌ Task 2: [Название] — НЕ НАЧАТО (0%)

**Проверка:**
- ❌ grep показывает отсутствие кода
- ❌ файл backend/src/domain/nullability_analysis.rs не существует
- ❌ тесты отсутствуют

**Необходимо:**
1. Создать модуль nullability_analysis
2. Реализовать NullabilityChecker
3. Написать unit тесты

---

### ⚠️ Task 3: [Название] — ЧАСТИЧНО (40%)

**Что есть:**
- ✅ backend/src/domain/type_narrowing.rs:120 — базовая структура TypeNarrowing
- ✅ cargo test type_narrowing — 3/3 тестов проходят

**Что отсутствует:**
- ❌ Не реализован метод narrow_by_condition()
- ❌ Нет интеграции с FlowSensitiveAnalyzer
- ❌ Отсутствуют тесты для сложных условий

**Необходимо:**
1. Дореализовать narrow_by_condition()
2. Добавить интеграцию с CFG
3. Написать 5+ дополнительных тестов

---

## 📊 Общий прогресс Milestone X.Y: 47% (1✅ + 0.4⚠️) / 3 tasks

**Заблокировано:** Task 2 блокирует Task 3 (зависимость)
**Следующий шаг:** Реализовать Task 2 полностью
```

## 🚨 Критические правила проверки

### ❌ ЗАПРЕЩЕНО:

1. **Предполагать наличие кода** без проверки через Read/Grep
2. **Заявлять о прохождении тестов** без запуска `cargo test`
3. **Утверждать о выполнении** без реальных доказательств
4. **Округлять прогресс вверх** (10% → 100%)

### ✅ ОБЯЗАТЕЛЬНО:

1. **Читать файлы** через Read tool для проверки кода
2. **Запускать тесты** через `cargo test` для проверки работоспособности
3. **Проверять компиляцию** через `cargo check`
4. **Указывать точные координаты** (файл:строка) для найденного кода
5. **Честно оценивать прогресс** (даже если это 5%)

## 📝 Примеры правильной проверки

### Пример 1: Проверка Tree-Sitter интеграции

```bash
# 1. Поиск зависимости
grep -n "tree-sitter-bsl" Cargo.toml backend/Cargo.toml

# 2. Проверка использования
grep -rn "TreeSitterAdapter\|tree_sitter" backend/src/

# 3. Чтение реализации
Read backend/src/system/tree_sitter_adapter.rs

# 4. Запуск тестов
cargo test -p bsl-backend tree_sitter
```

### Пример 2: Проверка SemanticIR Layer

```bash
# 1. Проверка структуры
find shared/src/ir -type f -name "*.rs"

# 2. Чтение ключевых модулей
Read shared/src/ir/mod.rs
Read shared/src/ir/semantic_program.rs

# 3. Поиск использования
grep -rn "SemanticProgram\|SemanticNode" backend/src/

# 4. Проверка тестов
cargo test -p bsl-shared ir
```

## 🎯 Использование

Запусти этот навык когда:
- Пользователь спрашивает "сколько выполнено из Milestone X.Y?"
- Нужен отчёт о прогрессе проекта
- Требуется проверка перед архивированием Milestone
- Необходима валидация перед коммитом в roadmap

**Команда:**
```
/roadmap-checker
```

**Или:**
```
Проверь выполнение Milestone 2.19
```

## 🔄 Интеграция с Roadmap процессом

Этот навык реализует требование из CLAUDE.md:

> **ПРИ ВЫПОЛНЕНИИ ЛЮБЫХ ЭТАПОВ ИЗ ROADMAP:**
>
> 1. **ПЕРЕД отчётом о выполнении** — ОБЯЗАТЕЛЬНО провести **контрольную проверку фактического выполнения**
> 2. **Проверка должна включать:** ✅ Чтение реального кода, ✅ Поиск реализованных функций, ✅ Запуск тестов, ✅ Проверка компиляции
> 3. **ЗАПРЕЩЕНО:** ❌ Утверждать о выполнении без проверки кода

**Цель:** Реальный прогресс вместо иллюзии выполнения.
