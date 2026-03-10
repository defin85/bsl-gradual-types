use std::collections::HashSet;

use bsl_shared::ir::{SemanticNode, SemanticNodeKind, SemanticProgram, Span};
use bsl_syntax::ast::{Expression, Program, Statement};

pub(super) fn project_semantic_program(program: &SemanticProgram) -> Program {
    Projection { program }.project_program()
}

struct Projection<'a> {
    program: &'a SemanticProgram,
}

impl Projection<'_> {
    fn project_program(&self) -> Program {
        Program {
            statements: self
                .root_statement_indices()
                .into_iter()
                .filter_map(|idx| self.statement_from_node_index(idx))
                .collect(),
        }
    }

    fn root_statement_indices(&self) -> Vec<usize> {
        let root_scope = self.program.symbols.root_scope;
        let referenced = self.referenced_indices();
        let mut candidates: Vec<usize> = self
            .program
            .nodes
            .iter()
            .enumerate()
            .filter(|(idx, node)| {
                node.scope_id == root_scope
                    && !referenced.contains(idx)
                    && self.can_project_root_statement(node)
            })
            .map(|(idx, _)| idx)
            .collect();

        let snapshot = candidates.clone();
        candidates.retain(|&idx| {
            !snapshot.iter().any(|&other| {
                other != idx && self.root_candidate_strictly_contains(other, idx)
            })
        });
        candidates.sort_by_key(|&idx| {
            let span = self.program.nodes[idx].span;
            (span.start, span.end)
        });
        candidates
    }

    fn referenced_indices(&self) -> HashSet<usize> {
        self.program
            .nodes
            .iter()
            .flat_map(direct_child_indices)
            .collect()
    }

    fn root_candidate_strictly_contains(&self, outer_idx: usize, inner_idx: usize) -> bool {
        let outer = self.program.nodes[outer_idx].span;
        let inner = self.program.nodes[inner_idx].span;
        outer.start <= inner.start
            && inner.end <= outer.end
            && (outer.start < inner.start || inner.end < outer.end)
    }

    fn can_project_root_statement(&self, node: &SemanticNode) -> bool {
        matches!(
            node.kind,
            SemanticNodeKind::VariableDeclaration { .. }
                | SemanticNodeKind::Assignment { .. }
                | SemanticNodeKind::FunctionDeclaration { .. }
                | SemanticNodeKind::ProcedureDeclaration { .. }
                | SemanticNodeKind::IfStatement { .. }
                | SemanticNodeKind::WhileLoop { .. }
                | SemanticNodeKind::ForLoop { .. }
                | SemanticNodeKind::ForEachLoop { .. }
                | SemanticNodeKind::ExecuteStatement { .. }
                | SemanticNodeKind::RaiseErrorStatement { .. }
                | SemanticNodeKind::AddHandlerStatement { .. }
                | SemanticNodeKind::RemoveHandlerStatement { .. }
                | SemanticNodeKind::AwaitStatement { .. }
                | SemanticNodeKind::Return { .. }
                | SemanticNodeKind::Break
                | SemanticNodeKind::Continue
                | SemanticNodeKind::TryExcept { .. }
        ) || self.can_project_expression(node)
    }

    fn can_project_expression(&self, node: &SemanticNode) -> bool {
        matches!(
            node.kind,
            SemanticNodeKind::VariableAccess { .. }
                | SemanticNodeKind::StringLiteral { .. }
                | SemanticNodeKind::NumberLiteral { .. }
                | SemanticNodeKind::BooleanLiteral { .. }
                | SemanticNodeKind::DateLiteral { .. }
                | SemanticNodeKind::NullLiteral
                | SemanticNodeKind::UndefinedLiteral
                | SemanticNodeKind::GlobalPropertyAccess { .. }
                | SemanticNodeKind::MemberAccess { .. }
                | SemanticNodeKind::IndexAccess { .. }
                | SemanticNodeKind::FunctionCall { .. }
                | SemanticNodeKind::BinaryExpression { .. }
                | SemanticNodeKind::UnaryExpression { .. }
                | SemanticNodeKind::TernaryExpression { .. }
                | SemanticNodeKind::AwaitExpression { .. }
                | SemanticNodeKind::NewExpression { .. }
        )
    }

    fn statements_from_indices(&self, indices: &[usize]) -> Vec<Statement> {
        indices
            .iter()
            .filter_map(|idx| self.statement_from_node_index(*idx))
            .collect()
    }

    fn statement_from_node_index(&self, idx: usize) -> Option<Statement> {
        let node = self.program.nodes.get(idx)?;
        Some(match &node.kind {
            SemanticNodeKind::VariableDeclaration {
                name, type_hint, ..
            } => Statement::VarDeclaration {
                name: name.clone(),
                type_hint: type_hint.clone(),
                span: node.span,
            },
            SemanticNodeKind::Assignment {
                variable,
                value_node,
                value_span,
            } => Statement::Assignment {
                target: self.identifier_expression(
                    variable,
                    identifier_span_at_start(node.span, variable),
                ),
                value: self.expression_from_optional_node(*value_node, *value_span),
                span: node.span,
            },
            SemanticNodeKind::FunctionDeclaration {
                name,
                params,
                body,
                compiler_directive,
                ..
            } => Statement::FunctionDecl {
                name: name.clone(),
                params: params.iter().map(|param| param.name.clone()).collect(),
                body: self.statements_from_indices(body),
                compiler_directive: *compiler_directive,
                is_export: false,
                span: node.span,
            },
            SemanticNodeKind::ProcedureDeclaration {
                name,
                params,
                body,
                compiler_directive,
                ..
            } => Statement::ProcedureDecl {
                name: name.clone(),
                params: params.iter().map(|param| param.name.clone()).collect(),
                body: self.statements_from_indices(body),
                compiler_directive: *compiler_directive,
                is_export: false,
                span: node.span,
            },
            SemanticNodeKind::IfStatement {
                condition_node,
                then_branch,
                else_branch,
            } => Statement::If {
                condition: self.expression_from_optional_node(*condition_node, node.span),
                then_body: self.statements_from_indices(then_branch),
                else_body: else_branch
                    .as_ref()
                    .map(|branch| self.statements_from_indices(branch)),
                span: node.span,
            },
            SemanticNodeKind::WhileLoop {
                condition_node,
                body,
            } => Statement::While {
                condition: self.expression_from_optional_node(*condition_node, node.span),
                body: self.statements_from_indices(body),
                span: node.span,
            },
            SemanticNodeKind::ForLoop {
                variable,
                start_node,
                end_node,
                body,
            } => Statement::For {
                variable: variable.clone(),
                start: self.expression_from_optional_node(*start_node, node.span),
                end: self.expression_from_optional_node(*end_node, node.span),
                body: self.statements_from_indices(body),
                span: node.span,
            },
            SemanticNodeKind::ForEachLoop {
                variable,
                collection_node,
                body,
            } => Statement::ForEach {
                variable: variable.clone(),
                collection: self.expression_from_optional_node(*collection_node, node.span),
                body: self.statements_from_indices(body),
                span: node.span,
            },
            SemanticNodeKind::ExecuteStatement { code_node } => Statement::Execute {
                code: self.expression_from_optional_node(*code_node, node.span),
                span: node.span,
            },
            SemanticNodeKind::RaiseErrorStatement { message_node } => Statement::RaiseError {
                message: message_node.map(|idx| self.expression_from_optional_node(Some(idx), node.span)),
                span: node.span,
            },
            SemanticNodeKind::AddHandlerStatement {
                event_node,
                handler_node,
            } => Statement::AddHandler {
                event: self.expression_from_optional_node(*event_node, node.span),
                handler: self.expression_from_optional_node(*handler_node, node.span),
                span: node.span,
            },
            SemanticNodeKind::RemoveHandlerStatement {
                event_node,
                handler_node,
            } => Statement::RemoveHandler {
                event: self.expression_from_optional_node(*event_node, node.span),
                handler: self.expression_from_optional_node(*handler_node, node.span),
                span: node.span,
            },
            SemanticNodeKind::AwaitStatement { expression_node } => Statement::Await {
                expression: self.expression_from_optional_node(*expression_node, node.span),
                span: node.span,
            },
            SemanticNodeKind::Return { value_node } => Statement::Return {
                value: value_node.map(|idx| self.expression_from_optional_node(Some(idx), node.span)),
                span: node.span,
            },
            SemanticNodeKind::Break => Statement::Break { span: node.span },
            SemanticNodeKind::Continue => Statement::Continue { span: node.span },
            SemanticNodeKind::TryExcept {
                try_body,
                except_body,
            } => Statement::Try {
                try_body: self.statements_from_indices(try_body),
                except_body: self.statements_from_indices(except_body),
                span: node.span,
            },
            _ if self.can_project_expression(node) => Statement::Call {
                expression: self.expression_from_node_index(idx)?,
                span: node.span,
            },
            _ => return None,
        })
    }

    fn expression_from_optional_node(&self, idx: Option<usize>, fallback_span: Span) -> Expression {
        idx.and_then(|idx| self.expression_from_node_index(idx))
            .unwrap_or_else(|| self.placeholder_expression(fallback_span))
    }

    fn expression_from_node_index(&self, idx: usize) -> Option<Expression> {
        let node = self.program.nodes.get(idx)?;
        Some(match &node.kind {
            SemanticNodeKind::VariableAccess { name } => {
                self.identifier_expression(name, node.span)
            }
            SemanticNodeKind::StringLiteral { value } => Expression::String {
                value: value.clone(),
                span: node.span,
            },
            SemanticNodeKind::NumberLiteral { value } => Expression::Number {
                value: *value,
                span: node.span,
            },
            SemanticNodeKind::BooleanLiteral { value } => Expression::Boolean {
                value: *value,
                span: node.span,
            },
            SemanticNodeKind::DateLiteral { value } => Expression::Date {
                value: value.clone(),
                span: node.span,
            },
            SemanticNodeKind::NullLiteral => self.identifier_expression("Null", node.span),
            SemanticNodeKind::UndefinedLiteral => {
                self.identifier_expression("Неопределено", node.span)
            }
            SemanticNodeKind::GlobalPropertyAccess { name } => {
                self.identifier_expression(name, node.span)
            }
            SemanticNodeKind::MemberAccess {
                object_node,
                object_name,
                object_span,
                member_name,
                ..
            } => Expression::PropertyAccess {
                object: Box::new(self.object_expression(
                    *object_node,
                    object_name.as_deref(),
                    *object_span,
                    node.span,
                )),
                property: member_name.clone(),
                span: node.span,
            },
            SemanticNodeKind::IndexAccess {
                object_node,
                object_name,
                object_span,
                index_node,
                index_span,
            } => Expression::IndexAccess {
                object: Box::new(self.object_expression(
                    *object_node,
                    object_name.as_deref(),
                    *object_span,
                    node.span,
                )),
                index: Box::new(self.expression_from_optional_node(
                    *index_node,
                    index_span.unwrap_or(node.span),
                )),
                span: node.span,
            },
            SemanticNodeKind::FunctionCall {
                function_name,
                object_name,
                object_node,
                object_span,
                arg_nodes,
                arg_spans,
            } => {
                let function = if object_node.is_some() || object_name.is_some() {
                    Expression::PropertyAccess {
                        object: Box::new(self.object_expression(
                            *object_node,
                            object_name.as_deref(),
                            *object_span,
                            node.span,
                        )),
                        property: function_name.clone(),
                        span: node.span,
                    }
                } else {
                    self.identifier_expression(function_name, node.span)
                };
                let args = arg_nodes
                    .iter()
                    .zip(arg_spans.iter().copied())
                    .map(|(idx, span)| self.expression_from_optional_node(*idx, span))
                    .collect();
                Expression::Call {
                    function: Box::new(function),
                    args,
                    span: node.span,
                }
            }
            SemanticNodeKind::BinaryExpression {
                operator,
                left_node,
                right_node,
            } => Expression::Binary {
                left: Box::new(self.expression_from_optional_node(*left_node, node.span)),
                operator: operator.clone(),
                right: Box::new(self.expression_from_optional_node(*right_node, node.span)),
                span: node.span,
            },
            SemanticNodeKind::UnaryExpression {
                operator,
                operand_node,
            } => Expression::Unary {
                operator: operator.clone(),
                operand: Box::new(self.expression_from_optional_node(*operand_node, node.span)),
                span: node.span,
            },
            SemanticNodeKind::TernaryExpression {
                condition_node,
                then_node,
                else_node,
            } => Expression::Ternary {
                condition: Box::new(self.expression_from_optional_node(*condition_node, node.span)),
                then_expr: Box::new(self.expression_from_optional_node(*then_node, node.span)),
                else_expr: Box::new(self.expression_from_optional_node(*else_node, node.span)),
                span: node.span,
            },
            SemanticNodeKind::AwaitExpression { expression_node } => Expression::Await {
                expression: Box::new(self.expression_from_optional_node(*expression_node, node.span)),
                span: node.span,
            },
            SemanticNodeKind::NewExpression {
                type_name,
                is_dynamic,
                arg_nodes,
                ..
            } => Expression::New {
                type_name: if *is_dynamic {
                    format!("\"{type_name}\"")
                } else {
                    type_name.clone()
                },
                args: arg_nodes
                    .iter()
                    .map(|idx| self.expression_from_optional_node(*idx, node.span))
                    .collect(),
                span: node.span,
            },
            _ => return None,
        })
    }

    fn object_expression(
        &self,
        object_node: Option<usize>,
        object_name: Option<&str>,
        object_span: Option<Span>,
        fallback_span: Span,
    ) -> Expression {
        if let Some(idx) = object_node {
            if let Some(expression) = self.expression_from_node_index(idx) {
                return expression;
            }
        }
        if let Some(name) = object_name {
            return self.identifier_expression(name, object_span.unwrap_or(fallback_span));
        }
        self.placeholder_expression(object_span.unwrap_or(fallback_span))
    }

    fn identifier_expression(&self, name: &str, span: Span) -> Expression {
        Expression::Identifier {
            name: name.to_string(),
            span,
        }
    }

    fn placeholder_expression(&self, span: Span) -> Expression {
        self.identifier_expression("__ir_missing__", span)
    }
}

fn identifier_span_at_start(statement_span: Span, name: &str) -> Span {
    let end = statement_span
        .start
        .saturating_add(name.len() as u32)
        .min(statement_span.end);
    Span::new(statement_span.start, end)
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
        SemanticNodeKind::MemberAccess { object_node, .. } => {
            object_node.iter().copied().collect()
        }
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
