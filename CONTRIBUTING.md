# Contributing to BSL Gradual Type System

Спасибо за интерес к участию в развитии BSL Gradual Type System! 🎉

## 🎯 О проекте

BSL Gradual Type System - это enterprise-ready система градуальной типизации для 1С:Предприятие BSL. Мы создаем инструменты которые помогают разработчикам 1С писать более надежный и качественный код.

## 🤝 Как участвовать

### 1. Сообщения об ошибках (Bug Reports)
- Используйте [GitHub Issues](https://github.com/yourusername/bsl-gradual-types/issues)
- Используйте шаблон "Bug Report"
- Приложите BSL код который вызывает проблему
- Укажите версию системы и окружение

### 2. Предложения улучшений (Feature Requests)
- Используйте [GitHub Issues](https://github.com/yourusername/bsl-gradual-types/issues)
- Используйте шаблон "Feature Request"
- Опишите use case и ожидаемое поведение
- Приложите примеры BSL кода если применимо

### 3. Участие в разработке (Code Contributions)
- Fork репозитория
- Создайте feature branch (`git checkout -b feature/amazing-feature`)
- Внесите изменения с тестами
- Убедитесь что все тесты проходят
- Создайте Pull Request

## 📋 Стандарты разработки

### Code Style
```bash
# Форматирование кода
cargo fmt

# Линтинг
cargo clippy -- -D warnings

# Проверка тестов  
cargo test
```

### Cargo.lock policy

- `Cargo.lock` хранится в git и считается источником истины для сборки.
- Не обновляй зависимости "случайно": избегай `cargo update` без явного запроса/PR.
- Если меняешь зависимости или фичи зависимостей, коммить соответствующий diff `Cargo.lock` в том же PR.

### Commit Message Format
```
type(scope): short description

Detailed description if needed

Fixes #issue_number
```

**Types:**
- `feat` - новая функциональность
- `fix` - исправление ошибки
- `docs` - изменения в документации
- `style` - форматирование кода
- `refactor` - рефакторинг без изменения функциональности
- `test` - добавление тестов
- `chore` - обновление build scripts, dependencies

**Scopes:**
- `core` - изменения в core layer
- `parser` - изменения в парсере
- `lsp` - изменения в LSP сервере
- `cli` - изменения в CLI инструментах
- `vscode` - изменения в VSCode extension
- `web` - изменения в web сервере
- `docs` - изменения в документации

### Examples:
```
feat(core): add flow-sensitive analysis for union types
fix(lsp): resolve crash on large file hover  
docs(api): add REST API documentation
test(parser): add comprehensive tree-sitter tests
```

## 🏗️ Development Setup

### Prerequisites
- Rust 1.70+
- Node.js 16+ (для VSCode extension)
- Git

### Local Development
```bash
# Клонирование
git clone https://github.com/yourusername/bsl-gradual-types
cd bsl-gradual-types

# Установка зависимостей
cargo build

# VSCode extension setup
cd vscode-extension
npm install

# Запуск тестов
cargo test
cd vscode-extension && npm test
```

### Testing
```bash
# Unit tests
cargo test --lib

# Integration tests  
cargo test --test "*"

# Performance tests
cargo run --bin bsl-profiler benchmark --iterations 5

# VSCode extension tests
cd vscode-extension
npm test
```

## 📝 Areas for Contribution

### 🔥 High Priority
1. **Performance Optimization**
   - Оптимизация memory usage для очень больших проектов
   - Улучшение кеширования между LSP сессиями
   - Профилирование и bottleneck analysis

2. **LSP Server Enhancement**
   - Дополнительные code actions
   - Улучшение semantic highlighting
   - Оптимизация incremental parsing

3. **Type System Extensions**
   - Расширение поддержки сложных BSL конструкций
   - Улучшение union types для edge cases
   - Cross-module type inference

### 🚀 Medium Priority
4. **IDE Integration**
   - IntelliJ IDEA plugin
   - Sublime Text LSP integration
   - Emacs/Vim support

5. **Tooling & Ecosystem**
   - 1C:EDT integration
   - Better error messages и diagnostics
   - Documentation generator

6. **Web Interface**
   - Улучшение UI/UX web browser
   - Add authentication для enterprise
   - GraphQL API support

### 💡 Ideas Welcome
7. **Research Areas**
   - Machine learning для type prediction
   - Static analysis для security issues
   - Code quality metrics и suggestions

## 🧪 Testing Guidelines

### Test Categories
1. **Unit Tests** (`backend/src/`, `bsl-agent/src/`, `cli/src/`, `shared/src/`) - Тестирование отдельных модулей
2. **Integration Tests** (`backend/tests/`, `bsl-agent/tests/`, `vscode-extension/src/test/`) - Тестирование взаимодействия компонентов
3. **Performance Tests** (benchmarks) - Тестирование производительности
4. **End-to-End Tests** - Тестирование полных сценариев

### Test Requirements
- Все новые функции должны быть покрыты тестами
- Тесты должны быть детерминированными
- Используйте `pretty_assertions` для лучших error messages
- Добавляйте performance тесты для критичных компонентов

### Test Examples
```rust
#[test]
fn test_flow_sensitive_analysis() {
    let context = create_test_context();
    let mut analyzer = FlowSensitiveAnalyzer::new(context);
    
    // Test setup
    let assignment = create_assignment("x", "\"string\"");
    analyzer.analyze_assignment(&assignment.target, &assignment.value);
    
    // Assertions
    let x_type = analyzer.get_variable_type("x").unwrap();
    assert_matches!(x_type.result, ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)));
}
```

## 📋 Documentation Guidelines

### Documentation Requirements
- Все публичные API должны быть документированы
- Используйте `///` для Rust doc comments
- Добавляйте examples в doc comments
- Обновляйте CHANGELOG.md при значительных изменениях

### Documentation Structure
```rust
/// Brief description of the function
/// 
/// # Arguments
/// * `param1` - Description of parameter
/// * `param2` - Description of parameter
/// 
/// # Returns
/// Description of return value
/// 
/// # Examples
/// ```rust
/// let result = my_function(arg1, arg2);
/// assert_eq!(result, expected);
/// ```
/// 
/// # Errors
/// Description of possible errors
pub fn my_function(param1: Type1, param2: Type2) -> Result<ReturnType> {
    // Implementation
}
```

## 🚦 Pull Request Process

### Before Submitting
1. ✅ Убедитесь, что обязательный verify path из `docs/agent/verification.md` пройден
2. ✅ Запустите `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` и `cargo test --workspace --locked`
3. ✅ Обновите документацию, если меняются команды, entry points или onboarding
4. ✅ Добавьте тесты для новой функциональности
5. ✅ Обновите CHANGELOG.md

### PR Review Process
1. **GitHub Actions** - workflow `CI` является default readiness gate для OpenSpec governance и agent-facing smoke checks по затронутым изменениям (см. `.github/workflows/ci.yml`).
2. **Code Review** - Минимум 1 approve от maintainer
3. **Testing** - Comprehensive testing в разных окружениях
4. **Documentation** - Проверка актуальности документации

### PR Requirements
- Описание изменений
- Ссылки на связанные issues
- Screenshots/examples если применимо
- Breaking changes (если есть)

## 🏷️ Issue Labels

- `bug` - Сообщения об ошибках
- `enhancement` - Предложения улучшений
- `good first issue` - Задачи для новичков
- `help wanted` - Нужна помощь сообщества
- `performance` - Вопросы производительности
- `documentation` - Улучшения документации
- `lsp` - LSP server related
- `vscode` - VSCode extension related
- `web` - Web server related

## 🎯 Project Roadmap

### Current Focus (v1.x)
- Performance optimization
- LSP server enhancements  
- Better IDE integration

### Future Versions (v2.x)
- Machine learning integration
- Advanced refactoring tools
- Cross-platform IDE support

## 👥 Community

### Communication Channels
- 💬 [GitHub Discussions](https://github.com/yourusername/bsl-gradual-types/discussions)
- 📧 Email: bsl-gradual-types@example.com
- 💬 Telegram: @bsl_gradual_types

### Code of Conduct
- Будьте вежливы и конструктивны
- Помогайте новичкам
- Уважайте различные точки зрения
- Фокусируйтесь на технических аспектах

## 📄 License

Участвуя в проекте, вы соглашаетесь что ваш код будет лицензирован под MIT License.

---

**🙏 Спасибо за ваш вклад в развитие экосистемы 1С:Предприятие!**
