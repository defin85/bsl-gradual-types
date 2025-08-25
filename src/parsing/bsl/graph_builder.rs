//! Построение графа зависимостей из AST

use crate::core::dependency_graph::{
    DependencyEdge, DependencyNode, DependencyType, Scope, SourceLocation, TypeDependencyGraph,
};
use crate::parser::ast::*;
use crate::parser::visitor::AstVisitor;

/// Построитель графа зависимостей из AST
pub struct DependencyGraphBuilder {
    graph: TypeDependencyGraph,
    current_scope: Scope,
    current_function: Option<String>,
    current_file: String,
    current_line: usize,
}

impl DependencyGraphBuilder {
    /// Создание нового построителя
    pub fn new(file_name: String) -> Self {
        Self {
            graph: TypeDependencyGraph::new(),
            current_scope: Scope::Module(file_name.clone()),
            current_function: None,
            current_file: file_name,
            current_line: 1,
        }
    }

    /// Построение графа из программы
    pub fn build(mut self, program: &Program) -> TypeDependencyGraph {
        self.visit_program(program);
        self.graph
    }

    /// Создание узла переменной в текущей области видимости
    fn make_variable_node(&self, name: &str) -> DependencyNode {
        DependencyNode::Variable {
            name: name.to_string(),
            scope: self.current_scope.clone(),
        }
    }

    /// Создание узла функции
    fn make_function_node(&self, name: &str, exported: bool) -> DependencyNode {
        DependencyNode::Function {
            name: name.to_string(),
            exported,
        }
    }

    /// Добавление ребра зависимости
    fn add_dependency(&self, from: DependencyNode, to: DependencyNode, dep_type: DependencyType) {
        self.graph.add_edge(DependencyEdge {
            from,
            to,
            dep_type,
            location: Some(SourceLocation {
                file: self.current_file.clone(),
                line: self.current_line,
                column: 0,
            }),
        });
    }

    /// Обработка выражения для поиска зависимостей
    fn process_expression_dependencies(&self, expr: &Expression, target: Option<&DependencyNode>) {
        match expr {
            Expression::Identifier(name) => {
                if let Some(target_node) = target {
                    let source = self.make_variable_node(name);
                    self.add_dependency(target_node.clone(), source, DependencyType::Expression);
                }
            }
            Expression::Binary { left, right, .. } => {
                self.process_expression_dependencies(left, target);
                self.process_expression_dependencies(right, target);
            }
            Expression::Unary { operand, .. } => {
                self.process_expression_dependencies(operand, target);
            }
            Expression::Call { function, args } => {
                // Если это вызов функции, добавляем зависимость
                if let Expression::Identifier(func_name) = &**function {
                    if let Some(target_node) = target {
                        let func_node = self.make_function_node(func_name, false);
                        self.add_dependency(
                            target_node.clone(),
                            func_node.clone(),
                            DependencyType::Expression,
                        );

                        // Обрабатываем аргументы
                        for (i, arg) in args.iter().enumerate() {
                            let param_node = DependencyNode::Parameter {
                                function: func_name.clone(),
                                name: format!("param_{}", i),
                            };
                            self.process_expression_dependencies(arg, Some(&param_node));
                        }
                    }
                } else if let Expression::MemberAccess { object, member } = &**function {
                    // Метод объекта
                    if let Expression::Identifier(obj_name) = &**object {
                        if let Some(target_node) = target {
                            let method_node = DependencyNode::Method {
                                object: obj_name.clone(),
                                method: member.clone(),
                            };
                            self.add_dependency(
                                target_node.clone(),
                                method_node,
                                DependencyType::MethodCall,
                            );
                        }
                    }
                }

                // Обрабатываем аргументы
                for arg in args {
                    self.process_expression_dependencies(arg, target);
                }
            }
            Expression::MemberAccess { object, member } => {
                if let Expression::Identifier(obj_name) = &**object {
                    if let Some(target_node) = target {
                        let field_node = DependencyNode::Field {
                            object: obj_name.clone(),
                            field: member.clone(),
                        };
                        self.add_dependency(
                            target_node.clone(),
                            field_node,
                            DependencyType::FieldAccess,
                        );
                    }
                }
                self.process_expression_dependencies(object, None);
            }
            Expression::Index { object, index } => {
                self.process_expression_dependencies(object, target);
                self.process_expression_dependencies(index, None);
            }
            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.process_expression_dependencies(condition, None);
                self.process_expression_dependencies(then_expr, target);
                self.process_expression_dependencies(else_expr, target);
            }
            Expression::Array(elements) => {
                for elem in elements {
                    self.process_expression_dependencies(elem, None);
                }
            }
            Expression::Structure(fields) => {
                for (_, value) in fields {
                    self.process_expression_dependencies(value, None);
                }
            }
            Expression::New { args, .. } => {
                for arg in args {
                    self.process_expression_dependencies(arg, None);
                }
            }
            _ => {}
        }
    }
}

impl AstVisitor for DependencyGraphBuilder {
    fn visit_var_declaration(&mut self, name: &str, value: Option<&Expression>, export: bool) {
        let var_node = self.make_variable_node(name);
        self.graph.add_node(var_node.clone());

        // Если переменная экспортируется, она глобальная
        if export {
            let global_var = DependencyNode::Variable {
                name: name.to_string(),
                scope: Scope::Global,
            };
            self.graph.add_node(global_var);
        }

        // Анализируем выражение инициализации
        if let Some(expr) = value {
            self.process_expression_dependencies(expr, Some(&var_node));
        }

        self.current_line += 1;
    }

    fn visit_procedure_decl(
        &mut self,
        name: &str,
        params: &[Parameter],
        body: &[Statement],
        export: bool,
    ) {
        let func_node = self.make_function_node(name, export);
        self.graph.add_node(func_node.clone());

        // Сохраняем текущую область видимости
        let prev_scope = self.current_scope.clone();
        let prev_function = self.current_function.clone();

        self.current_scope = Scope::Function(name.to_string());
        self.current_function = Some(name.to_string());

        // Добавляем параметры как узлы
        for param in params {
            let param_node = DependencyNode::Parameter {
                function: name.to_string(),
                name: param.name.clone(),
            };
            self.graph.add_node(param_node.clone());

            // Если есть значение по умолчанию, добавляем зависимость
            if let Some(default) = &param.default_value {
                self.process_expression_dependencies(default, Some(&param_node));
            }
        }

        // Обрабатываем тело процедуры
        for stmt in body {
            self.visit_statement(stmt);
        }

        // Восстанавливаем область видимости
        self.current_scope = prev_scope;
        self.current_function = prev_function;

        self.current_line += 1;
    }

    fn visit_function_decl(
        &mut self,
        name: &str,
        params: &[Parameter],
        body: &[Statement],
        return_value: Option<&Expression>,
        export: bool,
    ) {
        let func_node = self.make_function_node(name, export);
        self.graph.add_node(func_node.clone());

        // Сохраняем текущую область видимости
        let prev_scope = self.current_scope.clone();
        let prev_function = self.current_function.clone();

        self.current_scope = Scope::Function(name.to_string());
        self.current_function = Some(name.to_string());

        // Добавляем параметры как узлы
        for param in params {
            let param_node = DependencyNode::Parameter {
                function: name.to_string(),
                name: param.name.clone(),
            };
            self.graph.add_node(param_node.clone());

            // Если есть значение по умолчанию, добавляем зависимость
            if let Some(default) = &param.default_value {
                self.process_expression_dependencies(default, Some(&param_node));
            }
        }

        // Обрабатываем тело функции
        for stmt in body {
            self.visit_statement(stmt);
        }

        // Обрабатываем возвращаемое значение
        if let Some(ret_expr) = return_value {
            let return_node = DependencyNode::ReturnValue {
                function: name.to_string(),
            };
            self.graph.add_node(return_node.clone());
            self.process_expression_dependencies(ret_expr, Some(&return_node));
        }

        // Восстанавливаем область видимости
        self.current_scope = prev_scope;
        self.current_function = prev_function;

        self.current_line += 1;
    }

    fn visit_assignment(&mut self, target: &Expression, value: &Expression) {
        // Определяем целевой узел
        let target_node = match target {
            Expression::Identifier(name) => Some(self.make_variable_node(name)),
            Expression::MemberAccess { object, member } => {
                if let Expression::Identifier(obj_name) = &**object {
                    Some(DependencyNode::Field {
                        object: obj_name.clone(),
                        field: member.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(target_node) = target_node {
            self.graph.add_node(target_node.clone());
            self.process_expression_dependencies(value, Some(&target_node));
        }

        self.current_line += 1;
    }

    fn visit_procedure_call(&mut self, name: &str, args: &[Expression]) {
        let func_node = self.make_function_node(name, false);

        // Если мы внутри функции, добавляем зависимость вызова
        if let Some(current_func) = &self.current_function {
            let caller = self.make_function_node(current_func, false);
            self.add_dependency(caller, func_node.clone(), DependencyType::Expression);
        }

        // Обрабатываем аргументы
        for (i, arg) in args.iter().enumerate() {
            let param_node = DependencyNode::Parameter {
                function: name.to_string(),
                name: format!("param_{}", i),
            };
            self.process_expression_dependencies(arg, Some(&param_node));
        }

        self.current_line += 1;
    }

    fn visit_return(&mut self, value: Option<&Expression>) {
        if let Some(expr) = value {
            if let Some(func_name) = &self.current_function {
                let return_node = DependencyNode::ReturnValue {
                    function: func_name.clone(),
                };
                self.graph.add_node(return_node.clone());
                self.process_expression_dependencies(expr, Some(&return_node));
            }
        }

        self.current_line += 1;
    }

    fn visit_if(
        &mut self,
        condition: &Expression,
        then_branch: &[Statement],
        else_if_branches: &[(Expression, Vec<Statement>)],
        else_branch: Option<&Vec<Statement>>,
    ) {
        // Анализируем условие
        self.process_expression_dependencies(condition, None);

        // Обрабатываем then ветку
        for stmt in then_branch {
            self.visit_statement(stmt);
        }

        // Обрабатываем else if ветки
        for (cond, branch) in else_if_branches {
            self.process_expression_dependencies(cond, None);
            for stmt in branch {
                self.visit_statement(stmt);
            }
        }

        // Обрабатываем else ветку
        if let Some(branch) = else_branch {
            for stmt in branch {
                self.visit_statement(stmt);
            }
        }

        self.current_line += 1;
    }

    fn visit_for(
        &mut self,
        variable: &str,
        from: &Expression,
        to: &Expression,
        step: &Option<Expression>,
        body: &[Statement],
    ) {
        let var_node = self.make_variable_node(variable);
        self.graph.add_node(var_node.clone());

        self.process_expression_dependencies(from, Some(&var_node));
        self.process_expression_dependencies(to, None);
        if let Some(s) = step {
            self.process_expression_dependencies(s, None);
        }

        for stmt in body {
            self.visit_statement(stmt);
        }

        self.current_line += 1;
    }

    fn visit_for_each(&mut self, variable: &str, collection: &Expression, body: &[Statement]) {
        let var_node = self.make_variable_node(variable);
        self.graph.add_node(var_node.clone());

        self.process_expression_dependencies(collection, Some(&var_node));

        for stmt in body {
            self.visit_statement(stmt);
        }

        self.current_line += 1;
    }

    fn visit_while(&mut self, condition: &Expression, body: &[Statement]) {
        self.process_expression_dependencies(condition, None);

        for stmt in body {
            self.visit_statement(stmt);
        }

        self.current_line += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BslParser;

    #[test]
    fn test_build_simple_graph() {
        let code = r#"
            Перем А = 10;
            Перем Б = А + 5;
            
            Функция Сумма(Х, У)
                Возврат Х + У;
            КонецФункции
            
            В = Сумма(А, Б);
        "#;

        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();

        let builder = DependencyGraphBuilder::new("test.bsl".to_string());
        let graph = builder.build(&program);

        // Проверяем что узлы были добавлены
        let _var_a = DependencyNode::Variable {
            name: "А".to_string(),
            scope: Scope::Module("test.bsl".to_string()),
        };

        let var_b = DependencyNode::Variable {
            name: "Б".to_string(),
            scope: Scope::Module("test.bsl".to_string()),
        };

        // Б зависит от А
        let deps = graph.get_dependencies(&var_b);
        assert!(deps
            .iter()
            .any(|d| matches!(d, DependencyNode::Variable { name, .. } if name == "А")));
    }

    #[test]
    fn test_function_dependencies() {
        let code = r#"
            Функция Факториал(Н)
                Если Н <= 1 Тогда
                    Возврат 1;
                Иначе
                    Возврат Н * Факториал(Н - 1);
                КонецЕсли;
            КонецФункции
        "#;

        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();

        let builder = DependencyGraphBuilder::new("test.bsl".to_string());
        let graph = builder.build(&program);

        // Функция Факториал должна зависеть от себя (рекурсия)
        let func = DependencyNode::Function {
            name: "Факториал".to_string(),
            exported: false,
        };

        let deps = graph.get_dependencies(&func);
        println!("Dependencies for Факториал: {:?}", deps);

        // Проверяем зависимость через возвращаемое значение
        let return_node = DependencyNode::ReturnValue {
            function: "Факториал".to_string(),
        };

        let return_deps = graph.get_dependencies(&return_node);
        println!("Dependencies for return value: {:?}", return_deps);

        // Рекурсивный вызов должен быть в зависимостях возвращаемого значения
        assert!(return_deps
            .iter()
            .any(|d| matches!(d, DependencyNode::Function { name, .. } if name == "Факториал")));
    }
}
