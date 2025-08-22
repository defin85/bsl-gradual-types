//! Полноценная система Union типов
//!
//! Этот модуль реализует продвинутую работу с union типами,
//! включая их нормализацию, упрощение и вывод.

use crate::core::types::{
    Certainty, ConcreteType, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
    TypeResolution, WeightedType,
};

/// Менеджер Union типов
pub struct UnionTypeManager;

impl UnionTypeManager {
    /// Создать Union тип из нескольких типов
    pub fn create_union(types: Vec<TypeResolution>) -> TypeResolution {
        if types.is_empty() {
            return Self::create_never_type();
        }

        if types.len() == 1 {
            return types.into_iter().next().unwrap();
        }

        // Извлекаем конкретные типы
        let mut concrete_types = Vec::new();
        let mut total_confidence = 0.0;
        let mut count = 0;

        for type_res in &types {
            match &type_res.result {
                ResolutionResult::Concrete(concrete) => {
                    concrete_types.push(WeightedType {
                        type_: concrete.clone(),
                        weight: 1.0 / types.len() as f32,
                    });
                }
                ResolutionResult::Union(union_types) => {
                    // Разворачиваем вложенные union типы
                    for weighted_type in union_types {
                        concrete_types.push(WeightedType {
                            type_: weighted_type.type_.clone(),
                            weight: weighted_type.weight / types.len() as f32,
                        });
                    }
                }
                _ => {
                    // Для других типов создаем динамический тип
                    // TODO: Более сложная логика
                }
            }

            // Учитываем уверенность
            match type_res.certainty {
                Certainty::Known => total_confidence += 1.0,
                Certainty::Inferred(conf) => total_confidence += conf,
                Certainty::Unknown => {} // Не добавляем уверенности
            }
            count += 1;
        }

        // Нормализуем union
        let normalized_types = Self::normalize_union(concrete_types);

        // Упрощаем если возможно
        let simplified = Self::simplify_union(normalized_types);

        match simplified.len() {
            0 => Self::create_never_type(),
            1 => TypeResolution {
                certainty: Certainty::Inferred(total_confidence / count as f32),
                result: ResolutionResult::Concrete(simplified[0].type_.clone()),
                source: ResolutionSource::Inferred,
                metadata: ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec!["Union type simplified to concrete".to_string()],
                },
                active_facet: None,
                available_facets: vec![],
            },
            _ => TypeResolution {
                certainty: Certainty::Inferred((total_confidence / count as f32).min(0.9)),
                result: ResolutionResult::Union(simplified.clone()),
                source: ResolutionSource::Inferred,
                metadata: ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![format!("Union of {} types", simplified.len())],
                },
                active_facet: None,
                available_facets: vec![],
            },
        }
    }

    /// Нормализовать union тип (убрать дубликаты, объединить веса)
    pub fn normalize_union(types: Vec<WeightedType>) -> Vec<WeightedType> {
        // Простая нормализация без HashMap из-за проблем с Hash
        let mut result: Vec<WeightedType> = Vec::new();

        for weighted_type in types {
            // Ищем существующий тип
            if let Some(existing) = result.iter_mut().find(|wt| wt.type_ == weighted_type.type_) {
                // Объединяем веса
                existing.weight += weighted_type.weight;
            } else {
                // Добавляем новый тип
                result.push(weighted_type);
            }
        }

        // Сортируем по весу (самые вероятные сначала)
        result.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());

        result
    }

    /// Упростить union тип используя правила BSL
    fn simplify_union(types: Vec<WeightedType>) -> Vec<WeightedType> {
        if types.is_empty() {
            return types;
        }

        // Правило 1: Если все типы числовые, объединяем в один Number
        if types.iter().all(|t| Self::is_numeric_type(&t.type_)) {
            return vec![WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::Number),
                weight: types.iter().map(|t| t.weight).sum(),
            }];
        }

        // Правило 2: Если все типы строковые, объединяем в один String
        if types.iter().all(|t| Self::is_string_type(&t.type_)) {
            return vec![WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: types.iter().map(|t| t.weight).sum(),
            }];
        }

        // Правило 3: Удаляем типы с очень маленьким весом (< 0.05)
        let mut filtered: Vec<_> = types.into_iter().filter(|t| t.weight >= 0.05).collect();

        // Перенормализуем веса
        let total_weight: f32 = filtered.iter().map(|t| t.weight).sum();
        if total_weight > 0.0 {
            for weighted_type in &mut filtered {
                weighted_type.weight /= total_weight;
            }
        }

        // Правило 4: Ограничиваем количество типов в union (максимум 5)
        filtered.truncate(5);

        filtered
    }

    /// Проверить совместимость типа с union
    pub fn is_compatible_with_union(type_: &TypeResolution, union_types: &[WeightedType]) -> bool {
        if let ResolutionResult::Concrete(concrete) = &type_.result {
            return union_types
                .iter()
                .any(|ut| Self::types_compatible(concrete, &ut.type_));
        }

        // Для других случаев предполагаем совместимость
        true
    }

    /// Получить наиболее вероятный тип из union
    pub fn get_most_likely_type(union_types: &[WeightedType]) -> Option<&ConcreteType> {
        union_types
            .first() // Они уже отсортированы по весу
            .map(|wt| &wt.type_)
    }

    /// Получить все возможные типы из union
    pub fn get_all_types(union_types: &[WeightedType]) -> Vec<&ConcreteType> {
        union_types.iter().map(|wt| &wt.type_).collect()
    }

    /// Пересечение двух union типов
    pub fn intersect_unions(union1: &[WeightedType], union2: &[WeightedType]) -> Vec<WeightedType> {
        let mut result = Vec::new();

        for wt1 in union1 {
            for wt2 in union2 {
                if Self::types_compatible(&wt1.type_, &wt2.type_) {
                    result.push(WeightedType {
                        type_: wt1.type_.clone(),
                        weight: wt1.weight * wt2.weight,
                    });
                }
            }
        }

        Self::normalize_union(result)
    }

    /// Объединение двух union типов
    pub fn merge_unions(union1: &[WeightedType], union2: &[WeightedType]) -> Vec<WeightedType> {
        let mut all_types = union1.to_vec();
        all_types.extend(union2.iter().cloned());

        // Уменьшаем веса пропорционально
        for wt in &mut all_types {
            wt.weight *= 0.5;
        }

        Self::normalize_union(all_types)
    }

    /// Проверить, является ли тип числовым
    fn is_numeric_type(type_: &ConcreteType) -> bool {
        matches!(type_, ConcreteType::Primitive(PrimitiveType::Number))
    }

    /// Проверить, является ли тип строковым
    fn is_string_type(type_: &ConcreteType) -> bool {
        matches!(type_, ConcreteType::Primitive(PrimitiveType::String))
    }

    /// Проверить совместимость двух конкретных типов
    fn types_compatible(type1: &ConcreteType, type2: &ConcreteType) -> bool {
        // Простейшая проверка - равенство
        type1 == type2
    }

    /// Создать тип "никогда не происходит"
    fn create_never_type() -> TypeResolution {
        TypeResolution {
            certainty: Certainty::Unknown,
            result: ResolutionResult::Dynamic,
            source: ResolutionSource::Inferred,
            metadata: ResolutionMetadata {
                file: None,
                line: None,
                column: None,
                notes: vec!["Never type (empty union)".to_string()],
            },
            active_facet: None,
            available_facets: vec![],
        }
    }
}

/// Утилиты для работы с union типами
impl UnionTypeManager {
    /// Создать union из конкретных типов
    pub fn from_concrete_types(types: Vec<ConcreteType>) -> TypeResolution {
        let type_resolutions: Vec<_> = types
            .into_iter()
            .map(|concrete| TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(concrete),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata {
                    file: None,
                    line: None,
                    column: None,
                    notes: vec![],
                },
                active_facet: None,
                available_facets: vec![],
            })
            .collect();

        Self::create_union(type_resolutions)
    }

    /// Добавить тип к существующему union
    pub fn add_type_to_union(union: &TypeResolution, new_type: TypeResolution) -> TypeResolution {
        match &union.result {
            ResolutionResult::Union(union_types) => {
                // Преобразуем union типы обратно в TypeResolution
                let mut all_types: Vec<_> = union_types
                    .iter()
                    .map(|wt| TypeResolution {
                        certainty: Certainty::Inferred(wt.weight),
                        result: ResolutionResult::Concrete(wt.type_.clone()),
                        source: ResolutionSource::Inferred,
                        metadata: ResolutionMetadata {
                            file: None,
                            line: None,
                            column: None,
                            notes: vec![],
                        },
                        active_facet: None,
                        available_facets: vec![],
                    })
                    .collect();

                all_types.push(new_type);
                Self::create_union(all_types)
            }
            _ => {
                // Это не union, создаем новый union с двумя типами
                Self::create_union(vec![union.clone(), new_type])
            }
        }
    }

    /// Проверить, содержит ли union определенный тип
    pub fn contains_type(union_types: &[WeightedType], target_type: &ConcreteType) -> bool {
        union_types.iter().any(|wt| &wt.type_ == target_type)
    }

    /// Получить вес типа в union
    pub fn get_type_weight(union_types: &[WeightedType], target_type: &ConcreteType) -> f32 {
        union_types
            .iter()
            .find(|wt| &wt.type_ == target_type)
            .map(|wt| wt.weight)
            .unwrap_or(0.0)
    }

    /// Фильтровать union по предикату
    pub fn filter_union<F>(union_types: &[WeightedType], predicate: F) -> Vec<WeightedType>
    where
        F: Fn(&ConcreteType) -> bool,
    {
        let filtered: Vec<_> = union_types
            .iter()
            .filter(|wt| predicate(&wt.type_))
            .cloned()
            .collect();

        Self::normalize_union(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::PrimitiveType;

    #[test]
    fn test_create_simple_union() {
        let types = vec![
            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
            TypeResolution {
                certainty: Certainty::Known,
                result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
                source: ResolutionSource::Static,
                metadata: ResolutionMetadata::default(),
                active_facet: None,
                available_facets: vec![],
            },
        ];

        let union = UnionTypeManager::create_union(types);

        match union.result {
            ResolutionResult::Union(union_types) => {
                assert_eq!(union_types.len(), 2);
                assert!(union_types
                    .iter()
                    .any(|wt| wt.type_ == ConcreteType::Primitive(PrimitiveType::String)));
                assert!(union_types
                    .iter()
                    .any(|wt| wt.type_ == ConcreteType::Primitive(PrimitiveType::Number)));
            }
            _ => panic!("Expected Union type"),
        }
    }

    #[test]
    fn test_simplify_single_type() {
        let types = vec![TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        }];

        let union = UnionTypeManager::create_union(types);

        // Один тип должен быть упрощен до конкретного типа
        match union.result {
            ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)) => {
                // OK
            }
            _ => panic!("Expected concrete String type, got: {:?}", union.result),
        }
    }

    #[test]
    fn test_normalize_duplicates() {
        let weighted_types = vec![
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: 0.3,
            },
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: 0.2,
            },
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::Number),
                weight: 0.5,
            },
        ];

        let normalized = UnionTypeManager::normalize_union(weighted_types);

        assert_eq!(normalized.len(), 2);
        // String должен иметь вес 0.5 (0.3 + 0.2)
        let string_weight = normalized
            .iter()
            .find(|wt| wt.type_ == ConcreteType::Primitive(PrimitiveType::String))
            .unwrap()
            .weight;
        assert!((string_weight - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_is_compatible_with_union() {
        let union_types = vec![
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: 0.6,
            },
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::Number),
                weight: 0.4,
            },
        ];

        let string_type = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::String)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(UnionTypeManager::is_compatible_with_union(
            &string_type,
            &union_types
        ));

        let boolean_type = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Boolean)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        assert!(!UnionTypeManager::is_compatible_with_union(
            &boolean_type,
            &union_types
        ));
    }

    #[test]
    fn test_most_likely_type() {
        let union_types = vec![
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::String),
                weight: 0.3,
            },
            WeightedType {
                type_: ConcreteType::Primitive(PrimitiveType::Number),
                weight: 0.7,
            },
        ];

        // Типы должны быть отсортированы по весу при нормализации
        let normalized = UnionTypeManager::normalize_union(union_types);
        let most_likely = UnionTypeManager::get_most_likely_type(&normalized).unwrap();

        assert_eq!(*most_likely, ConcreteType::Primitive(PrimitiveType::Number));
    }
}
