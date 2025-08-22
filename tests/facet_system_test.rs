#[cfg(test)]
mod tests {
    use bsl_gradual_types::core::platform_resolver::PlatformTypeResolver;
    use bsl_gradual_types::core::types::FacetKind;

    #[test]
    fn test_catalog_has_facets() {
        let mut resolver = PlatformTypeResolver::new();

        // Debug: check if "Справочники" is loaded
        println!(
            "Platform globals count: {}",
            resolver.get_platform_globals_count()
        );
        if resolver.has_platform_global("Справочники") {
            println!("'Справочники' found in platform_globals");
        } else {
            println!("'Справочники' NOT found in platform_globals");
        }

        // Resolve a catalog type
        let resolution = resolver.resolve_expression("Справочники.Контрагенты");

        println!("Resolution: {:?}", resolution.active_facet);
        println!("Available facets: {:?}", resolution.available_facets);
        println!("Certainty: {:?}", resolution.certainty);

        // Check that it has facets
        assert!(resolution.active_facet.is_some());
        assert_eq!(resolution.active_facet, Some(FacetKind::Manager));

        // Check available facets
        assert!(resolution.available_facets.contains(&FacetKind::Manager));
        assert!(resolution.available_facets.contains(&FacetKind::Object));
        assert!(resolution.available_facets.contains(&FacetKind::Reference));
        assert!(resolution
            .available_facets
            .contains(&FacetKind::Constructor));
    }

    #[test]
    fn test_document_has_facets() {
        let mut resolver = PlatformTypeResolver::new();

        let resolution = resolver.resolve_expression("Документы.ЗаказПокупателя");

        assert_eq!(resolution.active_facet, Some(FacetKind::Manager));
        assert_eq!(resolution.available_facets.len(), 4);
    }

    #[test]
    fn test_enum_has_limited_facets() {
        let mut resolver = PlatformTypeResolver::new();

        let resolution = resolver.resolve_expression("Перечисления.СтатусыЗаказов");

        assert_eq!(resolution.active_facet, Some(FacetKind::Manager));
        // Enums only have Manager and Reference facets
        assert_eq!(resolution.available_facets.len(), 2);
        assert!(resolution.available_facets.contains(&FacetKind::Manager));
        assert!(resolution.available_facets.contains(&FacetKind::Reference));
    }

    #[test]
    fn test_platform_types_have_no_facets() {
        let mut resolver = PlatformTypeResolver::new();

        let resolution = resolver.resolve_expression("Справочники");

        // Platform globals don't have facets
        assert_eq!(resolution.active_facet, None);
        assert!(resolution.available_facets.is_empty());
    }

    #[test]
    fn test_facet_switching() {
        let mut resolver = PlatformTypeResolver::new();
        let mut resolution = resolver.resolve_expression("Справочники.Контрагенты");

        // Default should be Manager
        assert_eq!(resolution.active_facet, Some(FacetKind::Manager));

        // Switch to Object facet
        resolution = resolver.switch_facet(resolution, FacetKind::Object);
        assert_eq!(resolution.active_facet, Some(FacetKind::Object));

        // Switch to Reference facet
        resolution = resolver.switch_facet(resolution, FacetKind::Reference);
        assert_eq!(resolution.active_facet, Some(FacetKind::Reference));

        // Try to switch to unavailable facet (should not change)
        resolution = resolver.switch_facet(resolution, FacetKind::Singleton);
        assert_eq!(resolution.active_facet, Some(FacetKind::Reference));
    }

    #[test]
    fn test_facet_inference_from_context() {
        let resolver = PlatformTypeResolver::new();

        // Test constructor pattern
        assert_eq!(
            resolver.infer_facet_from_context("Справочники.Контрагенты.СоздатьЭлемент"),
            Some(FacetKind::Constructor)
        );

        // Test reference pattern
        assert_eq!(
            resolver.infer_facet_from_context("СправочникСсылка.Контрагенты"),
            Some(FacetKind::Reference)
        );

        // Test object pattern
        assert_eq!(
            resolver.infer_facet_from_context("СправочникОбъект.Контрагенты"),
            Some(FacetKind::Object)
        );

        // Test manager pattern
        assert_eq!(
            resolver.infer_facet_from_context("Справочники.Контрагенты"),
            Some(FacetKind::Manager)
        );
    }
}
