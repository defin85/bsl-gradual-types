//! Базовый type checker для BSL

use std::collections::HashMap;
use crate::parser::ast::*;
use crate::parser::visitor::AstVisitor;
use crate::core::types::{
    TypeResolution, Certainty, ResolutionResult,
    PrimitiveType, SpecialType
};
use crate::core::dependency_graph::{TypeDependencyGraph, Scope};
use crate::core::standard_types::{
    primitive_type, special_type, platform_type,
    is_number, is_string, is_boolean
};
use crate::parser::graph_builder::DependencyGraphBuilder;

/// Диагностическое сообщение о проблеме с типами
#[derive(Debug, Clone)]
pub struct TypeDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file: String,
}

/// Уровень серьёзности диагностики
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Контекст типов для анализа
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// Типы переменных
    pub variables: HashMap<String, TypeResolution>,
    /// Типы функций (параметры и возвращаемое значение)
    pub functions: HashMap<String, FunctionSignature>,
    /// Текущая область видимости
    pub current_scope: Scope,
    /// Стек областей видимости
    pub scope_stack: Vec<Scope>,
}

/// Сигнатура функции
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<(String, TypeResolution)>,
    pub return_type: TypeResolution,
    pub exported: bool,
}

/// Базовый type checker
pub struct TypeChecker {
    context: TypeContext,
    diagnostics: Vec<TypeDiagnostic>,
    dependency_graph: Option<TypeDependencyGraph>,
    current_file: String,
    current_line: usize,
}

impl TypeChecker {
    /// Создание нового type checker
    pub fn new(file_name: String) -> Self {
        Self {
            context: TypeContext {
                variables: HashMap::new(),
                functions: HashMap::new(),
                current_scope: Scope::Module(file_name.clone()),
                scope_stack: Vec::new(),
            },
            diagnostics: Vec::new(),
            dependency_graph: None,
            current_file: file_name,
            current_line: 1,
        }
    }
    
    /// Проверка типов в программе
    pub fn check(mut self, program: &Program) -> (TypeContext, Vec<TypeDiagnostic>) {
        // Сначала строим граф зависимостей
        let builder = DependencyGraphBuilder::new(self.current_file.clone());
        self.dependency_graph = Some(builder.build(program));
        
        // Затем анализируем типы
        self.visit_program(program);
        
        (self.context, self.diagnostics)
    }
    
    /// Добавление диагностики
    fn add_diagnostic(&mut self, severity: DiagnosticSeverity, message: String) {
        self.diagnostics.push(TypeDiagnostic {
            severity,
            message,
            line: self.current_line,
            column: 0,
            file: self.current_file.clone(),
        });
    }
    
    /// Вывод типа из выражения
    fn infer_expression_type(&mut self, expr: &Expression) -> TypeResolution {
        match expr {
            Expression::Number(_) => primitive_type(PrimitiveType::Number),
            
            Expression::String(_) => primitive_type(PrimitiveType::String),
            
            Expression::Boolean(_) => primitive_type(PrimitiveType::Boolean),
            
            Expression::Date(_) => primitive_type(PrimitiveType::Date),
            
            Expression::Undefined => special_type(SpecialType::Undefined),
            
            Expression::Null => special_type(SpecialType::Null),
            
            Expression::Identifier(name) => {
                // Ищем тип переменной в контексте
                if let Some(var_type) = self.context.variables.get(name) {
                    var_type.clone()
                } else {
                    self.add_diagnostic(
                        DiagnosticSeverity::Warning,
                        format!("Переменная '{}' используется без объявления", name),
                    );
                    TypeResolution::unknown()
                }
            }
            
            Expression::Binary { left, op, right } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);
                
                match op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                        // Арифметические операции
                        if is_number(&left_type) && is_number(&right_type) {
                            primitive_type(PrimitiveType::Number)
                        } else if is_string(&left_type) && matches!(op, BinaryOp::Add) {
                            // Конкатенация строк
                            primitive_type(PrimitiveType::String)
                        } else {
                            self.add_diagnostic(
                                DiagnosticSeverity::Warning,
                                format!("Несовместимые типы для операции {:?}", op),
                            );
                            TypeResolution::unknown()
                        }
                    }
                    
                    BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | 
                    BinaryOp::LessOrEqual | BinaryOp::Greater | BinaryOp::GreaterOrEqual => {
                        // Операции сравнения всегда возвращают булево
                        primitive_type(PrimitiveType::Boolean)
                    }
                    
                    BinaryOp::And | BinaryOp::Or => {
                        // Логические операции
                        if is_boolean(&left_type) && is_boolean(&right_type) {
                            primitive_type(PrimitiveType::Boolean)
                        } else {
                            self.add_diagnostic(
                                DiagnosticSeverity::Warning,
                                format!("Логическая операция {:?} требует булевы операнды", op),
                            );
                            TypeResolution::unknown()
                        }
                    }
                }
            }
            
            Expression::Unary { op, operand } => {
                let operand_type = self.infer_expression_type(operand);
                
                match op {
                    UnaryOp::Not => {
                        if is_boolean(&operand_type) {
                            primitive_type(PrimitiveType::Boolean)
                        } else {
                            self.add_diagnostic(
                                DiagnosticSeverity::Warning,
                                "Операция НЕ требует булев операнд".to_string(),
                            );
                            TypeResolution::unknown()
                        }
                    }
                    UnaryOp::Minus => {
                        if is_number(&operand_type) {
                            primitive_type(PrimitiveType::Number)
                        } else {
                            self.add_diagnostic(
                                DiagnosticSeverity::Warning,
                                "Унарный минус требует числовой операнд".to_string(),
                            );
                            TypeResolution::unknown()
                        }
                    }
                }
            }
            
            Expression::Call { function, args } => {
                if let Expression::Identifier(func_name) = &**function {
                    // Проверяем сигнатуру функции
                    if let Some(signature) = self.context.functions.get(func_name).cloned() {
                        // Проверяем количество аргументов
                        if args.len() != signature.params.len() {
                            self.add_diagnostic(
                                DiagnosticSeverity::Error,
                                format!(
                                    "Функция '{}' ожидает {} аргументов, передано {}",
                                    func_name,
                                    signature.params.len(),
                                    args.len()
                                ),
                            );
                        }
                        
                        // Проверяем типы аргументов
                        for (i, arg) in args.iter().enumerate() {
                            let arg_type = self.infer_expression_type(arg);
                            if let Some((param_name, param_type)) = signature.params.get(i) {
                                if !self.types_compatible(&arg_type, param_type) {
                                    self.add_diagnostic(
                                        DiagnosticSeverity::Warning,
                                        format!(
                                            "Несовместимый тип для параметра '{}' функции '{}'",
                                            param_name, func_name
                                        ),
                                    );
                                }
                            }
                        }
                        
                        signature.return_type
                    } else {
                        self.add_diagnostic(
                            DiagnosticSeverity::Info,
                            format!("Функция '{}' не найдена в контексте", func_name),
                        );
                        TypeResolution::unknown()
                    }
                } else if let Expression::MemberAccess { object, member: _ } = &**function {
                    // Вызов метода объекта
                    let _object_type = self.infer_expression_type(object);
                    // TODO: Проверить методы объекта
                    TypeResolution::unknown()
                } else {
                    TypeResolution::unknown()
                }
            }
            
            Expression::MemberAccess { object, member: _ } => {
                let _object_type = self.infer_expression_type(object);
                // TODO: Определить тип поля объекта
                TypeResolution::unknown()
            }
            
            Expression::Index { object, index } => {
                let _object_type = self.infer_expression_type(object);
                let _index_type = self.infer_expression_type(index);
                
                // TODO: Определить тип элементов массива
                TypeResolution::unknown()
            }
            
            Expression::New { type_name, args: _ } => {
                // Создание нового объекта
                match type_name.as_str() {
                    "Массив" | "Array" => platform_type("Массив"),
                    "Структура" | "Structure" => platform_type("Структура"),
                    "Соответствие" | "Map" => platform_type("Соответствие"),
                    _ => {
                        // Возможно, это платформенный тип
                        TypeResolution::unknown()
                    }
                }
            }
            
            Expression::Ternary { condition, then_expr, else_expr } => {
                let cond_type = self.infer_expression_type(condition);
                if !is_boolean(&cond_type) {
                    self.add_diagnostic(
                        DiagnosticSeverity::Warning,
                        "Условие тернарного оператора должно быть булевым".to_string(),
                    );
                }
                
                let then_type = self.infer_expression_type(then_expr);
                let else_type = self.infer_expression_type(else_expr);
                
                // Результат - объединение типов веток
                if self.types_equal(&then_type, &else_type) {
                    then_type
                } else {
                    // Для union типа просто возвращаем тип then (упрощение)
                    // TODO: Правильная обработка union типов
                    then_type
                }
            }
            
            Expression::Array(elements) => {
                // Анализируем типы элементов
                let _element_types: Vec<_> = elements.iter()
                    .map(|e| self.infer_expression_type(e))
                    .collect();
                
                platform_type("Массив")
            }
            
            Expression::Structure(fields) => {
                // Анализируем типы полей
                for (_name, value) in fields {
                    let _field_type = self.infer_expression_type(value);
                }
                
                platform_type("Структура")
            }
        }
    }
    
    
    /// Проверка совместимости типов
    fn types_compatible(&self, type1: &TypeResolution, type2: &TypeResolution) -> bool {
        // Если один из типов неизвестен, считаем совместимыми
        if matches!(type1.certainty, Certainty::Unknown) || matches!(type2.certainty, Certainty::Unknown) {
            return true;
        }
        
        // Проверяем равенство типов
        self.types_equal(type1, type2)
    }
    
    /// Проверка равенства типов
    fn types_equal(&self, type1: &TypeResolution, type2: &TypeResolution) -> bool {
        match (&type1.result, &type2.result) {
            (ResolutionResult::Concrete(t1), ResolutionResult::Concrete(t2)) => t1 == t2,
            _ => false,
        }
    }
    
    /// Вход в новую область видимости
    fn enter_scope(&mut self, scope: Scope) {
        self.context.scope_stack.push(self.context.current_scope.clone());
        self.context.current_scope = scope;
    }
    
    /// Выход из области видимости
    fn exit_scope(&mut self) {
        if let Some(prev_scope) = self.context.scope_stack.pop() {
            self.context.current_scope = prev_scope;
        }
    }
}

impl AstVisitor for TypeChecker {
    fn visit_var_declaration(&mut self, name: &str, value: Option<&Expression>, _export: bool) {
        let var_type = if let Some(expr) = value {
            self.infer_expression_type(expr)
        } else {
            TypeResolution::unknown()
        };
        
        self.context.variables.insert(name.to_string(), var_type);
        self.current_line += 1;
    }
    
    fn visit_procedure_decl(&mut self, name: &str, params: &[Parameter], body: &[Statement], export: bool) {
        // Входим в область видимости функции
        self.enter_scope(Scope::Function(name.to_string()));
        
        // Добавляем параметры в контекст
        let mut param_types = Vec::new();
        for param in params {
            let param_type = if let Some(default) = &param.default_value {
                self.infer_expression_type(default)
            } else {
                TypeResolution::unknown()
            };
            
            self.context.variables.insert(param.name.clone(), param_type.clone());
            param_types.push((param.name.clone(), param_type));
        }
        
        // Сохраняем сигнатуру процедуры
        self.context.functions.insert(
            name.to_string(),
            FunctionSignature {
                params: param_types,
                return_type: TypeResolution::unknown(), // Процедуры не возвращают значение
                exported: export,
            },
        );
        
        // Анализируем тело процедуры
        for stmt in body {
            self.visit_statement(stmt);
        }
        
        // Выходим из области видимости
        self.exit_scope();
        self.current_line += 1;
    }
    
    fn visit_function_decl(&mut self, name: &str, params: &[Parameter], body: &[Statement], return_value: Option<&Expression>, export: bool) {
        // Входим в область видимости функции
        self.enter_scope(Scope::Function(name.to_string()));
        
        // Добавляем параметры в контекст
        let mut param_types = Vec::new();
        for param in params {
            let param_type = if let Some(default) = &param.default_value {
                self.infer_expression_type(default)
            } else {
                TypeResolution::unknown()
            };
            
            self.context.variables.insert(param.name.clone(), param_type.clone());
            param_types.push((param.name.clone(), param_type));
        }
        
        // Анализируем тело функции
        for stmt in body {
            self.visit_statement(stmt);
        }
        
        // Определяем тип возвращаемого значения
        let return_type = if let Some(ret_expr) = return_value {
            self.infer_expression_type(ret_expr)
        } else {
            // Ищем операторы return в теле
            let mut found_return_type = TypeResolution::unknown();
            for stmt in body {
                if let Statement::Return(Some(expr)) = stmt {
                    found_return_type = self.infer_expression_type(expr);
                    break;
                }
            }
            found_return_type
        };
        
        // Сохраняем сигнатуру функции
        self.context.functions.insert(
            name.to_string(),
            FunctionSignature {
                params: param_types,
                return_type,
                exported: export,
            },
        );
        
        // Выходим из области видимости
        self.exit_scope();
        self.current_line += 1;
    }
    
    fn visit_assignment(&mut self, target: &Expression, value: &Expression) {
        let value_type = self.infer_expression_type(value);
        
        if let Expression::Identifier(var_name) = target {
            // Проверяем, была ли переменная объявлена
            if let Some(existing_type) = self.context.variables.get(var_name) {
                // Проверяем совместимость типов
                if !self.types_compatible(existing_type, &value_type) {
                    self.add_diagnostic(
                        DiagnosticSeverity::Warning,
                        format!("Несовместимое присваивание переменной '{}'", var_name),
                    );
                }
            }
            
            // Обновляем тип переменной
            self.context.variables.insert(var_name.clone(), value_type);
        }
        
        self.current_line += 1;
    }
    
    fn visit_if(&mut self, condition: &Expression, then_branch: &[Statement], else_if_branches: &[(Expression, Vec<Statement>)], else_branch: Option<&Vec<Statement>>) {
        // Проверяем тип условия
        let cond_type = self.infer_expression_type(condition);
        if !is_boolean(&cond_type) && !matches!(cond_type.certainty, Certainty::Unknown) {
            self.add_diagnostic(
                DiagnosticSeverity::Warning,
                "Условие должно быть булевым".to_string(),
            );
        }
        
        // Анализируем ветки
        for stmt in then_branch {
            self.visit_statement(stmt);
        }
        
        for (cond, branch) in else_if_branches {
            let cond_type = self.infer_expression_type(cond);
            if !is_boolean(&cond_type) && !matches!(cond_type.certainty, Certainty::Unknown) {
                self.add_diagnostic(
                    DiagnosticSeverity::Warning,
                    "Условие должно быть булевым".to_string(),
                );
            }
            
            for stmt in branch {
                self.visit_statement(stmt);
            }
        }
        
        if let Some(branch) = else_branch {
            for stmt in branch {
                self.visit_statement(stmt);
            }
        }
        
        self.current_line += 1;
    }
    
    fn visit_return(&mut self, value: Option<&Expression>) {
        if let Some(expr) = value {
            let _return_type = self.infer_expression_type(expr);
            // TODO: Проверить соответствие типа возвращаемого значения сигнатуре функции
        }
        
        self.current_line += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BslParser;
    
    #[test]
    fn test_simple_type_inference() {
        let code = r#"
            Перем Число = 42;
            Перем Строка = "Привет";
            Перем Булево = Истина;
            
            Перем Сумма = Число + 10;
            Перем Конкатенация = Строка + " мир";
        "#;
        
        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();
        
        let checker = TypeChecker::new("test.bsl".to_string());
        let (context, diagnostics) = checker.check(&program);
        
        // Проверяем выведенные типы
        use crate::core::standard_types::{is_number, is_string, is_boolean};
        
        assert!(context.variables.get("Число").map(is_number).unwrap_or(false));
        assert!(context.variables.get("Строка").map(is_string).unwrap_or(false));
        assert!(context.variables.get("Булево").map(is_boolean).unwrap_or(false));
        assert!(context.variables.get("Сумма").map(is_number).unwrap_or(false));
        assert!(context.variables.get("Конкатенация").map(is_string).unwrap_or(false));
        
        // Не должно быть ошибок
        assert!(diagnostics.iter().all(|d| d.severity != DiagnosticSeverity::Error));
    }
    
    #[test]
    fn test_function_signature_check() {
        let code = r#"
            Функция Сложить(А, Б)
                Возврат А + Б;
            КонецФункции
            
            Перем Результат = Сложить(10, 20);
            Перем Ошибка = Сложить(10); // Недостаточно аргументов
        "#;
        
        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();
        
        let checker = TypeChecker::new("test.bsl".to_string());
        let (_context, diagnostics) = checker.check(&program);
        
        // Должна быть ошибка о неправильном количестве аргументов
        assert!(diagnostics.iter().any(|d| 
            d.severity == DiagnosticSeverity::Error && 
            d.message.contains("ожидает 2 аргументов")
        ));
    }
    
    #[test]
    fn test_type_mismatch_warning() {
        let code = r#"
            Перем Число = 42;
            Перем Результат = Число И Истина; // Несовместимые типы
        "#;
        
        let mut parser = BslParser::new(code).unwrap();
        let program = parser.parse().unwrap();
        
        let checker = TypeChecker::new("test.bsl".to_string());
        let (_context, diagnostics) = checker.check(&program);
        
        // Должно быть предупреждение о несовместимых типах
        assert!(diagnostics.iter().any(|d| 
            d.severity == DiagnosticSeverity::Warning && 
            d.message.contains("булевы операнды")
        ));
    }
}