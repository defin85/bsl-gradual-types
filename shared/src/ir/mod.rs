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

pub use visitor::{FlowContext, SemanticVisitor, walk_program};

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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
    },

    /// Объявление функции
    FunctionDeclaration {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<String>,
        body_scope: ScopeId,
    },

    /// Объявление процедуры
    ProcedureDeclaration {
        name: String,
        params: Vec<Parameter>,
        body_scope: ScopeId,
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
    Return {
        value_type: Option<String>,
    },

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
    MemberAccess {
        object_type: String,
        member_name: String,
        is_method: bool, // true = метод, false = свойство
    },

    /// Вызов функции/метода
    FunctionCall {
        function_name: String,
        arg_types: Vec<String>,
        object_type: Option<String>, // Some если это метод объекта
    },

    // === Scope tracking ===

    /// Блок scope
    BlockScope {
        statements: Vec<usize>, // Индексы SemanticNode
        scope_id: ScopeId,
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
    pub variables: HashMap<String, (TypeHint, Span)>,

    /// Дочерние scope-ы
    pub children: Vec<ScopeId>,
}

/// Идентификатор scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopeId(pub usize);

/// Подсказка типа переменной
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeHint {
    /// Явно указанный тип: `Перем x: Число`
    Explicit(String),

    /// Выведенный из значения: `Перем x = 42`
    Inferred(String),

    /// Тип неизвестен
    Unknown,
}

/// Сигнатура функции/процедуры
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_export: bool,
}

/// Позиция в исходном коде
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    Statement {
        semantic_node_id: usize,
    },

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
        self.nodes.iter()
            .find(|node| node.span.contains(line, column))
    }

    /// Получить scope по ID
    pub fn get_scope(&self, scope_id: ScopeId) -> Option<&Scope> {
        self.symbols.scopes.get(&scope_id)
    }

    /// Получить переменную в scope (с поиском в родительских scope)
    ///
    /// Поиск идёт от текущего scope вверх по иерархии до root
    pub fn resolve_variable(&self, name: &str, scope_id: ScopeId) -> Option<(TypeHint, Span)> {
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
    /// 5. Вернуть (имя_переменной, TypeHint)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use bsl_shared::ir::{SemanticProgram, TypeHint};
    ///
    /// let program = SemanticProgram::new();
    /// // Предположим, в программе есть: МассивДанных = Новый Массив;
    /// if let Some((var_name, type_hint)) = program.find_variable_at_position(5, 10) {
    ///     match type_hint {
    ///         TypeHint::Inferred(type_name) => {
    ///             assert_eq!(var_name, "МассивДанных");
    ///             assert_eq!(type_name, "Массив");
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn find_variable_at_position(&self, line: u32, column: u32) -> Option<(String, TypeHint)> {
        // 1. Находим узел в позиции
        let node = self.find_node_at_position(line, column)?;

        // 2. Получаем scope узла
        let scope_id = node.scope_id;

        // 3. Извлекаем имя переменной из узла
        let var_name = match &node.kind {
            // Присваивание: МассивДанных = Новый Массив
            SemanticNodeKind::Assignment { variable, .. } => variable.clone(),

            // Доступ к члену: МассивДанных.Добавить
            SemanticNodeKind::MemberAccess { object_type, .. } => {
                // object_type может быть именем переменной или типом
                // Попробуем найти как переменную в scope
                object_type.clone()
            }

            // Объявление переменной: Перем МассивДанных
            SemanticNodeKind::VariableDeclaration { name, .. } => name.clone(),

            // Вызов функции: может быть вызов метода переменной
            SemanticNodeKind::FunctionCall { object_type, .. } => {
                if let Some(obj_type) = object_type {
                    obj_type.clone()
                } else {
                    // Обычный вызов функции, не метод переменной
                    return None;
                }
            }

            // Для остальных узлов не поддерживаем
            _ => return None,
        };

        // 4. Ищем переменную в scope (с поиском в родительских scope)
        let (type_hint, _span) = self.resolve_variable(&var_name, scope_id)?;

        // 5. Возвращаем результат
        Some((var_name, type_hint))
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
        scopes.insert(root_scope, Scope {
            id: root_scope,
            parent: None,
            variables: HashMap::new(),
            children: Vec::new(),
        });

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
    pub fn register_variable(&mut self, scope_id: ScopeId, name: String, hint: TypeHint, span: Span) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.variables.insert(name, (hint, span));
        }
    }

    /// Зарегистрировать глобальную функцию
    pub fn register_function(&mut self, signature: FunctionSignature) {
        self.global_functions.insert(signature.name.clone(), signature);
    }

    /// Зарегистрировать глобальную процедуру
    pub fn register_procedure(&mut self, signature: FunctionSignature) {
        self.global_procedures.insert(signature.name.clone(), signature);
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
            TypeHint::Explicit("Число".to_string()),
            Span::stub()
        );

        // Регистрируем переменную в child scope
        program.symbols.register_variable(
            child_scope,
            "localVar".to_string(),
            TypeHint::Explicit("Строка".to_string()),
            Span::stub()
        );

        // Поиск в child scope должен найти обе переменные
        assert!(program.resolve_variable("localVar", child_scope).is_some());
        assert!(program.resolve_variable("globalVar", child_scope).is_some());

        // Поиск в root scope должен найти только globalVar
        assert!(program.resolve_variable("globalVar", program.symbols.root_scope).is_some());
        assert!(program.resolve_variable("localVar", program.symbols.root_scope).is_none());
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
            },
            span: Span::new(2, 0, 2, 10),
            scope_id: program.symbols.root_scope,
        });

        // Поиск первого узла
        let node = program.find_node_at_position(1, 5);
        assert!(node.is_some());
        assert!(matches!(node.unwrap().kind, SemanticNodeKind::VariableDeclaration { .. }));

        // Поиск второго узла
        let node = program.find_node_at_position(2, 5);
        assert!(node.is_some());
        assert!(matches!(node.unwrap().kind, SemanticNodeKind::Assignment { .. }));

        // Поиск вне узлов
        assert!(program.find_node_at_position(10, 5).is_none());
    }
}
