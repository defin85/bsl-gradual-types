//! Visitor pattern для обхода AST

use super::ast::*;

/// Trait для обхода AST
pub trait AstVisitor {
    /// Посещение программы
    fn visit_program(&mut self, program: &Program) {
        for statement in &program.statements {
            self.visit_statement(statement);
        }
    }
    
    /// Посещение оператора
    fn visit_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VarDeclaration { name, value, export } => {
                self.visit_var_declaration(name, value.as_ref(), *export);
            }
            Statement::ProcedureDecl { name, params, body, export } => {
                self.visit_procedure_decl(name, params, body, *export);
            }
            Statement::FunctionDecl { name, params, body, return_value, export } => {
                self.visit_function_decl(name, params, body, return_value.as_ref(), *export);
            }
            Statement::Assignment { target, value } => {
                self.visit_assignment(target, value);
            }
            Statement::ProcedureCall { name, args } => {
                self.visit_procedure_call(name, args);
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch } => {
                self.visit_if(condition, then_branch, else_if_branches, else_branch.as_ref());
            }
            Statement::For { variable, from, to, body } => {
                self.visit_for(variable, from, to, body);
            }
            Statement::ForEach { variable, collection, body } => {
                self.visit_for_each(variable, collection, body);
            }
            Statement::While { condition, body } => {
                self.visit_while(condition, body);
            }
            Statement::Return(value) => {
                self.visit_return(value.as_ref());
            }
            Statement::Break => self.visit_break(),
            Statement::Continue => self.visit_continue(),
            Statement::Try { try_block, catch_block } => {
                self.visit_try(try_block, catch_block.as_ref());
            }
            Statement::Raise(message) => self.visit_raise(message),
        }
    }
    
    /// Посещение выражения
    fn visit_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Number(n) => self.visit_number(*n),
            Expression::String(s) => self.visit_string(s),
            Expression::Boolean(b) => self.visit_boolean(*b),
            Expression::Date(d) => self.visit_date(d),
            Expression::Undefined => self.visit_undefined(),
            Expression::Null => self.visit_null(),
            Expression::Identifier(name) => self.visit_identifier(name),
            Expression::MemberAccess { object, member } => {
                self.visit_member_access(object, member);
            }
            Expression::Index { object, index } => {
                self.visit_index(object, index);
            }
            Expression::Call { function, args } => {
                self.visit_call(function, args);
            }
            Expression::New { type_name, args } => {
                self.visit_new(type_name, args);
            }
            Expression::Binary { left, op, right } => {
                self.visit_binary(left, op, right);
            }
            Expression::Unary { op, operand } => {
                self.visit_unary(op, operand);
            }
            Expression::Ternary { condition, then_expr, else_expr } => {
                self.visit_ternary(condition, then_expr, else_expr);
            }
            Expression::Array(elements) => self.visit_array(elements),
            Expression::Structure(fields) => self.visit_structure(fields),
        }
    }
    
    // === Методы для переопределения в конкретных visitor'ах ===
    
    fn visit_var_declaration(&mut self, _name: &str, _value: Option<&Expression>, _export: bool) {}
    
    fn visit_procedure_decl(&mut self, _name: &str, _params: &[Parameter], _body: &[Statement], _export: bool) {}
    
    fn visit_function_decl(&mut self, _name: &str, _params: &[Parameter], _body: &[Statement], _return_value: Option<&Expression>, _export: bool) {}
    
    fn visit_assignment(&mut self, target: &Expression, value: &Expression) {
        self.visit_expression(target);
        self.visit_expression(value);
    }
    
    fn visit_procedure_call(&mut self, _name: &str, args: &[Expression]) {
        for arg in args {
            self.visit_expression(arg);
        }
    }
    
    fn visit_if(&mut self, condition: &Expression, then_branch: &[Statement], else_if_branches: &[(Expression, Vec<Statement>)], else_branch: Option<&Vec<Statement>>) {
        self.visit_expression(condition);
        for stmt in then_branch {
            self.visit_statement(stmt);
        }
        for (cond, branch) in else_if_branches {
            self.visit_expression(cond);
            for stmt in branch {
                self.visit_statement(stmt);
            }
        }
        if let Some(branch) = else_branch {
            for stmt in branch {
                self.visit_statement(stmt);
            }
        }
    }
    
    fn visit_for(&mut self, _variable: &str, from: &Expression, to: &Expression, body: &[Statement]) {
        self.visit_expression(from);
        self.visit_expression(to);
        for stmt in body {
            self.visit_statement(stmt);
        }
    }
    
    fn visit_for_each(&mut self, _variable: &str, collection: &Expression, body: &[Statement]) {
        self.visit_expression(collection);
        for stmt in body {
            self.visit_statement(stmt);
        }
    }
    
    fn visit_while(&mut self, condition: &Expression, body: &[Statement]) {
        self.visit_expression(condition);
        for stmt in body {
            self.visit_statement(stmt);
        }
    }
    
    fn visit_return(&mut self, value: Option<&Expression>) {
        if let Some(expr) = value {
            self.visit_expression(expr);
        }
    }
    
    fn visit_break(&mut self) {}
    
    fn visit_continue(&mut self) {}
    
    fn visit_try(&mut self, try_block: &[Statement], catch_block: Option<&Vec<Statement>>) {
        for stmt in try_block {
            self.visit_statement(stmt);
        }
        if let Some(block) = catch_block {
            for stmt in block {
                self.visit_statement(stmt);
            }
        }
    }
    
    fn visit_raise(&mut self, _message: &str) {}
    
    fn visit_number(&mut self, _value: f64) {}
    
    fn visit_string(&mut self, _value: &str) {}
    
    fn visit_boolean(&mut self, _value: bool) {}
    
    fn visit_date(&mut self, _value: &str) {}
    
    fn visit_undefined(&mut self) {}
    
    fn visit_null(&mut self) {}
    
    fn visit_identifier(&mut self, _name: &str) {}
    
    fn visit_member_access(&mut self, object: &Expression, _member: &str) {
        self.visit_expression(object);
    }
    
    fn visit_index(&mut self, object: &Expression, index: &Expression) {
        self.visit_expression(object);
        self.visit_expression(index);
    }
    
    fn visit_call(&mut self, function: &Expression, args: &[Expression]) {
        self.visit_expression(function);
        for arg in args {
            self.visit_expression(arg);
        }
    }
    
    fn visit_new(&mut self, _type_name: &str, args: &[Expression]) {
        for arg in args {
            self.visit_expression(arg);
        }
    }
    
    fn visit_binary(&mut self, left: &Expression, _op: &BinaryOp, right: &Expression) {
        self.visit_expression(left);
        self.visit_expression(right);
    }
    
    fn visit_unary(&mut self, _op: &UnaryOp, operand: &Expression) {
        self.visit_expression(operand);
    }
    
    fn visit_ternary(&mut self, condition: &Expression, then_expr: &Expression, else_expr: &Expression) {
        self.visit_expression(condition);
        self.visit_expression(then_expr);
        self.visit_expression(else_expr);
    }
    
    fn visit_array(&mut self, elements: &[Expression]) {
        for element in elements {
            self.visit_expression(element);
        }
    }
    
    fn visit_structure(&mut self, fields: &[(String, Expression)]) {
        for (_, value) in fields {
            self.visit_expression(value);
        }
    }
}