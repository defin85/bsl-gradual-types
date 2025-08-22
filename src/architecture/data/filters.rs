use super::TypeSource;
use crate::core::types::FacetKind;

#[derive(Debug, Default)]
pub struct TypeFilter {
    pub source: Option<TypeSource>,
    pub category: Option<String>,
    pub has_methods: Option<bool>,
    pub has_properties: Option<bool>,
    /// Подстрока для фильтрации по имени типа (регистр игнорируется)
    pub name_contains: Option<String>,
    /// Требуемая фасета среди доступных у типа
    pub facet: Option<FacetKind>,
}
