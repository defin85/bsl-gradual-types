//! Flow-sensitive анализ типов
//!
//! Этот модуль реализует анализ типов с учетом потока управления.
//! В отличие от обычного анализа, здесь тип переменной может изменяться
//! в зависимости от того, в какой точке программы мы находимся.
//!
//! Основные возможности:
//! - Отслеживание изменений типов переменных по мере выполнения
//! - Объединение типов на точках слияния потока управления
//! - Учет условных переходов и циклов
//! - Интеграция с type narrowing

use crate::core::type_checker::TypeContext;
use crate::core::type_narrowing::TypeNarrower;
use crate::domain::types::{
    Certainty, ConcreteType, ResolutionMetadata, ResolutionResult, ResolutionSource, TypeResolution,
};
use crate::core::union_types::UnionTypeManager;
use crate::parsing::bsl::ast::{BinaryOp, Expression, Statement};
use std::collections::{HashMap, HashSet};

/// Состояние типов в конкретной точке программы
#[derive(Debug, Clone)]
pub struct FlowState {
    /// Типы переменных в данной точке
    pub variable_types: HashMap<String, TypeResolution>,
    /// Уникальный идентификатор состояния
    pub id: StateId,
    /// Предыдущие состояния (для слияния)
    pub predecessors: Vec<StateId>,
}

/// Идентификатор состояния потока
pub type StateId = usize;

/// Точка слияния потоков управления
#[derive(Debug, Clone)]
pub struct MergePoint {
    /// Состояния, которые нужно объединить
    pub states: Vec<StateId>,
    /// Результирующее состояние
    pub merged_state: StateId,
}

/// Flow-sensitive анализатор типов
pub struct FlowSensitiveAnalyzer {
    /// Все состояния потока
    states: Vec<FlowState>,
    /// Текущее состояние
    current_state: StateId,
    /// Счетчик для генерации уникальных ID
    next_state_id: StateId,
    /// Точки слияния
    merge_points: Vec<MergePoint>,
    /// Базовый контекст типов
    base_context: TypeContext,
}

impl FlowSensitiveAnalyzer {
    /// Создать новый анализатор с начальным контекстом
    pub fn new(context: TypeContext) -> Self {
        let initial_state = FlowState {
            variable_types: context.variables.clone(),
            id: 0,
            predecessors: vec![],
        };

        Self {
            states: vec![initial_state],
            current_state: 0,
            next_state_id: 1,
            merge_points: vec![],
            base_context: context,
        }
    }

    /// Создать новое состояние на основе текущего
    fn create_new_state(&mut self, variable_types: HashMap<String, TypeResolution>) -> StateId {
        let state_id = self.next_state_id;
        self.next_state_id += 1;

        let new_state = FlowState {
            variable_types,
            id: state_id,
            predecessors: vec![self.current_state],
        };

        self.states.push(new_state);
        state_id
    }

    /// Получить текущее состояние
    fn current_state(&self) -> &FlowState {
        &self.states[self.current_state]
    }

    /// Получить мутабельную ссылку на текущее состояние
    #[allow(dead_code)]
    fn current_state_mut(&mut self) -> &mut FlowState {
        &mut self.states[self.current_state]
    }

    /// Обновить тип переменной в текущем состоянии
    pub fn update_variable_type(&mut self, var_name: &str, new_type: TypeResolution) {
        // Создаем новое состояние с обновленным типом
        let mut new_types = self.current_state().variable_types.clone();
        new_types.insert(var_name.to_string(), new_type);

        let new_state_id = self.create_new_state(new_types);
        self.current_state = new_state_id;
    }

    /// Получить текущий тип переменной
    pub fn get_variable_type(&self, var_name: &str) -> Option<&TypeResolution> {
        self.current_state().variable_types.get(var_name)
    }

    /// Анализировать оператор присваивания
    pub fn analyze_assignment(&mut self, target: &Expression, value: &Expression) {
        // Сначала анализируем выражение справа
        let value_type = self.analyze_expression(value);

        // Затем обновляем тип переменной
        if let Expression::Identifier(var_name) = target {
            self.update_variable_type(var_name, value_type);
        }
        // TODO: Поддержка составных целей присваивания (массивы, свойства)
    }

    /// Анализировать выражение и вернуть его тип
    pub fn analyze_expression(&self, expr: &Expression) -> TypeResolution {
        match expr {
            Expression::Identifier(var_name) => {
                // Возвращаем текущий тип переменной или Unknown
                self.get_variable_type(var_name)
                    .cloned()
                    .unwrap_or_else(|| self.create_unknown_type())
            }

            Expression::String(_) => self.create_string_type(),
            Expression::Number(_) => self.create_number_type(),
            Expression::Boolean(_) => self.create_boolean_type(),

            Expression::Binary { left, op, right } => {
                self.analyze_binary_expression(left, op, right)
            }

            Expression::Call { function, args: _ } => {
                // Простейший анализ вызовов функций
                if let Expression::Identifier(func_name) = &**function {
                    match func_name.as_str() {
                        "Строка" | "String" => self.create_string_type(),
                        "Число" | "Number" => self.create_number_type(),
                        "Булево" | "Boolean" => self.create_boolean_type(),
                        _ => self.create_unknown_type(),
                    }
                } else {
                    self.create_unknown_type()
                }
            }

            _ => self.create_unknown_type(),
        }
    }

    /// Анализировать бинарное выражение
    fn analyze_binary_expression(
        &self,
        left: &Expression,
        op: &BinaryOp,
        right: &Expression,
    ) -> TypeResolution {
        let left_type = self.analyze_expression(left);
        let right_type = self.analyze_expression(right);

        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                // Арифметические операции обычно возвращают числа
                // TODO: Более сложная логика для строк (конкатенация)
                if self.is_number_type(&left_type) || self.is_number_type(&right_type) {
                    self.create_number_type()
                } else {
                    self.create_unknown_type()
                }
            }

            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::Greater
            | BinaryOp::LessOrEqual
            | BinaryOp::GreaterOrEqual => {
                // Операции сравнения всегда возвращают булево
                self.create_boolean_type()
            }

            BinaryOp::And | BinaryOp::Or => {
                // Логические операции возвращают булево
                self.create_boolean_type()
            }

            BinaryOp::Modulo => {
                // Операция остаток от деления возвращает число
                self.create_number_type()
            }
        }
    }

    /// Анализировать условный оператор (if-then-else)
    pub fn analyze_conditional(
        &mut self,
        condition: &Expression,
        then_branch: &[Statement],
        else_branch: Option<&[Statement]>,
    ) {
        // Сохраняем текущее состояние
        let before_condition_state = self.current_state;

        // Анализируем условие для получения type narrowing
        let narrower = TypeNarrower::new(self.create_type_context());
        let refinements = narrower.analyze_condition(condition);

        // Создаем состояние для then-ветки с уточнениями типов
        let mut then_types = self.current_state().variable_types.clone();
        for refinement in &refinements {
            then_types.insert(refinement.variable.clone(), refinement.refined_type.clone());
        }
        let then_state_id = self.create_new_state(then_types);

        // Анализируем then-ветку
        self.current_state = then_state_id;
        for stmt in then_branch {
            self.analyze_statement(stmt);
        }
        let then_end_state = self.current_state;

        // Создаем состояние для else-ветки
        let else_end_state = if let Some(else_stmts) = else_branch {
            // Инвертируем уточнения для else-ветки
            let inverted_refinements = narrower.invert_refinements(&refinements);
            let mut else_types = self.states[before_condition_state].variable_types.clone();

            for refinement in &inverted_refinements {
                else_types.insert(refinement.variable.clone(), refinement.refined_type.clone());
            }
            let else_state_id = self.create_new_state(else_types);

            // Анализируем else-ветку
            self.current_state = else_state_id;
            for stmt in else_stmts {
                self.analyze_statement(stmt);
            }
            self.current_state
        } else {
            // Нет else-ветки, используем состояние до условия
            before_condition_state
        };

        // Объединяем состояния после веток
        self.merge_states(vec![then_end_state, else_end_state]);
    }

    /// Объединить несколько состояний
    fn merge_states(&mut self, state_ids: Vec<StateId>) {
        if state_ids.is_empty() {
            return;
        }

        if state_ids.len() == 1 {
            self.current_state = state_ids[0];
            return;
        }

        // Собираем все переменные из всех состояний
        let mut all_vars: HashSet<String> = HashSet::new();
        for &state_id in &state_ids {
            for var_name in self.states[state_id].variable_types.keys() {
                all_vars.insert(var_name.clone());
            }
        }

        // Для каждой переменной объединяем её типы из разных состояний
        let mut merged_types = HashMap::new();
        for var_name in all_vars {
            let mut types_to_merge = Vec::new();

            for &state_id in &state_ids {
                if let Some(var_type) = self.states[state_id].variable_types.get(&var_name) {
                    types_to_merge.push(var_type.clone());
                }
            }

            // Объединяем типы
            let merged_type = self.merge_types(types_to_merge);
            merged_types.insert(var_name, merged_type);
        }

        // Создаем новое состояние
        let merged_state_id = self.create_new_state(merged_types);

        // Обновляем предшественников
        self.states[merged_state_id].predecessors = state_ids.clone();

        // Сохраняем точку слияния
        self.merge_points.push(MergePoint {
            states: state_ids,
            merged_state: merged_state_id,
        });

        self.current_state = merged_state_id;
    }

    /// Объединить несколько типов в один
    fn merge_types(&self, types: Vec<TypeResolution>) -> TypeResolution {
        // Используем новую систему Union типов
        UnionTypeManager::create_union(types)
    }

    /// Проверить равенство типов
    #[allow(dead_code)]
    fn types_equal(&self, type1: &TypeResolution, type2: &TypeResolution) -> bool {
        // Упрощенная проверка по результату разрешения
        type1.result == type2.result
    }

    /// Анализировать оператор
    pub fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assignment { target, value } => {
                self.analyze_assignment(target, value);
            }

            Statement::If {
                condition,
                then_branch,
                else_if_branches: _,
                else_branch,
            } => {
                self.analyze_conditional(condition, then_branch, else_branch.as_deref());
            }

            Statement::While { condition: _, body } => {
                // Упрощенный анализ цикла
                // TODO: Более сложный анализ с учетом инвариантов цикла
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }

            Statement::For {
                variable: _,
                from: _,
                to: _,
                step: _,
                body,
            } => {
                // Упрощенный анализ цикла For
                for stmt in body {
                    self.analyze_statement(stmt);
                }
            }

            _ => {
                // Другие операторы пока не влияют на типы
            }
        }
    }

    /// Создать контекст типов из текущего состояния
    fn create_type_context(&self) -> TypeContext {
        TypeContext {
            variables: self.current_state().variable_types.clone(),
            functions: self.base_context.functions.clone(),
            current_scope: self.base_context.current_scope.clone(),
            scope_stack: self.base_context.scope_stack.clone(),
        }
    }

    /// Создать примитивные типы
    fn create_string_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(
                crate::core::types::PrimitiveType::String,
            )),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_number_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(
                crate::core::types::PrimitiveType::Number,
            )),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_boolean_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(
                crate::core::types::PrimitiveType::Boolean,
            )),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    fn create_unknown_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Unknown type in flow analysis".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Проверить, является ли тип числовым
    fn is_number_type(&self, type_res: &TypeResolution) -> bool {
        matches!(
            type_res.result,
            ResolutionResult::Concrete(ConcreteType::Primitive(
                crate::core::types::PrimitiveType::Number
            ))
        )
    }

    /// Получить итоговое состояние анализа
    pub fn get_final_state(&self) -> &FlowState {
        self.current_state()
    }

    /// Получить все состояния (для отладки)
    pub fn get_all_states(&self) -> &[FlowState] {
        &self.states
    }

    /// Получить точки слияния (для отладки)
    pub fn get_merge_points(&self) -> &[MergePoint] {
        &self.merge_points
    }
}
