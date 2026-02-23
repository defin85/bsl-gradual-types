use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bsl_analysis_v2::FileId as V2FileId;

#[derive(Debug, Clone)]
pub(crate) struct CompletionCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CompletionCancellationToken {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn same_inner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CompletionCancellationEntry {
    pub file_id: V2FileId,
    pub request_epoch: u64,
    pub token: CompletionCancellationToken,
}

#[derive(Debug, Default)]
pub(crate) struct CompletionCancellationRegistry {
    entries: Mutex<HashMap<String, CompletionCancellationEntry>>,
}

impl CompletionCancellationRegistry {
    pub(crate) fn register_request(
        self: &Arc<Self>,
        request_id: String,
        file_id: V2FileId,
        request_epoch: u64,
    ) -> CompletionCancellationRegistration {
        let token = CompletionCancellationToken::new();
        let new_entry = CompletionCancellationEntry {
            file_id,
            request_epoch,
            token: token.clone(),
        };
        let replaced = {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            entries.insert(request_id.clone(), new_entry)
        };
        if let Some(previous) = replaced {
            previous.token.cancel();
        }
        CompletionCancellationRegistration {
            request_id,
            token,
            registry: Arc::clone(self),
        }
    }

    #[cfg(test)]
    pub(crate) fn get(&self, request_id: &str) -> Option<CompletionCancellationEntry> {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        entries.get(request_id).cloned()
    }

    pub(crate) fn cancel_request(&self, request_id: &str) -> Option<CompletionCancellationEntry> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = entries.remove(request_id)?;
        entry.token.cancel();
        Some(entry)
    }

    pub(crate) fn remove_file(&self, file_id: V2FileId) -> usize {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let request_ids: Vec<String> = entries
            .iter()
            .filter_map(|(request_id, entry)| {
                if entry.file_id == file_id {
                    Some(request_id.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut removed = 0usize;
        for request_id in request_ids {
            if let Some(entry) = entries.remove(&request_id) {
                entry.token.cancel();
                removed += 1;
            }
        }
        removed
    }

    fn remove_if_matches(
        &self,
        request_id: &str,
        token: &CompletionCancellationToken,
    ) -> Option<CompletionCancellationEntry> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let should_remove = entries
            .get(request_id)
            .map(|entry| entry.token.same_inner(token))
            .unwrap_or(false);
        if should_remove {
            entries.remove(request_id)
        } else {
            None
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        let entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        entries.len()
    }
}

#[derive(Debug)]
pub(crate) struct CompletionCancellationRegistration {
    request_id: String,
    token: CompletionCancellationToken,
    registry: Arc<CompletionCancellationRegistry>,
}

impl CompletionCancellationRegistration {
    pub(crate) fn token(&self) -> CompletionCancellationToken {
        self.token.clone()
    }
}

impl Drop for CompletionCancellationRegistration {
    fn drop(&mut self) {
        let _ = self
            .registry
            .remove_if_matches(&self.request_id, &self.token);
    }
}

#[cfg(test)]
mod tests {
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
}
