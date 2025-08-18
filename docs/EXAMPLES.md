# Примеры использования BSL Gradual Type System v1.0.0

Comprehensive руководство по использованию всех возможностей системы типов.

## 🚀 Быстрый старт

### Анализ BSL файла

```bash
# Простая проверка типов
cargo run --bin type-check -- --file examples/bsl/test_example.bsl

# Детальный анализ с профилированием
cargo run --bin bsl-profiler profile --file examples/bsl/test_example.bsl --verbose
```

### LSP сервер для IDE

```bash
# Запуск enhanced LSP сервера
cargo run --bin lsp-server

# В VSCode: установка расширения
cd vscode-extension
npm install && npm run compile
vsce package
code --install-extension bsl-gradual-types-1.0.0.vsix
```

### Web-based анализ

```bash
# Запуск web сервера
cargo run --bin bsl-web-server --port 8080

# Открыть http://localhost:8080
# Ввести BSL код для live анализа
```

## 🔍 Flow-Sensitive Analysis Examples

### Пример 1: Отслеживание изменений типов
```bsl
Процедура ПримерОтслеживанияТипов()
    // Flow-sensitive анализ отследит изменения типа переменной
    переменная = "строка";        // Тип: Строка (Known)
    переменная = 123;             // Тип: Число (Known)
    
    Если переменная > 100 Тогда
        переменная = "большое число";  // В этой ветке: Строка
    КонецЕсли;
    
    // После условия: Union(Строка | Число) с весами
КонецПроцедуры
```

**Анализ в CLI:**
```bash
cargo run --bin type-check -- --file flow_example.bsl
# Результат: Flow-sensitive анализ покажет изменения типа переменной
```

### Пример 2: Type Narrowing в условиях
```bsl
Функция ПримерTypeNarrowing(значение)
    Если ТипЗнч(значение) = Тип("Строка") Тогда
        // В этой ветке система знает что значение - Строка
        длина = СтрДлина(значение);  // Автодополнение покажет строковые методы
        Возврат длина;
    ИначеЕсли ТипЗнч(значение) = Тип("Число") Тогда
        // В этой ветке - Число
        Возврат значение * 2;
    Иначе
        Возврат Неопределено;
    КонецЕсли;
КонецФункции
```

## 🔗 Union Types Examples

### Пример 3: Работа с Union типами
```bsl
Функция ПримерUnionТипов(условие)
    Если условие Тогда
        Возврат "текстовый результат";
    Иначе  
        Возврат 42;
    КонецЕсли;
    // Возвращает: Union(Строка 50% | Число 50%)
КонецФункции

Процедура ИспользованиеUnion()
    результат = ПримерUnionТипов(Истина);
    // Type hints покажет: результат: Строка~|Число~
    
    Если ТипЗнч(результат) = Тип("Строка") Тогда
        // Type narrowing: здесь результат точно Строка
        Сообщить("Длина: " + СтрДлина(результат));
    КонецЕсли;
КонецПроцедуры
```

## ⚡ Performance & Caching Examples

### Пример 4: Профилирование производительности
```bash
# Benchmark всей системы
cargo run --bin bsl-profiler benchmark --iterations 20

# Сравнение двух версий файла
cargo run --bin bsl-profiler compare old_version.bsl new_version.bsl --iterations 10

# Анализ большого проекта с профилированием
cargo run --bin bsl-profiler project /path/to/1c/project --threads 8 --benchmark
```

**Ожидаемые результаты:**
```
🔍 Отчет о производительности BSL Type System
📊 Сессия: 25.30ms
⏱️  Общее время анализа: 24.15ms
🔢 Общее количество вызовов: 180
📈 Среднее время вызова: 134μs

🐌 Самые медленные компоненты:
  1. parsing - 189μs
  2. type_checking - 125μs  
  3. flow_analysis - 175ns
```

### Пример 5: Кеширование для больших проектов
```bash
# Первый запуск (без кеша)
time cargo run --bin bsl-profiler project large_project/

# Второй запуск (с кешем)
time cargo run --bin bsl-profiler project large_project/
# Должен быть значительно быстрее благодаря кешированию
```

## 🌐 Web API Examples

### Пример 6: REST API для поиска типов
```bash
# Поиск платформенных типов
curl "http://localhost:8080/api/types?search=Массив"

# Детальная информация о типе
curl "http://localhost:8080/api/types/ТаблицаЗначений"

# Анализ кода через API
curl -X POST "http://localhost:8080/api/analyze" \
     -H "Content-Type: application/json" \
     -d '{
       "code": "Функция Тест()\n    Возврат Новый Массив;\nКонецФункции",
       "filename": "test.bsl"
     }'
```

**Response анализа:**
```json
{
  "success": true,
  "functions": 1,
  "variables": 0,
  "diagnostics": [],
  "analysis_time_ms": 12
}
```

## 🎯 VSCode Integration Examples

### Пример 7: Type Hints в VSCode
После установки расширения, в BSL коде автоматически появятся:

```bsl
переменная = НайтиМаксимум(10, 20);  // : Число~
массив = Новый Массив;               // : Массив
```

### Пример 8: Code Actions в VSCode
При наведении на ошибки:
- `Переменная 'х' используется без объявления` → Code Action: "Объявить переменную 'х'"
- `Несовместимое присваивание` → Code Action: "Добавить приведение типа"
- Выделение кода → Code Action: "Извлечь в функцию"

### Пример 9: Enhanced Hover в VSCode
При наведении на переменную:
```
**переменная**

*Тип:* Union(Строка 60% | Число 40%)
*Уверенность:* 85%  
*Источник:* Flow-sensitive analysis
*Варианты:* 2
```

## 🔧 Programmatic API Examples

### Пример 10: Использование в Rust коде
```rust
use bsl_gradual_types::core::type_checker::TypeChecker;
use bsl_gradual_types::core::flow_sensitive::FlowSensitiveAnalyzer;
use bsl_gradual_types::parser::common::ParserFactory;

// Базовый анализ
let mut parser = ParserFactory::create();
let program = parser.parse(bsl_code)?;

let type_checker = TypeChecker::new("module.bsl".to_string());
let (context, diagnostics) = type_checker.check(&program);

println!("Найдено функций: {}", context.functions.len());
println!("Найдено переменных: {}", context.variables.len());

// Flow-sensitive анализ  
let mut flow_analyzer = FlowSensitiveAnalyzer::new(context.clone());
for statement in &program.statements {
    flow_analyzer.analyze_statement(statement);
}

let final_state = flow_analyzer.get_final_state();
println!("Финальное состояние: {} переменных", final_state.variable_types.len());
```

### Пример 11: Union Types в коде
```rust
use bsl_gradual_types::core::union_types::UnionTypeManager;
use bsl_gradual_types::core::types::{ConcreteType, PrimitiveType};

// Создание Union типа
let union = UnionTypeManager::from_concrete_types(vec![
    ConcreteType::Primitive(PrimitiveType::String),
    ConcreteType::Primitive(PrimitiveType::Number),
]);

// Проверка совместимости
let string_type = create_string_type();
let is_compatible = UnionTypeManager::is_compatible_with_union(&string_type, &union_types);

// Получение наиболее вероятного типа
let most_likely = UnionTypeManager::get_most_likely_type(&union_types);
```

### Пример 12: Performance Profiling в коде
```rust
use bsl_gradual_types::core::performance::{BenchmarkSuite, PerformanceOptimizer};

// Бенчмарк парсинга
let metrics = BenchmarkSuite::benchmark_parsing(source_code, 50);
println!("Среднее время парсинга: {:?}", metrics.avg_time);

// Полный набор бенчмарков
let report = BenchmarkSuite::run_full_benchmark_suite();

// Анализ и рекомендации
let suggestions = PerformanceOptimizer::analyze_and_suggest(&report);
for suggestion in suggestions {
    println!("💡 {}", suggestion.suggestion);
}
```

## 📊 Integration Examples

### Пример 13: CI/CD интеграция
```yaml
# .github/workflows/bsl-analysis.yml
name: BSL Type Analysis
on: [push, pull_request]

jobs:
  type-check:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    
    - name: Install BSL Gradual Types
      run: cargo install --path .
      
    - name: Analyze BSL files
      run: |
        find . -name "*.bsl" -exec cargo run --bin type-check -- --file {} \;
        
    - name: Performance regression test
      run: cargo run --bin bsl-profiler benchmark --iterations 5
```

### Пример 14: Docker интеграция
```dockerfile
FROM rust:1.70-alpine

WORKDIR /app
COPY . .

RUN cargo build --release

# LSP сервер
EXPOSE 3000
CMD ["./target/release/lsp-server"]

# Или Web сервер
EXPOSE 8080  
CMD ["./target/release/bsl-web-server", "--port", "8080"]
```

## 🎯 Advanced Usage Examples

### Пример 15: Пользовательские анализаторы
```rust
// Создание custom анализатора с flow-sensitive возможностями
use bsl_gradual_types::core::{
    flow_sensitive::FlowSensitiveAnalyzer,
    interprocedural::{CallGraph, InterproceduralAnalyzer},
    union_types::UnionTypeManager
};

pub struct CustomAnalyzer {
    flow_analyzer: FlowSensitiveAnalyzer,
    interprocedural_analyzer: InterproceduralAnalyzer,
}

impl CustomAnalyzer {
    pub fn analyze_with_all_features(&mut self, program: &Program) {
        // Используем все продвинутые анализаторы
        for statement in &program.statements {
            self.flow_analyzer.analyze_statement(statement);
        }
        
        self.interprocedural_analyzer.analyze_all_functions();
        
        // Получаем comprehensive результаты
        let flow_state = self.flow_analyzer.get_final_state();
        let function_results = self.interprocedural_analyzer.get_analyzed_functions();
        
        // Обрабатываем результаты...
    }
}
```

### Пример 16: Memory Optimization для больших проектов
```rust
use bsl_gradual_types::core::memory_optimization::{
    MemoryOptimizationManager,
    StringInterner
};

let mut optimizer = MemoryOptimizationManager::new();

// Оптимизация контекста типов
let compact_context = optimizer.optimize_context(&type_context);

// Генерация отчета об экономии памяти
let report = optimizer.generate_optimization_report();
println!("{}", report.format_human_readable());

// Результат:
// 🧠 Отчет об оптимизации памяти
// 💾 Общая экономия: 15.2 KB
// 🧵 String Interning: 45% hit rate, 8.3 KB saved
```

## 📱 Real-world Usage Scenarios

### Сценарий 1: Анализ корпоративного проекта 1С
```bash
# Анализ большого проекта с полным профилированием
cargo run --bin bsl-profiler project "/path/to/corporate/1c" \
    --threads 8 \
    --benchmark \
    > analysis_report.txt

# Веб-интерфейс для команды разработки
cargo run --bin bsl-web-server \
    --project "/path/to/corporate/1c" \
    --port 8080

# Команда открывает http://localhost:8080 для просмотра типов
```

### Сценарий 2: CI/CD pipeline для 1С проекта
```bash
# В CI pipeline
./bsl-gradual-types/target/release/type-check --file src/CommonModules/*.bsl
./bsl-gradual-types/target/release/bsl-profiler project . --threads 4

# Если анализ не прошел - fail the build
if [ $? -ne 0 ]; then
  echo "❌ BSL type analysis failed"
  exit 1
fi
```

### Сценарий 3: Интерактивная разработка в VSCode
1. **Открыть BSL файл** → автоматически активируется enhanced LSP
2. **Набирать код** → real-time type hints и диагностика
3. **Hover над переменной** → детальная информация с flow-sensitive типами
4. **Ctrl+Space** → smart автодополнение с union types
5. **Lightbulb icon** → code actions для автоматических исправлений

## 🎛️ Настройка и конфигурация

### VSCode Settings
```json
{
  "bsl.typeHints.showVariableTypes": true,
  "bsl.typeHints.showReturnTypes": true,
  "bsl.typeHints.showUnionDetails": true,
  "bsl.typeHints.minCertainty": 0.8,
  "bsl.performance.enableProfiling": true,
  "bsl.analysis.useParallelProcessing": true,
  "bsl.analysis.enableCaching": true
}
```

### Конфигурация LSP сервера
```json
{
  "initializationOptions": {
    "enableFlowSensitiveAnalysis": true,
    "enableUnionTypes": true, 
    "enableInterproceduralAnalysis": true,
    "cacheDirectory": "./.bsl_cache",
    "performanceProfiling": {
      "enableProfiling": false,
      "maxMemoryUsageMB": 512
    }
  }
}
```

## 🔬 Advanced Analysis Examples

### Flow States Visualization
```rust
// Получение детальной информации о flow states
let flow_analyzer = FlowSensitiveAnalyzer::new(context);
// ... анализ ...

let all_states = flow_analyzer.get_all_states();
let merge_points = flow_analyzer.get_merge_points();

for (i, state) in all_states.iter().enumerate() {
    println!("State {}: {} variables", i, state.variable_types.len());
}
```

### Memory Usage Analysis
```rust
use bsl_gradual_types::core::memory_optimization::MemoryMonitor;

let mut monitor = MemoryMonitor::new();
let contexts = vec![&type_context1, &type_context2];

let profile = monitor.take_snapshot(&contexts);
println!("Memory usage: {:.2} MB", profile.total_memory_bytes as f64 / (1024.0 * 1024.0));

let stats = monitor.get_memory_stats();
if let Some(stats) = stats {
    println!("{}", stats.format());
}
```

---

## 📚 Дополнительные ресурсы

- [CLAUDE.md](../CLAUDE.md) - Полная документация проекта
- [API.md](API.md) - Детальная API документация
- [Architecture Overview](architecture/overview.md) - Архитектура системы
- [Examples Directory](../examples/) - Исполняемые примеры
- [VSCode Extension README](../vscode-extension/README.md) - Документация расширения

---

**💡 Tip: Начните с простых примеров type-check и постепенно изучайте advanced возможности!**