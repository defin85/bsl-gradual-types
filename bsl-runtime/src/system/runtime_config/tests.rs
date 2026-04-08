use super::*;

#[test]
fn registry_has_unique_names() {
    let mut names = std::collections::HashSet::new();
    for key in RuntimeKey::ALL {
        assert!(
            names.insert(key.spec().env),
            "duplicate key: {}",
            key.spec().env
        );
    }
}

#[test]
fn unknown_keys_are_ignored_with_report() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let mut overrides = HashMap::new();
    overrides.insert("BSL_NOT_A_REAL_KEY".to_string(), JsonValue::Bool(true));
    let report = store.replace_stable_overrides(&overrides);
    assert_eq!(
        report.ignored_unknown_keys,
        vec!["BSL_NOT_A_REAL_KEY".to_string()]
    );
}

#[test]
fn dev_overrides_respect_opt_in() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let mut dev = HashMap::new();
    dev.insert("BSL_COMPLETION_TRACE".to_string(), JsonValue::Bool(true));
    let report = store.replace_dev_overrides(&dev, false);
    assert!(report.dev_overrides_ignored);
    assert_eq!(store.get_bool(RuntimeKey::CompletionTrace), Some(false));
}

#[test]
fn stable_layer_does_not_accept_dev_only_keys() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let mut stable = HashMap::new();
    stable.insert("BSL_COMPLETION_TRACE".to_string(), JsonValue::Bool(true));
    let report = store.replace_stable_overrides(&stable);
    assert_eq!(
        report.ignored_wrong_tier_keys,
        vec!["BSL_COMPLETION_TRACE".to_string()]
    );
    assert_eq!(store.get_bool(RuntimeKey::CompletionTrace), Some(false));
}

#[test]
fn disabling_dev_overrides_clears_layer() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let mut dev = HashMap::new();
    dev.insert("BSL_COMPLETION_TRACE".to_string(), JsonValue::Bool(true));

    let report_enabled = store.replace_dev_overrides(&dev, true);
    assert!(!report_enabled.dev_overrides_ignored);
    assert_eq!(store.get_bool(RuntimeKey::CompletionTrace), Some(true));

    let report_disabled = store.replace_dev_overrides(&dev, false);
    assert!(report_disabled.dev_overrides_ignored);
    assert_eq!(store.get_bool(RuntimeKey::CompletionTrace), Some(false));
}

#[test]
fn snapshot_contains_mutability_map() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let snapshot = store.snapshot();

    assert_eq!(
        snapshot.mutability.get("BSL_CACHE_DIR"),
        Some(&KeyMutability::StartupOnly)
    );
    assert_eq!(
        snapshot.mutability.get("BSL_CACHE_DISABLE"),
        Some(&KeyMutability::Runtime)
    );
}

#[test]
fn startup_only_override_is_reported_as_requires_restart() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let mut stable = HashMap::new();
    stable.insert(
        "BSL_CACHE_DIR".to_string(),
        JsonValue::String("/tmp/runtime-config-restart-a".to_string()),
    );
    let _ = store.replace_stable_overrides(&stable);

    stable.insert(
        "BSL_CACHE_DIR".to_string(),
        JsonValue::String("/tmp/runtime-config-restart-b".to_string()),
    );
    let report = store.replace_stable_overrides(&stable);

    assert_eq!(
        report.requires_restart_keys,
        vec!["BSL_CACHE_DIR".to_string()]
    );
}

#[test]
fn runtime_override_is_not_reported_as_requires_restart() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let mut stable = HashMap::new();
    stable.insert("BSL_CACHE_DISABLE".to_string(), JsonValue::Bool(true));
    let report = store.replace_stable_overrides(&stable);
    assert!(report.requires_restart_keys.is_empty());
}

#[test]
fn completion_runtime_keys_have_stable_tier_and_defaults() {
    let store = RuntimeConfigStore::new_from_env_bootstrap();
    let snapshot = store.snapshot();

    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_SCALE_AWARE_POLICY_ENABLED"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_SCALE_AWARE_LARGE_DOC_BYTES"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_SCALE_AWARE_LARGE_DOC_LINES"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_SCALE_AWARE_CHURN_WINDOW_MS"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_SCALE_AWARE_CHURN_MIN_CHANGES"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot.tiers.get("BSL_INTELLISENSE_V2_COMPLETION_MODE"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY"),
        Some(&ConfigTier::Stable)
    );
    assert_eq!(
        snapshot
            .tiers
            .get("BSL_INTELLISENSE_V2_DID_SAVE_FOLLOWUP_LANE_QUOTA"),
        Some(&ConfigTier::Stable)
    );

    assert_eq!(
        store.get_string(RuntimeKey::IntellisenseV2CompletionMode),
        Some("on".to_string())
    );
    assert_eq!(
        store.get_bool(RuntimeKey::IntellisenseV2ScaleAwarePolicyEnabled),
        Some(true)
    );
    assert_eq!(
        store.get_usize(RuntimeKey::IntellisenseV2ScaleAwareLargeDocBytes),
        Some(64 * 1024)
    );
    assert_eq!(
        store.get_usize(RuntimeKey::IntellisenseV2ScaleAwareLargeDocLines),
        Some(2_000)
    );
    assert_eq!(
        store.get_u64(RuntimeKey::IntellisenseV2ScaleAwareChurnWindowMs),
        Some(1_500)
    );
    assert_eq!(
        store.get_u64(RuntimeKey::IntellisenseV2ScaleAwareChurnMinChanges),
        Some(6)
    );
    assert_eq!(
        store.get_u64(RuntimeKey::IntellisenseV2CompletionCanaryPercent),
        Some(0)
    );
    assert_eq!(
        store.get_usize(RuntimeKey::IntellisenseV2CompletionQueueCapacity),
        Some(256)
    );
    assert_eq!(
        store.get_usize(RuntimeKey::IntellisenseV2DidSaveFollowupLaneQuota),
        Some(1)
    );
}
