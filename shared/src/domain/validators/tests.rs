//! Tests for validators module

#[cfg(test)]
mod tests {
    use crate::domain::types::{
        Certainty, ConcreteType, PrimitiveType, ResolutionMetadata, ResolutionResult, ResolutionSource,
        TypeResolution,
    };
    use crate::domain::validators::TypeValidator;

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
}
