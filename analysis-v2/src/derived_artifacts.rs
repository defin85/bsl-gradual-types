use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use bsl_shared::ir::SemanticProgram;

use crate::type_inference_v2;
use crate::{DepsSnapshotId, FileId, ParseSnapshot, SettingsId};

const DERIVED_CACHE_KEEP_VERSIONS: i32 = 2;
const TYPE_INDEX_MAX_VERSIONS_PER_IDENTITY: usize = 2;
const TYPE_INDEX_ARTIFACTS_MAX_TOTAL: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TypeIndexIdentity {
    deps_id: DepsSnapshotId,
    settings_id: SettingsId,
}

impl TypeIndexIdentity {
    fn new(deps_id: DepsSnapshotId, settings_id: SettingsId) -> Self {
        Self {
            deps_id,
            settings_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TypeIndexArtifactKey {
    pub(crate) file_id: FileId,
    pub(crate) file_version: i32,
    pub(crate) deps_id: DepsSnapshotId,
    pub(crate) settings_id: SettingsId,
}

impl TypeIndexArtifactKey {
    pub(crate) fn new(
        file_id: FileId,
        file_version: i32,
        deps_id: DepsSnapshotId,
        settings_id: SettingsId,
    ) -> Self {
        Self {
            file_id,
            file_version,
            deps_id,
            settings_id,
        }
    }

    fn identity(&self) -> TypeIndexIdentity {
        TypeIndexIdentity::new(self.deps_id.clone(), self.settings_id.clone())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TypeIndexParseSnapshotMeta {
    pub(crate) incremental: bool,
    pub(crate) changed_ranges_count: usize,
    pub(crate) fallback_reason_present: bool,
}

impl TypeIndexParseSnapshotMeta {
    pub(crate) fn from_snapshot(snapshot: Option<&ParseSnapshot>) -> Self {
        let Some(snapshot) = snapshot else {
            return Self::default();
        };
        Self {
            incremental: snapshot.incremental,
            changed_ranges_count: snapshot.changed_ranges.len(),
            fallback_reason_present: snapshot.fallback_reason.is_some(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TypeIndexArtifact {
    pub(crate) type_index: Arc<type_inference_v2::TypeIndex>,
    pub(crate) build_profile: type_inference_v2::TypeIndexBuildProfile,
    pub(crate) parse_snapshot_meta: TypeIndexParseSnapshotMeta,
    pub(crate) produced_at_millis: u128,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TypeIndexStoreOutcome {
    pub(crate) evicted_per_file_window_total: u64,
    pub(crate) evicted_global_guard_total: u64,
}

#[derive(Clone, Default)]
struct DerivedVersionArtifacts {
    ir_by_deps_id: HashMap<DepsSnapshotId, Arc<SemanticProgram>>,
    type_index_by_identity: HashMap<TypeIndexIdentity, Arc<TypeIndexArtifact>>,
}

impl DerivedVersionArtifacts {
    fn type_index_artifacts_count(&self) -> usize {
        self.type_index_by_identity.len()
    }
}

#[derive(Clone, Default)]
pub(crate) struct DerivedArtifactsCache {
    by_file: HashMap<FileId, BTreeMap<i32, DerivedVersionArtifacts>>,
    latest_type_index_version_by_file: HashMap<FileId, i32>,
    latest_type_index_identity_by_file: HashMap<FileId, TypeIndexIdentity>,
    type_index_artifacts_total: usize,
}

impl DerivedArtifactsCache {
    pub(crate) fn clear_file(&mut self, file_id: FileId) {
        if let Some(versioned) = self.by_file.remove(&file_id) {
            let removed: usize = versioned
                .values()
                .map(DerivedVersionArtifacts::type_index_artifacts_count)
                .sum();
            self.type_index_artifacts_total =
                self.type_index_artifacts_total.saturating_sub(removed);
        }
        self.latest_type_index_version_by_file.remove(&file_id);
        self.latest_type_index_identity_by_file.remove(&file_id);
    }

    fn prune_empty_file_entry_if_needed(&mut self, file_id: FileId) {
        if self
            .by_file
            .get(&file_id)
            .is_some_and(|versioned| versioned.is_empty())
        {
            self.by_file.remove(&file_id);
            self.latest_type_index_version_by_file.remove(&file_id);
            self.latest_type_index_identity_by_file.remove(&file_id);
        }
    }

    pub(crate) fn retain_versions_for_file(
        &mut self,
        file_id: FileId,
        current_version: i32,
    ) -> TypeIndexStoreOutcome {
        let min_version_to_keep = current_version.saturating_sub(DERIVED_CACHE_KEEP_VERSIONS);
        let outcome = TypeIndexStoreOutcome::default();
        if let Some(versioned) = self.by_file.get_mut(&file_id) {
            let versions_to_remove: Vec<i32> = versioned
                .keys()
                .copied()
                .filter(|cached_version| *cached_version < min_version_to_keep)
                .collect();
            for version in versions_to_remove {
                let mut remove_version_entry = false;
                if let Some(artifacts) = versioned.get_mut(&version) {
                    // IR versions remain window-based, while type_index retention
                    // is handled by per-identity count semantics.
                    artifacts.ir_by_deps_id.clear();
                    remove_version_entry = artifacts.type_index_by_identity.is_empty();
                }
                if remove_version_entry {
                    versioned.remove(&version);
                }
            }
        }
        self.prune_empty_file_entry_if_needed(file_id);
        outcome
    }

    pub(crate) fn get_ir(
        &self,
        file_id: FileId,
        file_version: i32,
        deps_id: &DepsSnapshotId,
    ) -> Option<Arc<SemanticProgram>> {
        self.by_file
            .get(&file_id)?
            .get(&file_version)?
            .ir_by_deps_id
            .get(deps_id)
            .cloned()
    }

    pub(crate) fn store_ir(
        &mut self,
        file_id: FileId,
        file_version: i32,
        deps_id: DepsSnapshotId,
        program: Arc<SemanticProgram>,
    ) {
        let _ = self.retain_versions_for_file(file_id, file_version);
        self.by_file
            .entry(file_id)
            .or_default()
            .entry(file_version)
            .or_default()
            .ir_by_deps_id
            .insert(deps_id, program);
    }

    pub(crate) fn get_type_index_exact(
        &self,
        key: &TypeIndexArtifactKey,
    ) -> Option<Arc<TypeIndexArtifact>> {
        let identity = key.identity();
        self.by_file
            .get(&key.file_id)?
            .get(&key.file_version)?
            .type_index_by_identity
            .get(&identity)
            .cloned()
    }

    pub(crate) fn get_type_index_stale(
        &self,
        key: &TypeIndexArtifactKey,
    ) -> Option<(Arc<TypeIndexArtifact>, i32)> {
        let identity = key.identity();
        let versioned = self.by_file.get(&key.file_id)?;
        versioned
            .iter()
            .rev()
            .filter(|(version, _)| **version < key.file_version)
            .find_map(|(version, artifacts)| {
                artifacts
                    .type_index_by_identity
                    .get(&identity)
                    .cloned()
                    .map(|artifact| (artifact, *version))
            })
    }

    pub(crate) fn store_type_index(
        &mut self,
        key: TypeIndexArtifactKey,
        artifact: Arc<TypeIndexArtifact>,
    ) -> TypeIndexStoreOutcome {
        let mut outcome = self.retain_versions_for_file(key.file_id, key.file_version);
        let identity = key.identity();
        let versioned = self.by_file.entry(key.file_id).or_default();
        let artifacts = versioned.entry(key.file_version).or_default();
        let replaced = artifacts
            .type_index_by_identity
            .insert(identity.clone(), artifact)
            .is_some();
        if !replaced {
            self.type_index_artifacts_total = self.type_index_artifacts_total.saturating_add(1);
        }

        self.latest_type_index_version_by_file
            .insert(key.file_id, key.file_version);
        self.latest_type_index_identity_by_file
            .insert(key.file_id, identity);

        let evicted_per_file_window = self.retain_type_index_versions_for_identity(
            key.file_id,
            &key.identity(),
            key.file_version,
        );
        outcome.evicted_per_file_window_total = outcome
            .evicted_per_file_window_total
            .saturating_add(evicted_per_file_window);

        let evicted_global = self.evict_type_index_global_guard(&key);
        outcome.evicted_global_guard_total = outcome
            .evicted_global_guard_total
            .saturating_add(evicted_global);
        outcome
    }

    pub(crate) fn invalidate_type_index_for_deps(&mut self, deps_id: &DepsSnapshotId) -> u64 {
        let mut removed_total = 0_u64;
        for versioned in self.by_file.values_mut() {
            for artifacts in versioned.values_mut() {
                let before = artifacts.type_index_by_identity.len();
                artifacts
                    .type_index_by_identity
                    .retain(|identity, _| &identity.deps_id == deps_id);
                let removed = before.saturating_sub(artifacts.type_index_by_identity.len());
                if removed > 0 {
                    removed_total = removed_total.saturating_add(removed as u64);
                    self.type_index_artifacts_total =
                        self.type_index_artifacts_total.saturating_sub(removed);
                }
            }
        }
        self.latest_type_index_identity_by_file
            .retain(|_, identity| &identity.deps_id == deps_id);
        removed_total
    }

    pub(crate) fn invalidate_type_index_for_settings(&mut self, settings_id: &SettingsId) -> u64 {
        let mut removed_total = 0_u64;
        for versioned in self.by_file.values_mut() {
            for artifacts in versioned.values_mut() {
                let before = artifacts.type_index_by_identity.len();
                artifacts
                    .type_index_by_identity
                    .retain(|identity, _| &identity.settings_id == settings_id);
                let removed = before.saturating_sub(artifacts.type_index_by_identity.len());
                if removed > 0 {
                    removed_total = removed_total.saturating_add(removed as u64);
                    self.type_index_artifacts_total =
                        self.type_index_artifacts_total.saturating_sub(removed);
                }
            }
        }
        self.latest_type_index_identity_by_file
            .retain(|_, identity| &identity.settings_id == settings_id);
        removed_total
    }

    fn evict_type_index_global_guard(&mut self, protected: &TypeIndexArtifactKey) -> u64 {
        let mut evicted = 0_u64;
        while self.type_index_artifacts_total > TYPE_INDEX_ARTIFACTS_MAX_TOTAL {
            let Some(candidate) = self.pick_global_evict_candidate(protected) else {
                break;
            };
            if self.remove_type_index_candidate(candidate) {
                evicted = evicted.saturating_add(1);
            } else {
                break;
            }
        }
        evicted
    }

    fn retain_type_index_versions_for_identity(
        &mut self,
        file_id: FileId,
        identity: &TypeIndexIdentity,
        current_version: i32,
    ) -> u64 {
        let Some(versioned) = self.by_file.get_mut(&file_id) else {
            return 0;
        };

        let mut versions_for_identity: Vec<i32> = versioned
            .iter()
            .filter_map(|(version, artifacts)| {
                artifacts
                    .type_index_by_identity
                    .contains_key(identity)
                    .then_some(*version)
            })
            .collect();
        if versions_for_identity.len() <= TYPE_INDEX_MAX_VERSIONS_PER_IDENTITY {
            return 0;
        }
        versions_for_identity.sort_unstable();

        let remove_total = versions_for_identity
            .len()
            .saturating_sub(TYPE_INDEX_MAX_VERSIONS_PER_IDENTITY);
        let versions_to_remove: Vec<i32> = versions_for_identity
            .into_iter()
            .take(remove_total)
            .collect();

        let mut evicted = 0_u64;
        for version in versions_to_remove {
            if version == current_version {
                continue;
            }
            let mut remove_version_entry = false;
            if let Some(artifacts) = versioned.get_mut(&version) {
                if artifacts.type_index_by_identity.remove(identity).is_some() {
                    self.type_index_artifacts_total =
                        self.type_index_artifacts_total.saturating_sub(1);
                    evicted = evicted.saturating_add(1);
                }
                remove_version_entry = artifacts.ir_by_deps_id.is_empty()
                    && artifacts.type_index_by_identity.is_empty();
            }
            if remove_version_entry {
                versioned.remove(&version);
            }
        }
        self.prune_empty_file_entry_if_needed(file_id);
        evicted
    }

    fn pick_global_evict_candidate(
        &self,
        protected: &TypeIndexArtifactKey,
    ) -> Option<TypeIndexEvictCandidate> {
        let mut candidates = Vec::new();
        for (file_id, versioned) in &self.by_file {
            let latest_version = self
                .latest_type_index_version_by_file
                .get(file_id)
                .copied()
                .unwrap_or(i32::MIN);
            let latest_identity = self.latest_type_index_identity_by_file.get(file_id);
            for (file_version, artifacts) in versioned {
                for (identity, artifact) in &artifacts.type_index_by_identity {
                    if *file_id == protected.file_id
                        && *file_version == protected.file_version
                        && identity.deps_id == protected.deps_id
                        && identity.settings_id == protected.settings_id
                    {
                        continue;
                    }
                    let priority = if *file_version < latest_version {
                        0_u8
                    } else if latest_identity.is_some_and(|latest| latest != identity) {
                        1_u8
                    } else {
                        2_u8
                    };
                    candidates.push(TypeIndexEvictCandidate {
                        file_id: *file_id,
                        file_version: *file_version,
                        identity: identity.clone(),
                        produced_at_millis: artifact.produced_at_millis,
                        priority,
                    });
                }
            }
        }
        candidates.into_iter().min_by(TypeIndexEvictCandidate::cmp)
    }

    fn remove_type_index_candidate(&mut self, candidate: TypeIndexEvictCandidate) -> bool {
        let Some(versioned) = self.by_file.get_mut(&candidate.file_id) else {
            return false;
        };
        let Some(artifacts) = versioned.get_mut(&candidate.file_version) else {
            return false;
        };
        if artifacts
            .type_index_by_identity
            .remove(&candidate.identity)
            .is_none()
        {
            return false;
        }
        self.type_index_artifacts_total = self.type_index_artifacts_total.saturating_sub(1);
        if artifacts.ir_by_deps_id.is_empty() && artifacts.type_index_by_identity.is_empty() {
            versioned.remove(&candidate.file_version);
        }
        self.prune_empty_file_entry_if_needed(candidate.file_id);
        true
    }
}

#[derive(Clone)]
struct TypeIndexEvictCandidate {
    file_id: FileId,
    file_version: i32,
    identity: TypeIndexIdentity,
    produced_at_millis: u128,
    priority: u8,
}

impl TypeIndexEvictCandidate {
    fn cmp(a: &Self, b: &Self) -> Ordering {
        (
            a.priority,
            a.produced_at_millis,
            a.file_id.0,
            a.file_version,
            a.identity.deps_id.as_str(),
            a.identity.settings_id.as_str(),
        )
            .cmp(&(
                b.priority,
                b.produced_at_millis,
                b.file_id.0,
                b.file_version,
                b.identity.deps_id.as_str(),
                b.identity.settings_id.as_str(),
            ))
    }
}

#[cfg(test)]
#[path = "derived_artifacts/tests.rs"]
mod tests;
