//! Type narrowing в условных конструкциях
//!
//! Этот модуль реализует уточнение типов в условных ветках кода.
//! Например, после проверки `ТипЗнч(x) = Тип("Строка")` мы знаем,
//! что в then-ветке x имеет тип Строка.

use crate::core::type_checker::TypeContext;
use crate::domain::types::{
    Certainty, ConcreteType, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    SpecialType, TypeResolution,
};
use crate::parsing::bsl::ast::{BinaryOp, Expression};

/// Информация об уточнении типа переменной
#[derive(Debug, Clone)]
pub struct TypeRefinement {
    /// Имя переменной
    pub variable: String,
    /// Уточнённый тип
    pub refined_type: TypeResolution,
    /// Условие, при котором применяется уточнение
    pub condition: RefinementCondition,
}

/// Условие уточнения типа
#[derive(Debug, Clone, PartialEq)]
pub enum RefinementCondition {
    /// Проверка на равенство типу: ТипЗнч(x) = Тип("Строка")
    TypeEquals(String),
    /// Проверка на неравенство типу: ТипЗнч(x) <> Тип("Строка")
    TypeNotEquals(String),
    /// Проверка на Неопределено: x = Неопределено
    IsUndefined,
    /// Проверка на не Неопределено: x <> Неопределено
    IsNotUndefined,
    /// Проверка на Null: x = Null
    IsNull,
    /// Проверка на не Null: x <> Null
    IsNotNull,
    /// Проверка на истинность: x (в условии)
    IsTruthy,
    /// Проверка на ложность: НЕ x
    IsFalsy,
}

/// Type narrowing analyzer
pub struct TypeNarrower {
    /// Текущий контекст типов
    context: TypeContext,
}

impl TypeNarrower {
    /// Создать новый анализатор
    pub fn new(context: TypeContext) -> Self {
        Self { context }
    }

    /// Анализировать условие и извлечь уточнения типов
    pub fn analyze_condition(&self, condition: &Expression) -> Vec<TypeRefinement> {
        match condition {
            // Бинарные операции сравнения
            Expression::Binary { left, op, right } => {
                self.analyze_binary_condition(left, op, right)
            }

            // Унарное отрицание: НЕ x
            Expression::Unary { op: _, operand } => {
                // Для НЕ x мы знаем, что x ложно в then-ветке
                if let Expression::Identifier(name) = &**operand {
                    vec![TypeRefinement {
                        variable: name.clone(),
                        refined_type: self.create_boolean_type(false),
                        condition: RefinementCondition::IsFalsy,
                    }]
                } else {
                    vec![]
                }
            }

            // Просто переменная в условии: Если x Тогда
            Expression::Identifier(name) => {
                vec![TypeRefinement {
                    variable: name.clone(),
                    refined_type: self.create_truthy_type(),
                    condition: RefinementCondition::IsTruthy,
                }]
            }

            _ => vec![],
        }
    }

    /// Анализировать бинарное условие
    fn analyze_binary_condition(
        &self,
        left: &Expression,
        op: &BinaryOp,
        right: &Expression,
    ) -> Vec<TypeRefinement> {
        // Проверка ТипЗнч(x) = Тип("Строка")
        if let Expression::Call { function, args } = left {
            if let Expression::Identifier(func_name) = &**function {
                if func_name == "ТипЗнч" || func_name == "TypeOf" {
                    if let Some(Expression::Identifier(var_name)) = args.first() {
                        if let Expression::Call {
                            function: type_fn,
                            args: type_args,
                        } = right
                        {
                            if let Expression::Identifier(type_fn_name) = &**type_fn {
                                if type_fn_name == "Тип" || type_fn_name == "Type" {
                                    if let Some(Expression::String(type_name)) = type_args.first() {
                                        return self
                                            .create_type_check_refinement(var_name, type_name, op);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Проверка x = Неопределено или x = Null
        if let Expression::Identifier(var_name) = left {
            match right {
                Expression::Identifier(name) if name == "Неопределено" || name == "Undefined" =>
                {
                    return self.create_undefined_check_refinement(var_name, op);
                }
                Expression::Identifier(name) if name == "Null" => {
                    return self.create_null_check_refinement(var_name, op);
                }
                _ => {}
            }
        }

        vec![]
    }

    /// Создать уточнение для проверки типа
    fn create_type_check_refinement(
        &self,
        var_name: &str,
        type_name: &str,
        op: &BinaryOp,
    ) -> Vec<TypeRefinement> {
        let refined_type = match type_name {
            "Строка" | "String" => self.create_primitive_type(PrimitiveType::String),
            "Число" | "Number" => self.create_primitive_type(PrimitiveType::Number),
            "Булево" | "Boolean" => self.create_primitive_type(PrimitiveType::Boolean),
            "Дата" | "Date" => self.create_primitive_type(PrimitiveType::Date),
            "Массив" | "Array" => self.create_platform_type("Массив"),
            "Соответствие" | "Map" => self.create_platform_type("Соответствие"),
            "Структура" | "Structure" => self.create_platform_type("Структура"),
            "ТаблицаЗначений" | "ValueTable" => {
                self.create_platform_type("ТаблицаЗначений")
            }
            _ => return vec![],
        };

        let condition = match op {
            BinaryOp::Equal => RefinementCondition::TypeEquals(type_name.to_string()),
            BinaryOp::NotEqual => RefinementCondition::TypeNotEquals(type_name.to_string()),
            _ => return vec![],
        };

        vec![TypeRefinement {
            variable: var_name.to_string(),
            refined_type,
            condition,
        }]
    }

    /// Создать уточнение для проверки на Неопределено
    fn create_undefined_check_refinement(
        &self,
        var_name: &str,
        op: &BinaryOp,
    ) -> Vec<TypeRefinement> {
        let (refined_type, condition) = match op {
            BinaryOp::Equal => (
                self.create_special_type(SpecialType::Undefined),
                RefinementCondition::IsUndefined,
            ),
            BinaryOp::NotEqual => (
                // Если не Неопределено, то это какой-то конкретный тип
                // Но мы не знаем какой, поэтому оставляем как есть
                self.context
                    .variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| self.create_unknown_type()),
                RefinementCondition::IsNotUndefined,
            ),
            _ => return vec![],
        };

        vec![TypeRefinement {
            variable: var_name.to_string(),
            refined_type,
            condition,
        }]
    }

    /// Создать уточнение для проверки на Null
    fn create_null_check_refinement(&self, var_name: &str, op: &BinaryOp) -> Vec<TypeRefinement> {
        let (refined_type, condition) = match op {
            BinaryOp::Equal => (
                self.create_special_type(SpecialType::Null),
                RefinementCondition::IsNull,
            ),
            BinaryOp::NotEqual => (
                self.context
                    .variables
                    .get(var_name)
                    .cloned()
                    .unwrap_or_else(|| self.create_unknown_type()),
                RefinementCondition::IsNotNull,
            ),
            _ => return vec![],
        };

        vec![TypeRefinement {
            variable: var_name.to_string(),
            refined_type,
            condition,
        }]
    }

    /// Создать примитивный тип
    fn create_primitive_type(&self, primitive: PrimitiveType) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(primitive)),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Type narrowed from condition".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Создать специальный тип
    fn create_special_type(&self, special: SpecialType) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Special(special)),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Type narrowed from condition".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Создать платформенный тип
    fn create_platform_type(&self, name: &str) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Platform(
                crate::core::types::PlatformType {
                    name: name.to_string(),
                    methods: vec![],
                    properties: vec![],
                },
            )),
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Type narrowed from condition".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Создать булевый тип
    fn create_boolean_type(&self, _value: bool) -> TypeResolution {
        self.create_primitive_type(PrimitiveType::Boolean)
    }

    /// Создать тип для истинного значения
    fn create_truthy_type(&self) -> TypeResolution {
        // В BSL истинными считаются все значения кроме Ложь, Неопределено и 0
        TypeResolution {
            certainty: Certainty::Inferred(0.8),
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Truthy value in condition".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Создать неизвестный тип
    fn create_unknown_type(&self) -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Unknown type".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Применить уточнения к контексту для then-ветки
    pub fn apply_refinements_to_context(&mut self, refinements: &[TypeRefinement]) -> TypeContext {
        let mut refined_context = self.context.clone();

        for refinement in refinements {
            refined_context
                .variables
                .insert(refinement.variable.clone(), refinement.refined_type.clone());
        }

        refined_context
    }

    /// Инвертировать уточнения для else-ветки
    pub fn invert_refinements(&self, refinements: &[TypeRefinement]) -> Vec<TypeRefinement> {
        refinements
            .iter()
            .map(|r| {
                let inverted_condition = match &r.condition {
                    RefinementCondition::TypeEquals(t) => {
                        RefinementCondition::TypeNotEquals(t.clone())
                    }
                    RefinementCondition::TypeNotEquals(t) => {
                        RefinementCondition::TypeEquals(t.clone())
                    }
                    RefinementCondition::IsUndefined => RefinementCondition::IsNotUndefined,
                    RefinementCondition::IsNotUndefined => RefinementCondition::IsUndefined,
                    RefinementCondition::IsNull => RefinementCondition::IsNotNull,
                    RefinementCondition::IsNotNull => RefinementCondition::IsNull,
                    RefinementCondition::IsTruthy => RefinementCondition::IsFalsy,
                    RefinementCondition::IsFalsy => RefinementCondition::IsTruthy,
                };

                // Для инвертированных условий мы часто не можем точно определить тип
                let inverted_type = match &inverted_condition {
                    RefinementCondition::TypeNotEquals(_)
                    | RefinementCondition::IsNotUndefined
                    | RefinementCondition::IsNotNull => {
                        // Оставляем исходный тип из контекста или Unknown
                        self.context
                            .variables
                            .get(&r.variable)
                            .cloned()
                            .unwrap_or_else(|| self.create_unknown_type())
                    }
                    _ => r.refined_type.clone(),
                };

                TypeRefinement {
                    variable: r.variable.clone(),
                    refined_type: inverted_type,
                    condition: inverted_condition,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dependency_graph::Scope;
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
    fn test_type_check_narrowing() {
        let context = create_test_context();
        let narrower = TypeNarrower::new(context);

        // Создаём условие: ТипЗнч(x) = Тип("Строка")
        let condition = Expression::Binary {
            left: Box::new(Expression::Call {
                function: Box::new(Expression::Identifier("ТипЗнч".to_string())),
                args: vec![Expression::Identifier("x".to_string())],
            }),
            op: BinaryOp::Equal,
            right: Box::new(Expression::Call {
                function: Box::new(Expression::Identifier("Тип".to_string())),
                args: vec![Expression::String("Строка".to_string())],
            }),
        };

        let refinements = narrower.analyze_condition(&condition);

        assert_eq!(refinements.len(), 1);
        assert_eq!(refinements[0].variable, "x");
        assert_eq!(
            refinements[0].condition,
            RefinementCondition::TypeEquals("Строка".to_string())
        );

        // Проверяем что тип уточнён до String
        if let ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) =
            &refinements[0].refined_type.result
        {
            // OK
        } else {
            panic!("Expected String type");
        }
    }

    #[test]
    fn test_undefined_check_narrowing() {
        let context = create_test_context();
        let narrower = TypeNarrower::new(context);

        // Создаём условие: x = Неопределено
        let condition = Expression::Binary {
            left: Box::new(Expression::Identifier("x".to_string())),
            op: BinaryOp::Equal,
            right: Box::new(Expression::Identifier("Неопределено".to_string())),
        };

        let refinements = narrower.analyze_condition(&condition);

        assert_eq!(refinements.len(), 1);
        assert_eq!(refinements[0].variable, "x");
        assert_eq!(refinements[0].condition, RefinementCondition::IsUndefined);

        // Проверяем что тип уточнён до Undefined
        if let ResolutionResult::Concrete(ConcreteType::Special(SpecialType::Undefined)) =
            &refinements[0].refined_type.result
        {
            // OK
        } else {
            panic!("Expected Undefined type");
        }
    }

    #[test]
    fn test_invert_refinements() {
        let context = create_test_context();
        let narrower = TypeNarrower::new(context);

        let refinement = TypeRefinement {
            variable: "x".to_string(),
            refined_type: narrower.create_primitive_type(PrimitiveType::String),
            condition: RefinementCondition::TypeEquals("Строка".to_string()),
        };

        let inverted = narrower.invert_refinements(&[refinement]);

        assert_eq!(inverted.len(), 1);
        assert_eq!(
            inverted[0].condition,
            RefinementCondition::TypeNotEquals("Строка".to_string())
        );
    }
}
