use bsl_gradual_types::unified::data::{
    InMemoryTypeRepository, ParseMetadata, RawTypeData, TypeSource,
};
use bsl_gradual_types::unified::domain::{TypeContext, TypeResolutionService};
use std::sync::Arc;

#[tokio::test]
async fn type_resolution_service_updates_metrics() {
    // Arrange: empty repository is fine; BslCodeResolver will still produce a resolution
    let repo = Arc::new(InMemoryTypeRepository::new());
    let service = TypeResolutionService::new(repo);
    let ctx = TypeContext {
        file_path: None,
        line: None,
        column: None,
        local_variables: Default::default(),
        current_function: None,
        current_facet: None,
    };

    // Act: first resolve (cache miss)
    let _r1 = service
        .resolve_expression("ПроизвольноеВыражение", &ctx)
        .await;
    let m1 = service.get_metrics().await;

    assert_eq!(
        m1.total_resolutions, 1,
        "total_resolutions after first resolve"
    );
    assert_eq!(m1.cache_misses, 1, "cache_misses after first resolve");

    // Second resolve same expression -> should be cache hit, total_resolutions not incremented
    let _r2 = service
        .resolve_expression("ПроизвольноеВыражение", &ctx)
        .await;
    let m2 = service.get_metrics().await;

    assert!(
        m2.cache_hits >= 1,
        "cache_hits should be >= 1 after repeated resolve"
    );
    assert_eq!(
        m2.total_resolutions, 1,
        "total_resolutions should remain same on cache hit"
    );
}
