# Test Runner Skill

Комплексное тестирование всего BSL Gradual Types проекта с детальной отчётностью.

## 🎯 Назначение

Автоматизированный запуск всех типов тестов в проекте:
- ✅ Rust unit тесты
- ✅ Rust integration тесты
- ✅ TypeScript тесты (VSCode Extension)
- ✅ Компиляция и линтинг
- ✅ Сводный отчёт

## 🔧 Процесс тестирования

### 1. Rust Workspace тесты

```bash
# Все тесты в workspace
cargo test --workspace

# С детальным выводом
cargo test --workspace -- --nocapture

# Только конкретный крейт
cargo test -p bsl-backend
cargo test -p bsl-shared
cargo test -p bsl-frontend
```

### 2. Integration тесты

```bash
# Все integration тесты
cargo test --test '*'

# Конкретные integration тесты
cargo test --test inline_scope_analysis_test
cargo test --test hover_with_spans_test
cargo test --test hover_unknown_type_test
cargo test --test syntax_error_detection_test
cargo test --test semantic_visualization_test
```

### 3. Конкретные фичи

```bash
# Semantic IR тесты
cargo test -p bsl-shared ir

# Type Resolution тесты
cargo test -p bsl-shared resolver

# Tree-Sitter тесты
cargo test -p bsl-backend tree_sitter

# Parser тесты
cargo test -p bsl-backend parser
```

### 4. VSCode Extension тесты

```bash
cd vscode-extension

# Установка зависимостей (если нужно)
npm install

# TypeScript компиляция
npm run compile

# Линтинг
npm run lint

# Запуск тестов
npm test

cd ..
```

### 5. Компиляция и проверки

```bash
# Проверка компиляции всего workspace
cargo check --workspace

# Clippy (линтер)
cargo clippy --workspace --all-targets --all-features

# Форматирование (проверка)
cargo fmt --check
```

## 📊 Формат отчёта

```markdown
# 🧪 Отчёт о тестировании BSL Gradual Types

**Дата:** 2025-11-03
**Версия:** 0.4.0

---

## ✅ Rust Workspace тесты

**Команда:** `cargo test --workspace`

**Результат:**
- ✅ bsl-shared: **43/43** тестов прошли
- ✅ bsl-backend: **87/87** тестов прошли
- ✅ bsl-frontend: **12/12** тестов прошли
- ✅ bsl-cli: **5/5** тестов прошли

**Итого:** ✅ **147/147** тестов прошли успешно

---

## ✅ Integration тесты

**Команда:** `cargo test --test '*'`

**Результат:**
- ✅ inline_scope_analysis_test: **5/5** passed
- ✅ hover_with_spans_test: **6/6** passed
- ✅ hover_unknown_type_test: **3/3** passed
- ✅ syntax_error_detection_test: **4/4** passed
- ✅ semantic_visualization_test: **3/3** passed

**Итого:** ✅ **21/21** integration тестов прошли

---

## ✅ VSCode Extension

**Команда:** `cd vscode-extension && npm test`

**Результат:**
- ✅ Компиляция TypeScript: успешна
- ✅ ESLint проверка: 0 ошибок
- ✅ Extension тесты: **8/8** passed

---

## ✅ Компиляция и линтинг

**Команда:** `cargo check --workspace && cargo clippy`

**Результат:**
- ✅ cargo check: компиляция успешна
- ⚠️ cargo clippy: 3 предупреждения (не критично)
  - `backend/src/system/parser_coordinator.rs:45` — unused variable
  - `shared/src/domain/resolver.rs:120` — можно упростить if-let
  - `backend/src/presentation/lsp_server.rs:230` — missing docs

---

## 📊 Общий итог

| Категория | Результат | Статус |
|-----------|-----------|--------|
| Unit тесты | 147/147 | ✅ |
| Integration тесты | 21/21 | ✅ |
| VSCode Extension | 8/8 | ✅ |
| Компиляция | Успешна | ✅ |
| Clippy | 3 warning | ⚠️ |

**Общая оценка:** ✅ **Все тесты прошли успешно**

**Рекомендации:**
- Исправить 3 предупреждения clippy (низкий приоритет)
- Добавить документацию к lsp_server.rs:230

---

**Время выполнения:** 45 секунд
**Следующий шаг:** Сборка компонентов (`/build`)
```

## ❌ Обработка ошибок

### Если тесты провалились

```markdown
## ❌ Rust Workspace тесты

**Команда:** `cargo test --workspace`

**Результат:**
- ✅ bsl-shared: **43/43** тестов прошли
- ❌ bsl-backend: **85/87** тестов провалились (2 ошибки)
- ✅ bsl-frontend: **12/12** тестов прошли

**Итого:** ❌ **85/87** — 2 теста провалились

---

### ❌ Детали провалившихся тестов

#### 1. `backend::tests::flow_sensitive_analysis_test::test_type_narrowing`

**Ошибка:**
```
assertion failed: type_resolution.certainty == Certainty::Known
left: Inferred(0.8)
right: Known
```

**Файл:** `backend/tests/flow_analysis_test.rs:45`

**Причина:** FlowSensitiveAnalyzer не обновляет certainty после narrowing

**Рекомендация:** Исправить backend/src/domain/flow_analysis.rs:120-135

---

#### 2. `backend::tests::hover_test::test_hover_on_nil_check`

**Ошибка:**
```
panicked at 'called Result::unwrap() on an Err value: NodeNotFound'
```

**Файл:** `backend/tests/hover_test.rs:78`

**Причина:** find_node_at_position() не находит узел для nil-checked переменных

**Рекомендация:** Проверить Span extraction в tree_sitter_adapter.rs

---

## 🚨 Критический провал: Немедленно исправить!

**Не коммитить изменения** до исправления провалившихся тестов.
```

## 🎯 Использование

Запусти этот навык когда:
- Перед коммитом изменений
- После реализации новой функциональности
- Перед созданием Pull Request
- После обновления зависимостей
- Периодическая проверка здоровья проекта

**Команда:**
```
/test-runner
```

**Или:**
```
Запусти все тесты
```

## ⚙️ Опции запуска

### Быстрая проверка (только unit тесты)

```bash
cargo test --workspace --lib
```

### Полная проверка (все + integration + бенчмарки)

```bash
cargo test --workspace --all-targets
cargo bench --no-run
```

### Verbose режим (для отладки)

```bash
cargo test --workspace -- --nocapture --test-threads=1
```

## 🔄 Интеграция с CI/CD

Этот навык реплицирует логику CI pipeline:

1. ✅ Unit тесты
2. ✅ Integration тесты
3. ✅ Линтинг
4. ✅ Компиляция
5. ✅ Проверка форматирования

**Цель:** Локальная проверка перед push в репозиторий.

## ⚠️ Особенности проекта

### 1С проекты (встроенный язык) — НЕ тестируются

Согласно CLAUDE.md (глобальные инструкции):

> **ИСКЛЮЧЕНИЕ - Проекты НА ПЛАТФОРМЕ 1С (код на встроенном языке 1С):**
> - Pipeline: architect → coder → reviewer (без tester)
> - **Пропускай tester** → нет автоматизированного testing framework для встроенного языка 1С

**Но наш проект BSL Gradual Types написан на Rust/TypeScript** → тестируется полностью!

### GitBash на Windows

Все команды используют Unix-style синтаксис (работает в GitBash):

```bash
# ✅ Работает в GitBash
cargo test --workspace

# ❌ НЕ работает (PowerShell syntax)
cargo test -workspace
```
