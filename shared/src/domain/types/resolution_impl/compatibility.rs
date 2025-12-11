//! TypeResolution compatibility methods
//!
//! Type compatibility checking system (Milestone 3.13).

use super::super::compatibility::TypeCompatibility;
use super::super::concrete::ConcreteType;
use super::super::facets::FacetKind;
use super::super::generics::GenericType;
use super::super::resolution::{ResolutionResult, TypeResolution};

impl TypeResolution {
    // ============================================================================
    // Milestone 3.13: Object-Based Type Comparison
    // ============================================================================

    /// Object-based type comparison with semantic awareness
    pub fn is_compatible_with(&self, other: &TypeResolution) -> TypeCompatibility {
        // Dynamic/Unknown is compatible with everything (gradual typing)
        if matches!(self.result, ResolutionResult::Dynamic)
            || matches!(other.result, ResolutionResult::Dynamic)
        {
            return TypeCompatibility::Compatible;
        }

        match (&self.result, &other.result) {
            // Concrete types
            (ResolutionResult::Concrete(a), ResolutionResult::Concrete(b)) => {
                Self::check_concrete_compatibility(a, b, self.active_facet, other.active_facet)
            }

            // Union - actual must be compatible with at least one member
            (_, ResolutionResult::Union(variants)) => {
                for variant in variants {
                    let variant_resolution = TypeResolution::known(variant.type_.clone());
                    if self.is_compatible_with(&variant_resolution).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Не совместим ни с одним вариантом Union".to_string(),
                }
            }

            // Generic - check base type and parameters
            (ResolutionResult::Generic(g1), ResolutionResult::Generic(g2)) => {
                Self::check_generic_compatibility(g1, g2)
            }

            // Nullable
            (ResolutionResult::Nullable(inner), other_result) => {
                let inner_resolution = TypeResolution::known(*inner.clone());
                inner_resolution.is_compatible_with(&TypeResolution {
                    result: other_result.clone(),
                    ..TypeResolution::unknown()
                })
            }

            // Intersection - actual must be compatible with ALL types
            (_, ResolutionResult::Intersection(types)) => {
                for concrete in types {
                    let type_resolution = TypeResolution::known(concrete.clone());
                    if !self.is_compatible_with(&type_resolution).is_compatible() {
                        return TypeCompatibility::Incompatible {
                            reason: "Не совместим со всеми типами Intersection".to_string(),
                        };
                    }
                }
                TypeCompatibility::Compatible
            }

            // Intersection as actual - must contain at least one compatible type
            (ResolutionResult::Intersection(types), _) => {
                for concrete in types {
                    let type_resolution = TypeResolution::known(concrete.clone());
                    if type_resolution.is_compatible_with(other).is_compatible() {
                        return TypeCompatibility::Compatible;
                    }
                }
                TypeCompatibility::Incompatible {
                    reason: "Ни один тип из Intersection не совместим".to_string(),
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: format!("Типы {:?} и {:?} несовместимы", self.result, other.result),
            },
        }
    }

    /// Check concrete type compatibility
    fn check_concrete_compatibility(
        from: &ConcreteType,
        to: &ConcreteType,
        from_facet: Option<FacetKind>,
        to_facet: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            // Primitives - exact match
            (ConcreteType::Primitive(a), ConcreteType::Primitive(b)) => {
                if a == b {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Примитивы {:?} и {:?} несовместимы", a, b),
                    }
                }
            }

            // Configuration types - consider facets!
            (ConcreteType::Configuration(cfg1), ConcreteType::Configuration(cfg2)) => {
                if cfg1.kind != cfg2.kind || !Self::names_equal(&cfg1.name, &cfg2.name) {
                    return TypeCompatibility::Incompatible {
                        reason: format!(
                            "Разные типы конфигурации: {}.{} vs {}.{}",
                            cfg1.kind.to_prefix(),
                            cfg1.name,
                            cfg2.kind.to_prefix(),
                            cfg2.name
                        ),
                    };
                }
                Self::check_facet_compatibility(from_facet.or(cfg1.facet), to_facet.or(cfg2.facet))
            }

            // Platform types - case-insensitive name comparison
            (ConcreteType::Platform(pt1), ConcreteType::Platform(pt2)) => {
                if Self::names_equal(&pt1.name, &pt2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!(
                            "Платформенные типы {} и {} несовместимы",
                            pt1.name, pt2.name
                        ),
                    }
                }
            }

            // Special types
            (ConcreteType::Special(s1), ConcreteType::Special(s2)) => {
                if s1 == s2 {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Специальные типы {:?} и {:?} несовместимы", s1, s2),
                    }
                }
            }

            // TabularRow - compare by parent_type and tabular_section_name
            (ConcreteType::TabularRow(tr1), ConcreteType::TabularRow(tr2)) => {
                if Self::names_equal(&tr1.parent_type, &tr2.parent_type)
                    && Self::names_equal(&tr1.tabular_section_name, &tr2.tabular_section_name)
                {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!(
                            "Разные строки табличных частей: {}.{} vs {}.{}",
                            tr1.parent_type, tr1.tabular_section_name,
                            tr2.parent_type, tr2.tabular_section_name
                        ),
                    }
                }
            }

            // GlobalFunction - functions as types are incompatible with each other
            (ConcreteType::GlobalFunction(f1), ConcreteType::GlobalFunction(f2)) => {
                if Self::names_equal(&f1.name, &f2.name) {
                    TypeCompatibility::Compatible
                } else {
                    TypeCompatibility::Incompatible {
                        reason: format!("Разные глобальные функции: {} vs {}", f1.name, f2.name),
                    }
                }
            }

            _ => TypeCompatibility::Incompatible {
                reason: "Несовместимые категории типов".to_string(),
            },
        }
    }

    /// Check facet compatibility
    fn check_facet_compatibility(
        from: Option<FacetKind>,
        to: Option<FacetKind>,
    ) -> TypeCompatibility {
        match (from, to) {
            (None, _) | (_, None) => TypeCompatibility::Compatible,
            (Some(f1), Some(f2)) if f1 == f2 => TypeCompatibility::Compatible,
            // Object -> Reference: allowed (implicit conversion)
            (Some(FacetKind::Object), Some(FacetKind::Reference)) => TypeCompatibility::Compatible,
            // Reference -> Object: NOT allowed (need GetObject())
            (Some(FacetKind::Reference), Some(FacetKind::Object)) => {
                TypeCompatibility::Incompatible {
                    reason: "Ссылка не может быть неявно преобразована в Объект (используйте ПолучитьОбъект())".to_string(),
                }
            }
            // Manager -> any other: NOT allowed
            (Some(FacetKind::Manager), Some(other)) => TypeCompatibility::Incompatible {
                reason: format!("Менеджер несовместим с фасетом {:?}", other),
            },
            (Some(f1), Some(f2)) => TypeCompatibility::Incompatible {
                reason: format!("Фасет {:?} несовместим с {:?}", f1, f2),
            },
        }
    }

    /// Check generic type compatibility
    fn check_generic_compatibility(g1: &GenericType, g2: &GenericType) -> TypeCompatibility {
        // Check base type
        if !Self::names_equal(&g1.base_type, &g2.base_type) {
            return TypeCompatibility::Incompatible {
                reason: format!(
                    "Несовместимые базовые типы: {} vs {}",
                    g1.base_type, g2.base_type
                ),
            };
        }

        // Check parameters
        if g1.type_params.len() != g2.type_params.len() {
            return TypeCompatibility::Incompatible {
                reason: "Разное количество параметров Generic".to_string(),
            };
        }

        for (p1, p2) in g1.type_params.iter().zip(g2.type_params.iter()) {
            let r1 = TypeResolution::known(p1.clone());
            let r2 = TypeResolution::known(p2.clone());
            if !r1.is_compatible_with(&r2).is_compatible() {
                return TypeCompatibility::Incompatible {
                    reason: format!("Параметры Generic {:?} и {:?} несовместимы", p1, p2),
                };
            }
        }

        TypeCompatibility::Compatible
    }

    fn names_equal(a: &str, b: &str) -> bool {
        a.to_lowercase() == b.to_lowercase()
    }
}
