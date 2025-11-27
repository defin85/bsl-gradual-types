//! Visitor паттерн для обхода SemanticProgram
//!
//! Позволяет выполнять различные анализы над IR без изменения самой структуры.

use super::*;
use crate::domain::types::TypeResolution;

/// Контекст для flow-sensitive анализа
///
/// Отслеживает текущее состояние анализа при обходе дерева
///
/// # Phase 3: TypeResolution для variable_types
///
/// `variable_types` теперь хранит `TypeResolution` вместо `String`,
/// что позволяет отслеживать:
/// - Certainty (уверенность в типе)
/// - ResolutionSource (откуда пришёл тип)
/// - Unknown типы (для graceful degradation)
pub struct FlowContext {
    /// Текущий scope
    pub current_scope: ScopeId,

    /// Типы переменных в текущей точке выполнения
    /// (Phase 3: TypeResolution вместо String для flow-sensitive анализа)
    pub variable_types: HashMap<String, TypeResolution>,

    /// Путь выполнения (для branch-aware analysis)
    pub execution_path: Vec<CfgNodeId>,
}

impl FlowContext {
    /// Создать новый контекст для root scope
    pub fn new(root_scope: ScopeId) -> Self {
        Self {
            current_scope: root_scope,
            variable_types: HashMap::new(),
            execution_path: Vec::new(),
        }
    }

    /// Обновить тип переменной в текущей точке
    /// Phase 3: Теперь принимает TypeResolution вместо String
    pub fn update_variable_type(&mut self, name: String, resolution: TypeResolution) {
        self.variable_types.insert(name, resolution);
    }

    /// Получить тип переменной в текущей точке
    /// Phase 3: Теперь возвращает TypeResolution вместо String
    pub fn get_variable_type(&self, name: &str) -> Option<&TypeResolution> {
        self.variable_types.get(name)
    }

    /// Войти в новый scope
    pub fn enter_scope(&mut self, scope_id: ScopeId) {
        self.current_scope = scope_id;
    }

    /// Выйти из scope (вернуться к родительскому)
    pub fn exit_scope(&mut self, parent: Option<ScopeId>) {
        if let Some(parent_id) = parent {
            self.current_scope = parent_id;
        }
    }
}

/// Visitor для обхода семантического дерева
///
/// # Примеры
///
/// ```
/// use bsl_shared::ir::{SemanticVisitor, SemanticNode, FlowContext};
///
/// struct TypeCollector {
///     types: Vec<String>,
/// }
///
/// impl SemanticVisitor for TypeCollector {
///     fn visit_node(&mut self, node: &SemanticNode, context: &mut FlowContext) {
///         // Собираем все типы из переменных
///         // Phase 3: type_hint теперь Option<TypeResolution>
///         match &node.kind {
///             bsl_shared::ir::SemanticNodeKind::VariableDeclaration { type_hint, .. } => {
///                 if let Some(resolution) = type_hint {
///                     self.types.push(resolution.type_name());
///                 }
///             }
///             _ => {}
///         }
///     }
/// }
/// ```
pub trait SemanticVisitor {
    /// Посетить узел дерева
    fn visit_node(&mut self, node: &SemanticNode, context: &mut FlowContext);

    /// Посетить объявление переменной
    /// Phase 3: type_hint теперь Option<TypeResolution>
    fn visit_variable_declaration(
        &mut self,
        _name: &str,
        _type_hint: &Option<TypeResolution>,
        _context: &mut FlowContext,
    ) {
        // Default implementation — ничего не делаем
    }

    /// Посетить присваивание
    /// Phase 3: value_type теперь TypeResolution
    fn visit_assignment(&mut self, _variable: &str, _value_type: &TypeResolution, _value_node: Option<usize>, _context: &mut FlowContext) {
        // Default implementation
    }

    /// Посетить вызов функции
    /// Phase 3: args теперь Vec<TypeResolution>
    fn visit_function_call(
        &mut self,
        _function: &str,
        _args: &[TypeResolution],
        _context: &mut FlowContext,
    ) {
        // Default implementation
    }

    /// Посетить условный оператор
    /// Phase 3: condition_type теперь TypeResolution
    fn visit_if_statement(
        &mut self,
        _condition_type: &TypeResolution,
        _then_branch: &[usize],
        _else_branch: &Option<Vec<usize>>,
        _context: &mut FlowContext,
    ) {
        // Default implementation
    }

    /// Войти в scope
    fn enter_scope(&mut self, scope_id: ScopeId, context: &mut FlowContext) {
        context.enter_scope(scope_id);
    }

    /// Выйти из scope
    fn exit_scope(&mut self, parent: Option<ScopeId>, context: &mut FlowContext) {
        context.exit_scope(parent);
    }
}

/// Обход программы с visitor
///
/// # Примеры
///
/// ```
/// use bsl_shared::ir::{SemanticProgram, walk_program, SemanticVisitor};
///
/// struct MyVisitor;
///
/// impl SemanticVisitor for MyVisitor {
///     fn visit_node(&mut self, node: &bsl_shared::ir::SemanticNode, context: &mut bsl_shared::ir::FlowContext) {
///         // Анализ узла
///     }
/// }
///
/// let program = SemanticProgram::new();
/// let mut visitor = MyVisitor;
/// walk_program(&program, &mut visitor);
/// ```
pub fn walk_program<V: SemanticVisitor>(program: &SemanticProgram, visitor: &mut V) {
    let mut context = FlowContext::new(program.symbols.root_scope);

    // Обходим только root-level узлы (избегаем двойного обхода вложенных узлов)
    for node in &program.nodes {
        if node.scope_id == program.symbols.root_scope {
            walk_node(node, visitor, &mut context, program);
        }
    }
}

/// Рекурсивный обход узла
fn walk_node<V: SemanticVisitor>(
    node: &SemanticNode,
    visitor: &mut V,
    context: &mut FlowContext,
    program: &SemanticProgram,
) {
    // Сначала посещаем узел
    visitor.visit_node(node, context);

    // Затем обрабатываем специфичные типы узлов
    match &node.kind {
        SemanticNodeKind::VariableDeclaration {
            name, type_hint, ..
        } => {
            visitor.visit_variable_declaration(name, type_hint, context);

            // Обновляем контекст типов
            // Phase 3: type_hint уже TypeResolution, просто клонируем
            if let Some(resolution) = type_hint {
                context.update_variable_type(name.clone(), resolution.clone());
            }
        }

        SemanticNodeKind::Assignment {
            variable,
            value_type,
            value_node,
        } => {
            // Phase 3: value_type теперь TypeResolution
            visitor.visit_assignment(variable, value_type, *value_node, context);

            // Обновляем контекст типов (flow-sensitive)
            // Phase 3: context.update_variable_type теперь принимает TypeResolution
            context.update_variable_type(variable.clone(), value_type.clone());
        }

        SemanticNodeKind::FunctionCall {
            function_name,
            arg_types,
            ..
        } => {
            visitor.visit_function_call(function_name, arg_types, context);
        }

        SemanticNodeKind::IfStatement {
            condition_type,
            then_branch,
            else_branch,
        } => {
            visitor.visit_if_statement(condition_type, then_branch, else_branch, context);

            // Обходим then ветку
            for &node_idx in then_branch {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }

            // Обходим else ветку
            if let Some(else_nodes) = else_branch {
                for &node_idx in else_nodes {
                    if let Some(child_node) = program.nodes.get(node_idx) {
                        walk_node(child_node, visitor, context, program);
                    }
                }
            }
        }

        SemanticNodeKind::WhileLoop { body, .. }
        | SemanticNodeKind::ForLoop { body, .. }
        | SemanticNodeKind::ForEachLoop { body, .. } => {
            // Обходим тело цикла
            for &node_idx in body {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
        }

        SemanticNodeKind::TryExcept {
            try_body,
            except_body,
        } => {
            // Обходим try блок
            for &node_idx in try_body {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }

            // Обходим except блок
            for &node_idx in except_body {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
        }

        SemanticNodeKind::FunctionDeclaration { body_scope, .. }
        | SemanticNodeKind::ProcedureDeclaration { body_scope, .. } => {
            // Входим в scope функции/процедуры
            visitor.enter_scope(*body_scope, context);

            // Обходим тело (узлы с этим scope_id)
            for child_node in &program.nodes {
                if child_node.scope_id == *body_scope {
                    walk_node(child_node, visitor, context, program);
                }
            }

            // Выходим из scope
            if let Some(scope) = program.get_scope(*body_scope) {
                visitor.exit_scope(scope.parent, context);
            }
        }

        SemanticNodeKind::BlockScope {
            statements,
            scope_id,
        } => {
            // Входим в блок scope
            visitor.enter_scope(*scope_id, context);

            // Обходим statements
            for &node_idx in statements {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }

            // Выходим из scope
            if let Some(scope) = program.get_scope(*scope_id) {
                visitor.exit_scope(scope.parent, context);
            }
        }

        _ => {
            // Остальные узлы не имеют дочерних элементов
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Простой visitor для подсчёта узлов по типам
    struct NodeCounter {
        var_decls: usize,
        assignments: usize,
        function_calls: usize,
    }

    impl SemanticVisitor for NodeCounter {
        fn visit_node(&mut self, node: &SemanticNode, _context: &mut FlowContext) {
            match &node.kind {
                SemanticNodeKind::VariableDeclaration { .. } => {
                    self.var_decls += 1;
                }
                SemanticNodeKind::Assignment { .. } => {
                    self.assignments += 1;
                }
                SemanticNodeKind::FunctionCall { .. } => {
                    self.function_calls += 1;
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_visitor_counts_nodes() {
        let mut program = SemanticProgram::new();

        // Добавляем узлы
        // Phase 3: type_hint и initial_value_type теперь Option<TypeResolution>
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableDeclaration {
                name: "x".to_string(),
                type_hint: Some(TypeResolution::explicit("Число")),
                is_export: false,
                initial_value_type: None,
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::Assignment {
                variable: "x".to_string(),
                // Phase 3: value_type теперь TypeResolution
                value_type: TypeResolution::explicit("Число"),
                value_node: None,
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: "Сообщить".to_string(),
                object_name: None,
                object_type: None,
                // Phase 3: arg_types теперь Vec<TypeResolution>
                arg_types: vec![TypeResolution::explicit("Строка")],
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        // Обходим программу
        let mut counter = NodeCounter {
            var_decls: 0,
            assignments: 0,
            function_calls: 0,
        };

        walk_program(&program, &mut counter);

        // Проверяем результаты
        assert_eq!(counter.var_decls, 1);
        assert_eq!(counter.assignments, 1);
        assert_eq!(counter.function_calls, 1);
    }

    #[test]
    fn test_flow_context_tracks_types() {
        let mut program = SemanticProgram::new();

        // Phase 3: type_hint теперь Option<TypeResolution>
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableDeclaration {
                name: "x".to_string(),
                type_hint: Some(TypeResolution::explicit("Число")),
                is_export: false,
                initial_value_type: None,
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::Assignment {
                variable: "x".to_string(),
                // Phase 3: value_type теперь TypeResolution
                value_type: TypeResolution::explicit("Строка"), // Переприсваиваем другой тип
                value_node: None,
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        // Visitor для отслеживания типов
        struct TypeTracker {
            types_seen: Vec<String>,
        }

        impl SemanticVisitor for TypeTracker {
            fn visit_node(&mut self, _node: &SemanticNode, _context: &mut FlowContext) {
                // Не используем, отслеживаем только через visit_assignment
            }

            // Phase 3: value_type теперь TypeResolution
            fn visit_assignment(
                &mut self,
                variable: &str,
                value_type: &TypeResolution,
                _value_node: Option<usize>,
                _context: &mut FlowContext,
            ) {
                // Собираем типы после обновления контекста
                if variable == "x" {
                    self.types_seen.push(value_type.type_name());
                }
            }
        }

        let mut tracker = TypeTracker {
            types_seen: Vec::new(),
        };
        walk_program(&program, &mut tracker);

        // Должны увидеть оба типа: начальный Число и переприсвоенный Строка
        // Но visitor::visit_assignment вызывается только для Assignment узла
        assert_eq!(tracker.types_seen.len(), 1);
        assert_eq!(tracker.types_seen[0], "Строка");
    }
}
