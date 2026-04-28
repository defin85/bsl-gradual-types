//! Tests for validators module

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::domain::metadata_lookup::TypeMetadataLookup;
    use crate::domain::repository::{InMemoryTypeRepository, TypeRepository};
    use crate::domain::types::{
        Certainty, ConcreteType, PlatformType, PrimitiveType, RawDataSource, RawMethodData,
        RawTypeData, ResolutionMetadata, ResolutionResult, ResolutionSource, SpecialType,
        StructuralMember, TypeResolution, WeightedType,
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

    #[test]
    fn test_validate_method_exists_accepts_method_on_single_concrete_nullish_union() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        repo.load_types(vec![RawTypeData {
            name: "РезультатЗапроса".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Пустой".to_string(),
                english_name: "IsEmpty".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }])
        .unwrap();
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let validator = TypeValidator::new(&metadata_lookup);
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Union(vec![
                WeightedType::new(ConcreteType::Special(SpecialType::Undefined)),
                WeightedType::new(ConcreteType::Platform(PlatformType {
                    name: "РезультатЗапроса".to_string(),
                })),
            ]),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let error = validator.validate_method_exists_with_variable(
            &resolution,
            "Пустой",
            Some("РезультатЗапроса".to_string()),
        );

        assert!(error.is_none());
    }

    #[test]
    fn test_validate_method_exists_rejects_method_when_union_has_multiple_concrete_variants() {
        let repo = Arc::new(InMemoryTypeRepository::new());
        repo.load_types(vec![RawTypeData {
            name: "РезультатЗапроса".to_string(),
            source: RawDataSource::Platform,
            methods: vec![RawMethodData {
                name: "Пустой".to_string(),
                english_name: "IsEmpty".to_string(),
                return_type: "Булево".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }])
        .unwrap();
        let metadata_lookup = TypeMetadataLookup::new(repo);
        let validator = TypeValidator::new(&metadata_lookup);
        let resolution = TypeResolution {
            certainty: Certainty::Known,
            result: ResolutionResult::Union(vec![
                WeightedType::new(ConcreteType::Platform(PlatformType {
                    name: "РезультатЗапроса".to_string(),
                })),
                WeightedType::new(ConcreteType::Primitive(PrimitiveType::String)),
            ]),
            source: ResolutionSource::Static,
            metadata: ResolutionMetadata::default(),
            active_facet: None,
            available_facets: vec![],
        };

        let error = validator.validate_method_exists_with_variable(
            &resolution,
            "Пустой",
            Some("РезультатЗапроса".to_string()),
        );

        assert!(matches!(
            error,
            Some(TypeErrorKind::NonExistentMethod { method_name, .. })
                if method_name == "Пустой"
        ));
    }
}
