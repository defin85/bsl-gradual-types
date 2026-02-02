//! Visitor паттерн для обхода SemanticProgram
//!
//! Позволяет выполнять различные анализы над IR без изменения самой структуры.

use std::collections::HashMap;

use super::*;

/// Контекст для flow-sensitive анализа
///
/// Отслеживает текущее состояние анализа при обходе дерева
///
/// # Phase 4: VariableState для variable_states
///
/// `variable_states` теперь хранит `VariableState` вместо `TypeResolution`,
/// что позволяет отслеживать:
/// - Certainty (уверенность в типе) через resolution
/// - ResolutionSource (откуда пришёл тип)
/// - initialized флаг (инициализирована ли переменная)
/// - declaration_span (позиция объявления)
pub struct FlowContext {
    /// Текущий scope
    pub current_scope: ScopeId,

    /// Состояния переменных в текущей точке выполнения
    /// (Phase 4: VariableState вместо TypeResolution для flow-sensitive анализа)
    pub variable_states: HashMap<String, VariableState>,

    /// Путь выполнения (для branch-aware analysis)
    pub execution_path: Vec<CfgNodeId>,
}

impl FlowContext {
    /// Создать новый контекст для root scope
    pub fn new(root_scope: ScopeId) -> Self {
        Self {
            current_scope: root_scope,
            variable_states: HashMap::new(),
            execution_path: Vec::new(),
        }
    }

    /// Обновить состояние переменной в текущей точке
    /// Phase 4: Теперь принимает VariableState вместо TypeResolution
    pub fn update_variable_state(&mut self, name: String, state: VariableState) {
        self.variable_states.insert(name, state);
    }

    /// Получить состояние переменной в текущей точке
    /// Phase 4: Теперь возвращает VariableState вместо TypeResolution
    pub fn get_variable_state(&self, name: &str) -> Option<&VariableState> {
        self.variable_states.get(name)
    }

    /// Проверить, объявлена ли переменная в текущем контексте.
    pub fn is_declared(&self, name: &str) -> bool {
        self.variable_states.contains_key(name)
    }

    /// Проверить, инициализирована ли переменная
    pub fn is_initialized(&self, name: &str) -> bool {
        self.variable_states
            .get(name)
            .map(|state| state.initialized)
            .unwrap_or(false)
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
///         // Собираем все явные type hints из переменных
///         match &node.kind {
///             bsl_shared::ir::SemanticNodeKind::VariableDeclaration { type_hint, .. } => {
///                 if let Some(resolution) = type_hint {
///                     self.types.push(resolution.clone());
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
            name,
            initial_value_node,
            ..
        } => {
            let initialized = initial_value_node.is_some();
            let state = VariableState::new(node.span, initialized);
            context.update_variable_state(name.clone(), state);
        }

        SemanticNodeKind::Assignment {
            variable,
            value_node,
            ..
        } => {
            let updated = match context.variable_states.get(variable).cloned() {
                Some(mut state) => {
                    state.mark_initialized();
                    state
                }
                None => VariableState::initialized(node.span),
            };
            context.update_variable_state(variable.clone(), updated);
            let _ = value_node;
        }

        SemanticNodeKind::IfStatement {
            then_branch,
            else_branch,
            ..
        } => {
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
            ..
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
        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableDeclaration {
                name: "x".to_string(),
                type_hint: Some("Число".to_string()),
                is_export: false,
                initial_value_node: None,
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::Assignment {
                variable: "x".to_string(),
                value_node: None,
                value_span: Span::stub(),
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::FunctionCall {
                function_name: "Сообщить".to_string(),
                object_name: None,
                object_node: None,
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
    fn test_flow_context_tracks_initialization() {
        let mut program = SemanticProgram::new();

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableDeclaration {
                name: "x".to_string(),
                type_hint: Some("Число".to_string()),
                is_export: false,
                initial_value_node: None,
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableAccess {
                name: "x".to_string(),
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::Assignment {
                variable: "x".to_string(),
                value_node: None,
                value_span: Span::stub(),
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        program.nodes.push(SemanticNode {
            kind: SemanticNodeKind::VariableAccess {
                name: "x".to_string(),
            },
            span: Span::stub(),
            scope_id: program.symbols.root_scope,
        });

        // Visitor для отслеживания инициализации
        struct InitTracker {
            initialized_on_access: Vec<bool>,
        }

        impl SemanticVisitor for InitTracker {
            fn visit_node(&mut self, node: &SemanticNode, context: &mut FlowContext) {
                let SemanticNodeKind::VariableAccess { name } = &node.kind else {
                    return;
                };
                if name.eq_ignore_ascii_case("x") {
                    self.initialized_on_access.push(context.is_initialized("x"));
                }
            }
        }

        let mut tracker = InitTracker {
            initialized_on_access: Vec::new(),
        };
        walk_program(&program, &mut tracker);

        assert_eq!(tracker.initialized_on_access, vec![false, true]);
    }
}
