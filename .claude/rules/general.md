# BSL Gradual Type System

AI-ассистент инструкции для проекта статической типизации языка 1С.

## Ключевые принципы

### 1. Right-Sized Architecture
**6-8 компонентов** вместо 25-30. Start simple, scale up по необходимости.

### 2. Semantic IR Layer
**Независимость от парсера** через SemanticProgram:
```
AST → IR (SemanticProgram) → AnalysisEngine → TypeResolver
```

### 3. Честная проверка выполнения
**ОБЯЗАТЕЛЬНО:** grep/Read/cargo test **ПЕРЕД** отчётом о выполнении Milestone задач.
**См.:** [docs/guides/roadmap-verification.md](docs/guides/roadmap-verification.md)

### 4. Модульная документация
Каждая тема в отдельном файле в `docs/`. Легко найти, легко обновить.

### 5. Фасетная система типов
Один тип 1С = множество представлений: Manager | Object | Reference | Selection | List
**Научная основа:** Balyuk & Popova (2021)

## Философия

**Реальный прогресс вместо иллюзии выполнения.**

- Честная оценка прогресса с доказательствами
- Модульная документация для простоты поддержки
- Автоматизация через Claude Skills
- Right-Sized Architecture: простота масштабируется лучше сложности

## Быстрый старт

```bash
# Сборка
cargo build --release

# LSP Server
cargo run --bin bsl-lsp-server

# Web Server (с типами платформы)
cargo run -p bsl-backend --bin bsl-web-server -- \
  --port 3002 \
  --enable-cors true \
  --syntax-helper-path examples/syntax_helper

# Тесты
cargo test --workspace
```

## Полезные ссылки

- **Научная статья:** [Balyuk & Popova (2021)](https://ceur-ws.org/Vol-2984/paper13.pdf)
- **MCP Documentation:** https://modelcontextprotocol.io/
- **Claude Code Docs:** https://docs.claude.com/en/docs/claude-code/

---

**Версия проекта:** 0.4.0
**Прогресс Версии 2.0:** ~65% завершено (13/20 Milestones)
