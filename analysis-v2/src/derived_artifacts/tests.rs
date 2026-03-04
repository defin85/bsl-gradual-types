use super::*;

fn test_key(file_id: FileId, file_version: i32) -> TypeIndexArtifactKey {
    TypeIndexArtifactKey::new(
        file_id,
        file_version,
        DepsSnapshotId::from_hash("deps-retention-test"),
        SettingsId::from_hash("settings-retention-test"),
    )
}

fn test_artifact(produced_at_millis: u128) -> Arc<TypeIndexArtifact> {
    Arc::new(TypeIndexArtifact {
        type_index: Arc::new(type_inference_v2::TypeIndex::default()),
        build_profile: type_inference_v2::TypeIndexBuildProfile::default(),
        parse_snapshot_meta: TypeIndexParseSnapshotMeta::default(),
        produced_at_millis,
    })
}

#[test]
fn type_index_retention_keeps_latest_two_versions_per_identity() {
    let mut cache = DerivedArtifactsCache::default();
    let file_id = FileId(1);

    let out_v1 = cache.store_type_index(test_key(file_id, 1), test_artifact(1));
    let out_v2 = cache.store_type_index(test_key(file_id, 2), test_artifact(2));
    let out_v3 = cache.store_type_index(test_key(file_id, 3), test_artifact(3));
    let out_v4 = cache.store_type_index(test_key(file_id, 4), test_artifact(4));

    assert_eq!(out_v1.evicted_per_file_window_total, 0);
    assert_eq!(out_v2.evicted_per_file_window_total, 0);
    assert_eq!(out_v3.evicted_per_file_window_total, 1);
    assert_eq!(out_v4.evicted_per_file_window_total, 1);

    let latest_key = test_key(file_id, 4);
    assert!(
        cache.get_type_index_exact(&latest_key).is_some(),
        "latest exact artifact must stay available"
    );

    let stale = cache
        .get_type_index_stale(&latest_key)
        .expect("latest key must have stale fallback");
    assert_eq!(
        stale.1, 3,
        "stale fallback must target latest previous version"
    );

    assert!(
        cache.get_type_index_exact(&test_key(file_id, 2)).is_none(),
        "version V2 must be evicted from latest-2 window"
    );
    assert!(
        cache.get_type_index_exact(&test_key(file_id, 1)).is_none(),
        "version V1 must be evicted from latest-2 window"
    );
}

#[test]
fn global_guard_candidate_picker_does_not_select_protected_exact_key() {
    let mut cache = DerivedArtifactsCache::default();
    let file_id = FileId(7);
    let old_key = test_key(file_id, 1);
    let protected_key = test_key(file_id, 2);

    cache.store_type_index(old_key.clone(), test_artifact(10));
    cache.store_type_index(protected_key.clone(), test_artifact(20));

    let candidate = cache
        .pick_global_evict_candidate(&protected_key)
        .expect("candidate expected");

    assert!(
        !(candidate.file_id == protected_key.file_id
            && candidate.file_version == protected_key.file_version
            && candidate.identity.deps_id == protected_key.deps_id
            && candidate.identity.settings_id == protected_key.settings_id),
        "global guard candidate must never be the protected latest exact key"
    );
}
