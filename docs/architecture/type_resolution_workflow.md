# Workflow вычисления типов: Контрольные точки

## Общая диаграмма с контрольными точками

```mermaid
flowchart TB
    subgraph "ENTRY POINTS (Точки входа)"
        LSP_Hover["LSP Hover Request<br/>hover.rs:16"]
        LSP_Diag["LSP Diagnostics<br/>text_document.rs:17"]
        Web_Hover["/api/hover/enhanced<br/>handlers.rs:298"]
        Web_Diag["/api/diagnostics<br/>handlers.rs:186"]
        Web_Semantic["/api/semantic-tree<br/>handlers.rs:376"]
    end

    subgraph "APPLICATION LAYER"
        TSS["TypeSystemService<br/>service.rs:70"]
        HoverSvc["hover_service.rs<br/>get_hover_info()"]
        ValidSvc["validation_service.rs<br/>validate_semantics()"]
    end

    subgraph "CHECKPOINT 1: IR Cache"
        IRCache{{"IR Cache<br/>hash(content) → Arc<SemanticProgram><br/>~80-90% hit rate"}}
    end

    subgraph "CHECKPOINT 2: Parsing"
        Parser["ParserCoordinator<br/>parse_to_ir()"]
        TreeSitter["tree-sitter BSL<br/>→ AST"]
    end

    subgraph "CHECKPOINT 3: AST → IR Conversion"
        AstToIr["AstToIrConverter<br/>convert_with_resolver()"]
        Pass1["Проход 1:<br/>collect_global_symbols()<br/>→ SymbolTable.global_functions"]
        Pass2["Проход 2:<br/>convert_statement()<br/>→ SemanticNode + type_inference"]
    end

    subgraph "CHECKPOINT 4: Type Inference"
        TypeInf["type_inference.rs<br/>infer_type_resolution()"]
        InfExpr["Литералы: Число, Строка, Булево"]
        InfIdent["Identifier → SymbolTable lookup"]
        InfProp["PropertyAccess → resolve_member_type()"]
        InfCall["FunctionCall → SignatureIndex"]
    end

    subgraph "IR (SemanticProgram)"
        SP["SemanticProgram"]
        Nodes["nodes: Vec<SemanticNode>"]
        Symbols["symbols: SymbolTable"]
        CFG["cfg: ControlFlowGraph"]
    end

    subgraph "CHECKPOINT 5: AnalysisEngine"
        Engine["AnalysisEngine<br/>analyze_ir()"]
        FlowCtx["FlowContext<br/>variable_states: HashMap"]
        Visitor["IrTypeResolverVisitor<br/>visit_node_indexed()"]
    end

    subgraph "CHECKPOINT 6: TypeResolver"
        TR["TypeResolver<br/>resolve_expression_sync()"]

        subgraph "Resolution Strategy (8 шагов)"
            Step1["1. Direct lookup<br/>repository.find_type()"]
            Step2["2. Member access<br/>parse_member_access()"]
            Step3["3. Union types<br/>expression.contains('|')"]
            Step4["4. Intersection<br/>expression.contains('&')"]
            Step5["5. Generic types<br/>contains('<') && contains('>')"]
            Step6["6. Nullable<br/>ends_with('?')"]
            Step7["7. Primitives<br/>try_resolve_primitive()"]
            Step8["8. Fallback<br/>TypeResolution::unknown()"]
        end
    end

    subgraph "CHECKPOINT 7: TypeMetadataLookup"
        TML["TypeMetadataLookup<br/>core.rs"]
        GetMethods["get_methods()<br/>4-уровневый приоритет"]
        GetProps["get_properties()<br/>с учётом active_facet"]
        FacetLogic["FacetKind.shows_properties()<br/>Manager=false, Object/Ref=true"]
    end

    subgraph "CHECKPOINT 8: Data Layer"
        Repo["TypeRepository<br/>3927 типов"]
        TypeId["TypeId<br/>normalized + display"]
        RawData["RawTypeData<br/>methods, properties, facets"]
        SigIndex["SignatureIndex<br/>platform_methods + config_methods"]
    end

    subgraph "OUTPUT"
        TypeRes["TypeResolution<br/>certainty, result, active_facet"]
        HoverInfo["HoverInfo<br/>type_name, description, methods"]
        Diagnostics["Vec<Diagnostic><br/>errors, warnings"]
    end

    %% Entry → Application
    LSP_Hover --> TSS
    LSP_Diag --> TSS
    Web_Hover --> TSS
    Web_Diag --> TSS
    Web_Semantic --> TSS

    TSS --> HoverSvc
    TSS --> ValidSvc

    %% Application → IR Cache
    HoverSvc --> IRCache
    ValidSvc --> IRCache

    %% Cache hit/miss
    IRCache -->|"hit"| SP
    IRCache -->|"miss"| Parser

    %% Parsing flow
    Parser --> TreeSitter
    TreeSitter --> AstToIr

    %% AST → IR
    AstToIr --> Pass1
    Pass1 --> Pass2
    Pass2 --> TypeInf

    %% Type Inference branches
    TypeInf --> InfExpr
    TypeInf --> InfIdent
    TypeInf --> InfProp
    TypeInf --> InfCall

    InfIdent --> Symbols
    InfProp --> TR
    InfCall --> SigIndex

    %% IR construction
    Pass2 --> Nodes
    Pass2 --> Symbols
    Nodes --> SP
    Symbols --> SP
    CFG --> SP

    %% Analysis flow
    SP --> Engine
    Engine --> FlowCtx
    Engine --> Visitor
    Visitor --> TR

    %% TypeResolver strategy
    TR --> Step1
    Step1 -->|"not found"| Step2
    Step2 -->|"not found"| Step3
    Step3 -->|"not union"| Step4
    Step4 -->|"not intersection"| Step5
    Step5 -->|"not generic"| Step6
    Step6 -->|"not nullable"| Step7
    Step7 -->|"not primitive"| Step8

    Step1 --> Repo
    Step2 --> TML

    %% TypeMetadataLookup
    TML --> GetMethods
    TML --> GetProps
    GetMethods --> FacetLogic
    GetProps --> FacetLogic

    %% Data Layer
    GetMethods --> SigIndex
    GetProps --> RawData
    Repo --> TypeId
    TypeId --> RawData

    %% Outputs
    TR --> TypeRes
    TypeRes --> HoverInfo
    TypeRes --> Diagnostics
```

## Детальные контрольные точки

### CHECKPOINT 1: Entry Points

| Entry Point | Файл:строка | Первый контакт с типами |
|-------------|-------------|------------------------|
| LSP Hover | `hover.rs:16` | `resolver.resolve_variable_with_context()` |
| LSP Diagnostics | `text_document.rs:17` | `AstToIrConverter::convert_with_resolver()` |
| /api/hover/enhanced | `handlers.rs:298` | Делегирует в hover_service |
| /api/diagnostics | `handlers.rs:186` | Делегирует в validation_service |
| /api/semantic-tree | `handlers.rs:376` | `parser.parse_to_ir()` |

### CHECKPOINT 2: IR Cache (Milestone 2.13)

```
hash(file_content) → Option<Arc<SemanticProgram>>
```

- **Hit rate:** ~80-90%
- **Hover с cache hit:** ~3-5ms
- **Hover с cache miss:** ~50-105ms

### CHECKPOINT 3: AST → IR Conversion

**Двухпроходная конвертация:**

```
Проход 1: collect_global_symbols()
├─ Регистрирует функции/процедуры в SymbolTable
└─ Создаёт FunctionSignature с типами параметров

Проход 2: convert_statement()
├─ Строит scope hierarchy (Global → Function → Block)
├─ Вызывает type_inference для каждого выражения
└─ Обновляет SymbolTable переменными с их TypeResolution
```

### CHECKPOINT 4: Type Inference

**infer_type_resolution() в type_inference.rs:**

| Выражение | Логика | Результат |
|-----------|--------|-----------|
| `123` | Литерал | `TypeResolution::primitive("Число")` |
| `"текст"` | Литерал | `TypeResolution::primitive("Строка")` |
| `Истина` | Литерал | `TypeResolution::primitive("Булево")` |
| `x` (identifier) | SymbolTable lookup | `symbol_table.get_variable_type(scope, "x")` |
| `obj.prop` | resolve_member_type() | TypeMetadataLookup → SignatureIndex |
| `Func()` | SignatureIndex | `signature_index.find_method()` |

### CHECKPOINT 5: TypeResolver Strategy

**8-шаговая стратегия resolve_expression_sync():**

```mermaid
flowchart LR
    Input["Входное выражение"]

    Input --> C1{"1. Direct lookup<br/>repository.find_type()"}
    C1 -->|найден| Return1["TypeResolution"]
    C1 -->|не найден| C2{"2. Member access<br/>Base.Member?"}

    C2 -->|да| MemberRes["MemberResolver<br/>+ Three-level Certainty"]
    C2 -->|нет| C3{"3. Union?<br/>contains('|')"}

    C3 -->|да| Union["UnionStrategy"]
    C3 -->|нет| C4{"4. Intersection?<br/>contains('&')"}

    C4 -->|да| Inter["IntersectionStrategy"]
    C4 -->|нет| C5{"5. Generic?<br/>'<' && '>'"}

    C5 -->|да| Generic["GenericStrategy"]
    C5 -->|нет| C6{"6. Nullable?<br/>ends_with('?')"}

    C6 -->|да| Nullable["NullableStrategy"]
    C6 -->|нет| C7{"7. Primitive?"}

    C7 -->|да| Prim["TypeResolution::known()"]
    C7 -->|нет| C8["8. TypeResolution::unknown()"]
```

### CHECKPOINT 6: Member Access Resolution

**Three-level Certainty для конфигурационных типов:**

| has_metadata | config_loaded | Certainty | Смысл |
|--------------|---------------|-----------|-------|
| true | any | `Known` | Тип найден в метаданных |
| false | false | `Inferred(0.5)` | Graceful degradation |
| false | true | `Unknown` | Ошибка: объект не найден |

### CHECKPOINT 7: Фасетная система

**FacetKind влияет на доступные свойства:**

```
Manager:   shows_properties() = false  → только методы (Create, Find)
Object:    shows_properties() = true   → свойства + методы (Write, Delete)
Reference: shows_properties() = true   → свойства readonly + методы (GetObject)
Selection: shows_properties() = false  → методы итерации
List:      shows_properties() = false  → методы списка
```

### CHECKPOINT 8: Data Layer

**TypeId нормализация:**
```
"ТаблицаЗначений" → TypeId { normalized: "таблицазначений", display: "ТаблицаЗначений" }
"TableOfValues"   → TypeId { normalized: "tableofvalues", display: "TableOfValues" }
```

**O(1) lookup:** `type_index[TypeId] → usize → types[usize] → RawTypeData`

## Структуры данных

### TypeResolution (результат резолвинга)

```rust
TypeResolution {
    certainty: Certainty,           // Known | Inferred(f32) | Unknown
    result: ResolutionResult,       // Concrete | Union | Generic | Unknown
    source: ResolutionSource,       // Explicit | Inferred | Platform
    active_facet: Option<FacetKind>,// Manager | Object | Reference
    available_facets: Vec<FacetKind>,
}
```

### SemanticProgram (IR)

```rust
SemanticProgram {
    symbols: SymbolTable,          // Переменные и функции
    nodes: Vec<SemanticNode>,      // Все узлы программы
    source_info: SourceInfo,       // path, hash
    cfg: Option<ControlFlowGraph>, // Для flow-sensitive
}
```

### RawTypeData (хранилище)

```rust
RawTypeData {
    name: String,                  // "ТаблицаЗначений"
    methods: Vec<RawMethodData>,   // Методы типа
    properties: Vec<RawPropertyData>,
    facets: Vec<FacetKind>,        // [Manager, Object, Reference]
    kind: Option<MetadataKind>,    // Catalog, Document, etc.
    tabular_sections: Vec<...>,    // Табличные части
}
```

## Временные метрики

| Операция | Время | Примечание |
|----------|-------|-----------|
| IR Cache hit | <1 ms | Hash lookup |
| parse_to_ir() | 40-60 ms | tree-sitter + конвертация |
| analyze_ir() | 60-100 ms | Резолвинг всех узлов |
| resolve_expression_sync() | 0.1-0.5 ms | Один тип |
| find_type() | <0.001 ms | O(1) HashMap |
| **Hover total** | **3-105 ms** | Зависит от cache |
| **Diagnostics total** | **60-160 ms** | Всегда полный анализ |

## Оценка архитектуры

### Плюсы (НЕ костыль)

1. **Чёткое разделение слоёв:** Presentation → Application → Domain → Data
2. **IR независимость от парсера:** SemanticProgram не знает о tree-sitter
3. **O(1) lookup:** TypeId + HashMap обеспечивает константное время
4. **Фасетная система:** Научно обоснована (Balyuk & Popova 2021)
5. **Gradual typing:** TypeResolution всегда возвращается (не exceptions)
6. **Cache стратегия:** IR Cache даёт 80-90% hit rate

### Потенциальные улучшения

1. **TypeResolver 8 шагов:** Можно оптимизировать через pattern matching вместо последовательных проверок
2. **Member resolution:** Three-level certainty добавляет сложность, но необходим для graceful degradation
3. **SignatureIndex merge:** Двойной источник (platform + config) требует merge логики

### Вывод

**Архитектура НЕ является костылём.** Это продуманная система с:
- Научным обоснованием (фасеты)
- Чётким разделением ответственности
- Оптимизациями производительности (cache, O(1) lookup)
- Graceful degradation (Certainty levels)
