# Эволюционная архитектура системы типов BSL
## От MVP к полноценному type-safe анализу

## 🎯 Принципы проектирования

1. **Start Simple** - начинаем с минимально работающего решения
2. **Evolution Ready** - заранее закладываем точки расширения
3. **No Breaking Changes** - каждая версия обратно совместима
4. **Graceful Degradation** - система работает даже с неполными данными

## 📊 Архитектура: Type Resolution Pipeline

```rust
// Центральная абстракция - не тип, а "резолюция типа"
pub struct TypeResolution {
    // Уровень уверенности в разрешении
    certainty: Certainty,
    
    // Результат разрешения
    result: ResolutionResult,
    
    // Источник информации
    source: ResolutionSource,
    
    // Метаданные для отладки
    metadata: ResolutionMetadata,
}

pub enum Certainty {
    Known,           // 100% точно известен
    Inferred(f32),   // Выведен с уверенностью 0.0-1.0
    Unknown,         // Невозможно определить
}

pub enum ResolutionResult {
    // MVP: Простые случаи
    Concrete(ConcreteType),           // Точный тип
    
    // v1.1: Множественные варианты
    Union(Vec<WeightedType>),         // Один из нескольких
    
    // v1.2: Условные типы
    Conditional(Box<ConditionalType>), // Зависит от условий
    
    // v2.0: Эффекты и контексты
    Contextual(Box<ContextualType>),   // С эффектами
    
    // Fallback
    Dynamic,                           // Определяется в runtime
}
```

## 🚀 Phase 1: MVP (2-3 недели)

### Минимальный функционал

```rust
// Упрощённая система для базового автодополнения
pub struct MvpTypeSystem {
    // Только статические типы из платформы
    platform_types: HashMap<String, PlatformType>,
    
    // Простые типы из конфигурации
    config_types: HashMap<String, ConfigType>,
    
    // Базовое разрешение
    resolver: BasicResolver,
}

pub struct PlatformType {
    name: String,
    methods: Vec<Method>,
    properties: Vec<Property>,
    // Фасеты пока игнорируем - всё в одном типе
}

pub struct ConfigType {
    kind: MetadataKind,  // Справочник, Документ и т.д.
    name: String,
    attributes: Vec<Attribute>,
    // Наследование пока игнорируем
}

impl BasicResolver {
    // MVP: Только прямое разрешение
    pub fn resolve(&self, expression: &str) -> TypeResolution {
        match self.parse_expression(expression) {
            // Справочники.Товары
            Expression::GlobalProperty(prop, member) => {
                if prop == "Справочники" {
                    if let Some(config_type) = self.config_types.get(member) {
                        return TypeResolution {
                            certainty: Certainty::Known,
                            result: ResolutionResult::Concrete(
                                ConcreteType::ConfigObject(config_type.clone())
                            ),
                            source: ResolutionSource::Static,
                            metadata: Default::default(),
                        };
                    }
                }
                TypeResolution::unknown()
            },
            
            // Новый Массив()
            Expression::Constructor(type_name, _args) => {
                if let Some(platform_type) = self.platform_types.get(type_name) {
                    return TypeResolution {
                        certainty: Certainty::Known,
                        result: ResolutionResult::Concrete(
                            ConcreteType::PlatformObject(platform_type.clone())
                        ),
                        source: ResolutionSource::Static,
                        metadata: Default::default(),
                    };
                }
                TypeResolution::unknown()
            },
            
            _ => TypeResolution::unknown()
        }
    }
}
```

### Что работает в MVP:
- ✅ Базовое автодополнение для `Справочники.`, `Документы.`
- ✅ Методы платформенных типов (`Массив`, `Соответствие`)
- ✅ Свойства объектов конфигурации
- ✅ Глобальные функции (`СтрНайти`, `Сообщить`)

### Что НЕ работает в MVP:
- ❌ Переходы между фасетами (manager → object → reference)
- ❌ Вывод типов из контекста
- ❌ Цепочки вызовов
- ❌ Динамические типы

## 📈 Phase 2: Фасеты и контекст (+ 2-3 недели)

### Добавляем слой фасетов поверх MVP

```rust
// Расширяем существующую систему, не ломая её
pub struct EnhancedTypeSystem {
    // Базовая система остаётся
    base: MvpTypeSystem,
    
    // Добавляем фасеты
    facet_registry: FacetRegistry,
    
    // Контекстное разрешение
    context_resolver: ContextResolver,
}

// Фасеты как декораторы над базовыми типами
pub struct FacetRegistry {
    templates: HashMap<MetadataKind, FacetTemplates>,
}

pub struct FacetTemplates {
    manager: FacetTemplate,
    object: FacetTemplate,
    reference: FacetTemplate,
    metadata: FacetTemplate,
}

// Адаптер для обратной совместимости
impl TypeResolver for EnhancedTypeSystem {
    fn resolve(&self, expr: &str, context: Option<&Context>) -> TypeResolution {
        // Сначала пробуем базовое разрешение
        let base_resolution = self.base.resolve(expr);
        
        // Если есть контекст, уточняем фасет
        if let Some(ctx) = context {
            self.refine_with_context(base_resolution, ctx)
        } else {
            base_resolution
        }
    }
    
    fn refine_with_context(&self, resolution: TypeResolution, ctx: &Context) -> TypeResolution {
        match resolution.result {
            ResolutionResult::Concrete(ConcreteType::ConfigObject(obj)) => {
                // Определяем активный фасет
                let facet = self.context_resolver.determine_facet(&obj, ctx);
                
                // Обогащаем тип методами фасета
                let enriched = self.facet_registry.apply_facet(obj, facet);
                
                TypeResolution {
                    result: ResolutionResult::Concrete(enriched),
                    ..resolution
                }
            },
            _ => resolution
        }
    }
}
```

### Новые возможности v2:
- ✅ Правильные методы для `Справочники.Товары.СоздатьЭлемент()`
- ✅ Переходы между фасетами
- ✅ Контекстное автодополнение

## 🎨 Phase 3: Вывод типов (+ 3-4 недели)

### Добавляем constraint solver

```rust
pub struct InferenceTypeSystem {
    // Предыдущие слои
    enhanced: EnhancedTypeSystem,
    
    // Новый слой вывода
    inference_engine: InferenceEngine,
}

pub struct InferenceEngine {
    constraints: ConstraintCollector,
    solver: ConstraintSolver,
    cache: InferenceCache,
}

// Плагин для вывода типов
impl TypeResolver for InferenceTypeSystem {
    fn resolve(&self, expr: &str, context: Option<&Context>) -> TypeResolution {
        // Пробуем кеш
        if let Some(cached) = self.inference_engine.cache.get(expr, context) {
            return cached;
        }
        
        // Пробуем базовое/контекстное разрешение
        let base = self.enhanced.resolve(expr, context);
        
        // Если не удалось - пробуем вывести
        let resolution = match base.certainty {
            Certainty::Unknown => self.try_infer(expr, context),
            _ => base
        };
        
        // Кешируем результат
        self.inference_engine.cache.put(expr, context, &resolution);
        resolution
    }
    
    fn try_infer(&self, expr: &str, context: Option<&Context>) -> TypeResolution {
        // Собираем ограничения из окружающего кода
        let constraints = self.inference_engine.constraints.collect(expr, context);
        
        // Решаем систему ограничений
        match self.inference_engine.solver.solve(constraints) {
            Ok(solution) => TypeResolution {
                certainty: Certainty::Inferred(solution.confidence),
                result: ResolutionResult::Concrete(solution.type_),
                source: ResolutionSource::Inferred,
                metadata: solution.metadata,
            },
            Err(_) => TypeResolution::unknown()
        }
    }
}
```

### Новые возможности v3:
- ✅ Вывод типов переменных из присваиваний
- ✅ Определение типов параметров функций
- ✅ Поддержка цепочек вызовов

## 🚦 Phase 4: Runtime контракты (+ 2-3 недели)

### Опциональный слой для критичного кода

```rust
pub struct ContractTypeSystem {
    inference: InferenceTypeSystem,
    contract_generator: ContractGenerator,
    config: ContractConfig,
}

pub struct ContractConfig {
    enabled: bool,
    threshold: f32,  // Минимальная уверенность для проверки
    mode: ContractMode,
}

pub enum ContractMode {
    Warning,      // Только предупреждения
    Assert,       // Добавлять assert в код
    Report,       // Логировать нарушения
}

impl ContractGenerator {
    fn generate_contract(&self, resolution: &TypeResolution) -> Option<Contract> {
        match resolution.certainty {
            Certainty::Inferred(conf) if conf < self.config.threshold => {
                Some(Contract::RuntimeCheck {
                    expected: resolution.result.clone(),
                    check_code: self.generate_check_code(&resolution.result),
                })
            },
            _ => None
        }
    }
}
```

## 🎯 Phase 5: Полная система (+ 4-6 недель)

### Финальная интеграция всех подходов

```rust
pub struct FullTypeSystem {
    // Все предыдущие слои
    contract_system: ContractTypeSystem,
    
    // Дополнительные анализаторы
    flow_analyzer: Option<AbstractInterpreter>,
    ml_predictor: Option<MLTypePredictor>,
    
    // Единый граф типов
    type_graph: TypeDependencyGraph,
}

// Единая точка входа с полным функционалом
impl FullTypeSystem {
    pub fn analyze(&mut self, program: &Program) -> AnalysisResult {
        // Phase 1: Быстрый статический анализ
        let static_types = self.quick_static_pass(program);
        
        // Phase 2: Контекстное уточнение
        let contextual = self.refine_with_context(static_types);
        
        // Phase 3: Вывод неизвестных
        let inferred = self.infer_unknown(contextual);
        
        // Phase 4: Опциональные проверки
        let with_contracts = self.add_contracts(inferred);
        
        // Phase 5: Дополнительные анализы (если включены)
        let final_types = self.run_optional_analyses(with_contracts);
        
        self.generate_report(final_types)
    }
}
```

## 🔌 Точки расширения

### 1. Плагины для новых источников типов

```rust
pub trait TypeSource {
    fn can_resolve(&self, expr: &Expression) -> bool;
    fn resolve(&self, expr: &Expression) -> Option<TypeResolution>;
    fn priority(&self) -> i32;
}

// Легко добавлять новые источники
impl TypeSystemBuilder {
    pub fn add_source(mut self, source: Box<dyn TypeSource>) -> Self {
        self.sources.push(source);
        self.sources.sort_by_key(|s| -s.priority());
        self
    }
}
```

### 2. Стратегии разрешения

```rust
pub trait ResolutionStrategy {
    fn resolve(&self, expr: &Expression, sources: &[Box<dyn TypeSource>]) -> TypeResolution;
}

pub struct FirstMatch;  // MVP - первое совпадение
pub struct BestMatch;   // v2 - лучшее по уверенности
pub struct Consensus;   // v3 - консенсус источников
pub struct MLAssisted;  // v4 - с ML предсказаниями
```

### 3. Кастомные анализаторы

```rust
pub trait TypeAnalyzer {
    fn analyze(&self, resolution: TypeResolution) -> TypeResolution;
    fn name(&self) -> &str;
}

// Подключаемые анализаторы
registry.add_analyzer(Box::new(NullabilityAnalyzer));
registry.add_analyzer(Box::new(MutabilityAnalyzer));
registry.add_analyzer(Box::new(ContextAnalyzer));
```

## 📋 План реализации

### Sprint 1 (недели 1-2): MVP
- [ ] Базовые структуры данных
- [ ] Парсинг платформенных типов
- [ ] Парсинг Configuration.xml
- [ ] Простое разрешение типов
- [ ] Базовое автодополнение

### Sprint 2 (недели 3-4): Фасеты
- [ ] FacetRegistry
- [ ] ContextResolver
- [ ] Переходы между фасетами
- [ ] Контекстное автодополнение

### Sprint 3 (недели 5-7): Вывод типов
- [ ] ConstraintCollector
- [ ] ConstraintSolver
- [ ] InferenceCache
- [ ] Цепочки вызовов

### Sprint 4 (недели 8-9): Контракты
- [ ] ContractGenerator
- [ ] Runtime проверки
- [ ] Конфигурация уровня строгости

### Sprint 5 (недели 10-13): Полная система
- [ ] AbstractInterpreter (опционально)
- [ ] ML предсказания (опционально)
- [ ] TypeDependencyGraph
- [ ] Отчёты и диагностика

## 🎯 Ключевые преимущества подхода

1. **Инкрементальная разработка** - каждая фаза даёт работающий функционал
2. **Обратная совместимость** - новые слои не ломают старые
3. **Модульность** - компоненты слабо связаны
4. **Расширяемость** - легко добавлять новые анализаторы
5. **Практичность** - MVP работает уже через 2 недели

## 🔧 Технические детали реализации

### Хранение данных

```rust
// Вместо монолитного индекса - слоистая архитектура
pub struct LayeredTypeStorage {
    // Слой 1: Статические данные (кешируется на диске)
    static_layer: StaticTypeLayer,
    
    // Слой 2: Выведенные типы (в памяти, пересчитывается)
    inference_layer: InferenceLayer,
    
    // Слой 3: Runtime информация (опционально)
    runtime_layer: Option<RuntimeLayer>,
}

impl LayeredTypeStorage {
    // Прозрачный доступ через слои
    pub fn get_type(&self, id: &TypeId) -> Option<TypeResolution> {
        self.static_layer.get(id)
            .or_else(|| self.inference_layer.get(id))
            .or_else(|| self.runtime_layer.as_ref()?.get(id))
    }
}
```

### API для расширений

```rust
// Публичный API остаётся стабильным
pub trait BslTypeSystem {
    // v1.0 - базовое разрешение
    fn resolve_type(&self, expr: &str) -> TypeResolution;
    
    // v2.0 - с контекстом
    fn resolve_with_context(&self, expr: &str, ctx: &Context) -> TypeResolution;
    
    // v3.0 - батчевое разрешение
    fn resolve_batch(&self, exprs: &[String]) -> Vec<TypeResolution>;
    
    // Автодополнение
    fn get_completions(&self, position: &Position) -> Vec<Completion>;
    
    // Диагностика
    fn check_types(&self, ast: &AST) -> Vec<Diagnostic>;
}
```

Это позволяет внутренне переписывать реализацию, сохраняя API.