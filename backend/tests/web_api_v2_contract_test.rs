mod support;

use bsl_backend::application::type_system::web_api_service;
use bsl_shared::domain::TypeMetadataLookup;

#[tokio::test]
async fn search_types_as_dto_prefers_exact_match_and_preserves_shape() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let deps = deps_bundle.semantic_deps.as_ref();
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let query = "Массив";
    let result = web_api_service::search_types_as_dto(deps, &metadata_lookup, query)
        .await
        .expect("search_types_as_dto");

    assert!(!result.types.is_empty(), "expected non-empty types");
    assert_eq!(result.types[0].name, query, "expected exact match first");

    let value = serde_json::to_value(&result).expect("serialize AnalysisResultDto");
    assert!(value.get("types").is_some());
    assert!(value.get("categories").is_some());
    assert!(value.get("metrics").is_some());
    assert!(value.get("connections").is_some());
    assert!(
        value.get("pagination").is_none(),
        "search_types_as_dto should not include pagination"
    );
}

#[test]
fn get_all_types_as_dto_includes_pagination() {
    let deps_bundle = support::deps_bundle_v2_with_syntax_helper();
    let deps = deps_bundle.semantic_deps.as_ref();
    let metadata_lookup = TypeMetadataLookup::new(deps.repository.clone());

    let result = web_api_service::get_all_types_as_dto(
        deps,
        &metadata_lookup,
        10,
        0,
        Vec::new(),
        Vec::new(),
        false,
    );

    assert!(result.pagination.is_some(), "expected pagination");
    let pagination = result.pagination.as_ref().expect("pagination");
    assert_eq!(pagination.current_page, 1);
    assert_eq!(pagination.page_size, 10);

    let value = serde_json::to_value(&result).expect("serialize AnalysisResultDto");
    assert!(value.get("pagination").is_some());
}
