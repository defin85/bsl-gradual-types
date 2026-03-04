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
#[path = "completion_cancellation/tests.rs"]
mod tests;
