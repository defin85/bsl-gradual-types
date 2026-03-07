//! Domain Layer: Type Resolution Strategies
//!
//! Стратегии резолюции составных типов: Union, Intersection, Generic, Nullable

use crate::domain::types::{
    Certainty, ConcreteType, GenericType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution, WeightedType,
};

// ===== Union Types Strategy =====

/// Стратегия резолюции Union типов: "Строка | Число | Null"
pub struct UnionStrategy;

impl UnionStrategy {
    /// Разрешение Union типа из строки: "Строка | Число | Null"
    pub fn resolve(expression: &str, resolver: &super::TypeResolver) -> TypeResolution {
        // Парсим Union: "Строка | Число | Null"
        let type_names: Vec<&str> = expression
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if type_names.is_empty() {
            return TypeResolution::unknown();
        }

        // Если один тип — возвращаем его напрямую
        if type_names.len() == 1 {
            return resolver.resolve_expression_sync(type_names[0]);
        }

        // Разрешаем каждый тип и собираем в WeightedType
        let mut weighted_types = Vec::new();
        let weight_per_type = 1.0 / type_names.len() as f32;

        for type_name in type_names {
            let resolved = resolver.resolve_expression_sync(type_name);

            // Извлекаем ConcreteType из TypeResolution
            let concrete_type = match resolved.result {
                ResolutionResult::Concrete(ct) => ct,
                _ => continue, // Пропускаем Unknown/Dynamic и сложные типы
            };

            weighted_types.push(WeightedType::with_weight(concrete_type, weight_per_type));
        }

        // Нормализация через ResolutionResult::normalize_union
        let normalized_result = ResolutionResult::normalize_union(weighted_types);

        TypeResolution {
            certainty: Certainty::Known,
            result: normalized_result,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!("Union type: {}", expression)],
                uncertainty_reason: None,
                structural_members: vec![],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Проверка совместимости присваивания для Union типов
    ///
    /// Значение совместимо с Union, если оно совместимо с любым из типов Union
    pub fn is_assignable_to_union(
        value: &TypeResolution,
        union_resolution: &TypeResolution,
        resolver: &super::TypeResolver,
    ) -> bool {
        if let ResolutionResult::Union(union_types) = &union_resolution.result {
            // Проверяем, совместимо ли значение с любым из типов Union
            for weighted in union_types {
                let union_member = TypeResolution {
                    certainty: union_resolution.certainty,
                    result: ResolutionResult::Concrete(weighted.type_.clone()),
                    source: union_resolution.source,
                    metadata: union_resolution.metadata.clone(),
                    active_facet: union_resolution.active_facet,
                    available_facets: union_resolution.available_facets.clone(),
                };

                if resolver.is_assignment_compatible(value, &union_member) {
                    return true;
                }
            }
            false
        } else {
            // Не Union тип — используем обычную проверку
            resolver.is_assignment_compatible(value, union_resolution)
        }
    }
}

// ===== Intersection Types Strategy =====

/// Стратегия резолюции Intersection типов: "TypeA & TypeB"
pub struct IntersectionStrategy;

impl IntersectionStrategy {
    /// Разрешение Intersection типа из строки: "TypeA & TypeB"
    pub fn resolve(expression: &str, resolver: &super::TypeResolver) -> TypeResolution {
        // Парсим Intersection: "TypeA & TypeB"
        let type_names: Vec<&str> = expression
            .split('&')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if type_names.is_empty() {
            return TypeResolution::unknown();
        }

        // Если один тип — возвращаем его напрямую
        if type_names.len() == 1 {
            return resolver.resolve_expression_sync(type_names[0]);
        }

        // Разрешаем каждый тип и собираем в ConcreteType
        let mut concrete_types = Vec::new();

        for type_name in type_names {
            let resolved = resolver.resolve_expression_sync(type_name);

            // Извлекаем ConcreteType из TypeResolution
            let concrete_type = match resolved.result {
                ResolutionResult::Concrete(ct) => ct,
                _ => continue, // Пропускаем Unknown/Dynamic и сложные типы
            };

            concrete_types.push(concrete_type);
        }

        // Если не удалось разрешить ни один тип
        if concrete_types.is_empty() {
            return TypeResolution::unknown();
        }

        // Создаем Intersection через normalize
        let normalized_result = ResolutionResult::intersection(concrete_types);

        TypeResolution {
            certainty: Certainty::Known,
            result: normalized_result,
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec![format!("Intersection type: {}", expression)],
                uncertainty_reason: None,
                structural_members: vec![],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }

    /// Проверка совместимости типов для Intersection
    ///
    /// Проверяет, можно ли объединить два типа в Intersection
    pub fn are_compatible(type_a: &TypeResolution, type_b: &TypeResolution) -> bool {
        match (&type_a.result, &type_b.result) {
            // Оба типа должны быть Concrete для проверки
            (ResolutionResult::Concrete(a), ResolutionResult::Concrete(b)) => {
                match (a, b) {
                    // Primitive типы несовместимы между собой
                    (ConcreteType::Primitive(_), ConcreteType::Primitive(_)) => {
                        // Разные примитивы несовместимы
                        format!("{:?}", a) == format!("{:?}", b)
                    }

                    // Platform типы совместимы, если не одинаковые
                    (ConcreteType::Platform(pa), ConcreteType::Platform(pb)) => pa.name != pb.name,

                    // Configuration типы совместимы, если не одинаковые
                    (ConcreteType::Configuration(ca), ConcreteType::Configuration(cb)) => {
                        ca.name != cb.name
                    }

                    // Special типы (Null, Undefined) несовместимы с примитивами
                    (ConcreteType::Special(_), ConcreteType::Primitive(_))
                    | (ConcreteType::Primitive(_), ConcreteType::Special(_)) => false,

                    // Остальные комбинации совместимы
                    _ => true,
                }
            }

            // Dynamic совместим с любым типом
            (ResolutionResult::Dynamic, _) | (_, ResolutionResult::Dynamic) => true,

            // Остальные случаи несовместимы
            _ => false,
        }
    }
}

// ===== Generic Types Strategy =====

/// Стратегия резолюции Generic типов: "Массив<Строка>", "Соответствие<Строка, Число>"
pub struct GenericStrategy;

impl GenericStrategy {
    /// Разрешение Generic типа из строки
    pub fn resolve(expression: &str, resolver: &super::TypeResolver) -> TypeResolution {
        // Парсим Generic: "Массив<Строка>" или "Соответствие<Строка, Число>"
        if let Some((base_type, params_str)) = Self::parse_syntax(expression) {
            // Парсим параметры типа
            let type_params: Vec<&str> = params_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if type_params.is_empty() {
                return TypeResolution::unknown();
            }

            // Разрешаем каждый параметр типа
            let mut concrete_params = Vec::new();
            for param in type_params {
                let resolved = resolver.resolve_expression_sync(param);

                let concrete_type = match resolved.result {
                    ResolutionResult::Concrete(ct) => ct,
                    _ => continue, // Пропускаем неразрешенные типы
                };

                concrete_params.push(concrete_type);
            }

            if concrete_params.is_empty() {
                return TypeResolution::unknown();
            }

            // Создаем GenericType
            let generic_type = GenericType {
                base_type: base_type.to_string(),
                type_params: concrete_params,
            };

            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Generic(generic_type),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![format!("Generic type: {}", expression)],
                    uncertainty_reason: None,
                    structural_members: vec![],
                },
                active_facet: None,
                available_facets: vec![],
            }
        } else {
            TypeResolution::unknown()
        }
    }

    /// Парсинг Generic синтаксиса: "Массив<Строка>" → ("Массив", "Строка")
    pub fn parse_syntax(expr: &str) -> Option<(&str, &str)> {
        let open_bracket = expr.find('<')?;
        let close_bracket = expr.rfind('>')?;

        if close_bracket <= open_bracket {
            return None;
        }

        let base_type = expr[..open_bracket].trim();
        let params = expr[open_bracket + 1..close_bracket].trim();

        if base_type.is_empty() || params.is_empty() {
            return None;
        }

        Some((base_type, params))
    }

    /// Извлечь generic тип из строки типа
    /// "Массив<Число>" → Some("Число")
    /// "Массив" → None
    pub fn extract_from_type(type_str: &str) -> Option<String> {
        if let Some(start) = type_str.find('<') {
            if let Some(end) = type_str.rfind('>') {
                if end > start {
                    return Some(type_str[start + 1..end].trim().to_string());
                }
            }
        }
        None
    }
}

// ===== Nullable Types Strategy =====

/// Стратегия резолюции Nullable типов: "Строка?"
pub struct NullableStrategy;

impl NullableStrategy {
    /// Разрешение Nullable типа из строки: "Строка?"
    pub fn resolve(expression: &str, resolver: &super::TypeResolver) -> TypeResolution {
        // Парсим Nullable: "Строка?"
        if let Some(base_type_str) = expression.strip_suffix('?') {
            let base_type_str = base_type_str.trim();

            if base_type_str.is_empty() {
                return TypeResolution::unknown();
            }

            // Разрешаем базовый тип
            let base_resolved = resolver.resolve_expression_sync(base_type_str);

            let base_concrete = match base_resolved.result {
                ResolutionResult::Concrete(ct) => ct,
                _ => return TypeResolution::unknown(), // Nullable применим только к Concrete типам
            };

            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::nullable(base_concrete),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![format!("Nullable type: {}", expression)],
                    uncertainty_reason: None,
                    structural_members: vec![],
                },
                active_facet: None,
                available_facets: vec![],
            }
        } else {
            TypeResolution::unknown()
        }
    }

    /// Type narrowing для Nullable - убрать null из типа после проверки
    ///
    /// # Примеры
    ///
    /// ```bsl
    /// Перем x: Строка?;
    /// Если x <> Неопределено Тогда
    ///     // Здесь x: Строка (не nullable)
    ///     x.Длина; // Безопасно
    /// КонецЕсли;
    /// ```
    pub fn narrow(nullable_resolution: &TypeResolution) -> TypeResolution {
        match &nullable_resolution.result {
            ResolutionResult::Nullable(base_type) => {
                // Убираем Nullable обертку, возвращаем базовый тип
                TypeResolution {
                    certainty: Certainty::Known,
                    result: ResolutionResult::Concrete((**base_type).clone()),
                    source: nullable_resolution.source,
                    metadata: ResolutionMetadata {
                        file: nullable_resolution.metadata.file.clone(),
                        line: nullable_resolution.metadata.line,
                        column: nullable_resolution.metadata.column,
                        notes: vec!["Type narrowed from nullable".to_string()],
                        uncertainty_reason: None,
                        structural_members: nullable_resolution.metadata.structural_members.clone(),
                    },
                    active_facet: nullable_resolution.active_facet,
                    available_facets: nullable_resolution.available_facets.clone(),
                }
            }
            _ => nullable_resolution.clone(), // Не nullable - возвращаем как есть
        }
    }
}
