use super::{
    advance_large_churn_state, completion_canary_routing_key, completion_dispatch_enabled_for_mode,
    completion_publish_allowed, completion_route_canary_event_driven, completion_routing_plan,
    completion_shadow_internal_trigger_payload, completion_shadow_internal_trigger_value,
    should_defer_heavy_diagnostics_for_large_churn, should_schedule_profile,
    CompletionResponseRoute, LargeChurnTransition,
};
use bsl_runtime::application::{
    CompletionMode, DiagnosticsProfile, DiagnosticsTrigger, ScaleAwareDiagnosticsKnobs,
};
use std::time::{Duration, Instant};
use tower_lsp::lsp_types::{Position, Url};

#[test]
fn idle_heavy_runs_for_save_trigger_even_when_flow_sensitive_disabled() {
    assert!(!should_schedule_profile(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::IdleHeavy,
        false
    ));
    assert!(should_schedule_profile(
        DiagnosticsTrigger::DidSave,
        DiagnosticsProfile::IdleHeavy,
        false
    ));
    assert!(should_schedule_profile(
        DiagnosticsTrigger::Idle,
        DiagnosticsProfile::IdleHeavy,
        false
    ));
    assert!(should_schedule_profile(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::IdleHeavy,
        true
    ));
    assert!(should_schedule_profile(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::Fast,
        false
    ));
    assert!(should_schedule_profile(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::DebouncedFull,
        false
    ));
}

#[test]
fn large_churn_defers_heavy_profiles_for_did_change_only() {
    assert!(should_defer_heavy_diagnostics_for_large_churn(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::DebouncedFull,
        true
    ));
    assert!(should_defer_heavy_diagnostics_for_large_churn(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::IdleHeavy,
        true
    ));
    assert!(!should_defer_heavy_diagnostics_for_large_churn(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::Fast,
        true
    ));
    assert!(!should_defer_heavy_diagnostics_for_large_churn(
        DiagnosticsTrigger::DidSave,
        DiagnosticsProfile::DebouncedFull,
        true
    ));
    assert!(!should_defer_heavy_diagnostics_for_large_churn(
        DiagnosticsTrigger::DidChange,
        DiagnosticsProfile::DebouncedFull,
        false
    ));
}

#[test]
fn large_churn_state_enters_on_threshold_and_exits_after_window_reset() {
    let knobs = ScaleAwareDiagnosticsKnobs {
        enabled: true,
        large_doc_bytes: 64 * 1024,
        large_doc_lines: 2_000,
        churn_window: Duration::from_millis(150),
        churn_min_changes: 3,
    };
    let start = Instant::now();
    let mut state = crate::server::ScaleAwareChurnStateV2 {
        window_started_at: start,
        changes_in_window: 0,
        large_churn_active: false,
    };

    assert_eq!(
        advance_large_churn_state(&mut state, start, true, knobs),
        LargeChurnTransition::None
    );
    assert_eq!(
        advance_large_churn_state(&mut state, start + Duration::from_millis(40), true, knobs),
        LargeChurnTransition::None
    );
    assert_eq!(
        advance_large_churn_state(&mut state, start + Duration::from_millis(80), true, knobs),
        LargeChurnTransition::Entered
    );
    assert!(state.large_churn_active);
    assert_eq!(state.changes_in_window, 3);

    assert_eq!(
        advance_large_churn_state(&mut state, start + Duration::from_millis(300), true, knobs),
        LargeChurnTransition::Exited
    );
    assert!(!state.large_churn_active);
    assert_eq!(state.changes_in_window, 1);
}

#[test]
fn completion_publish_guard_requires_latest_epoch() {
    assert!(completion_publish_allowed(3, Some(3)));
    assert!(completion_publish_allowed(1, None));
    assert!(!completion_publish_allowed(3, Some(4)));
}

#[test]
fn completion_publish_guard_rejects_superseded_epochs_in_burst() {
    let latest_epoch = Some(11);
    assert!(!completion_publish_allowed(9, latest_epoch));
    assert!(!completion_publish_allowed(10, latest_epoch));
    assert!(completion_publish_allowed(11, latest_epoch));
}

#[test]
fn completion_dispatch_disabled_only_for_off_mode() {
    assert!(!completion_dispatch_enabled_for_mode(CompletionMode::Off));
    assert!(completion_dispatch_enabled_for_mode(CompletionMode::Shadow));
    assert!(completion_dispatch_enabled_for_mode(CompletionMode::Canary));
    assert!(completion_dispatch_enabled_for_mode(CompletionMode::On));
}

#[test]
fn completion_canary_routing_is_deterministic_for_same_key() {
    let key = "file:///test.bsl:10:5:invoked:0:3";
    let first = completion_route_canary_event_driven(key, 37);
    for _ in 0..16 {
        assert_eq!(completion_route_canary_event_driven(key, 37), first);
    }
}

#[test]
fn completion_canary_routing_uses_threshold_bucket() {
    let key = "file:///test.bsl:1:2:trigger_character:46:9";
    let bucket = (bsl_shared::utils::hash::hash_content(key) % 100) as u8;
    assert!(!completion_route_canary_event_driven(key, bucket));
    let next_threshold = bucket.saturating_add(1).max(1);
    assert!(completion_route_canary_event_driven(key, next_threshold));
}

#[test]
fn completion_routing_plan_follows_mode_contract() {
    let key = "file:///test.bsl:2:4:invoked:0:-1";

    let off = completion_routing_plan(CompletionMode::Off, 100, key);
    assert_eq!(off.response_route, CompletionResponseRoute::Legacy);
    assert!(!off.run_shadow_event_driven);

    let shadow = completion_routing_plan(CompletionMode::Shadow, 100, key);
    assert_eq!(shadow.response_route, CompletionResponseRoute::Legacy);
    assert!(shadow.run_shadow_event_driven);

    let canary_zero = completion_routing_plan(CompletionMode::Canary, 0, key);
    assert_eq!(canary_zero.response_route, CompletionResponseRoute::Legacy);
    assert!(!canary_zero.run_shadow_event_driven);

    let canary_hundred = completion_routing_plan(CompletionMode::Canary, 100, key);
    assert_eq!(
        canary_hundred.response_route,
        CompletionResponseRoute::EventDriven
    );
    assert!(!canary_hundred.run_shadow_event_driven);

    let on = completion_routing_plan(CompletionMode::On, 0, key);
    assert_eq!(on.response_route, CompletionResponseRoute::EventDriven);
    assert!(!on.run_shadow_event_driven);
}

#[test]
fn completion_mode_parity_groups_are_stable_for_fixed_revision() {
    let key = "file:///test_fixed_revision.bsl:15:9:trigger_character:46:42";

    let off = completion_routing_plan(CompletionMode::Off, 50, key).response_route;
    let shadow = completion_routing_plan(CompletionMode::Shadow, 50, key).response_route;
    let canary_zero = completion_routing_plan(CompletionMode::Canary, 0, key).response_route;
    let canary_hundred = completion_routing_plan(CompletionMode::Canary, 100, key).response_route;
    let on = completion_routing_plan(CompletionMode::On, 50, key).response_route;

    assert_eq!(off, CompletionResponseRoute::Legacy);
    assert_eq!(shadow, CompletionResponseRoute::Legacy);
    assert_eq!(canary_zero, CompletionResponseRoute::Legacy);
    assert_eq!(canary_hundred, CompletionResponseRoute::EventDriven);
    assert_eq!(on, CompletionResponseRoute::EventDriven);
}

#[test]
fn completion_shadow_internal_trigger_roundtrip_keeps_original_char() {
    let dot_encoded = completion_shadow_internal_trigger_value(Some('.'));
    assert_eq!(
        completion_shadow_internal_trigger_payload(&dot_encoded),
        Some(Some('.'))
    );

    let none_encoded = completion_shadow_internal_trigger_value(None);
    assert_eq!(
        completion_shadow_internal_trigger_payload(&none_encoded),
        Some(None)
    );
}

#[test]
fn completion_canary_routing_key_is_stable_for_same_inputs() {
    let uri = Url::parse("file:///test.bsl").expect("url");
    let first =
        completion_canary_routing_key(&uri, Position::new(10, 4), "invoked", Some('.'), Some(7));
    let second =
        completion_canary_routing_key(&uri, Position::new(10, 4), "invoked", Some('.'), Some(7));
    assert_eq!(first, second);
}
