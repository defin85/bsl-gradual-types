//! Tests for validators module

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::domain::metadata_lookup::TypeMetadataLookup;
    use crate::domain::repository::InMemoryTypeRepository;
    use crate::domain::types::{
        Certainty, ConcreteType, PrimitiveType, ResolutionMetadata, ResolutionResult,
        ResolutionSource, StructuralMember, TypeResolution,
    };
    use crate::domain::validators::{TypeErrorKind, TypeValidator};
    use std::sync::Arc;

    #[test]
    fn test_simple_type_as_collection() {
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Concrete(ConcreteType::Primitive(PrimitiveType::Number)),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let error = TypeValidator::validate_collection_operation(&resolution, "Добавить");

        assert!(error.is_some());
        assert!(matches!(
            error.unwrap(),
            crate::domain::validators::TypeErrorKind::SimpleTypeAsCollection { .. }
        ));
    }

    #[test]
    fn test_validate_property_exists_accepts_structural_member() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let validator = TypeValidator::new(&metadata_lookup);
        let resolution = TypeResolution::explicit("Структура").with_structural_member(
            StructuralMember::known("Идентификатор", TypeResolution::primitive("Строка")),
        );

        let error = validator.validate_property_exists(&resolution, "идентификатор");

        assert!(error.is_none());
    }

    #[test]
    fn test_validate_property_exists_reports_unknown_structural_member() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let validator = TypeValidator::new(&metadata_lookup);
        let resolution = TypeResolution::explicit("Структура").with_structural_member(
            StructuralMember::known("Идентификатор", TypeResolution::primitive("Строка")),
        );

        let error = validator.validate_property_exists(&resolution, "Идентифкатор");

        assert!(matches!(
            error,
            Some(TypeErrorKind::NonExistentProperty { property_name, .. })
                if property_name == "Идентифкатор"
        ));
    }
}
