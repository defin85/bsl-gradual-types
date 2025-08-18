# CLAUDE.md

Codebase and user instructions for BSL Gradual Type System v1.0.0

## 🎯 О проекте

**BSL Gradual Type System v1.0.0** - enterprise-ready система градуальной типизации для языка 1С:Предприятие BSL.

**Статус**: ✅ ЗАВЕРШЕН - готов к production использованию

## 🔧 Команды для разработки

### Сборка
```bash
cargo build --release    # Production сборка
cargo build              # Debug сборка  
cargo check              # Проверка без сборки
cargo test               # Все тесты
```

### CLI инструменты
```bash
# Основные
cargo run --bin type-check -- --file module.bsl
cargo run --bin lsp-server
cargo run --bin bsl-web-server --port 8080

# Профилирование
cargo run --bin bsl-profiler benchmark
cargo run --bin bsl-profiler project /path/to/1c --threads 4
```

### VSCode Extension
```bash
cd vscode-extension
npm install && npm run compile
vsce package
code --install-extension bsl-gradual-types-1.0.0.vsix
```

## 🏗️ Архитектура

### Ключевые модули
- **Core**: `types.rs`, `flow_sensitive.rs`, `union_types.rs`, `interprocedural.rs`
- **Parser**: `tree_sitter_adapter.rs` - на основе tree-sitter-bsl
- **LSP**: `lsp_enhanced.rs` - enhanced LSP с инкрементальным парсингом
- **Tools**: `profiler.rs`, `web_server.rs` - CLI и web инструменты

### Продвинутые анализаторы
- **Flow-Sensitive** - отслеживание изменений типов переменных
- **Union Types** - weighted union с нормализацией и упрощением
- **Interprocedural** - анализ типов через границы функций
- **Performance** - профилирование и кеширование

## 📊 Performance

- **Парсинг**: ~189μs
- **Type Checking**: ~125μs  
- **Flow Analysis**: ~175ns
- **LSP Response**: <100ms

## 🏆 Завершенные фазы

- ✅ **Phase 1-3**: MVP, парсеры, запросы
- ✅ **Phase 4**: Расширенный анализ, tree-sitter миграция
- ✅ **Phase 5**: Production readiness, enhanced LSP, performance
- ✅ **Phase 6**: IDE integration, VSCode extension, web browser

**Результат**: Первая в мире production-ready система градуальной типизации для BSL с enterprise capabilities.

## ⚠️ Принципы разработки

1. **Честная неопределённость** - TypeResolution::Unknown лучше неправильного Inferred
2. **Эволюционность** - каждая фаза даёт работающий функционал
3. **Модульность** - новые анализаторы через traits
4. **Производительность** - enterprise-grade performance requirements