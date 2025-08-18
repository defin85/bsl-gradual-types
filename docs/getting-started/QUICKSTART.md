# Quick Start Guide

Быстрый старт с BSL Gradual Type System v1.0.0 - от установки до первого анализа за 5 минут!

## ⚡ 5-минутный старт

### 1️⃣ Установка (2 минуты)
```bash
# Клонирование
git clone https://github.com/yourusername/bsl-gradual-types.git
cd bsl-gradual-types

# Быстрая сборка
cargo build --release
```

### 2️⃣ Первый анализ (1 минута)
```bash
# Создание тестового BSL файла
cat > test.bsl << 'EOF'
Функция ТестоваяФункция(параметр1, параметр2)
    Если параметр1 > 0 Тогда
        результат = параметр1 + параметр2;
        Возврат Строка(результат);
    Иначе
        Возврат "ошибка";
    КонецЕсли;
КонецФункции

Процедура ГлавнаяПроцедура()
    значение = ТестоваяФункция(10, 20);
    Сообщить("Результат: " + значение);
КонецПроцедуры
EOF

# Анализ файла
./target/release/type-check --file test.bsl
```

**Ожидаемый результат:**
```
✅ Type checking completed
📊 Functions: 1, Variables: 2
🚨 Diagnostics: 0 errors, 0 warnings
⏱️  Analysis time: 125μs
```

### 3️⃣ Web интерфейс (1 минута)
```bash
# Запуск web browser
./target/release/bsl-web-server --port 8080 &

# Открыть в браузере
open http://localhost:8080
# Или: start http://localhost:8080 (Windows)
```

### 4️⃣ VSCode интеграция (1 минута)
```bash
# Сборка extension
cd vscode-extension
npm install && npm run compile
vsce package

# Установка в VSCode
code --install-extension bsl-gradual-types-1.0.0.vsix

# Открытие BSL файла
code ../test.bsl
```

## 🎯 Что вы получите

### ✨ В CLI
- **Type checking** с detailed информацией
- **Performance metrics** в реальном времени
- **Flow-sensitive analysis** изменений типов
- **Union types** для complex scenarios

### 🌐 В Web Browser
- **Live type search** среди 476+ платформенных функций
- **Code analysis** в браузере с instant результатами
- **Performance dashboard** с system stats
- **REST API** для integration с другими tools

### 💻 В VSCode
- **Type hints** inline в коде
- **Enhanced hover** с detailed type info
- **Smart completion** с context awareness
- **Code actions** для automatic fixes
- **Real-time diagnostics** без задержек

## 🚀 Следующие шаги

### Изучение возможностей
1. **[Examples Guide](../EXAMPLES.md)** - Практические примеры
2. **[API Documentation](../API.md)** - Полная справка API
3. **[Architecture](../architecture/overview.md)** - Понимание архитектуры

### Интеграция в проект
1. **[LSP Server Setup](../usage/LSP_SERVER.md)** - Настройка для IDE
2. **[CI/CD Integration](../deployment/CICD.md)** - Добавление в build pipeline
3. **[Performance Tuning](../usage/PERFORMANCE.md)** - Оптимизация для больших проектов

### Advanced Usage
1. **[Flow-Sensitive Examples](../examples/FLOW_SENSITIVE.md)** - Advanced type tracking
2. **[Union Types Guide](../examples/UNION_TYPES.md)** - Complex type scenarios
3. **[Custom Analysis](../examples/API_USAGE.md)** - Programmatic usage

## 🎪 Демонстрация возможностей

### Performance Demo
```bash
# Benchmark производительности
./target/release/bsl-profiler benchmark --iterations 10

# Ожидаемые результаты:
# 🔍 Отчет о производительности BSL Type System
# ⏱️  Общее время анализа: ~12ms
# 📈 Среднее время вызова: ~133μs
# 
# 🐌 Самые медленные компоненты:
#   1. parsing - 189μs
#   2. type_checking - 125μs  
#   3. flow_analysis - 175ns
```

### Interactive Visualization
```bash
# Запуск интерактивной визуализации типов
cargo run --example visualize_parser_v3

# Откроется type_hierarchy_v3_visualization.html с:
# - 4361 тип в системе
# - 276 категорий  
# - 6975 методов
# - 13357 свойств
# - Interactive search и filtering
```

### Real Project Analysis
```bash
# Анализ реального проекта 1С
./target/release/bsl-profiler project /path/to/your/1c/project --threads 4

# Результат:
# 🚀 Parallel analysis завершен
# 📁 Files: 156 
# ✅ Successful: 153
# 🔧 Functions: 89
# 📦 Variables: 234
# ⏱️  Total time: 3.2s
```

## 🎯 Common Use Cases

### Use Case 1: Code Review Assistant
```bash
# Анализ измененных файлов в PR
git diff --name-only HEAD~1 | grep '\.bsl$' | xargs -I {} ./target/release/type-check --file {}
```

### Use Case 2: Project Health Check
```bash
# Full project analysis с reporting
./target/release/bsl-profiler project . --output health_report.json
```

### Use Case 3: Performance Regression Testing
```bash
# Before changes
./target/release/bsl-profiler benchmark --output baseline.json

# After changes  
./target/release/bsl-profiler benchmark --output current.json

# Compare
./target/release/bsl-profiler analyze current.json
```

## ❓ FAQ

**Q: Что если LSP сервер не запускается?**
A: Проверьте что `cargo build --release` завершился успешно и файл `target/release/lsp-server` существует.

**Q: Почему в VSCode нет type hints?**
A: Убедитесь что:
1. Extension установлен и активен
2. Открыт .bsl файл  
3. LSP connection установлен (см. Output → "BSL Gradual Types")
4. Настройка `bsl.typeHints.showVariableTypes: true`

**Q: Медленная производительность?**
A: 
1. Включите кеширование: `bsl.analysis.enableCaching: true`
2. Используйте parallel analysis: `bsl.analysis.useParallelProcessing: true`
3. Проверьте performance: `Ctrl+Shift+P` → "BSL: Run Performance Profiling"

**Q: Как обновить до новой версии?**
A:
```bash
git pull origin master
cargo build --release
cd vscode-extension && npm run compile && vsce package
```

---

## 🎉 Готово!

За 5 минут вы получили:
- ✅ **Работающую систему типов** для BSL
- ✅ **VSCode integration** с type hints  
- ✅ **Web interface** для browse типов
- ✅ **Performance profiling** capabilities

**Начинайте использовать BSL Gradual Type System в ваших 1С проектах!** 🚀