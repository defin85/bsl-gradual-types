use super::*;

#[test]
fn register_and_lookup_tracks_file_epoch_and_token() {
    let registry = Arc::new(CompletionCancellationRegistry::default());
    let registration = registry.register_request("42".to_string(), V2FileId(7), 3);

    let entry = registry.get("42").expect("registry entry");
    assert_eq!(entry.file_id, V2FileId(7));
    assert_eq!(entry.request_epoch, 3);
    assert!(!entry.token.is_cancelled());
    assert!(!registration.token().is_cancelled());
}

#[test]
fn replacing_same_request_id_cancels_previous_token() {
    let registry = Arc::new(CompletionCancellationRegistry::default());
    let first = registry.register_request("42".to_string(), V2FileId(7), 1);
    let first_token = first.token();
    let _second = registry.register_request("42".to_string(), V2FileId(7), 2);

    assert!(first_token.is_cancelled());
    assert_eq!(registry.get("42").expect("active entry").request_epoch, 2);
}

#[test]
fn cancel_request_removes_entry_and_sets_token() {
    let registry = Arc::new(CompletionCancellationRegistry::default());
    let registration = registry.register_request("42".to_string(), V2FileId(7), 1);
    let token = registration.token();
    assert!(!token.is_cancelled());

    let cancelled = registry.cancel_request("42").expect("cancelled entry");
    assert_eq!(cancelled.file_id, V2FileId(7));
    assert_eq!(cancelled.request_epoch, 1);
    assert!(cancelled.token.is_cancelled());
    assert!(token.is_cancelled());
    assert!(registry.get("42").is_none());
}

#[test]
fn dropping_registration_cleans_up_entry() {
    let registry = Arc::new(CompletionCancellationRegistry::default());
    let registration = registry.register_request("42".to_string(), V2FileId(7), 1);
    assert_eq!(registry.len(), 1);

    drop(registration);
    assert_eq!(registry.len(), 0);
}

#[test]
fn remove_file_cleans_all_entries_for_file() {
    let registry = Arc::new(CompletionCancellationRegistry::default());
    let first = registry.register_request("one".to_string(), V2FileId(7), 1);
    let second = registry.register_request("two".to_string(), V2FileId(7), 2);
    let other = registry.register_request("other".to_string(), V2FileId(8), 1);
    let first_token = first.token();
    let second_token = second.token();
    let other_token = other.token();

    let removed = registry.remove_file(V2FileId(7));
    assert_eq!(removed, 2);
    assert!(registry.get("one").is_none());
    assert!(registry.get("two").is_none());
    assert!(registry.get("other").is_some());
    assert!(first_token.is_cancelled());
    assert!(second_token.is_cancelled());
    assert!(!other_token.is_cancelled());
}
