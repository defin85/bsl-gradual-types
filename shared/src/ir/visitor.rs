//! Visitor паттерн для обхода SemanticProgram
//!
//! Позволяет выполнять различные анализы над IR без изменения самой структуры.

use std::collections::HashMap;
use std::collections::HashSet;

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
    let child_indices: HashSet<usize> = program
        .nodes
        .iter()
        .flat_map(direct_child_indices)
        .collect();

    // Обходим только root-level узлы (избегаем двойного обхода вложенных узлов)
    for (idx, node) in program.nodes.iter().enumerate() {
        if node.scope_id == program.symbols.root_scope && !child_indices.contains(&idx) {
            walk_node(node, visitor, &mut context, program);
        }
    }
}

fn direct_child_indices(node: &SemanticNode) -> Vec<usize> {
    match &node.kind {
        SemanticNodeKind::VariableDeclaration {
            initial_value_node, ..
        } => initial_value_node.iter().copied().collect(),
        SemanticNodeKind::Assignment { value_node, .. }
        | SemanticNodeKind::Return { value_node } => value_node.iter().copied().collect(),
        SemanticNodeKind::BinaryExpression {
            left_node,
            right_node,
            ..
        } => left_node
            .iter()
            .copied()
            .chain(right_node.iter().copied())
            .collect(),
        SemanticNodeKind::UnaryExpression { operand_node, .. } => {
            operand_node.iter().copied().collect()
        }
        SemanticNodeKind::TernaryExpression {
            condition_node,
            then_node,
            else_node,
        } => condition_node
            .iter()
            .copied()
            .chain(then_node.iter().copied())
            .chain(else_node.iter().copied())
            .collect(),
        SemanticNodeKind::AwaitExpression { expression_node }
        | SemanticNodeKind::AwaitStatement { expression_node } => {
            expression_node.iter().copied().collect()
        }
        SemanticNodeKind::FunctionDeclaration { body, .. }
        | SemanticNodeKind::ProcedureDeclaration { body, .. }
        | SemanticNodeKind::BlockScope {
            statements: body, ..
        } => body.clone(),
        SemanticNodeKind::IfStatement {
            condition_node,
            then_branch,
            else_branch,
        } => {
            let mut indices: Vec<usize> = condition_node.iter().copied().collect();
            indices.extend(then_branch.iter().copied());
            if let Some(else_branch) = else_branch {
                indices.extend(else_branch.iter().copied());
            }
            indices
        }
        SemanticNodeKind::WhileLoop {
            condition_node,
            body,
        }
        | SemanticNodeKind::ForEachLoop {
            collection_node: condition_node,
            body,
            ..
        } => condition_node
            .iter()
            .copied()
            .chain(body.iter().copied())
            .collect(),
        SemanticNodeKind::ForLoop {
            start_node,
            end_node,
            body,
            ..
        } => start_node
            .iter()
            .copied()
            .chain(end_node.iter().copied())
            .chain(body.iter().copied())
            .collect(),
        SemanticNodeKind::TryExcept {
            try_body,
            except_body,
        } => try_body
            .iter()
            .copied()
            .chain(except_body.iter().copied())
            .collect(),
        SemanticNodeKind::FunctionCall {
            object_node,
            arg_nodes,
            ..
        } => object_node
            .iter()
            .copied()
            .chain(arg_nodes.iter().flatten().copied())
            .collect(),
        SemanticNodeKind::MemberAccess { object_node, .. } => object_node.iter().copied().collect(),
        SemanticNodeKind::IndexAccess {
            object_node,
            index_node,
            ..
        } => object_node
            .iter()
            .copied()
            .chain(index_node.iter().copied())
            .collect(),
        SemanticNodeKind::NewExpression { arg_nodes, .. } => {
            arg_nodes.iter().flatten().copied().collect()
        }
        SemanticNodeKind::ExecuteStatement { code_node } => code_node.iter().copied().collect(),
        SemanticNodeKind::RaiseErrorStatement { message_node } => {
            message_node.iter().copied().collect()
        }
        SemanticNodeKind::AddHandlerStatement {
            event_node,
            handler_node,
        }
        | SemanticNodeKind::RemoveHandlerStatement {
            event_node,
            handler_node,
        } => event_node
            .iter()
            .copied()
            .chain(handler_node.iter().copied())
            .collect(),
        SemanticNodeKind::VariableAccess { .. }
        | SemanticNodeKind::StringLiteral { .. }
        | SemanticNodeKind::NumberLiteral { .. }
        | SemanticNodeKind::BooleanLiteral { .. }
        | SemanticNodeKind::DateLiteral { .. }
        | SemanticNodeKind::NullLiteral
        | SemanticNodeKind::UndefinedLiteral
        | SemanticNodeKind::GlobalPropertyAccess { .. }
        | SemanticNodeKind::Break
        | SemanticNodeKind::Continue => Vec::new(),
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

        SemanticNodeKind::UnaryExpression {
            operand_node: Some(operand_idx),
            ..
        } => {
            if let Some(child_node) = program.nodes.get(*operand_idx) {
                walk_node(child_node, visitor, context, program);
            }
        }
        SemanticNodeKind::UnaryExpression {
            operand_node: None, ..
        } => {}

        SemanticNodeKind::TernaryExpression {
            condition_node,
            then_node,
            else_node,
        } => {
            for child_idx in condition_node
                .iter()
                .chain(then_node.iter())
                .chain(else_node.iter())
            {
                if let Some(child_node) = program.nodes.get(*child_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
        }

        SemanticNodeKind::AwaitExpression { expression_node }
        | SemanticNodeKind::AwaitStatement { expression_node } => {
            if let Some(expression_idx) = expression_node {
                if let Some(child_node) = program.nodes.get(*expression_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
        }

        SemanticNodeKind::IfStatement {
            condition_node,
            then_branch,
            else_branch,
        } => {
            if let Some(condition_idx) = condition_node {
                if let Some(child_node) = program.nodes.get(*condition_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }

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

        SemanticNodeKind::WhileLoop {
            condition_node,
            body,
            ..
        } => {
            if let Some(condition_idx) = condition_node {
                if let Some(child_node) = program.nodes.get(*condition_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
            for &node_idx in body {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
        }

        SemanticNodeKind::ForLoop {
            start_node,
            end_node,
            body,
            ..
        } => {
            for child_idx in start_node.iter().chain(end_node.iter()) {
                if let Some(child_node) = program.nodes.get(*child_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
            for &node_idx in body {
                if let Some(child_node) = program.nodes.get(node_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
        }

        SemanticNodeKind::ForEachLoop {
            collection_node,
            body,
            ..
        } => {
            if let Some(collection_idx) = collection_node {
                if let Some(child_node) = program.nodes.get(*collection_idx) {
                    walk_node(child_node, visitor, context, program);
                }
            }
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

        SemanticNodeKind::ExecuteStatement {
            code_node: Some(code_idx),
        } => {
            if let Some(child_node) = program.nodes.get(*code_idx) {
                walk_node(child_node, visitor, context, program);
            }
        }
        SemanticNodeKind::ExecuteStatement { code_node: None } => {}

        SemanticNodeKind::RaiseErrorStatement {
            message_node: Some(message_idx),
        } => {
            if let Some(child_node) = program.nodes.get(*message_idx) {
                walk_node(child_node, visitor, context, program);
            }
        }
        SemanticNodeKind::RaiseErrorStatement { message_node: None } => {}

        SemanticNodeKind::AddHandlerStatement {
            event_node,
            handler_node,
        }
        | SemanticNodeKind::RemoveHandlerStatement {
            event_node,
            handler_node,
        } => {
            for child_idx in event_node.iter().chain(handler_node.iter()) {
                if let Some(child_node) = program.nodes.get(*child_idx) {
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
#[path = "visitor/tests.rs"]
mod tests;
