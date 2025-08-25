//! Межпроцедурный анализ типов
//!
//! Этот модуль реализует анализ типов через границы функций и процедур,
//! позволяя точнее определять типы путем анализа вызовов функций.

use crate::core::dependency_graph::Scope;
use crate::core::type_checker::{FunctionSignature, TypeContext};
use crate::domain::types::{
    Certainty, ConcreteType, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution,
};
use crate::parsing::bsl::ast::{Expression, Parameter, Program, Statement};
use std::collections::{HashMap, HashSet};

/// Информация о вызове функции/процедуры
#[derive(Debug, Clone)]
pub struct CallSite {
    /// Имя вызываемой функции/процедуры
    pub callee_name: String,
    /// Аргументы с их типами
    pub arguments: Vec<TypeResolution>,
    /// Ожидаемый тип возврата (если известен)
    pub expected_return_type: Option<TypeResolution>,
    /// Место вызова (файл, строка)
    pub location: CallLocation,
}

/// Местоположение вызова
#[derive(Debug, Clone)]
pub struct CallLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// Граф вызовов для межпроцедурного анализа
#[derive(Debug)]
pub struct CallGraph {
    /// Функции/процедуры в программе
    pub functions: HashMap<String, FunctionInfo>,
    /// Граф вызовов: функция -> список вызываемых функций
    pub call_edges: HashMap<String, Vec<CallSite>>,
    /// Обратный граф: функция -> список функций которые её вызывают
    pub callers: HashMap<String, Vec<String>>,
}

/// Информация о функции/процедуре
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Имя функции
    pub name: String,
    /// Параметры с их типами
    pub parameters: Vec<ParameterInfo>,
    /// Возвращаемый тип (для функций)
    pub return_type: Option<TypeResolution>,
    /// Тело функции
    pub body: Vec<Statement>,
    /// Экспортируется ли функция
    pub exported: bool,
    /// Область видимости
    pub scope: Scope,
}

/// Информация о параметре функции
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Имя параметра
    pub name: String,
    /// Тип параметра (может быть выведен)
    pub type_: TypeResolution,
    /// Значение по умолчанию
    pub default_value: Option<Expression>,
    /// По ссылке ли передается
    pub by_reference: bool,
}

/// Межпроцедурный анализатор типов
pub struct InterproceduralAnalyzer {
    pub call_graph: CallGraph,
    type_context: TypeContext,
    /// Кеш результатов анализа функций
    function_results: HashMap<String, TypeResolution>,
    /// Функции в процессе анализа (для обнаружения рекурсии)
    analyzing: HashSet<String>,
}

impl Default for CallGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl CallGraph {
    /// Создать новый граф вызовов
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            call_edges: HashMap::new(),
            callers: HashMap::new(),
        }
    }

    /// Построить граф вызовов из программы
    pub fn build_from_program(program: &Program) -> Self {
        let mut graph = Self::new();

        // Сначала собираем все функции и процедуры
        for statement in &program.statements {
            match statement {
                Statement::FunctionDecl {
                    name,
                    params,
                    body,
                    export,
                    ..
                } => {
                    let function_info = FunctionInfo {
                        name: name.clone(),
                        parameters: params
                            .iter()
                            .map(|p| ParameterInfo {
                                name: p.name.clone(),
                                type_: Self::infer_parameter_type(p),
                                default_value: p.default_value.clone(),
                                by_reference: !p.by_value,
                            })
                            .collect(),
                        return_type: None, // Будет выведен позже
                        body: body.clone(),
                        exported: *export,
                        scope: Scope::Global,
                    };
                    graph.functions.insert(name.clone(), function_info);
                }

                Statement::ProcedureDecl {
                    name,
                    params,
                    body,
                    export,
                } => {
                    let function_info = FunctionInfo {
                        name: name.clone(),
                        parameters: params
                            .iter()
                            .map(|p| ParameterInfo {
                                name: p.name.clone(),
                                type_: Self::infer_parameter_type(p),
                                default_value: p.default_value.clone(),
                                by_reference: !p.by_value,
                            })
                            .collect(),
                        return_type: None, // Процедуры не возвращают значения
                        body: body.clone(),
                        exported: *export,
                        scope: Scope::Global,
                    };
                    graph.functions.insert(name.clone(), function_info);
                }

                _ => {}
            }
        }

        // Затем анализируем вызовы
        for function_info in graph.functions.values() {
            let call_sites = Self::extract_call_sites(&function_info.body);
            graph
                .call_edges
                .insert(function_info.name.clone(), call_sites);
        }

        // Строим обратный граф
        graph.build_reverse_graph();

        graph
    }

    /// Вывести тип параметра из его определения
    fn infer_parameter_type(_param: &Parameter) -> TypeResolution {
        // TODO: Более сложная логика вывода типов параметров
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Parameter type to be inferred".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Извлечь места вызовов из тела функции
    fn extract_call_sites(body: &[Statement]) -> Vec<CallSite> {
        let mut call_sites = Vec::new();

        for statement in body {
            Self::extract_calls_from_statement(statement, &mut call_sites);
        }

        call_sites
    }

    /// Извлечь вызовы из оператора
    fn extract_calls_from_statement(statement: &Statement, call_sites: &mut Vec<CallSite>) {
        match statement {
            Statement::ProcedureCall { name, args } => {
                let call_site = CallSite {
                    callee_name: name.clone(),
                    arguments: args.iter().map(|_| Self::create_unknown_type()).collect(),
                    expected_return_type: None,
                    location: CallLocation {
                        file: "unknown".to_string(),
                        line: 0,
                        column: 0,
                    },
                };
                call_sites.push(call_site);
            }

            Statement::Assignment { target: _, value } => {
                Self::extract_calls_from_expression(value, call_sites);
            }

            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                Self::extract_calls_from_expression(condition, call_sites);

                for stmt in then_branch {
                    Self::extract_calls_from_statement(stmt, call_sites);
                }

                for (cond, branch) in else_if_branches {
                    Self::extract_calls_from_expression(cond, call_sites);
                    for stmt in branch {
                        Self::extract_calls_from_statement(stmt, call_sites);
                    }
                }

                if let Some(branch) = else_branch {
                    for stmt in branch {
                        Self::extract_calls_from_statement(stmt, call_sites);
                    }
                }
            }

            Statement::While { condition, body } => {
                Self::extract_calls_from_expression(condition, call_sites);
                for stmt in body {
                    Self::extract_calls_from_statement(stmt, call_sites);
                }
            }

            Statement::For {
                variable: _,
                from,
                to,
                step,
                body,
            } => {
                Self::extract_calls_from_expression(from, call_sites);
                Self::extract_calls_from_expression(to, call_sites);
                if let Some(step_expr) = step {
                    Self::extract_calls_from_expression(step_expr, call_sites);
                }
                for stmt in body {
                    Self::extract_calls_from_statement(stmt, call_sites);
                }
            }

            Statement::Return(value) => {
                if let Some(expr) = value {
                    Self::extract_calls_from_expression(expr, call_sites);
                }
            }

            _ => {}
        }
    }

    /// Извлечь вызовы из выражения
    fn extract_calls_from_expression(expression: &Expression, call_sites: &mut Vec<CallSite>) {
        match expression {
            Expression::Call { function, args } => {
                if let Expression::Identifier(name) = &**function {
                    let call_site = CallSite {
                        callee_name: name.clone(),
                        arguments: args.iter().map(|_| Self::create_unknown_type()).collect(),
                        expected_return_type: Some(Self::create_unknown_type()),
                        location: CallLocation {
                            file: "unknown".to_string(),
                            line: 0,
                            column: 0,
                        },
                    };
                    call_sites.push(call_site);
                }

                // Рекурсивно обрабатываем аргументы
                for arg in args {
                    Self::extract_calls_from_expression(arg, call_sites);
                }
            }

            Expression::Binary { left, op: _, right } => {
                Self::extract_calls_from_expression(left, call_sites);
                Self::extract_calls_from_expression(right, call_sites);
            }

            Expression::Unary { op: _, operand } => {
                Self::extract_calls_from_expression(operand, call_sites);
            }

            Expression::Array(elements) => {
                for element in elements {
                    Self::extract_calls_from_expression(element, call_sites);
                }
            }

            _ => {}
        }
    }

    /// Создать неизвестный тип
    fn create_unknown_type() -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Построить обратный граф вызовов
    fn build_reverse_graph(&mut self) {
        for (caller, call_sites) in &self.call_edges {
            for call_site in call_sites {
                self.callers
                    .entry(call_site.callee_name.clone())
                    .or_default()
                    .push(caller.clone());
            }
        }
    }

    /// Получить информацию о функции
    pub fn get_function_info(&self, name: &str) -> Option<&FunctionInfo> {
        self.functions.get(name)
    }

    /// Получить вызовы из функции
    pub fn get_calls_from(&self, function_name: &str) -> Option<&Vec<CallSite>> {
        self.call_edges.get(function_name)
    }

    /// Получить функции, которые вызывают данную
    pub fn get_callers(&self, function_name: &str) -> Option<&Vec<String>> {
        self.callers.get(function_name)
    }

    /// Получить все функции в порядке топологической сортировки
    pub fn topological_sort(&self) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut temp_visited = HashSet::new();

        for function_name in self.functions.keys() {
            if !visited.contains(function_name) {
                self.topological_visit(function_name, &mut visited, &mut temp_visited, &mut result);
            }
        }

        // Не реверсируем, так как мы добавляем в правильном порядке
        result
    }

    /// Вспомогательный метод для топологической сортировки
    fn topological_visit(
        &self,
        function_name: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) {
        if temp_visited.contains(function_name) {
            // Обнаружена циклическая зависимость
            return;
        }

        if visited.contains(function_name) {
            return;
        }

        temp_visited.insert(function_name.to_string());

        // Посещаем все вызываемые функции
        if let Some(call_sites) = self.call_edges.get(function_name) {
            for call_site in call_sites {
                // Проверяем существует ли вызываемая функция в нашем графе
                if self.functions.contains_key(&call_site.callee_name) {
                    self.topological_visit(&call_site.callee_name, visited, temp_visited, result);
                }
            }
        }

        temp_visited.remove(function_name);
        visited.insert(function_name.to_string());
        result.push(function_name.to_string());
    }
}

impl InterproceduralAnalyzer {
    /// Создать новый межпроцедурный анализатор
    pub fn new(call_graph: CallGraph, context: TypeContext) -> Self {
        Self {
            call_graph,
            type_context: context,
            function_results: HashMap::new(),
            analyzing: HashSet::new(),
        }
    }

    /// Проанализировать типы для всех функций
    pub fn analyze_all_functions(&mut self) {
        // Анализируем функции в топологическом порядке
        let sorted_functions = self.call_graph.topological_sort();

        for function_name in sorted_functions {
            if !self.function_results.contains_key(&function_name) {
                self.analyze_function(&function_name);
            }
        }
    }

    /// Проанализировать конкретную функцию
    pub fn analyze_function(&mut self, function_name: &str) -> Option<TypeResolution> {
        // Проверяем кеш
        if let Some(cached_result) = self.function_results.get(function_name) {
            return Some(cached_result.clone());
        }

        // Проверяем циклические зависимости
        if self.analyzing.contains(function_name) {
            // Рекурсивный вызов - возвращаем неизвестный тип
            return Some(self.create_unknown_type("Recursive call detected"));
        }

        let function_info = self.call_graph.get_function_info(function_name)?;

        // Отмечаем функцию как анализируемую
        self.analyzing.insert(function_name.to_string());

        // Анализируем тело функции
        let return_type = self.analyze_function_body(&function_info.body);

        // Убираем из множества анализируемых
        self.analyzing.remove(function_name);

        // Кешируем результат
        self.function_results
            .insert(function_name.to_string(), return_type.clone());

        Some(return_type)
    }

    /// Проанализировать тело функции
    fn analyze_function_body(&self, body: &[Statement]) -> TypeResolution {
        let mut return_types = Vec::new();

        for statement in body {
            if let Some(return_type) = self.extract_return_type(statement) {
                return_types.push(return_type);
            }
        }

        if return_types.is_empty() {
            // Нет явных return - это процедура
            self.create_void_type()
        } else if return_types.len() == 1 {
            return_types.into_iter().next().unwrap()
        } else {
            // Множественные возвраты - создаем union
            crate::core::union_types::UnionTypeManager::create_union(return_types)
        }
    }

    /// Извлечь тип возврата из оператора
    fn extract_return_type(&self, statement: &Statement) -> Option<TypeResolution> {
        match statement {
            Statement::Return(value) => {
                if let Some(expr) = value {
                    Some(self.infer_expression_type(expr))
                } else {
                    Some(self.create_void_type())
                }
            }

            Statement::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let mut types = Vec::new();

                // Анализируем then-ветку
                for stmt in then_branch {
                    if let Some(ret_type) = self.extract_return_type(stmt) {
                        types.push(ret_type);
                    }
                }

                // Анализируем else_if ветки
                for (_, branch) in else_if_branches {
                    for stmt in branch {
                        if let Some(ret_type) = self.extract_return_type(stmt) {
                            types.push(ret_type);
                        }
                    }
                }

                // Анализируем else ветку
                if let Some(branch) = else_branch {
                    for stmt in branch {
                        if let Some(ret_type) = self.extract_return_type(stmt) {
                            types.push(ret_type);
                        }
                    }
                }

                if types.is_empty() {
                    None
                } else {
                    Some(crate::core::union_types::UnionTypeManager::create_union(
                        types,
                    ))
                }
            }

            _ => None,
        }
    }

    /// Вывести тип выражения
    fn infer_expression_type(&self, expression: &Expression) -> TypeResolution {
        match expression {
            Expression::String(_) => self.create_string_type(),
            Expression::Number(_) => self.create_number_type(),
            Expression::Boolean(_) => self.create_boolean_type(),

            Expression::Call { function, args: _ } => {
                if let Expression::Identifier(func_name) = &**function {
                    // Если это анализируемая нами функция, получаем её тип
                    if let Some(result_type) = self.function_results.get(func_name) {
                        result_type.clone()
                    } else {
                        // Пытаемся проанализировать функцию рекурсивно
                        // (это может привести к бесконечной рекурсии, но она обработана выше)
                        self.create_unknown_type("Function not analyzed yet")
                    }
                } else {
                    self.create_unknown_type("Complex function call")
                }
            }

            Expression::Identifier(name) => {
                // Пытаемся найти тип переменной в контексте
                self.type_context
                    .variables
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.create_unknown_type("Unknown variable"))
            }

            _ => self.create_unknown_type("Complex expression"),
        }
    }

    /// Получить подпись функции
    pub fn get_function_signature(&self, function_name: &str) -> Option<FunctionSignature> {
        let function_info = self.call_graph.get_function_info(function_name)?;
        let return_type = self
            .function_results
            .get(function_name)
            .cloned()
            .unwrap_or_else(|| self.create_unknown_type("Not analyzed"));

        Some(FunctionSignature {
            params: function_info
                .parameters
                .iter()
                .map(|p| (p.name.clone(), p.type_.clone()))
                .collect(),
            return_type,
            exported: function_info.exported,
        })
    }

    /// Получить все проанализированные функции
    pub fn get_analyzed_functions(&self) -> &HashMap<String, TypeResolution> {
        &self.function_results
    }

    /// Обновить контекст типов на основе анализа
    pub fn update_type_context(&mut self) {
        for function_name in self.function_results.keys() {
            if let Some(signature) = self.get_function_signature(function_name) {
                self.type_context
                    .functions
                    .insert(function_name.clone(), signature);
            }
        }
    }

    /// Создать примитивные типы
    fn create_string_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_number_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_boolean_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_void_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Void type (procedure)".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_unknown_type(&self, reason: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![reason.to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_context() -> TypeContext {
        TypeContext {
            variables: HashMap::new(),
            functions: HashMap::new(),
            current_scope: Scope::Global,
            scope_stack: vec![],
        }
    }

    #[test]
    fn test_call_graph_creation() {
        let program = Program {
            statements: vec![Statement::FunctionDecl {
                name: "TestFunction".to_string(),
                params: vec![],
                body: vec![Statement::Return(Some(Expression::String(
                    "test".to_string(),
                )))],
                return_value: None,
                export: false,
            }],
        };

        let call_graph = CallGraph::build_from_program(&program);

        assert!(call_graph.get_function_info("TestFunction").is_some());

        let function_info = call_graph.get_function_info("TestFunction").unwrap();
        assert_eq!(function_info.name, "TestFunction");
        assert_eq!(function_info.parameters.len(), 0);
        assert!(!function_info.exported);
    }

    #[test]
    fn test_function_analysis() {
        let program = Program {
            statements: vec![Statement::FunctionDecl {
                name: "GetString".to_string(),
                params: vec![],
                body: vec![Statement::Return(Some(Expression::String(
                    "result".to_string(),
                )))],
                return_value: None,
                export: false,
            }],
        };

        let call_graph = CallGraph::build_from_program(&program);
        let mut analyzer = InterproceduralAnalyzer::new(call_graph, create_test_context());

        let result = analyzer.analyze_function("GetString");
        assert!(result.is_some());

        let return_type = result.unwrap();
        match return_type.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) => {
                // OK
            }
            _ => panic!("Expected String type, got: {:?}", return_type.result),
        }
    }

    #[test]
    fn test_topological_sort() {
        let program = Program {
            statements: vec![
                Statement::FunctionDecl {
                    name: "A".to_string(),
                    params: vec![],
                    body: vec![Statement::Return(Some(Expression::Call {
                        function: Box::new(Expression::Identifier("B".to_string())),
                        args: vec![],
                    }))],
                    return_value: None,
                    export: false,
                },
                Statement::FunctionDecl {
                    name: "B".to_string(),
                    params: vec![],
                    body: vec![Statement::Return(Some(Expression::String(
                        "B result".to_string(),
                    )))],
                    return_value: None,
                    export: false,
                },
            ],
        };

        let call_graph = CallGraph::build_from_program(&program);
        let sorted = call_graph.topological_sort();

        // B должно быть проанализировано перед A
        let b_pos = sorted.iter().position(|name| name == "B").unwrap();
        let a_pos = sorted.iter().position(|name| name == "A").unwrap();

        assert!(b_pos < a_pos, "B should come before A in topological order");
    }
}
