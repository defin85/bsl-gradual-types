//! Intermediate Representation (IR) — семантический слой между Syntax и Domain
//!
//! IR не зависит от конкретного парсера (tree-sitter, regex, etc.)
//! и представляет программу в терминах, удобных для type analysis.
//!
//! # Архитектура
//!
//! ```text
//! Source Code
//!     ↓
//! AST (Syntax, backend) ← tree-sitter-bsl
//!     ↓
//! IR (Semantics, shared) ← AstToIrConverter (backend)
//!     ↓
//! Types (Domain, shared) ← AnalysisEngine, TypeResolver
//! ```
//!
//! # Основные компоненты
//!
//! - [`SemanticProgram`] — корневая структура IR
//! - [`SemanticNode`] — узлы программы (переменные, функции, control flow)
//! - [`SymbolTable`] — таблица символов с scope hierarchy
//! - [`ControlFlowGraph`] — граф потока управления (для flow-sensitive анализа)

mod visitor;

pub use visitor::{walk_program, FlowContext, SemanticVisitor};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::types::TypeResolution;

/// Семантическое представление программы
///
/// Не зависит от конкретного парсера и представляет программу
/// в терминах, необходимых для типового анализа.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticProgram {
    /// Таблица символов (переменные, функции)
    pub symbols: SymbolTable,

    /// Семантические узлы программы
    pub nodes: Vec<SemanticNode>,

    /// Информация об исходном файле
    pub source_info: SourceInfo,

    /// Граф потока управления (для flow-sensitive анализа)
    pub cfg: Option<ControlFlowGraph>,
}

/// Упрощённое семантическое представление элементов программы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    /// Тип узла
    pub kind: SemanticNodeKind,

    /// Позиция в исходном коде (для diagnostics и hover)
    pub span: Span,

    /// ID scope, к которому принадлежит узел
    pub scope_id: ScopeId,
}

/// Виды семантических узлов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SemanticNodeKind {
    // === Базовые объявления ===
    /// Объявление переменной: `Перем x: Число;`
    VariableDeclaration {
        name: String,
        type_hint: Option<String>,
        is_export: bool,
        initial_value_type: Option<String>,
    },

    /// Присваивание: `x = 42;`
    Assignment {
        variable: String,
        value_type: String,
        value_node: Option<usize>, // ✅ MILESTONE 3.5: индекс узла value expression (для hover)
    },

    /// Объявление функции
    FunctionDeclaration {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<String>,
        body_scope: ScopeId,
        body: Vec<usize>, // ✅ НОВОЕ: индексы узлов тела функции
    },

    /// Объявление процедуры
    ProcedureDeclaration {
        name: String,
        params: Vec<Parameter>,
        body_scope: ScopeId,
        body: Vec<usize>, // ✅ НОВОЕ: индексы узлов тела процедуры
    },

    // === Control Flow (КРИТИЧНО для Milestone 2.3 flow-sensitive) ===
    /// Условный оператор: `Если условие Тогда ... Иначе ... КонецЕсли`
    IfStatement {
        condition_type: String,
        then_branch: Vec<usize>, // Индексы SemanticNode в then ветке
        else_branch: Option<Vec<usize>>,
    },

    /// Цикл While: `Пока условие Цикл ... КонецЦикла`
    WhileLoop {
        condition_type: String,
        body: Vec<usize>,
    },

    /// Цикл For: `Для i = 1 По 10 Цикл ... КонецЦикла`
    ForLoop {
        variable: String,
        range_type: String,
        body: Vec<usize>,
    },

    /// Цикл ForEach: `Для Каждого элемент Из коллекция Цикл ... КонецЦикла`
    ForEachLoop {
        variable: String,
        collection_type: String,
        body: Vec<usize>,
    },

    /// Возврат из функции: `Возврат значение;`
    Return { value_type: Option<String> },

    /// Прерывание цикла: `Прервать;`
    Break,

    /// Продолжение цикла: `Продолжить;`
    Continue,

    /// Обработка исключений: `Попытка ... Исключение ... КонецПопытки`
    TryExcept {
        try_body: Vec<usize>,
        except_body: Vec<usize>,
    },

    // === Member Access (КРИТИЧНО для LSP hover) ===
    /// Доступ к члену объекта: `объект.свойство` или `объект.Метод()`
    ///
    /// # Семантика полей
    ///
    /// - `object_name`: **Имя переменной** из исходного кода (Some("МассивДанных"))
    ///   - Some(name) для простых переменных (Identifier)
    ///   - None для сложных выражений (PropertyAccess, Call, New, etc.)
    /// - `object_type`: **Тип объекта** (результат type inference, например, "Массив")
    /// - `member_name`: Имя свойства или метода (например, "Добавить")
    /// - `is_method`: true если это вызов метода, false если доступ к свойству
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// МассивДанных = Новый Массив();
    /// МассивДанных.Добавить("текст");  // object_name=Some("МассивДанных"), object_type="Массив"
    ///
    /// obj.prop1.prop2.Метод();  // object_name=None, object_type="<inferred_type>"
    /// ```
    MemberAccess {
        object_name: Option<String>, // ✅ ИСПРАВЛЕНО: Option вместо String
        object_type: String,         // Тип переменной (результат inference)
        member_name: String,
        is_method: bool, // true = метод, false = свойство
    },

    /// Вызов функции или метода: `Функция()` или `объект.Метод(args)`
    ///
    /// # Семантика полей
    ///
    /// - `function_name`: Имя функции/метода
    /// - `object_name`: **Имя переменной-объекта** для вызовов методов
    ///   - Some(name) для методов переменных: `МассивДанных.Добавить("x")`
    ///   - None для глобальных функций: `Сообщить("текст")`
    ///   - None для сложных выражений: `ПолучитьОбъект().Метод()`
    /// - `object_type`: **Тип объекта** для вызовов методов
    ///   - Some(type) для методов: `object_type = "Массив"`
    ///   - None для глобальных функций
    /// - `arg_types`: Типы аргументов вызова
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// Сообщить("текст");  // object_name=None, object_type=None
    /// МассивДанных.Добавить("x");  // object_name=Some("МассивДанных"), object_type=Some("Массив")
    /// ```
    ///
    /// # Milestone 3.5 Phase 2
    ///
    /// `object_name` введён для flow-sensitive анализа, чтобы различать:
    /// - **Имя переменной** (для lookup в SymbolTable)
    /// - **Тип объекта** (для резолюции методов)
    FunctionCall {
        function_name: String,
        object_name: Option<String>,
        object_type: Option<String>,
        arg_types: Vec<String>,
    },

    // === Scope tracking ===
    /// Блок scope
    BlockScope {
        statements: Vec<usize>, // Индексы SemanticNode
        scope_id: ScopeId,
    },

    // === Конструкторы (Constructor Support) ===
    /// Выражение конструктора: `Новый Массив`, `Новый Массив(10)`, `Новый("Строка")`
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// // Конструктор без параметров
    /// МассивДанных = Новый Массив;
    ///
    /// // Конструктор с параметрами
    /// МассивФиксированный = Новый Массив(10);
    ///
    /// // Динамический конструктор через строку
    /// Ссылка = Новый("СправочникСсылка.Номенклатура");
    /// ```
    ///
    /// # Семантика
    ///
    /// - `type_name` - имя типа для создания ("Массив", "ТаблицаЗначений", etc.)
    /// - `arg_types` - типы аргументов конструктора (для валидации и inference)
    /// - `is_dynamic` - `true` для динамических конструкторов `Новый("Тип")`
    /// - `result_type` - результирующий тип (обычно равен type_name, для Generic может быть "Массив<T>")
    /// - `generic_params` - параметры Generic типов для коллекций (None если не Generic)
    NewExpression {
        /// Имя типа для создания ("Массив", "ТаблицаЗначений", "СправочникСсылка.Номенклатура")
        type_name: String,

        /// Типы аргументов конструктора
        ///
        /// # Примеры
        /// - `Новый Массив(10)` → `vec!["Число"]`
        /// - `Новый ТаблицаЗначений` → `vec![]` (без аргументов)
        arg_types: Vec<String>,

        /// Динамический конструктор через строку
        ///
        /// `true` для выражений вида `Новый("СправочникСсылка.Номенклатура")`
        is_dynamic: bool,

        /// Результирующий тип конструктора
        ///
        /// Обычно равен `type_name`, но для Generic коллекций может включать параметры:
        /// - `Массив` → "Массив" (без параметров, inference отложен)
        /// - `Массив<Число>` → "Массив<Число>" (явно указанный Generic параметр)
        result_type: String,

        /// Generic параметры для коллекций (None если тип не Generic)
        ///
        /// # Примеры
        /// - `Массив` → None (inference будет выполнен позже из использования)
        /// - `Массив<Число>` → Some(vec!["Число"]) (явный параметр)
        /// - `Соответствие<Строка, Число>` → Some(vec!["Строка", "Число"])
        generic_params: Option<Vec<String>>,
    },
}

/// Параметр функции/процедуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_hint: Option<String>,
    pub default_value: Option<String>,
    pub is_val: bool, // ByVal параметр
}

/// Таблица символов с иерархией scope-ов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTable {
    /// Все scope-ы программы
    pub scopes: HashMap<ScopeId, Scope>,

    /// Корневой scope (глобальная область видимости)
    pub root_scope: ScopeId,

    /// Глобальные функции
    pub global_functions: HashMap<String, FunctionSignature>,

    /// Глобальные процедуры
    pub global_procedures: HashMap<String, FunctionSignature>,
}

/// Область видимости (scope)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    /// ID scope
    pub id: ScopeId,

    /// Родительский scope (None для root)
    pub parent: Option<ScopeId>,

    /// Переменные в этом scope
    pub variables: HashMap<String, (TypeResolution, Span)>,

    /// Дочерние scope-ы
    pub children: Vec<ScopeId>,
}

/// Идентификатор scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeId(pub usize);

/// Сигнатура функции/процедуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_export: bool,
}

/// Позиция в исходном коде
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl Span {
    /// Проверить, содержит ли span указанную позицию
    pub fn contains(&self, line: u32, column: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && column < self.start_column {
            return false;
        }
        if line == self.end_line && column > self.end_column {
            return false;
        }
        true
    }

    /// Создать stub span (для тестов и временного использования)
    pub fn stub() -> Self {
        Self {
            start_line: 0,
            start_column: 0,
            end_line: 0,
            end_column: 0,
        }
    }

    /// Создать span из координат
    pub fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// Создать span из tree-sitter позиций (для совместимости с backend)
    pub fn from_positions(start: (u32, u32), end: (u32, u32)) -> Self {
        Self {
            start_line: start.0,
            start_column: start.1,
            end_line: end.0,
            end_column: end.1,
        }
    }
}

/// Информация об исходном файле
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInfo {
    pub path: String,
    pub content_hash: u64,
}

/// Граф потока управления (для flow-sensitive анализа)
///
/// CFG представляет все возможные пути выполнения программы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlFlowGraph {
    pub nodes: HashMap<CfgNodeId, CfgNode>,
    pub edges: HashMap<CfgNodeId, Vec<CfgNodeId>>,
    pub entry: CfgNodeId,
    pub exit: CfgNodeId,
}

/// Идентификатор узла CFG
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CfgNodeId(pub usize);

/// Узел графа потока управления
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CfgNode {
    /// Обычный statement
    Statement { semantic_node_id: usize },

    /// Точка ветвления (if, while condition)
    Branch {
        condition_node_id: usize,
        true_branch: CfgNodeId,
        false_branch: CfgNodeId,
    },

    /// Слияние путей выполнения
    Merge,

    /// Точка выхода из функции
    Exit,
}

// === Реализация методов ===

impl SemanticProgram {
    /// Найти семантический узел по позиции в исходном коде
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SemanticProgram, Span};
    /// # let program = SemanticProgram::new();
    /// if let Some(node) = program.find_node_at_position(10, 5) {
    ///     println!("Нашли узел на строке 10, колонке 5");
    /// }
    /// ```
    pub fn find_node_at_position(&self, line: u32, column: u32) -> Option<&SemanticNode> {
        use tracing::debug;

        // Возвращаем САМЫЙ МАЛЕНЬКИЙ узел, содержащий позицию
        // (наиболее специфичный, самый вложенный)
        // Это важно для hover: если есть Assignment и вложенный FunctionCall,
        // курсор на вызове метода должен показывать FunctionCall, а не Assignment

        debug!("🔍 find_node_at_position: line={}, col={}", line, column);

        let candidates: Vec<_> = self.nodes
            .iter()
            .filter(|node| node.span.contains(line, column))
            .collect();

        debug!("📍 Найдено {} кандидатов для позиции {}:{}", candidates.len(), line, column);
        for (i, node) in candidates.iter().enumerate() {
            let type_name = match &node.kind {
                SemanticNodeKind::Assignment { variable, .. } => format!("Assignment({})", variable),
                SemanticNodeKind::FunctionCall { function_name, object_name, .. } =>
                    format!("FunctionCall({}.{})", object_name.as_deref().unwrap_or("?"), function_name),
                SemanticNodeKind::MemberAccess { object_name, member_name, .. } =>
                    format!("MemberAccess({}.{})", object_name.as_deref().unwrap_or("?"), member_name),
                SemanticNodeKind::VariableDeclaration { name, .. } => format!("VarDecl({})", name),
                _ => format!("{:?}", node.kind),
            };
            debug!("  [{}] {} span={:?}", i, type_name, node.span);
        }

        let result = self.nodes
            .iter()
            .filter(|node| node.span.contains(line, column))
            .min_by_key(|node| {
                // Сортируем по размеру span (площадь покрытия)
                let lines = node.span.end_line.saturating_sub(node.span.start_line);
                let cols = if lines == 0 {
                    node.span.end_column.saturating_sub(node.span.start_column)
                } else {
                    // Для многострочных span используем количество строк
                    lines * 1000
                };

                // ✅ MILESTONE 2.11: Приоритизация по типу узла
                // Когда несколько узлов имеют одинаковый размер span,
                // выбираем более специфичный тип (меньшее число = выше приоритет)
                let type_priority = match &node.kind {
                    SemanticNodeKind::FunctionCall { .. } => 0,       // Высший приоритет
                    SemanticNodeKind::MemberAccess { .. } => 1,       // Высокий приоритет
                    SemanticNodeKind::VariableDeclaration { .. } => 2, // Средний приоритет
                    SemanticNodeKind::Assignment { .. } => 10,        // Низкий приоритет
                    _ => 5,                                           // Остальные - средний приоритет
                };

                // Сортировка: сначала по размеру span, затем по приоритету типа
                (lines * 1000 + cols, type_priority)
            });

        if let Some(node) = result {
            let type_name = match &node.kind {
                SemanticNodeKind::Assignment { variable, .. } => format!("Assignment({})", variable),
                SemanticNodeKind::FunctionCall { function_name, object_name, .. } =>
                    format!("FunctionCall({}.{})", object_name.as_deref().unwrap_or("?"), function_name),
                _ => format!("{:?}", node.kind),
            };
            debug!("✅ Выбран узел: {} span={:?}", type_name, node.span);
        } else {
            debug!("❌ Узел не найден для позиции {}:{}", line, column);
        }

        result
    }

    /// Получить scope по ID
    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&Scope> {
        self.symbols.scopes.get(&scope_id)
    }

    /// Получить переменную в scope (с поиском в родительских scope)
    ///
    /// Поиск идёт от текущего scope вверх по иерархии до root
    pub fn resolve_variable(&self, name: &str, scope_id: ScopeId) -> Option<(TypeResolution, Span)> {
        let mut current_scope_id = Some(scope_id);

        while let Some(sid) = current_scope_id {
            if let Some(scope) = self.get_scope(sid) {
                if let Some(var) = scope.variables.get(name) {
                    return Some(var.clone());
                }
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Найти переменную в позиции (line, column) для Inline Scope Analysis
    ///
    /// Используется для LSP hover: находит переменную в scope в указанной позиции
    /// и возвращает её имя и тип.
    ///
    /// # Алгоритм
    ///
    /// 1. Найти SemanticNode в позиции (line, column)
    /// 2. Получить scope_id узла
    /// 3. Извлечь имя переменной из узла (Assignment, MemberAccess, VariableDeclaration)
    /// 4. Вызвать resolve_variable() для поиска в scope hierarchy
    /// 5. Вернуть (имя_переменной, TypeResolution)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_shared::ir::SemanticProgram;
    /// use bsl_shared::domain::types::TypeResolution;
    ///
    /// let program = SemanticProgram::new();
    /// // Предположим, в программе есть: МассивДанных = Новый Массив;
    /// if let Some((var_name, resolution)) = program.find_variable_at_position(5, 10) {
    ///     assert_eq!(var_name, "МассивДанных");
    ///     assert_eq!(resolution.type_name(), "Массив");
    /// }
    /// ```
    /// Извлечь имя переменной из узла IR (если узел содержит переменную)
    ///
    /// # Returns
    /// - `Some(name)` если узел содержит переменную с именем
    /// - `None` если узел не содержит переменную или это сложное выражение
    ///
    /// # Phase 4: DRY refactoring
    /// Устраняет дублирование между find_variable_at_position() и find_variable_with_scope()
    fn extract_variable_name(node: &SemanticNode) -> Option<String> {
        match &node.kind {
            // Присваивание: МассивДанных = Новый Массив
            SemanticNodeKind::Assignment { variable, .. } => Some(variable.clone()),

            // Доступ к члену: МассивДанных.Добавить
            SemanticNodeKind::MemberAccess {
                object_name: Some(obj_name),
                ..
            } => Some(obj_name.clone()),
            SemanticNodeKind::MemberAccess {
                object_name: None, ..
            } => None,

            // Объявление переменной: Перем МассивДанных
            SemanticNodeKind::VariableDeclaration { name, .. } => Some(name.clone()),

            // Вызов функции: может быть вызов метода переменной
            SemanticNodeKind::FunctionCall {
                object_name: Some(obj_name),
                ..
            } => Some(obj_name.clone()),
            SemanticNodeKind::FunctionCall {
                object_name: None, ..
            } => None,

            // Для остальных узлов не поддерживаем
            _ => None,
        }
    }

    pub fn find_variable_at_position(&self, line: u32, column: u32) -> Option<(String, TypeResolution)> {
        // 1. Находим узел в позиции
        let node = self.find_node_at_position(line, column)?;

        // 2. Получаем scope узла
        let scope_id = node.scope_id;

        // 3. Извлекаем имя переменной из узла (✅ Phase 4: используем общий метод)
        let var_name = Self::extract_variable_name(node)?;

        // 4. Ищем переменную в scope (с поиском в родительских scope)
        let (resolution, _span) = self.resolve_variable(&var_name, scope_id)?;

        // 5. Возвращаем результат
        Some((var_name, resolution))
    }

    /// Найти переменную по позиции с возвратом scope_id для резолюции через TypeResolver
    ///
    /// Расширенная версия `find_variable_at_position()`, возвращает scope_id для использования
    /// в `TypeResolver::resolve_variable_with_context()`.
    ///
    /// # Direction 2: Generic Collections Inference
    ///
    /// Используется для hover с Generic типами:
    /// ```bsl
    /// МассивСтрок = Новый Массив();
    /// МассивСтрок.Добавить("текст");  // ← cursor здесь
    /// // find_variable_with_scope(3, 12) → ("МассивСтрок", TypeResolution::generic(...), scope_id)
    /// // → TypeResolver::resolve_variable_with_context("МассивСтрок", symbols, scope_id)
    /// // → TypeResolution::Generic(Массив<Строка>) с методами
    /// ```
    pub fn find_variable_with_scope(
        &self,
        line: u32,
        column: u32,
    ) -> Option<(String, TypeResolution, ScopeId)> {
        // 1. Находим узел в позиции
        let node = self.find_node_at_position(line, column)?;

        // 2. Получаем scope узла
        let scope_id = node.scope_id;

        // 3. Извлекаем имя переменной из узла (✅ Phase 4: используем общий метод)
        let var_name = Self::extract_variable_name(node)?;

        // 4. Ищем переменную в scope (с поиском в родительских scope)
        let (resolution, _span) = self.resolve_variable(&var_name, scope_id)?;

        // 5. Возвращаем результат с scope_id
        Some((var_name, resolution, scope_id))
    }

    /// Создать новую пустую программу
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            nodes: Vec::new(),
            source_info: SourceInfo {
                path: String::new(),
                content_hash: 0,
            },
            cfg: None,
        }
    }
}

impl SymbolTable {
    /// Создать новую таблицу символов с root scope
    pub fn new() -> Self {
        let root_scope = ScopeId(0);
        let mut scopes = HashMap::new();
        scopes.insert(
            root_scope,
            Scope {
                id: root_scope,
                parent: None,
                variables: HashMap::new(),
                children: Vec::new(),
            },
        );

        Self {
            scopes,
            root_scope,
            global_functions: HashMap::new(),
            global_procedures: HashMap::new(),
        }
    }

    /// Создать новый дочерний scope
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, ScopeId};
    /// let mut table = SymbolTable::new();
    /// let child_scope = table.create_scope(table.root_scope);
    /// ```
    pub fn create_scope(&mut self, parent: ScopeId) -> ScopeId {
        let id = ScopeId(self.scopes.len());
        let scope = Scope {
            id,
            parent: Some(parent),
            variables: HashMap::new(),
            children: Vec::new(),
        };
        self.scopes.insert(id, scope);

        // Добавляем в дочерние для родителя
        if let Some(parent_scope) = self.scopes.get_mut(&parent) {
            parent_scope.children.push(id);
        }

        id
    }

    /// Зарегистрировать переменную в scope
    pub fn register_variable(
        &mut self,
        scope_id: ScopeId,
        name: String,
        resolution: TypeResolution,
        span: Span,
    ) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.variables.insert(name, (resolution, span));
        }
    }

    /// Зарегистрировать глобальную функцию
    pub fn register_function(&mut self, signature: FunctionSignature) {
        self.global_functions
            .insert(signature.name.clone(), signature);
    }

    /// Зарегистрировать глобальную процедуру
    pub fn register_procedure(&mut self, signature: FunctionSignature) {
        self.global_procedures
            .insert(signature.name.clone(), signature);
    }

    /// Получить тип переменной из текущего или родительского scope
    pub fn get_variable_type(&self, scope_id: ScopeId, name: &str) -> Option<TypeResolution> {
        let mut current_scope_id = Some(scope_id);

        while let Some(sid) = current_scope_id {
            if let Some(scope) = self.scopes.get(&sid) {
                if let Some((resolution, _span)) = scope.variables.get(name) {
                    return Some(resolution.clone());
                }
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Обновить тип переменной в указанном scope
    pub fn update_variable_type(
        &mut self,
        scope_id: ScopeId,
        name: String,
        new_resolution: TypeResolution,
    ) -> bool {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if let Some((resolution, _span)) = scope.variables.get_mut(&name) {
                *resolution = new_resolution;
                return true;
            }
        }
        false
    }

    /// Инициализировать переменную как Generic тип с неизвестными параметрами
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// # use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
    /// let mut table = SymbolTable::new();
    /// table.initialize_as_generic(table.root_scope, "МассивСтрок".to_string(), "Массив".to_string(), 1);
    ///
    /// let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");
    /// assert!(resolution.is_some());
    /// let res = resolution.unwrap();
    /// assert_eq!(res.type_name(), "Массив<Неопределено>");
    /// assert!(matches!(res.certainty, Certainty::Inferred(c) if c == 0.0));
    /// ```
    pub fn initialize_as_generic(
        &mut self,
        scope_id: ScopeId,
        var_name: String,
        base_type: String,
        type_param_count: usize,
    ) {
        // Создаём пустые параметры (неизвестные типы = "?")
        let type_params: Vec<&str> = vec!["?"; type_param_count];

        // Используем TypeResolution::generic() с certainty = 0.0 (параметры неизвестны)
        let resolution = TypeResolution::generic(&base_type, &type_params, 0.0);

        // Регистрируем или обновляем переменную
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            // Если переменная уже зарегистрирована, получаем её span
            let span = scope
                .variables
                .get(&var_name)
                .map(|(_, s)| *s)
                .unwrap_or_else(Span::stub);

            scope.variables.insert(var_name, (resolution, span));
        }
    }

    /// Обновить Generic параметр переменной
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// # use bsl_shared::domain::types::{TypeResolution, Certainty, ResolutionResult};
    /// let mut table = SymbolTable::new();
    /// table.initialize_as_generic(table.root_scope, "МассивСтрок".to_string(), "Массив".to_string(), 1);
    /// table.update_generic_param(table.root_scope, "МассивСтрок", 0, "Строка".to_string());
    ///
    /// let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");
    /// assert!(resolution.is_some());
    /// let res = resolution.unwrap();
    /// assert_eq!(res.type_name(), "Массив<Строка>");
    /// assert!(matches!(res.certainty, Certainty::Known));
    /// ```
    pub fn update_generic_param(
        &mut self,
        scope_id: ScopeId,
        var_name: &str,
        param_index: usize,
        param_type: String,
    ) -> bool {
        use crate::domain::types::{Certainty, ConcreteType, ResolutionResult, SpecialType};

        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if let Some((resolution, _span)) = scope.variables.get_mut(var_name) {
                match &mut resolution.result {
                    ResolutionResult::Generic(gen) => {
                        // Получаем новый ConcreteType для параметра
                        let new_param = TypeResolution::string_to_concrete(&param_type);

                        // Обновляем конкретный параметр (расширяем вектор, если нужно)
                        if param_index < gen.type_params.len() {
                            gen.type_params[param_index] = new_param;
                        } else {
                            gen.type_params.push(new_param);
                        }

                        // Вычисляем новую уверенность
                        // Если ВСЕ параметры известны (не Undefined), то certainty = Known
                        let all_known = gen.type_params.iter().all(|p| {
                            !matches!(p, ConcreteType::Special(SpecialType::Undefined))
                        });
                        resolution.certainty = if all_known {
                            Certainty::Known
                        } else {
                            Certainty::Inferred(0.5)
                        };

                        return true;
                    }
                    _ => {
                        // Не Generic тип — попробуем конвертировать
                        // (например, если раньше был Inferred, теперь становится Generic)
                        tracing::warn!(
                            "update_generic_param: {} не Generic тип, пропускаем",
                            var_name
                        );
                    }
                }
            }
        }

        false
    }

    /// Поиск переменной в заданной области видимости
    ///
    /// Возвращает TypeResolution переменной, если она существует в указанном scope.
    /// Не выполняет поиск в родительских scope (для этого используйте `lookup_variable_in_hierarchy`).
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "x".to_string(),
    ///     TypeResolution::explicit("Число"),
    ///     Span::stub(),
    /// );
    ///
    /// let resolution = table.lookup_variable(table.root_scope, "x");
    /// assert!(resolution.is_some());
    /// ```
    pub fn lookup_variable(&self, scope_id: ScopeId, name: &str) -> Option<&TypeResolution> {
        self.scopes
            .get(&scope_id)?
            .variables
            .get(name)
            .map(|(resolution, _)| resolution)
    }

    /// Поиск переменной с подъёмом по цепочке родительских scope
    ///
    /// Ищет переменную начиная с указанного scope и поднимаясь вверх по иерархии
    /// до root scope. Возвращает scope_id где была найдена переменная и её TypeResolution.
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "globalVar".to_string(),
    ///     TypeResolution::explicit("Число"),
    ///     Span::stub(),
    /// );
    ///
    /// let child = table.create_scope(table.root_scope);
    /// let result = table.lookup_variable_in_hierarchy(child, "globalVar");
    /// assert!(result.is_some());
    /// ```
    pub fn lookup_variable_in_hierarchy(
        &self,
        scope_id: ScopeId,
        name: &str,
    ) -> Option<(ScopeId, &TypeResolution)> {
        let mut current = Some(scope_id);
        while let Some(sid) = current {
            if let Some(scope) = self.scopes.get(&sid) {
                if let Some((resolution, _)) = scope.variables.get(name) {
                    return Some((sid, resolution));
                }
                current = scope.parent;
            } else {
                break;
            }
        }
        None
    }

    /// Обновление типа переменной в заданной области видимости с обработкой ошибок
    ///
    /// Альтернатива `update_variable_type()`, возвращающая `Result` для более явной обработки ошибок.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку, если:
    /// - Scope с указанным ID не существует
    /// - Переменная с указанным именем не найдена в scope
    pub fn update_variable_type_checked(
        &mut self,
        scope_id: ScopeId,
        name: &str,
        resolution: TypeResolution,
    ) -> Result<(), String> {
        let scope = self
            .scopes
            .get_mut(&scope_id)
            .ok_or_else(|| format!("Scope {:?} not found", scope_id))?;

        let (existing_resolution, _span) = scope
            .variables
            .get_mut(name)
            .ok_or_else(|| format!("Variable '{}' not found in scope {:?}", name, scope_id))?;

        *existing_resolution = resolution;
        Ok(())
    }

    /// Проверка существования переменной в scope
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, Span};
    /// # use bsl_shared::domain::types::TypeResolution;
    /// let mut table = SymbolTable::new();
    /// table.register_variable(
    ///     table.root_scope,
    ///     "x".to_string(),
    ///     TypeResolution::explicit("Число"),
    ///     Span::stub(),
    /// );
    ///
    /// assert!(table.has_variable(table.root_scope, "x"));
    /// assert!(!table.has_variable(table.root_scope, "y"));
    /// ```
    pub fn has_variable(&self, scope_id: ScopeId, name: &str) -> bool {
        self.scopes
            .get(&scope_id)
            .map(|s| s.variables.contains_key(name))
            .unwrap_or(false)
    }

    /// Поиск функции по имени
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::{SymbolTable, FunctionSignature, Parameter};
    /// let mut table = SymbolTable::new();
    /// table.register_function(FunctionSignature {
    ///     name: "МояФункция".to_string(),
    ///     params: vec![],
    ///     return_type: Some("Число".to_string()),
    ///     is_export: false,
    /// });
    ///
    /// assert!(table.find_function("МояФункция").is_some());
    /// ```
    pub fn find_function(&self, name: &str) -> Option<&FunctionSignature> {
        self.global_functions.get(name)
    }

    /// Поиск процедуры по имени
    pub fn find_procedure(&self, name: &str) -> Option<&FunctionSignature> {
        self.global_procedures.get(name)
    }

    /// Получить родительский scope
    ///
    /// # Примеры
    ///
    /// ```
    /// # use bsl_shared::ir::SymbolTable;
    /// let mut table = SymbolTable::new();
    /// let child = table.create_scope(table.root_scope);
    ///
    /// assert_eq!(table.get_parent_scope(child), Some(table.root_scope));
    /// assert_eq!(table.get_parent_scope(table.root_scope), None);
    /// ```
    pub fn get_parent_scope(&self, scope_id: ScopeId) -> Option<ScopeId> {
        self.scopes.get(&scope_id)?.parent
    }

    /// Итератор по всем переменным в scope
    ///
    /// Возвращает итератор по парам (имя переменной, TypeResolution) для указанного scope.
    /// Не включает переменные из родительских scope.
    pub fn variables_in_scope(
        &self,
        scope_id: ScopeId,
    ) -> Option<impl Iterator<Item = (&String, &TypeResolution)>> {
        self.scopes
            .get(&scope_id)
            .map(|scope| scope.variables.iter().map(|(name, (resolution, _))| (name, resolution)))
    }

    /// Получить количество глобальных функций
    pub fn functions_count(&self) -> usize {
        self.global_functions.len()
    }

    /// Получить количество глобальных процедур
    pub fn procedures_count(&self) -> usize {
        self.global_procedures.len()
    }

    /// Итератор по всем глобальным функциям
    ///
    /// Возвращает итератор по парам (имя функции, FunctionSignature)
    pub fn iter_functions(&self) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        self.global_functions.iter()
    }

    /// Итератор по всем глобальным процедурам
    ///
    /// Возвращает итератор по парам (имя процедуры, FunctionSignature)
    pub fn iter_procedures(&self) -> impl Iterator<Item = (&String, &FunctionSignature)> {
        self.global_procedures.iter()
    }

    /// Итератор по всем scopes в таблице символов
    ///
    /// Используется для конвертации в DTO и других операций, требующих доступа ко всем scopes.
    pub fn iter_all_scopes(&self) -> impl Iterator<Item = (&ScopeId, &Scope)> {
        self.scopes.iter()
    }

    /// Количество scopes в таблице символов
    pub fn scopes_count(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for SemanticProgram {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE: extract_runtime_types() удалена в Milestone 2.9
// Используем Inline Scope Analysis вместо загрузки runtime типов в TypeRepository
// См. SemanticProgram::find_variable_at_position() для нового подхода

// === Конверторы IR → DTO (Milestone 2.12) ===

use crate::api::semantic_dtos::*;

impl SemanticProgram {
    /// Конвертировать SemanticProgram в SemanticTreeDto для передачи клиентам
    ///
    /// # Milestone 2.12: Real-time Semantic Tree Visualization
    ///
    /// Этот метод преобразует внутреннее представление IR в DTO,
    /// пригодное для передачи через LSP и Web API.
    pub fn to_dto(
        &self,
        include_call_graph: bool,
        include_flow_sensitive: bool,
    ) -> SemanticTreeDto {
        let start_time = std::time::Instant::now();

        // Конвертируем root-level узлы (только узлы в root scope!)
        let root_nodes = self
            .nodes
            .iter()
            .filter(|node| {
                // ✅ ИСПРАВЛЕНИЕ: Фильтруем по scope_id, а не по типу узла
                // Показываем только узлы, которые находятся в root scope
                node.scope_id == self.symbols.root_scope
            })
            .map(|node| self.node_to_dto(node, 0))
            .collect();

        // Конвертируем таблицу символов
        let symbol_table = self.symbols_to_dto(include_flow_sensitive);

        // Собираем граф вызовов (если requested)
        let call_graph = if include_call_graph {
            self.extract_call_graph()
        } else {
            Vec::new()
        };

        // Вычисляем метрики
        let metrics = self.calculate_metrics();

        let analysis_time_ms = start_time.elapsed().as_millis() as u64;

        SemanticTreeDto {
            file_path: self.source_info.path.clone(),
            root_nodes,
            symbol_table,
            call_graph,
            metrics: SemanticMetricsDto {
                analysis_time_ms,
                ..metrics
            },
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Конвертировать узел в DTO
    fn node_to_dto(&self, node: &SemanticNode, depth: usize) -> SemanticNodeDto {
        let (kind, name, attributes) = self.extract_node_info(&node.kind);

        // Рекурсивно конвертируем children (с ограничением глубины)
        let children = if depth < 10 {
            self.get_node_children(node, depth + 1)
        } else {
            Vec::new()
        };

        SemanticNodeDto {
            kind,
            name,
            location: SourceLocationDto {
                line: node.span.start_line,
                column: node.span.start_column,
            },
            range: Some(SourceRangeDto {
                start: SourceLocationDto {
                    line: node.span.start_line,
                    column: node.span.start_column,
                },
                end: SourceLocationDto {
                    line: node.span.end_line,
                    column: node.span.end_column,
                },
            }),
            children,
            attributes,
        }
    }

    /// Извлечь информацию из SemanticNodeKind
    fn extract_node_info(
        &self,
        kind: &SemanticNodeKind,
    ) -> (String, Option<String>, HashMap<String, String>) {
        let mut attributes = HashMap::new();

        match kind {
            SemanticNodeKind::VariableDeclaration {
                name,
                type_hint,
                is_export,
                ..
            } => {
                if let Some(hint) = type_hint {
                    attributes.insert("type".to_string(), hint.clone());
                }
                attributes.insert("is_export".to_string(), is_export.to_string());
                ("Variable".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::FunctionDeclaration {
                name,
                params,
                return_type,
                ..
            } => {
                attributes.insert("parameter_count".to_string(), params.len().to_string());
                if let Some(ret) = return_type {
                    attributes.insert("return_type".to_string(), ret.clone());
                }
                ("Function".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::ProcedureDeclaration { name, params, .. } => {
                attributes.insert("parameter_count".to_string(), params.len().to_string());
                ("Procedure".to_string(), Some(name.clone()), attributes)
            }
            SemanticNodeKind::Assignment {
                variable,
                value_type,
                value_node,
            } => {
                attributes.insert("variable".to_string(), variable.clone());
                attributes.insert("value_type".to_string(), value_type.clone());
                if let Some(vn) = value_node {
                    attributes.insert("value_node".to_string(), vn.to_string());
                }
                // ✅ Показываем имя переменной в UI
                (
                    "Assignment".to_string(),
                    Some(format!("{} = {}", variable, value_type)),
                    attributes,
                )
            }
            SemanticNodeKind::IfStatement { .. } => ("IfStatement".to_string(), None, attributes),
            SemanticNodeKind::ForLoop { .. } => ("ForLoop".to_string(), None, attributes),
            SemanticNodeKind::WhileLoop { .. } => ("WhileLoop".to_string(), None, attributes),
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                arg_types,
                ..
            } => {
                attributes.insert("function_name".to_string(), function_name.clone());
                attributes.insert("arg_count".to_string(), arg_types.len().to_string());

                // ✅ MILESTONE 3.5: Показываем полное имя вызова (объект.метод)
                let display_name = if let Some(obj_name) = object_name {
                    attributes.insert("object_name".to_string(), obj_name.clone());
                    format!("{}.{}", obj_name, function_name)
                } else {
                    function_name.clone()
                };

                (
                    "FunctionCall".to_string(),
                    Some(display_name),
                    attributes,
                )
            }
            SemanticNodeKind::Return { .. } => ("Return".to_string(), None, attributes),
            SemanticNodeKind::TryExcept { .. } => ("TryExcept".to_string(), None, attributes),
            SemanticNodeKind::Break => ("Break".to_string(), None, attributes),
            SemanticNodeKind::Continue => ("Continue".to_string(), None, attributes),
            SemanticNodeKind::ForEachLoop {
                variable,
                collection_type,
                ..
            } => {
                attributes.insert("variable".to_string(), variable.clone());
                attributes.insert("collection_type".to_string(), collection_type.clone());
                ("ForEachLoop".to_string(), None, attributes)
            }
            SemanticNodeKind::MemberAccess {
                object_name,
                object_type,
                member_name,
                is_method,
            } => {
                // object_name теперь Option<String>
                if let Some(name) = object_name {
                    attributes.insert("object_name".to_string(), name.clone());
                }
                attributes.insert("object_type".to_string(), object_type.clone());
                attributes.insert("member_name".to_string(), member_name.clone());
                attributes.insert("is_method".to_string(), is_method.to_string());

                let description = object_name
                    .as_ref()
                    .map(|name| format!("{}.{}", name, member_name))
                    .unwrap_or_else(|| format!("<expr>.{}", member_name));

                ("MemberAccess".to_string(), Some(description), attributes)
            }
            SemanticNodeKind::BlockScope { .. } => ("BlockScope".to_string(), None, attributes),
            SemanticNodeKind::NewExpression {
                type_name,
                arg_types,
                is_dynamic,
                result_type,
                generic_params,
            } => {
                attributes.insert("type_name".to_string(), type_name.clone());
                attributes.insert("arg_count".to_string(), arg_types.len().to_string());
                attributes.insert("is_dynamic".to_string(), is_dynamic.to_string());
                attributes.insert("result_type".to_string(), result_type.clone());

                if let Some(params) = generic_params {
                    attributes.insert("generic_params".to_string(), params.join(", "));
                }

                // Форматируем имя для отображения в UI
                let display_name = if *is_dynamic {
                    format!("Новый(\"{}\")", type_name)
                } else if arg_types.is_empty() {
                    format!("Новый {}", type_name)
                } else {
                    format!("Новый {}({} args)", type_name, arg_types.len())
                };

                ("NewExpression".to_string(), Some(display_name), attributes)
            }
        }
    }

    /// Получить дочерние узлы (для построения иерархии)
    fn get_node_children(&self, parent: &SemanticNode, depth: usize) -> Vec<SemanticNodeDto> {
        use SemanticNodeKind::*;

        // Извлекаем индексы дочерних узлов в зависимости от типа родителя
        let child_indices: Vec<usize> = match &parent.kind {
            // ✅ НОВОЕ: Извлекаем body из FunctionDeclaration
            FunctionDeclaration { body, .. } => body.clone(),

            // ✅ НОВОЕ: Извлекаем body из ProcedureDeclaration
            ProcedureDeclaration { body, .. } => body.clone(),

            // Существующие узлы с индексами
            IfStatement {
                then_branch,
                else_branch,
                ..
            } => {
                let mut indices = then_branch.clone();
                if let Some(else_idx) = else_branch {
                    indices.extend(else_idx);
                }
                indices
            }
            WhileLoop { body, .. } => body.clone(),
            ForLoop { body, .. } => body.clone(),
            ForEachLoop { body, .. } => body.clone(),
            TryExcept {
                try_body,
                except_body,
            } => {
                let mut indices = try_body.clone();
                indices.extend(except_body);
                indices
            }
            BlockScope { statements, .. } => statements.clone(),

            // ✅ MILESTONE 3.5: Assignment может содержать вложенный FunctionCall
            Assignment { value_node, .. } => {
                value_node.iter().copied().collect()
            }

            // Листовые узлы (нет детей)
            _ => Vec::new(),
        };

        // Конвертируем дочерние узлы в DTO (рекурсивно)
        child_indices
            .iter()
            .filter_map(|&idx| self.nodes.get(idx))
            .map(|node| self.node_to_dto(node, depth))
            .collect()
    }

    /// Конвертировать таблицу символов в DTO
    fn symbols_to_dto(&self, _include_flow_sensitive: bool) -> HashMap<String, SymbolInfoDto> {
        let mut result = HashMap::new();

        // Обходим все scopes используя публичное API
        for (_scope_id, scope) in self.symbols.iter_all_scopes() {
            for (var_name, (resolution, span)) in &scope.variables {
                let symbol = SymbolInfoDto {
                    name: var_name.clone(),
                    kind: "Variable".to_string(),
                    resolved_type: self.type_resolution_to_dto(resolution),
                    scope: "Local".to_string(), // TODO: различать Global/Local
                    declaration_location: SourceLocationDto {
                        line: span.start_line,
                        column: span.start_column,
                    },
                    // TODO: flow-sensitive analysis (пока не реализовано)
                    flow_variants: Vec::new(),
                    metadata: HashMap::new(),
                };

                result.insert(var_name.clone(), symbol);
            }
        }

        // Добавляем функции используя публичное API
        for (fn_name, sig) in self.symbols.iter_functions() {
            let symbol = SymbolInfoDto {
                name: fn_name.clone(),
                kind: if sig.return_type.is_some() {
                    "Function".to_string()
                } else {
                    "Procedure".to_string()
                },
                resolved_type: sig.return_type.as_ref().map(|rt| TypeResolutionDto {
                    name: rt.clone(),
                    category: "Unknown".to_string(),
                    certainty: "Inferred".to_string(),
                    certainty_percent: 50,
                    active_facet: None,
                    methods: Vec::new(),
                    properties: Vec::new(),
                    is_union: None,
                    union_components: Vec::new(),
                }),
                scope: "Global".to_string(),
                declaration_location: SourceLocationDto { line: 0, column: 0 }, // TODO: store location
                flow_variants: Vec::new(),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("is_export".to_string(), sig.is_export.to_string());
                    meta.insert("parameter_count".to_string(), sig.params.len().to_string());
                    meta
                },
            };

            result.insert(fn_name.clone(), symbol);
        }

        result
    }

    /// Конвертировать TypeResolution в TypeResolutionDto
    fn type_resolution_to_dto(&self, resolution: &TypeResolution) -> Option<TypeResolutionDto> {
        use crate::domain::types::{Certainty, ResolutionResult};

        if matches!(resolution.certainty, Certainty::Unknown) {
            return None;
        }

        let type_name = resolution.type_name();

        let (category, certainty_str, certainty_percent) = match &resolution.certainty {
            Certainty::Known => ("Platform".to_string(), "Known".to_string(), 100u8),
            Certainty::Inferred(conf) => {
                let percent = (*conf * 100.0) as u8;
                (
                    if matches!(resolution.result, ResolutionResult::Generic(_)) {
                        "Generic".to_string()
                    } else {
                        "Inferred".to_string()
                    },
                    if *conf > 0.8 { "Known".to_string() } else { "Inferred".to_string() },
                    percent,
                )
            }
            Certainty::Unknown => return None,
        };

        Some(TypeResolutionDto {
            name: type_name,
            category,
            certainty: certainty_str,
            certainty_percent,
            active_facet: None,
            methods: Vec::new(),
            properties: Vec::new(),
            is_union: None,
            union_components: Vec::new(),
        })
    }

    /// Извлечь граф вызовов функций
    fn extract_call_graph(&self) -> Vec<CallEdgeDto> {
        // TODO: Реализовать извлечение call graph из узлов
        // Для MVP возвращаем пустой граф

        Vec::new()
    }

    /// Вычислить метрики семантического анализа
    fn calculate_metrics(&self) -> SemanticMetricsDto {
        use crate::domain::types::Certainty;

        let mut procedure_count = 0;
        let mut function_count = 0;
        let mut variable_count = 0;
        let mut known_types = 0;
        let mut inferred_types = 0;
        let mut unknown_types = 0;

        // Подсчёт procedures и functions
        for node in &self.nodes {
            match &node.kind {
                SemanticNodeKind::ProcedureDeclaration { .. } => procedure_count += 1,
                SemanticNodeKind::FunctionDeclaration { .. } => function_count += 1,
                SemanticNodeKind::VariableDeclaration { .. } => variable_count += 1,
                _ => {}
            }
        }

        // Подсчёт типов
        for scope in self.symbols.scopes.values() {
            for (resolution, _span) in scope.variables.values() {
                match &resolution.certainty {
                    Certainty::Known => known_types += 1,
                    Certainty::Inferred(conf) => {
                        if *conf > 0.8 {
                            known_types += 1;
                        } else {
                            inferred_types += 1;
                        }
                    }
                    Certainty::Unknown => unknown_types += 1,
                }
            }
        }

        let total_types = known_types + inferred_types + unknown_types;
        let average_certainty = if total_types > 0 {
            (known_types as f32 + inferred_types as f32 * 0.75) / total_types as f32
        } else {
            0.0
        };

        SemanticMetricsDto {
            procedure_count,
            function_count,
            variable_count,
            parameter_count: 0, // TODO: calculate
            known_types,
            inferred_types,
            unknown_types,
            average_certainty,
            analysis_time_ms: 0, // Will be set by caller
            node_count: self.nodes.len(),
            tree_depth: self.calculate_tree_depth(),
            call_count: 0, // TODO: calculate
        }
    }

    /// Вычислить максимальную глубину дерева
    fn calculate_tree_depth(&self) -> usize {
        // TODO: Реализовать подсчёт глубины
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_contains() {
        let span = Span::new(5, 10, 5, 20);

        assert!(span.contains(5, 15)); // В середине
        assert!(span.contains(5, 10)); // Начало
        assert!(span.contains(5, 20)); // Конец
        assert!(!span.contains(4, 15)); // До начала
        assert!(!span.contains(6, 15)); // После конца
        assert!(!span.contains(5, 5)); // До start_column
        assert!(!span.contains(5, 25)); // После end_column
    }

    #[test]
    fn test_symbol_table_creation() {
        let table = SymbolTable::new();

        assert_eq!(table.scopes.len(), 1); // Только root scope
        assert_eq!(table.root_scope, ScopeId(0));
        assert!(table.global_functions.is_empty());
    }

    #[test]
    fn test_scope_hierarchy() {
        let mut table = SymbolTable::new();

        let child1 = table.create_scope(table.root_scope);
        let child2 = table.create_scope(table.root_scope);
        let grandchild = table.create_scope(child1);

        assert_eq!(table.scopes.len(), 4); // root + 2 children + 1 grandchild

        // Проверяем родительские связи
        assert_eq!(table.scopes[&child1].parent, Some(ScopeId(0)));
        assert_eq!(table.scopes[&grandchild].parent, Some(child1));

        // Проверяем дочерние связи
        let root = &table.scopes[&table.root_scope];
        assert_eq!(root.children.len(), 2);
        assert!(root.children.contains(&child1));
        assert!(root.children.contains(&child2));
    }

    #[test]
    fn test_variable_resolution() {
        let mut program = SemanticProgram::new();
        let child_scope = program.symbols.create_scope(program.symbols.root_scope);

        // Регистрируем переменную в root scope
        program.symbols.register_variable(
            program.symbols.root_scope,
            "globalVar".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        // Регистрируем переменную в child scope
        program.symbols.register_variable(
            child_scope,
            "localVar".to_string(),
            TypeResolution::explicit("Строка"),
            Span::stub(),
        );

        // Поиск в child scope должен найти обе переменные
        assert!(program.resolve_variable("localVar", child_scope).is_some());
        assert!(program.resolve_variable("globalVar", child_scope).is_some());

        // Поиск в root scope должен найти только globalVar
        assert!(program
            .resolve_variable("globalVar", program.symbols.root_scope)
            .is_some());
        assert!(program
            .resolve_variable("localVar", program.symbols.root_scope)
            .is_none());
    }

    #[test]
    fn test_find_node_at_position() {
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableDeclaration {
                name: "x".to_string(),
                type_hint: Some("Число".to_string()),
                is_export: false,
                initial_value_type: None,
            },
            span: Span::new(1, 0, 1, 15),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::Assignment {
                variable: "x".to_string(),
                value_type: "Число".to_string(),
                value_node: None,
            },
            span: Span::new(2, 0, 2, 10),
            scope_id: program.symbols.root_scope,
        });

        // Поиск первого узла
        let node = program.find_node_at_position(1, 5);
        assert!(node.is_some());
        assert!(matches!(
            node.unwrap().kind,
            SemanticNodeKind::VariableDeclaration { .. }
        ));

        // Поиск второго узла
        let node = program.find_node_at_position(2, 5);
        assert!(node.is_some());
        assert!(matches!(
            node.unwrap().kind,
            SemanticNodeKind::Assignment { .. }
        ));

        // Поиск вне узлов
        assert!(program.find_node_at_position(10, 5).is_none());
    }

    // === Tests for Generic TypeResolution functionality ===

    #[test]
    fn test_initialize_as_generic() {
        use crate::domain::types::{Certainty, ResolutionResult};

        let mut table = SymbolTable::new();

        table.initialize_as_generic(
            table.root_scope,
            "МассивСтрок".to_string(),
            "Массив".to_string(),
            1,
        );

        let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");

        assert!(resolution.is_some(), "Variable should exist");
        let res = resolution.unwrap();

        // Проверяем, что это Generic тип
        if let ResolutionResult::Generic(gen) = &res.result {
            assert_eq!(gen.base_type, "Массив");
            assert_eq!(gen.type_params.len(), 1);
        } else {
            panic!("Expected Generic resolution, got {:?}", res.result);
        }

        // Проверяем certainty = Inferred(0.0) (параметры неизвестны)
        assert!(matches!(res.certainty, Certainty::Inferred(c) if (c - 0.0).abs() < 0.001));
    }

    #[test]
    fn test_update_generic_param() {
        use crate::domain::types::{Certainty, ResolutionResult};

        let mut table = SymbolTable::new();

        // Инициализируем как Generic
        table.initialize_as_generic(
            table.root_scope,
            "МассивСтрок".to_string(),
            "Массив".to_string(),
            1,
        );

        // Обновляем параметр
        let success =
            table.update_generic_param(table.root_scope, "МассивСтрок", 0, "Строка".to_string());

        assert!(success, "update_generic_param должна вернуть true");

        let resolution = table.get_variable_type(table.root_scope, "МассивСтрок");

        assert!(resolution.is_some());
        let res = resolution.unwrap();

        // Проверяем имя типа
        assert_eq!(res.type_name(), "Массив<Строка>");

        // Проверяем certainty = Known (все параметры известны)
        assert!(matches!(res.certainty, Certainty::Known));
    }

    #[test]
    fn test_update_map_generic_params() {
        use crate::domain::types::{Certainty, ResolutionResult};

        let mut table = SymbolTable::new();

        // Инициализируем Соответствие с 2 параметрами
        table.initialize_as_generic(
            table.root_scope,
            "Словарь".to_string(),
            "Соответствие".to_string(),
            2,
        );

        // Обновляем первый параметр (ключ)
        table.update_generic_param(table.root_scope, "Словарь", 0, "Строка".to_string());

        // Обновляем второй параметр (значение)
        table.update_generic_param(table.root_scope, "Словарь", 1, "Число".to_string());

        let resolution = table.get_variable_type(table.root_scope, "Словарь");

        assert!(resolution.is_some());
        let res = resolution.unwrap();

        // Проверяем имя типа
        assert_eq!(res.type_name(), "Соответствие<Строка, Число>");

        // Проверяем certainty = Known (все параметры известны)
        assert!(matches!(res.certainty, Certainty::Known));
    }

    #[test]
    fn test_partial_generic_params() {
        use crate::domain::types::Certainty;
        let mut table = SymbolTable::new();

        // Инициализируем с 3 параметрами (не существует такого типа, но для теста)
        table.initialize_as_generic(
            table.root_scope,
            "МойТип".to_string(),
            "МойКонтейнер".to_string(),
            3,
        );

        // Обновляем только первый параметр
        table.update_generic_param(table.root_scope, "МойТип", 0, "Строка".to_string());

        let resolution = table.get_variable_type(table.root_scope, "МойТип");
        assert!(resolution.is_some());
        let res = resolution.unwrap();

        // Проверяем что тип - Generic с частично заполненными параметрами
        assert_eq!(res.type_name(), "МойКонтейнер<Строка, Неопределено, Неопределено>");

        // Проверяем certainty - промежуточная уверенность (не все параметры заполнены)
        match &res.certainty {
            Certainty::Inferred(c) => assert!((*c - 0.5).abs() < 0.01),
            _ => panic!("Expected Inferred certainty for partial generic"),
        }
    }

    #[test]
    fn test_generic_scope_hierarchy() {
        let mut program = SemanticProgram::new();
        let child_scope = program.symbols.create_scope(program.symbols.root_scope);

        // Регистрируем Generic переменную в root scope
        program.symbols.initialize_as_generic(
            program.symbols.root_scope,
            "МассивВРуте".to_string(),
            "Массив".to_string(),
            1,
        );

        // Регистрируем Generic переменную в child scope
        program.symbols.initialize_as_generic(
            child_scope,
            "МассивВОтпроцедуре".to_string(),
            "Массив".to_string(),
            1,
        );

        // Обновляем параметр в child scope
        program.symbols.update_generic_param(
            child_scope,
            "МассивВОтпроцедуре",
            0,
            "Число".to_string(),
        );

        // Проверяем видимость из child scope
        let resolution = program
            .symbols
            .get_variable_type(child_scope, "МассивВРуте");

        // Должна быть видна переменная из root scope
        assert!(resolution.is_some(), "Should see Generic variable from parent scope");
        // Проверяем что это действительно Generic (начинается с "Массив<")
        assert!(resolution.unwrap().type_name().starts_with("Массив<"));

        // Проверяем, что child переменная не видна из root scope
        let hint = program
            .symbols
            .get_variable_type(program.symbols.root_scope, "МассивВОтпроцедуре");
        assert!(
            hint.is_none(),
            "Child scope variable should not be visible from parent"
        );
    }

    // === Tests for NewExpression ===

    #[test]
    fn test_new_expression_simple() {
        let mut program = SemanticProgram::new();

        // Простой конструктор: Новый Массив
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::NewExpression {
                type_name: "Массив".to_string(),
                arg_types: vec![],
                is_dynamic: false,
                result_type: "Массив".to_string(),
                generic_params: None,
            },
            span: Span::new(1, 0, 1, 12),
            scope_id: program.symbols.root_scope,
        });

        assert_eq!(program.nodes.len(), 1);

        if let SemanticNodeKind::NewExpression {
            type_name,
            is_dynamic,
            ..
        } = &program.nodes[0].kind
        {
            assert_eq!(type_name, "Массив");
            assert!(!is_dynamic);
        } else {
            panic!("Expected NewExpression node");
        }
    }

    #[test]
    fn test_new_expression_with_args() {
        let mut program = SemanticProgram::new();

        // Конструктор с параметром: Новый Массив(10)
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::NewExpression {
                type_name: "Массив".to_string(),
                arg_types: vec!["Число".to_string()],
                is_dynamic: false,
                result_type: "Массив".to_string(),
                generic_params: None,
            },
            span: Span::new(1, 0, 1, 16),
            scope_id: program.symbols.root_scope,
        });

        if let SemanticNodeKind::NewExpression { arg_types, .. } = &program.nodes[0].kind {
            assert_eq!(arg_types.len(), 1);
            assert_eq!(arg_types[0], "Число");
        } else {
            panic!("Expected NewExpression node");
        }
    }

    #[test]
    fn test_new_expression_dynamic() {
        let mut program = SemanticProgram::new();

        // Динамический конструктор: Новый("СправочникСсылка.Номенклатура")
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::NewExpression {
                type_name: "СправочникСсылка.Номенклатура".to_string(),
                arg_types: vec![],
                is_dynamic: true,
                result_type: "СправочникСсылка.Номенклатура".to_string(),
                generic_params: None,
            },
            span: Span::new(1, 0, 1, 40),
            scope_id: program.symbols.root_scope,
        });

        if let SemanticNodeKind::NewExpression {
            type_name,
            is_dynamic,
            ..
        } = &program.nodes[0].kind
        {
            assert_eq!(type_name, "СправочникСсылка.Номенклатура");
            assert!(is_dynamic);
        } else {
            panic!("Expected NewExpression node");
        }
    }

    #[test]
    fn test_new_expression_with_generics() {
        let mut program = SemanticProgram::new();

        // Generic конструктор: Новый Массив<Число>
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::NewExpression {
                type_name: "Массив".to_string(),
                arg_types: vec![],
                is_dynamic: false,
                result_type: "Массив<Число>".to_string(),
                generic_params: Some(vec!["Число".to_string()]),
            },
            span: Span::new(1, 0, 1, 20),
            scope_id: program.symbols.root_scope,
        });

        if let SemanticNodeKind::NewExpression {
            result_type,
            generic_params,
            ..
        } = &program.nodes[0].kind
        {
            assert_eq!(result_type, "Массив<Число>");
            assert!(generic_params.is_some());
            let params = generic_params.as_ref().unwrap();
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], "Число");
        } else {
            panic!("Expected NewExpression node");
        }
    }

    #[test]
    fn test_new_expression_to_dto() {
        let mut program = SemanticProgram::new();

        // Добавляем NewExpression узел
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::NewExpression {
                type_name: "Массив".to_string(),
                arg_types: vec!["Число".to_string()],
                is_dynamic: false,
                result_type: "Массив".to_string(),
                generic_params: None,
            },
            span: Span::new(1, 0, 1, 16),
            scope_id: program.symbols.root_scope,
        });

        // Конвертируем в DTO
        let dto = program.to_dto(false, false);

        assert_eq!(dto.root_nodes.len(), 1);

        let node_dto = &dto.root_nodes[0];
        assert_eq!(node_dto.kind, "NewExpression");
        assert!(node_dto.name.is_some());

        // Проверяем атрибуты
        assert_eq!(
            node_dto.attributes.get("type_name"),
            Some(&"Массив".to_string())
        );
        assert_eq!(node_dto.attributes.get("arg_count"), Some(&"1".to_string()));
        assert_eq!(
            node_dto.attributes.get("is_dynamic"),
            Some(&"false".to_string())
        );
    }

    // === НОВЫЕ ТЕСТЫ: Unit тестирование методов инкапсуляции SymbolTable (Milestone 2.19) ===

    #[test]
    fn test_lookup_variable_found() {
        let mut table = SymbolTable::new();

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        let hint = table.lookup_variable(table.root_scope, "x");
        assert!(hint.is_some());

        assert_eq!(hint.unwrap().type_name(), "Число");
    }

    #[test]
    fn test_lookup_variable_not_found() {
        let table = SymbolTable::new();

        let hint = table.lookup_variable(table.root_scope, "nonexistent");
        assert!(hint.is_none());
    }

    #[test]
    fn test_lookup_variable_in_hierarchy_finds_in_current_scope() {
        let mut table = SymbolTable::new();

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Строка"),
            Span::stub(),
        );

        let result = table.lookup_variable_in_hierarchy(table.root_scope, "x");
        assert!(result.is_some());

        let (scope_id, hint) = result.unwrap();
        assert_eq!(scope_id, table.root_scope);
        assert_eq!(hint.type_name(), "Строка");
    }

    #[test]
    fn test_lookup_variable_in_hierarchy_finds_in_parent_scope() {
        let mut table = SymbolTable::new();

        // Регистрируем в root scope
        table.register_variable(
            table.root_scope,
            "global_var".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        // Создаём child scope
        let child = table.create_scope(table.root_scope);

        // Ищем из child scope
        let result = table.lookup_variable_in_hierarchy(child, "global_var");
        assert!(result.is_some());

        let (found_scope, hint) = result.unwrap();
        assert_eq!(found_scope, table.root_scope);
        assert_eq!(hint.type_name(), "Число");
    }

    #[test]
    fn test_lookup_variable_in_hierarchy_shadow_local_over_parent() {
        let mut table = SymbolTable::new();

        // Регистрируем в root scope
        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        // Создаём child scope
        let child = table.create_scope(table.root_scope);

        // Регистрируем с тем же именем в child scope
        table.register_variable(
            child,
            "x".to_string(),
            TypeResolution::explicit("Строка"),
            Span::stub(),
        );

        // Ищем из child scope - должны найти локальную переменную
        let result = table.lookup_variable_in_hierarchy(child, "x");
        assert!(result.is_some());

        let (found_scope, hint) = result.unwrap();
        assert_eq!(found_scope, child);
        assert_eq!(hint.type_name(), "Строка");
    }

    #[test]
    fn test_lookup_variable_in_hierarchy_deeply_nested() {
        let mut table = SymbolTable::new();

        // Создаём иерархию scope: root -> level1 -> level2 -> level3
        let level1 = table.create_scope(table.root_scope);
        let level2 = table.create_scope(level1);
        let level3 = table.create_scope(level2);

        // Регистрируем переменную в level1
        table.register_variable(
            level1,
            "mid_var".to_string(),
            TypeResolution::explicit("Булево"),
            Span::stub(),
        );

        // Ищем из level3
        let result = table.lookup_variable_in_hierarchy(level3, "mid_var");
        assert!(result.is_some());

        let (found_scope, _) = result.unwrap();
        assert_eq!(found_scope, level1);
    }

    #[test]
    fn test_lookup_variable_in_hierarchy_not_found() {
        let mut table = SymbolTable::new();
        let child = table.create_scope(table.root_scope);

        let result = table.lookup_variable_in_hierarchy(child, "nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_update_variable_type_checked_success() {
        let mut table = SymbolTable::new();

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        // Обновляем тип
        let result = table.update_variable_type_checked(
            table.root_scope,
            "x",
            TypeResolution::explicit("Строка"),
        );

        assert!(result.is_ok());

        // Проверяем что тип обновлён
        let hint = table.lookup_variable(table.root_scope, "x");
        assert!(hint.is_some());
        assert_eq!(hint.unwrap().type_name(), "Строка");
    }

    #[test]
    fn test_update_variable_type_checked_invalid_scope() {
        let mut table = SymbolTable::new();

        let invalid_scope = ScopeId(9999);

        let result = table.update_variable_type_checked(
            invalid_scope,
            "x",
            TypeResolution::explicit("Число"),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Scope"));
    }

    #[test]
    fn test_update_variable_type_checked_nonexistent_variable() {
        let mut table = SymbolTable::new();

        let result = table.update_variable_type_checked(
            table.root_scope,
            "nonexistent",
            TypeResolution::explicit("Число"),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_has_variable_true() {
        let mut table = SymbolTable::new();

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        assert!(table.has_variable(table.root_scope, "x"));
    }

    #[test]
    fn test_has_variable_false() {
        let table = SymbolTable::new();

        assert!(!table.has_variable(table.root_scope, "nonexistent"));
    }

    #[test]
    fn test_has_variable_different_scope() {
        let mut table = SymbolTable::new();
        let child = table.create_scope(table.root_scope);

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        // has_variable проверяет только конкретный scope, не иерархию
        assert!(table.has_variable(table.root_scope, "x"));
        assert!(!table.has_variable(child, "x"));
    }

    #[test]
    fn test_find_function_found() {
        let mut table = SymbolTable::new();

        let sig = FunctionSignature {
            name: "МояФункция".to_string(),
            params: vec![],
            return_type: Some("Число".to_string()),
            is_export: false,
        };

        table.register_function(sig.clone());

        let found = table.find_function("МояФункция");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "МояФункция");
        assert_eq!(found.unwrap().return_type, Some("Число".to_string()));
    }

    #[test]
    fn test_find_function_not_found() {
        let table = SymbolTable::new();

        let found = table.find_function("NonExistentFunction");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_function_case_insensitive() {
        let mut table = SymbolTable::new();

        let sig = FunctionSignature {
            name: "МояФункция".to_string(),
            params: vec![],
            return_type: Some("Число".to_string()),
            is_export: false,
        };

        table.register_function(sig);

        // Проверяем что функции регистрируются с оригинальным именем
        let found = table.find_function("МояФункция");
        assert!(found.is_some());
    }

    #[test]
    fn test_find_procedure_found() {
        let mut table = SymbolTable::new();

        let sig = FunctionSignature {
            name: "МояПроцедура".to_string(),
            params: vec![],
            return_type: None,
            is_export: false,
        };

        table.register_procedure(sig.clone());

        let found = table.find_procedure("МояПроцедура");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "МояПроцедура");
        assert_eq!(found.unwrap().return_type, None);
    }

    #[test]
    fn test_find_procedure_not_found() {
        let table = SymbolTable::new();

        let found = table.find_procedure("NonExistentProcedure");
        assert!(found.is_none());
    }

    #[test]
    fn test_get_parent_scope_root() {
        let table = SymbolTable::new();

        let parent = table.get_parent_scope(table.root_scope);
        assert_eq!(parent, None);
    }

    #[test]
    fn test_get_parent_scope_child() {
        let mut table = SymbolTable::new();
        let child = table.create_scope(table.root_scope);

        let parent = table.get_parent_scope(child);
        assert_eq!(parent, Some(table.root_scope));
    }

    #[test]
    fn test_get_parent_scope_grandchild() {
        let mut table = SymbolTable::new();
        let child = table.create_scope(table.root_scope);
        let grandchild = table.create_scope(child);

        let parent = table.get_parent_scope(grandchild);
        assert_eq!(parent, Some(child));
    }

    #[test]
    fn test_get_parent_scope_invalid() {
        let table = SymbolTable::new();

        let parent = table.get_parent_scope(ScopeId(9999));
        assert_eq!(parent, None);
    }

    #[test]
    fn test_variables_in_scope_empty() {
        let table = SymbolTable::new();

        let vars = table.variables_in_scope(table.root_scope);
        assert!(vars.is_some());

        let count: usize = vars.unwrap().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_variables_in_scope_single() {
        let mut table = SymbolTable::new();

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        let vars = table.variables_in_scope(table.root_scope);
        assert!(vars.is_some());

        let collected: Vec<_> = vars.unwrap().collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].0, "x");
    }

    #[test]
    fn test_variables_in_scope_multiple() {
        let mut table = SymbolTable::new();

        table.register_variable(
            table.root_scope,
            "x".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        table.register_variable(
            table.root_scope,
            "y".to_string(),
            TypeResolution::explicit("Строка"),
            Span::stub(),
        );

        table.register_variable(
            table.root_scope,
            "z".to_string(),
            TypeResolution::explicit("Булево"),
            Span::stub(),
        );

        let vars = table.variables_in_scope(table.root_scope);
        assert!(vars.is_some());

        let collected: Vec<_> = vars.unwrap().collect();
        assert_eq!(collected.len(), 3);
    }

    #[test]
    fn test_variables_in_scope_does_not_include_parent_scope() {
        let mut table = SymbolTable::new();

        // Регистрируем в root scope
        table.register_variable(
            table.root_scope,
            "global".to_string(),
            TypeResolution::explicit("Число"),
            Span::stub(),
        );

        // Создаём child scope и регистрируем переменную там
        let child = table.create_scope(table.root_scope);
        table.register_variable(
            child,
            "local".to_string(),
            TypeResolution::explicit("Строка"),
            Span::stub(),
        );

        // Проверяем что в child scope только одна переменная
        let vars = table.variables_in_scope(child);
        assert!(vars.is_some());

        let collected: Vec<_> = vars.unwrap().collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].0, "local");
    }

    #[test]
    fn test_variables_in_scope_invalid_scope() {
        let table = SymbolTable::new();

        let vars = table.variables_in_scope(ScopeId(9999));
        assert!(vars.is_none());
    }
}
