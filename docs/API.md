# BSL Gradual Type System - API Documentation

Документация API для всех компонентов BSL Gradual Type System v1.0.0

## 🔗 LSP Server API

### Enhanced LSP Capabilities

BSL Gradual Type System предоставляет enhanced LSP сервер с расширенными возможностями:

#### Standard LSP Methods
- `textDocument/hover` - Enhanced hover с flow-sensitive типами
- `textDocument/completion` - Smart completion с union types
- `textDocument/publishDiagnostics` - Real-time диагностика
- `textDocument/inlayHint` - Type hints для inline отображения
- `textDocument/codeAction` - Автоматические исправления

#### Custom LSP Methods
- `bsl/enhancedHover` - Детальная информация о типах
- `bsl/performanceProfiling` - Профилирование производительности
- `bsl/projectAnalysis` - Анализ проекта с параллельной обработкой
- `bsl/clearCache` - Управление кешем анализа
- `bsl/cacheStats` - Статистика кеширования

### LSP Request Examples

#### Enhanced Hover Request
```typescript
// Request
{
  "method": "bsl/enhancedHover",
  "params": {
    "uri": "file:///path/to/file.bsl",
    "position": { "line": 10, "character": 15 }
  }
}

// Response
{
  "contents": {
    "value": "**Тип:** `Строка`\n**Уверенность:** 95%\n**Источник:** Flow-sensitive analysis"
  }
}
```

#### Performance Profiling Request  
```typescript
// Request
{
  "method": "bsl/performanceProfiling",
  "params": {
    "filePath": "/path/to/module.bsl"
  }
}

// Response
{
  "humanReadableReport": "📊 Парсинг: 189μs\n🔍 Type checking: 125μs",
  "jsonReport": "{\"parsing_time_us\": 189, \"type_checking_time_us\": 125}"
}
```

#### Project Analysis Request
```typescript
// Request
{
  "method": "bsl/projectAnalysis", 
  "params": {
    "projectPath": "/path/to/1c/project",
    "options": {
      "useParallelAnalysis": true,
      "enableCaching": true,
      "numThreads": 4
    }
  }
}

// Response
{
  "stats": {
    "totalFiles": 150,
    "successfulFiles": 147,
    "totalFunctions": 89,
    "totalVariables": 234,
    "totalDiagnostics": 12
  },
  "totalTime": "2.3s"
}
```

## 🌐 Web Server API

### Base URL
```
http://localhost:8080/api
```

### Endpoints

#### GET /api/types
Поиск типов BSL

**Parameters:**
- `search` (string, optional) - Поисковый запрос
- `page` (number, optional) - Номер страницы (default: 1)
- `per_page` (number, optional) - Элементов на странице (default: 20, max: 100)

**Example:**
```bash
curl "http://localhost:8080/api/types?search=Массив&page=1&per_page=10"
```

**Response:**
```json
{
  "types": [
    {
      "name": "Массив",
      "category": "Platform",
      "description": "Коллекция элементов с индексным доступом",
      "methods_count": 15,
      "properties_count": 2,
      "result_type": "Platform"
    }
  ],
  "total": 1,
  "page": 1,
  "per_page": 10
}
```

#### GET /api/types/{name}
Детальная информация о типе

**Example:**
```bash
curl "http://localhost:8080/api/types/Массив"
```

**Response:**
```json
{
  "name": "Массив",
  "category": "Platform",
  "description": "Коллекция элементов",
  "methods": [
    {
      "name": "Добавить",
      "parameters": ["Значение"],
      "return_type": null,
      "description": "Добавляет элемент в массив"
    }
  ],
  "properties": [
    {
      "name": "Количество",
      "type_name": "Число",
      "readonly": true,
      "description": "Количество элементов в массиве"
    }
  ],
  "related_types": ["Структура", "Соответствие"],
  "usage_examples": ["массив = Новый Массив;", "массив.Добавить(элемент);"]
}
```

#### POST /api/analyze
Анализ фрагмента BSL кода

**Request Body:**
```json
{
  "code": "Функция ТестоваяФункция(параметр)\n    Возврат Строка(параметр);\nКонецФункции",
  "filename": "test.bsl"
}
```

**Response:**
```json
{
  "success": true,
  "functions": 1,
  "variables": 1,
  "diagnostics": [],
  "analysis_time_ms": 15
}
```

#### GET /api/stats
Статистика системы

**Response:**
```json
{
  "total_functions": 89,
  "total_variables": 234,
  "platform_types": 476,
  "analysis_cache_size": 1024,
  "memory_usage_mb": 15.7
}
```

## 🛠️ CLI Tools API

### bsl-profiler

#### Benchmark Command
```bash
cargo run --bin bsl-profiler benchmark [OPTIONS]
```

**Options:**
- `--iterations <N>` - Количество итераций (default: 10)
- `--output <FILE>` - Сохранить результаты в JSON файл

**Output:**
```
🔍 Отчет о производительности BSL Type System
📊 Сессия: 12.50ms
⏱️  Общее время анализа: 12.02ms
🔢 Общее количество вызовов: 90
📈 Среднее время вызова: 133.51µs
```

#### Project Analysis Command
```bash
cargo run --bin bsl-profiler project <PATH> [OPTIONS]
```

**Options:**
- `--threads <N>` - Количество потоков
- `--benchmark` - Показать сравнение последовательный vs параллельный
- `--no-cache` - Отключить кеширование

### bsl-web-server

#### Start Web Server
```bash
cargo run --bin bsl-web-server [OPTIONS]
```

**Options:**
- `--port <PORT>` - Порт сервера (default: 8080)
- `--project <PATH>` - Путь к проекту 1С для анализа
- `--hot-reload` - Включить hot reload для разработки
- `--static-dir <PATH>` - Путь к статическим файлам

### type-check

#### Type Check File
```bash
cargo run --bin type-check -- --file <FILE>
```

**Output:**
```
✅ Type checking completed
📊 Functions: 5, Variables: 12
🚨 Diagnostics: 2 warnings
```

## 📋 Rust Library API

### Core Types

#### TypeResolution
```rust
pub struct TypeResolution {
    pub certainty: Certainty,        // Known | Inferred(f32) | Unknown
    pub result: ResolutionResult,    // Concrete | Union | Dynamic | Conditional
    pub source: ResolutionSource,    // Static | Inferred | Runtime | Predicted
    pub metadata: ResolutionMetadata,
    pub active_facet: Option<FacetKind>,
    pub available_facets: Vec<FacetKind>,
}
```

#### Union Types
```rust
// Создание Union типа
let union = UnionTypeManager::create_union(vec![string_type, number_type]);

// Проверка совместимости
let compatible = UnionTypeManager::is_compatible_with_union(&test_type, &union_types);

// Получение наиболее вероятного типа
let most_likely = UnionTypeManager::get_most_likely_type(&union_types);
```

### Flow-Sensitive Analysis
```rust
// Создание анализатора
let mut analyzer = FlowSensitiveAnalyzer::new(context);

// Анализ присваивания
analyzer.analyze_assignment(&target, &value);

// Анализ условий
analyzer.analyze_conditional(&condition, &then_branch, &else_branch);

// Получение финального состояния
let final_state = analyzer.get_final_state();
```

### Interprocedural Analysis
```rust
// Построение графа вызовов
let call_graph = CallGraph::build_from_program(&program);

// Создание анализатора
let mut analyzer = InterproceduralAnalyzer::new(call_graph, context);

// Анализ всех функций
analyzer.analyze_all_functions();

// Получение результатов
let function_results = analyzer.get_analyzed_functions();
```

### Performance Profiling
```rust
// Включение профилирования
let mut profiler = PerformanceProfiler::new();
profiler.enable();

// Измерение операции
let result = profiler.measure("parsing", || {
    parser.parse(source_code)
});

// Генерация отчета
let report = profiler.generate_report();
println!("{}", report.format_human_readable());
```

### Analysis Caching
```rust
// Создание cache manager
let mut cache_manager = AnalysisCacheManager::new("./cache", "1.0.0")?;

// Получение из кеша
let cache_key = CacheKey::from_content(&file_content, "1.0.0");
if let Some(cached) = cache_manager.get(&cache_key) {
    // Используем кешированные результаты
}

// Сохранение в кеш
cache_manager.put(cache_key, analysis_results)?;
```

## 🔧 Configuration

### VSCode Extension Settings
```json
{
  "bsl.typeHints.showVariableTypes": true,
  "bsl.typeHints.showReturnTypes": true,
  "bsl.typeHints.showUnionDetails": true,
  "bsl.typeHints.minCertainty": 0.7,
  "bsl.performance.enableProfiling": false,
  "bsl.analysis.useParallelProcessing": true,
  "bsl.analysis.enableCaching": true,
  "bsl.analysis.cacheDirectory": ""
}
```

### LSP Server Configuration
```json
{
  "initializationOptions": {
    "enableFlowSensitiveAnalysis": true,
    "enableUnionTypes": true,
    "enableInterproceduralAnalysis": true,
    "enableTypeHints": true,
    "cacheDirectory": "./.bsl_cache",
    "performanceProfiling": {
      "enableProfiling": false,
      "maxMemoryUsageMB": 512
    }
  }
}
```

## 📚 Examples

### Basic Usage
```rust
use bsl_gradual_types::core::type_checker::TypeChecker;
use bsl_gradual_types::parser::common::ParserFactory;

// Парсинг и анализ
let mut parser = ParserFactory::create();
let program = parser.parse(bsl_code)?;

let type_checker = TypeChecker::new("module.bsl".to_string());
let (context, diagnostics) = type_checker.check(&program);

println!("Functions: {}", context.functions.len());
println!("Variables: {}", context.variables.len());
```

### Performance Profiling
```rust
use bsl_gradual_types::core::performance::BenchmarkSuite;

// Запуск полного набора бенчмарков
let report = BenchmarkSuite::run_full_benchmark_suite();
println!("{}", report.format_human_readable());
```

### Web Server Integration
```bash
# Запуск web сервера с проектом
cargo run --bin bsl-web-server --project /path/to/1c/project --port 8080

# API запросы
curl "http://localhost:8080/api/types?search=Массив"
curl -X POST "http://localhost:8080/api/analyze" \
     -H "Content-Type: application/json" \
     -d '{"code": "переменная = Новый Массив;"}'
```

---

## 📞 Support

- 🐛 [Issues](https://github.com/yourusername/bsl-gradual-types/issues)
- 📖 [Documentation](https://github.com/yourusername/bsl-gradual-types/tree/master/docs)
- 💬 [Discussions](https://github.com/yourusername/bsl-gradual-types/discussions)