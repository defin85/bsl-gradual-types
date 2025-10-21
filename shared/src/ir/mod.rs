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
        self.nodes
            .iter()
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
        hint: TypeHint,
        span: Span,
    ) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.variables.insert(name, (hint, span));
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
            } => {
                attributes.insert("variable".to_string(), variable.clone());
                attributes.insert("value_type".to_string(), value_type.clone());
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
                arg_types,
                ..
            } => {
                attributes.insert("function_name".to_string(), function_name.clone());
                attributes.insert("arg_count".to_string(), arg_types.len().to_string());
                (
                    "FunctionCall".to_string(),
                    Some(function_name.clone()),
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
                object_type,
                member_name,
                is_method,
            } => {
                attributes.insert("object_type".to_string(), object_type.clone());
                attributes.insert("member_name".to_string(), member_name.clone());
                attributes.insert("is_method".to_string(), is_method.to_string());
                (
                    "MemberAccess".to_string(),
                    Some(member_name.clone()),
                    attributes,
                )
            }
            SemanticNodeKind::BlockScope { .. } => ("BlockScope".to_string(), None, attributes),
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
    fn symbols_to_dto(&self, include_flow_sensitive: bool) -> HashMap<String, SymbolInfoDto> {
        let mut result = HashMap::new();

        // Обходим все scopes
        for (_scope_id, scope) in &self.symbols.scopes {
            for (var_name, (type_hint, span)) in &scope.variables {
                let symbol = SymbolInfoDto {
                    name: var_name.clone(),
                    kind: "Variable".to_string(),
                    resolved_type: self.type_hint_to_dto(type_hint),
                    scope: "Local".to_string(), // TODO: различать Global/Local
                    declaration_location: SourceLocationDto {
                        line: span.start_line,
                        column: span.start_column,
                    },
                    flow_variants: if include_flow_sensitive {
                        Vec::new() // TODO: flow-sensitive analysis
                    } else {
                        Vec::new()
                    },
                    metadata: HashMap::new(),
                };

                result.insert(var_name.clone(), symbol);
            }
        }

        // Добавляем функции
        for (fn_name, sig) in &self.symbols.global_functions {
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

    /// Конвертировать TypeHint в TypeResolutionDto
    fn type_hint_to_dto(&self, hint: &TypeHint) -> Option<TypeResolutionDto> {
        match hint {
            TypeHint::Explicit(type_name) => Some(TypeResolutionDto {
                name: type_name.clone(),
                category: "Platform".to_string(),
                certainty: "Known".to_string(),
                certainty_percent: 100,
                active_facet: None,
                methods: Vec::new(),
                properties: Vec::new(),
                is_union: None,
                union_components: Vec::new(),
            }),
            TypeHint::Inferred(type_name) => Some(TypeResolutionDto {
                name: type_name.clone(),
                category: "Inferred".to_string(),
                certainty: "Inferred".to_string(),
                certainty_percent: 75,
                active_facet: None,
                methods: Vec::new(),
                properties: Vec::new(),
                is_union: None,
                union_components: Vec::new(),
            }),
            TypeHint::Unknown => None,
        }
    }

    /// Извлечь граф вызовов функций
    fn extract_call_graph(&self) -> Vec<CallEdgeDto> {
        let edges = Vec::new();

        // TODO: Реализовать извлечение call graph из узлов
        // Для MVP возвращаем пустой граф

        edges
    }

    /// Вычислить метрики семантического анализа
    fn calculate_metrics(&self) -> SemanticMetricsDto {
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
        for (_scope_id, scope) in &self.symbols.scopes {
            for (_var_name, (type_hint, _span)) in &scope.variables {
                match type_hint {
                    TypeHint::Explicit(_) => known_types += 1,
                    TypeHint::Inferred(_) => inferred_types += 1,
                    TypeHint::Unknown => unknown_types += 1,
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
            TypeHint::Explicit("Число".to_string()),
            Span::stub(),
        );

        // Регистрируем переменную в child scope
        program.symbols.register_variable(
            child_scope,
            "localVar".to_string(),
            TypeHint::Explicit("Строка".to_string()),
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
}
