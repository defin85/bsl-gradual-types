# Альтернативные подходы к type-safe анализу BSL

## 🎯 Цель: Полноценный type-safe анализ динамического языка 1С

## Подход 1: Effect System + Algebraic Data Types

### Концепция
Вместо попыток статически определить тип, отслеживаем "эффекты" которые могут произойти с типом.

```rust
// Тип как множество возможных состояний
pub enum BslType {
    Known(ConcreteType),
    Union(Vec<BslType>),                    // Один из нескольких
    Intersection(Vec<BslType>),             // Все одновременно  
    Conditional(Condition, Box<BslType>),   // Зависит от условия
    Effect(Box<BslType>, Vec<TypeEffect>),  // Тип + эффекты
    Unknown,
}

pub enum TypeEffect {
    MayBeNull,
    RequiresTransaction,
    RequiresLock(String),
    ModifiedByExtension(ExtensionId),
    AvailableInContext(ExecutionContext),
    MayThrow(ErrorType),
}

// Пример использования
impl TypeAnalyzer {
    fn analyze_expression(&self, expr: &Expression) -> BslType {
        match expr {
            // Справочники.Товары.НайтиПоКоду("123")
            Expression::MethodCall(obj, "НайтиПоКоду", args) => {
                BslType::Effect(
                    Box::new(BslType::Union(vec![
                        BslType::Known(ConcreteType::Reference("Товары")),
                        BslType::Known(ConcreteType::Null),
                    ])),
                    vec![
                        TypeEffect::MayBeNull,
                        TypeEffect::MayThrow(ErrorType::DatabaseError),
                    ]
                )
            },
            // ...
        }
    }
}
```

### Преимущества
- ✅ Честно представляет неопределённость
- ✅ Отслеживает контекстные зависимости
- ✅ Поддерживает композицию эффектов

### Недостатки
- ❌ Сложность экспоненциально растёт
- ❌ Трудно визуализировать для пользователя

---

## Подход 2: Gradual Typing с Runtime Contracts

### Концепция
Комбинация статического анализа где возможно и runtime проверок где необходимо.

```rust
pub struct GradualType {
    static_type: Option<StaticType>,      // Что знаем статически
    runtime_contract: Contract,           // Что проверяем в runtime
    confidence: f32,                      // Уверенность 0.0 - 1.0
}

pub enum Contract {
    TypeCheck(String),                    // ТипЗнч(х) = Тип("...")
    PropertyExists(String),               // х.Свойство <> Неопределено
    MethodCallable(String, Vec<Contract>), // Проверка сигнатуры
    Custom(String),                        // Произвольный BSL код
}

impl GradualTypeChecker {
    fn check_assignment(&self, target: &Variable, value: &Expression) -> CheckResult {
        let value_type = self.infer_type(value);
        
        match value_type.confidence {
            c if c > 0.9 => {
                // Высокая уверенность - статическая проверка
                self.static_check(target, &value_type.static_type)
            },
            c if c > 0.5 => {
                // Средняя уверенность - добавляем runtime assert
                self.inject_runtime_check(target, value_type.runtime_contract)
            },
            _ => {
                // Низкая уверенность - предупреждение
                CheckResult::Warning("Type cannot be verified statically")
            }
        }
    }
    
    fn inject_runtime_check(&self, var: &Variable, contract: Contract) -> CheckResult {
        // Генерируем BSL код проверки
        let check_code = match contract {
            Contract::TypeCheck(type_name) => 
                format!("Если ТипЗнч({}) <> Тип(\"{}\") Тогда 
                         ВызватьИсключение \"Type mismatch\";
                     КонецЕсли;", var.name, type_name),
            // ...
        };
        CheckResult::InjectCode(check_code)
    }
}
```

### Преимущества
- ✅ Практичный баланс между безопасностью и гибкостью
- ✅ Можно внедрять постепенно
- ✅ Runtime проверки ловят реальные ошибки

### Недостатки
- ❌ Изменяет исходный код
- ❌ Overhead в runtime

---

## Подход 3: Constraint-Based Type Inference

### Концепция
Собираем ограничения (constraints) на типы и решаем их как систему уравнений.

```rust
pub struct ConstraintSystem {
    variables: HashMap<TypeVar, TypeConstraints>,
    constraints: Vec<Constraint>,
}

pub enum Constraint {
    Equal(TypeVar, TypeVar),
    SubtypeOf(TypeVar, ConcreteType),
    HasMethod(TypeVar, String, Signature),
    HasProperty(TypeVar, String, TypeVar),
    OneOf(TypeVar, Vec<ConcreteType>),
    DependsOn(TypeVar, TypeVar, Relation),
}

impl ConstraintSolver {
    fn analyze_program(&mut self, ast: &AST) {
        // Фаза 1: Собираем ограничения
        self.collect_constraints(ast);
        
        // Фаза 2: Решаем систему
        self.solve();
        
        // Фаза 3: Проверяем неразрешимые
        self.report_conflicts();
    }
    
    fn collect_from_assignment(&mut self, var: &str, expr: &Expression) {
        let var_type = self.get_or_create_typevar(var);
        let expr_type = self.get_or_create_typevar_for_expr(expr);
        
        self.constraints.push(Constraint::Equal(var_type, expr_type));
    }
    
    fn collect_from_method_call(&mut self, obj: &str, method: &str, args: &[Expression]) {
        let obj_type = self.get_or_create_typevar(obj);
        
        // Объект должен иметь метод
        self.constraints.push(Constraint::HasMethod(
            obj_type,
            method.to_string(),
            self.infer_signature(method, args)
        ));
    }
    
    fn solve(&mut self) -> Solution {
        // Алгоритм Hindley-Milner или его вариация
        loop {
            let changed = self.propagate_constraints();
            if !changed { break; }
        }
        
        self.extract_solution()
    }
}
```

### Преимущества
- ✅ Мощный вывод типов
- ✅ Находит неочевидные несоответствия
- ✅ Математически обоснован

### Недостатки
- ❌ Сложные сообщения об ошибках
- ❌ Может не сойтись для динамического кода

---

## Подход 4: Abstract Interpretation

### Концепция
Выполняем "абстрактную" интерпретацию программы, отслеживая возможные типы.

```rust
pub struct AbstractInterpreter {
    abstract_heap: HashMap<VarId, AbstractValue>,
    control_flow: ControlFlowGraph,
    worklist: VecDeque<BasicBlock>,
}

pub enum AbstractValue {
    Type(AbstractType),
    Set(HashSet<AbstractType>),      // Множество возможных типов
    Range(AbstractType, AbstractType), // Диапазон типов
    Top,                              // Любой тип
    Bottom,                           // Невозможное состояние
}

impl AbstractInterpreter {
    fn interpret(&mut self, program: &Program) -> TypeMap {
        // Строим граф потока управления
        self.control_flow = ControlFlowGraph::build(program);
        
        // Инициализируем worklist точками входа
        for entry in self.control_flow.entries() {
            self.worklist.push_back(entry);
        }
        
        // Fixed-point iteration
        while let Some(block) = self.worklist.pop_front() {
            let changed = self.interpret_block(block);
            
            if changed {
                // Добавляем последователей в worklist
                for successor in self.control_flow.successors(block) {
                    self.worklist.push_back(successor);
                }
            }
        }
        
        self.extract_types()
    }
    
    fn interpret_statement(&mut self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Assignment(var, expr) => {
                let old_type = self.abstract_heap.get(&var).cloned();
                let new_type = self.evaluate_abstract(expr);
                
                // Объединяем с предыдущим значением (widening)
                let merged = self.merge(old_type, new_type);
                self.abstract_heap.insert(var, merged);
                
                old_type != Some(merged)
            },
            Statement::If(cond, then_branch, else_branch) => {
                // Уточняем типы на основе условия
                let (then_env, else_env) = self.refine_by_condition(cond);
                // ...
            },
            // ...
        }
    }
}
```

### Преимущества
- ✅ Учитывает поток управления
- ✅ Может найти недостижимый код
- ✅ Хорошо работает с циклами

### Недостатки
- ❌ Консервативная over-approximation
- ❌ Требует построения CFG

---

## Подход 5: Probabilistic Type Inference

### Концепция
Используем вероятностную модель и машинное обучение для предсказания типов.

```rust
pub struct ProbabilisticTypeInference {
    model: NeuralTypeModel,
    context_embeddings: HashMap<String, Vector>,
    type_distributions: HashMap<VarId, TypeDistribution>,
}

pub struct TypeDistribution {
    types: Vec<(ConcreteType, f32)>, // Тип и вероятность
    entropy: f32,                    // Мера неопределённости
}

impl ProbabilisticTypeInference {
    fn infer_type(&mut self, var: &Variable, context: &Context) -> TypeDistribution {
        // Собираем признаки
        let features = self.extract_features(var, context);
        
        // Получаем embedding контекста
        let context_vec = self.encode_context(context);
        
        // Предсказываем распределение типов
        let predictions = self.model.predict(features, context_vec);
        
        // Корректируем на основе жёстких ограничений
        let refined = self.apply_hard_constraints(predictions, var);
        
        TypeDistribution {
            types: refined,
            entropy: self.calculate_entropy(&refined),
        }
    }
    
    fn suggest_type_annotation(&self, var: &Variable) -> Option<String> {
        let dist = &self.type_distributions[&var.id];
        
        // Предлагаем аннотацию если уверенность высока
        if dist.entropy < 0.1 {
            let (best_type, confidence) = &dist.types[0];
            if *confidence > 0.95 {
                return Some(format!("// @type {}", best_type));
            }
        }
        None
    }
}
```

### Преимущества
- ✅ Учится на реальном коде
- ✅ Может предсказывать намерения
- ✅ Хорошо работает с идиомами

### Недостатки
- ❌ Требует обучающих данных
- ❌ "Чёрный ящик"
- ❌ Может галлюцинировать

---

## Подход 6: Multi-Phase Hybrid System

### Концепция
Комбинируем несколько подходов в многофазную систему.

```rust
pub struct HybridTypeSystem {
    // Фаза 1: Быстрый статический анализ
    static_analyzer: StaticTypeAnalyzer,
    
    // Фаза 2: Constraint solving для сложных случаев
    constraint_solver: ConstraintSolver,
    
    // Фаза 3: Abstract interpretation для потока управления
    abstract_interpreter: AbstractInterpreter,
    
    // Фаза 4: ML для неразрешимых случаев
    ml_predictor: Option<ProbabilisticTypeInference>,
    
    // Результирующий граф типов
    type_graph: TypeDependencyGraph,
}

impl HybridTypeSystem {
    fn analyze(&mut self, program: &Program) -> AnalysisResult {
        // Фаза 1: Быстрый проход - очевидные типы
        let static_types = self.static_analyzer.analyze(program);
        self.type_graph.add_static(static_types);
        
        // Фаза 2: Constraints для неразрешённых
        let unresolved = self.type_graph.get_unresolved();
        if !unresolved.is_empty() {
            let constraints = self.constraint_solver.solve(unresolved);
            self.type_graph.add_constraints(constraints);
        }
        
        // Фаза 3: Control flow для условных типов  
        let conditional = self.type_graph.get_conditional();
        if !conditional.is_empty() {
            let flow_types = self.abstract_interpreter.analyze(conditional);
            self.type_graph.add_flow_analysis(flow_types);
        }
        
        // Фаза 4: ML для оставшихся (опционально)
        if let Some(ml) = &mut self.ml_predictor {
            let unknown = self.type_graph.get_unknown();
            let predictions = ml.predict_batch(unknown);
            self.type_graph.add_predictions(predictions);
        }
        
        // Генерируем отчёт
        self.generate_report()
    }
}

pub struct TypeDependencyGraph {
    nodes: HashMap<TypeId, TypeNode>,
    edges: Vec<TypeDependency>,
    confidence_threshold: f32,
}

pub enum TypeNode {
    Resolved(ConcreteType, Confidence),
    Conditional(Vec<(Condition, ConcreteType)>),
    Probabilistic(TypeDistribution),
    Unknown(Vec<TypeHint>),
}
```

### Преимущества
- ✅ Использует сильные стороны каждого подхода
- ✅ Graceful degradation
- ✅ Настраиваемый уровень строгости

### Недостатки
- ❌ Сложность реализации
- ❌ Трудно отлаживать
- ❌ Потенциально медленный

---

## 🎯 Рекомендация

Для production системы type-safe анализа BSL рекомендую **Подход 6: Multi-Phase Hybrid System** с акцентом на:

1. **Фаза 1** - покроет 80% случаев быстро
2. **Фаза 2** - разрешит сложные зависимости
3. **Фаза 3** - опционально для критичного кода
4. **Фаза 4** - экспериментально для UX улучшений

### Ключевые принципы реализации:

```rust
// Центральная абстракция - тип как граф возможностей
pub struct TypePossibilityGraph {
    root: TypeNode,
    confidence: Confidence,
    source: TypeSource,
    dependencies: Vec<TypeDependency>,
}

pub enum TypeSource {
    Static,           // Известно из кода
    Inferred,         // Выведено анализатором
    Annotated,        // Указано пользователем
    Runtime,          // Из runtime информации
    Predicted,        // ML предсказание
}

pub enum Confidence {
    Certain,          // 100% уверенность
    Likely(f32),      // Вероятностная оценка
    Unknown,          // Невозможно определить
}
```

Это позволит:
- Честно представлять неопределённость
- Давать полезные подсказки даже для динамического кода
- Постепенно улучшать анализ без полной переделки