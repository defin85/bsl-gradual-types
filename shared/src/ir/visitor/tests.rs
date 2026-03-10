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
            object_span: None,
            arg_nodes: Vec::new(),
            arg_spans: Vec::new(),
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
