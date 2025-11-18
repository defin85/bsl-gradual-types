# Milestone 4.4: MCP Debug Server — Development Worktree

**Ветка:** `feature/milestone-4.4-mcp-debug-server`
**Worktree:** `C:\1CProject\bsl-gradual-types-milestone-4.4`
**Статус:** 🚧 В РАЗРАБОТКЕ

---

## 📋 О worktree

Это **git worktree** для изолированной разработки Milestone 4.4 (MCP Debug Server).

Git worktree позволяет работать с несколькими ветками одновременно в разных директориях, не переключая ветки в основном проекте.

---

## 📁 Структура

```
bsl-gradual-types-milestone-4.4/
├── mcp-debug-server/         # Новый crate (будет создан)
│   ├── src/
│   │   ├── main.rs
│   │   ├── server/
│   │   ├── session/
│   │   ├── dap/
│   │   └── ...
│   ├── tests/
│   └── Cargo.toml
│
├── docs/milestones/
│   └── milestone-4.4-mcp-debug-server-plan.md  # Детальный план
│
└── MILESTONE_4.4_README.md   # Этот файл
```

---

## 🎯 План реализации

**См. детальный план:** [docs/milestones/milestone-4.4-mcp-debug-server-plan.md](docs/milestones/milestone-4.4-mcp-debug-server-plan.md)

**11 этапов** (17 дней):

1. ✅ Создание crate структуры + Cargo.toml (0.5 дня)
2. ⏳ DAP Client implementation (3 дня)
3. ⏳ Session Manager (2 дня)
4. ⏳ MCP Server skeleton (1 день)
5. ⏳ MCP Tools (основные 6) (2 дня)
6. ⏳ MCP Tools (продвинутые 6) (1.5 дня)
7. ⏳ MCP Resources (1 день)
8. ⏳ Event handling (2 дня)
9. ⏳ Error handling + logging (1 день)
10. ⏳ Интеграционные тесты (2 дня)
11. ⏳ Документация + примеры (1 день)

---

## 🚀 Быстрый старт

### Переключение в worktree

```bash
cd C:\1CProject\bsl-gradual-types-milestone-4.4
```

### Создание структуры проекта (Этап 1)

```bash
# Создать новый crate
cargo new --lib mcp-debug-server
cd mcp-debug-server

# Создать структуру модулей
mkdir -p src/{server,session,dap,types,config}
mkdir -p tests/integration
```

### Сборка и тестирование

```bash
# Сборка
cargo build

# Тесты
cargo test

# Запуск MCP server
cargo run --bin mcp-debug
```

---

## 🔧 Технический стек

- **Язык:** Rust
- **MCP SDK:** rmcp 0.8.5
- **DAP Client:** dap-rs 0.2.0-alpha1
- **Async runtime:** tokio
- **Первый adapter:** CodeLLDB (LLDB)

---

## 📝 Git workflow

### Коммиты

```bash
# Обычный коммит
git add .
git commit -m "feat(mcp-debug): ..."

# Коммит с Co-Authored-By
git commit -m "$(cat <<'EOF'
feat(mcp-debug): Описание изменений

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

### Синхронизация с master

```bash
# Обновить worktree с изменениями из master
git fetch origin
git merge origin/master
```

### Push в remote

```bash
# Push feature ветки
git push -u origin feature/milestone-4.4-mcp-debug-server
```

### Возврат в основной worktree

```bash
cd C:\1CProject\bsl-gradual-types
```

---

## 🧹 Удаление worktree (после завершения)

```bash
# Из основного проекта
cd C:\1CProject\bsl-gradual-types

# Удалить worktree
git worktree remove ../bsl-gradual-types-milestone-4.4

# Удалить ветку (после merge)
git branch -d feature/milestone-4.4-mcp-debug-server
```

---

## 📚 Полезные ссылки

- **Детальный план:** [docs/milestones/milestone-4.4-mcp-debug-server-plan.md](docs/milestones/milestone-4.4-mcp-debug-server-plan.md)
- **ROADMAP_2025.md:** [Milestone 4.4](../ROADMAP_2025.md)
- **DAP Specification:** https://microsoft.github.io/debug-adapter-protocol/
- **RMCP SDK:** https://github.com/modelcontextprotocol/rust-sdk
- **dap-rs:** https://github.com/sztomi/dap-rs

---

## ✅ Контрольный список

- [x] Worktree создан
- [x] План от Architect сохранён
- [ ] Структура проекта создана (Этап 1)
- [ ] DAP Client реализован (Этап 2)
- [ ] Session Manager реализован (Этап 3)
- [ ] MCP Server skeleton (Этап 4)
- [ ] MCP Tools (Этапы 5-6)
- [ ] MCP Resources (Этап 7)
- [ ] Event Handling (Этап 8)
- [ ] Error handling (Этап 9)
- [ ] Тесты (Этап 10)
- [ ] Документация (Этап 11)

---

**Версия:** 1.0
**Дата создания:** 2025-11-18
**Автор:** Claude Code
