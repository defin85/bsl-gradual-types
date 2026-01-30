//! Domain Layer: Flow Analyzer
//!
//! Анализатор для построения flow-sensitive типизации из AST

use std::sync::Arc;
use bsl_shared::domain::type_id::TypeId;
use bsl_shared::domain::{FlowAnalysisContext, ControlFlowGraph, CfgNode, CfgNodeKind, EdgeKind};
use bsl_shared::domain::TypeResolver;
use bsl_shared::domain::types::TypeResolution;
use crate::parsing::bsl::ast::{Program, Statement, Expression};

/// Flow-sensitive анализатор для отслеживания типов переменных
pub struct FlowAnalyzer {
    resolver: Arc<TypeResolver>,
}

impl FlowAnalyzer {
    pub fn new(resolver: Arc<TypeResolver>) -> Self {
        Self { resolver }
    }

    /// Анализировать программу и построить CFG с flow-sensitive типами
    pub fn analyze_program(&self, program: &Program) -> FlowAnalysisResult {
        let mut context = FlowAnalysisContext::new();
        let mut cfg = ControlFlowGraph::new();

        // Добавляем entry узел
        let entry_id = cfg.add_node(CfgNode {
            id: 0,
            kind: CfgNodeKind::Entry,
            context_in: Some(context.clone()),
            context_out: None,
        });

        // Анализируем все statement'ы
        let mut current_id = entry_id;
        for statement in &program.statements {
            let new_id = self.analyze_statement(statement, &mut context, &mut cfg, current_id);
            current_id = new_id;
        }

        // Добавляем exit узел
        let exit_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::Exit,
            context_in: Some(context.clone()),
            context_out: Some(context.clone()),
        });
        cfg.add_edge(current_id, exit_id, EdgeKind::Unconditional);

        FlowAnalysisResult {
            context: context.clone(),
            cfg,
            variables: context.get_all_variables().clone(),
        }
    }

    /// Анализировать отдельный statement
    fn analyze_statement(
        &self,
        statement: &Statement,
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        match statement {
            Statement::Assignment { variable, value } => {
                self.analyze_assignment(variable, value, context, cfg, prev_id)
            }

            Statement::IfStatement { condition, then_branch, else_branch } => {
                self.analyze_if_statement(condition, then_branch, else_branch, context, cfg, prev_id)
            }

            Statement::WhileLoop { condition, body } => {
                self.analyze_while_loop(condition, body, context, cfg, prev_id)
            }

            Statement::ForLoop { variable, start, end, body } => {
                self.analyze_for_loop(variable, start, end, body, context, cfg, prev_id)
            }

            Statement::Return { value } => {
                self.analyze_return(value, context, cfg, prev_id)
            }

            Statement::MethodCall { object, method, arguments } => {
                self.analyze_method_call(object, method, arguments, context, cfg, prev_id)
            }

            _ => {
                // Другие типы statement пока не обрабатываем
                prev_id
            }
        }
    }

    /// Анализировать присваивание: переменная = выражение
    fn analyze_assignment(
        &self,
        variable: &str,
        value: &Expression,
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        // Вычисляем тип выражения
        let value_type = self.resolve_expression(value, context);

        // Обновляем тип переменной в контексте
        context.set_variable(variable.to_string(), value_type.clone());

        // Создаём узел для assignment
        let node_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::BasicBlock {
                statements: vec![format!("{} = {:?}", variable, value)],
            },
            context_in: Some(context.clone()),
            context_out: Some(context.clone()),
        });

        cfg.add_edge(prev_id, node_id, EdgeKind::Unconditional);
        node_id
    }

    /// Анализировать if-then-else
    fn analyze_if_statement(
        &self,
        _condition: &Expression,
        then_branch: &[Statement],
        else_branch: &Option<Vec<Statement>>,
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        // Создаём conditional узел
        let cond_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::Conditional {
                condition: "if-condition".to_string(),
            },
            context_in: Some(context.clone()),
            context_out: None,
        });
        cfg.add_edge(prev_id, cond_id, EdgeKind::Unconditional);

        // Анализируем then ветку
        context.enter_scope();
        let mut then_context = context.fork();
        let mut then_id = cond_id;
        for stmt in then_branch {
            then_id = self.analyze_statement(stmt, &mut then_context, cfg, then_id);
        }
        context.exit_scope();

        // Анализируем else ветку (если есть)
        let else_id = if let Some(else_stmts) = else_branch {
            context.enter_scope();
            let mut else_context = context.fork();
            let mut else_id = cond_id;
            for stmt in else_stmts {
                else_id = self.analyze_statement(stmt, &mut else_context, cfg, else_id);
            }
            context.exit_scope();

            // Объединяем контексты из обеих веток
            then_context.merge(&else_context);
            Some(else_id)
        } else {
            None
        };

        // Создаём merge узел после if-else
        let merge_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["merge-after-if".to_string()],
            },
            context_in: Some(then_context.clone()),
            context_out: Some(then_context.clone()),
        });

        cfg.add_edge(then_id, merge_id, EdgeKind::Unconditional);
        if let Some(else_id) = else_id {
            cfg.add_edge(else_id, merge_id, EdgeKind::Unconditional);
        } else {
            cfg.add_edge(cond_id, merge_id, EdgeKind::ConditionalFalse);
        }

        // Обновляем основной контекст
        *context = then_context;

        merge_id
    }

    /// Анализировать while цикл
    fn analyze_while_loop(
        &self,
        _condition: &Expression,
        body: &[Statement],
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        // Создаём loop header узел
        let header_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::LoopHeader {
                condition: "while-condition".to_string(),
            },
            context_in: Some(context.clone()),
            context_out: None,
        });
        cfg.add_edge(prev_id, header_id, EdgeKind::Unconditional);

        // Анализируем тело цикла
        context.enter_scope();
        let mut loop_context = context.fork();
        let mut body_id = header_id;
        for stmt in body {
            body_id = self.analyze_statement(stmt, &mut loop_context, cfg, body_id);
        }
        context.exit_scope();

        // Создаём обратное ребро в начало цикла
        cfg.add_edge(body_id, header_id, EdgeKind::LoopBack);

        // Создаём exit узел для выхода из цикла
        let exit_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::BasicBlock {
                statements: vec!["loop-exit".to_string()],
            },
            context_in: Some(context.clone()),
            context_out: Some(context.clone()),
        });
        cfg.add_edge(header_id, exit_id, EdgeKind::LoopExit);

        exit_id
    }

    /// Анализировать for цикл
    fn analyze_for_loop(
        &self,
        variable: &str,
        start: &Expression,
        end: &Expression,
        body: &[Statement],
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        // Инициализируем переменную цикла (обычно Число)
        let start_type = self.resolve_expression(start, context);
        context.set_variable(variable.to_string(), start_type);

        // Далее аналогично while
        self.analyze_while_loop(end, body, context, cfg, prev_id)
    }

    /// Анализировать return statement
    fn analyze_return(
        &self,
        value: &Option<Expression>,
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        let return_type = value.as_ref()
            .map(|expr| self.resolve_expression(expr, context))
            .unwrap_or_else(|| TypeResolution::unknown());

        let node_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::BasicBlock {
                statements: vec![format!("return {:?}", return_type)],
            },
            context_in: Some(context.clone()),
            context_out: Some(context.clone()),
        });

        cfg.add_edge(prev_id, node_id, EdgeKind::Unconditional);
        node_id
    }

    /// Анализировать вызов метода
    fn analyze_method_call(
        &self,
        object: &Option<Expression>,
        method: &str,
        _arguments: &[Expression],
        context: &mut FlowAnalysisContext,
        cfg: &mut ControlFlowGraph,
        prev_id: usize,
    ) -> usize {
        // Если вызов на объекте, получаем его тип
        if let Some(obj_expr) = object {
            let _obj_type = self.resolve_expression(obj_expr, context);
            // TODO: использовать TypeMetadataLookup для получения типа возврата метода
        }

        let node_id = cfg.add_node(CfgNode {
            id: cfg.nodes().len(),
            kind: CfgNodeKind::BasicBlock {
                statements: vec![format!("method_call: {}", method)],
            },
            context_in: Some(context.clone()),
            context_out: Some(context.clone()),
        });

        cfg.add_edge(prev_id, node_id, EdgeKind::Unconditional);
        node_id
    }

    /// Разрешить тип выражения в текущем контексте
    fn resolve_expression(
        &self,
        expression: &Expression,
        context: &FlowAnalysisContext,
    ) -> TypeResolution {
        match expression {
            Expression::Identifier(name) => {
                // Сначала ищем в flow context
                if let Some(resolution) = context.get_variable(name) {
                    return resolution.clone();
                }
                // Затем используем resolver
                self.resolver.resolve_expression_sync(name)
            }

            Expression::Literal(lit) => {
                use crate::parsing::bsl::ast::Literal;
                use bsl_shared::domain::types::{ConcreteType, PlatformType};

                let type_name = match lit {
                    Literal::String(_) => "Строка",
                    Literal::Number(_) => "Число",
                    Literal::Boolean(_) => "Булево",
                    Literal::Null => "Неопределено",
                };

                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: type_name.to_string(),
                }))
            }

            Expression::MemberAccess { object, member } => {
                let _obj_type = self.resolve_expression(object, context);
                // TODO: использовать TypeMetadataLookup для получения типа member
                self.resolver.resolve_expression_sync(member)
            }

            Expression::Call { function, .. } => {
                self.resolve_expression(function, context)
            }

            Expression::BinaryOp { left, operator, right } => {
                let _left_type = self.resolve_expression(left, context);
                let _right_type = self.resolve_expression(right, context);

                // Простая эвристика: возвращаем тип левого операнда
                // TODO: более точный вывод типа на основе оператора
                use bsl_shared::domain::types::{ConcreteType, PlatformType};
                TypeResolution::known(ConcreteType::Platform(PlatformType {
                    name: match operator.as_str() {
                        "+" | "-" | "*" | "/" => "Число",
                        "=" | "<>" | ">" | "<" | ">=" | "<=" => "Булево",
                        _ => "Произвольный",
                    }.to_string(),
                }))
            }

            _ => TypeResolution::unknown(),
        }
    }
}

/// Результат flow-sensitive анализа
#[derive(Debug, Clone)]
pub struct FlowAnalysisResult {
    /// Финальный контекст с типами всех переменных
    pub context: FlowAnalysisContext,

    /// Граф потока управления
    pub cfg: ControlFlowGraph,

    /// Типы переменных на выходе (ключ: TypeId для регистронезависимого поиска)
    pub variables: std::collections::HashMap<TypeId, TypeResolution>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsl_shared::domain::repository::InMemoryTypeRepository;
    use crate::parsing::bsl::ast::{Program, Statement, Expression, Literal};

    #[test]
    fn test_flow_analyzer_simple_assignment() {
        let repo = Arc::new(InMemoryTypeRepository::with_platform_types());
        let resolver = Arc::new(TypeResolver::new(repo));
        let analyzer = FlowAnalyzer::new(resolver);

        let program = Program {
            statements: vec![
                Statement::Assignment {
                    variable: "x".to_string(),
                    value: Expression::Literal(Literal::Number(42.0)),
                },
            ],
        };

        let result = analyzer.analyze_program(&program);
        assert!(result.variables.contains_key(&TypeId::new("x")));
    }

    #[test]
    fn test_flow_analyzer_if_statement() {
        let repo = Arc::new(InMemoryTypeRepository::with_platform_types());
        let resolver = Arc::new(TypeResolver::new(repo));
        let analyzer = FlowAnalyzer::new(resolver);

        let program = Program {
            statements: vec![
                Statement::IfStatement {
                    condition: Expression::Literal(Literal::Boolean(true)),
                    then_branch: vec![
                        Statement::Assignment {
                            variable: "x".to_string(),
                            value: Expression::Literal(Literal::String("text".to_string())),
                        },
                    ],
                    else_branch: Some(vec![
                        Statement::Assignment {
                            variable: "x".to_string(),
                            value: Expression::Literal(Literal::Number(42.0)),
                        },
                    ]),
                },
            ],
        };

        let result = analyzer.analyze_program(&program);
        assert!(result.variables.contains_key(&TypeId::new("x")));

        // Должен быть union type (Строка | Число)
        if let Some(x_type) = result.variables.get(&TypeId::new("x")) {
            assert!(matches!(x_type.result, crate::domain::types::ResolutionResult::Union(_)));
        }
    }
}
