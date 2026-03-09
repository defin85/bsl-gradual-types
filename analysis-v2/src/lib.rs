use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use salsa::Setter;

pub use bsl_line_index::{byte_offset_to_utf16, utf16_to_byte_offset, LineIndex};

pub mod ast_to_ir;
pub use ast_to_ir::AstToIrConverter;

mod derived_artifacts;
mod implicit_bindings;
mod type_inference_v2;
use derived_artifacts::{
    DerivedArtifactsCache, TypeIndexArtifact, TypeIndexArtifactKey, TypeIndexParseSnapshotMeta,
    TypeIndexStoreOutcome,
};

use bsl_diagnostics::{SemanticTypeHints, SemanticValidationVisitor};
use bsl_shared::analysis::{detect_type_guards, NarrowingEngine};
use bsl_shared::domain::repository::{InMemoryTypeRepository, TypeRepository};
use bsl_shared::domain::resolver::TypeResolver;
use bsl_shared::domain::signature_index::SignatureIndex;
use bsl_shared::domain::types::TypeResolution;
use bsl_shared::domain::types::{DiagnosticSeverity, ParseError, TypeDiagnostic};
use bsl_shared::domain::validators::TypeValidator;
use bsl_shared::domain::TypeMetadataLookup;
use bsl_shared::domain::{FlowAnalysisContext, NullSafetyAnalyzer};
use bsl_shared::formatting::DetailLevel;
use bsl_shared::ir::walk_program;
use bsl_shared::ir::SemanticProgram;
use bsl_shared::ir::{CfgNodeKind, NodeAtByteOffsetBias};
use bsl_shared::utils::hash::hash_content;
use bsl_syntax::ParseOptions;
use tree_sitter::Tree;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

pub const DEPS_SCHEMA_VERSION: &str = "deps-snapshot-v1";
pub const SETTINGS_SCHEMA_VERSION: &str = "settings-v1";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepsSnapshotId(String);

impl DepsSnapshotId {
    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DepsSnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettingsId(String);

impl SettingsId {
    pub fn from_hash(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SettingsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub struct SemanticDeps {
    pub repository: Arc<dyn TypeRepository>,
    pub signature_index: SignatureIndex,
    pub resolver: Option<Arc<TypeResolver>>,
    /// Явный флаг: платформа (Syntax Helper) загружена и SignatureIndex считается полным
    /// для целей диагностики "Неопределенная процедура или функция".
    pub platform_signatures_loaded: bool,
}

impl std::fmt::Debug for SemanticDeps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticDeps")
            .field("has_resolver", &self.resolver.is_some())
            .field(
                "platform_signatures_loaded",
                &self.platform_signatures_loaded,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct DepsDataSnapshot(pub Arc<SemanticDeps>);

impl PartialEq for DepsDataSnapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for DepsDataSnapshot {}

unsafe impl salsa::Update for DepsDataSnapshot {
    unsafe fn maybe_update(old_pointer: *mut Self, new_value: Self) -> bool {
        let old_value: &mut Self = unsafe { &mut *old_pointer };
        *old_value = new_value;
        true
    }
}

#[derive(Debug, Clone)]
pub enum Change {
    SetFile {
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
    },
    SetFileWithSnapshot {
        file_id: FileId,
        text: Arc<str>,
        version: i32,
        path: Arc<str>,
        parse_snapshot: ParseSnapshot,
    },
    RemoveFile {
        file_id: FileId,
    },
    SetDepsSnapshot {
        deps_id: DepsSnapshotId,
        deps: Arc<SemanticDeps>,
    },
    SetSettingsSnapshot {
        settings_id: SettingsId,
        diagnostics_detail_level: DetailLevel,
    },
}

pub type Cancellable<T> = Result<T, salsa::Cancelled>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeAtByteOffsetProfile {
    pub index_fetch_ms: u128,
    pub index_fetch_wait_ms: u128,
    pub index_fetch_unattributed_ms: u128,
    pub index_fetch_pre_first_salsa_event_wait_ms: u128,
    pub index_fetch_post_last_salsa_event_tail_ms: u128,
    pub index_fetch_inside_salsa_window_ms: u128,
    pub index_fetch_first_will_execute_type_index_ms: u128,
    pub index_fetch_last_will_execute_type_index_ms: u128,
    pub index_fetch_first_will_execute_parse_result_ms: u128,
    pub index_fetch_last_will_execute_parse_result_ms: u128,
    pub index_fetch_first_will_execute_other_ms: u128,
    pub index_fetch_last_will_execute_other_ms: u128,
    pub index_fetch_first_will_iterate_cycle_ms: u128,
    pub index_fetch_last_will_iterate_cycle_ms: u128,
    pub index_fetch_first_will_check_cancellation_ms: u128,
    pub index_fetch_last_will_check_cancellation_ms: u128,
    pub index_fetch_first_will_check_to_first_will_execute_type_index_ms: u128,
    pub index_fetch_last_will_check_to_first_will_execute_type_index_ms: u128,
    pub index_fetch_last_will_execute_parse_result_to_first_will_execute_type_index_ms: u128,
    pub index_fetch_idle_before_first_will_execute_type_index_ms: u128,
    pub index_fetch_active_at_entry: u64,
    pub index_fetch_events_before_first_will_execute_type_index_total: u64,
    pub index_fetch_will_check_before_first_will_execute_type_index_total: u64,
    pub index_fetch_will_execute_parse_result_before_first_will_execute_type_index_total: u64,
    pub index_fetch_first_will_execute_type_index_seen_total: u64,
    pub index_fetch_will_block_on_total: u64,
    pub index_fetch_will_block_on_type_index_total: u64,
    pub index_fetch_will_block_on_parse_result_total: u64,
    pub index_fetch_will_block_on_other_total: u64,
    pub index_fetch_will_execute_total: u64,
    pub index_fetch_will_execute_type_index_total: u64,
    pub index_fetch_will_execute_parse_result_total: u64,
    pub index_fetch_will_execute_other_total: u64,
    pub index_fetch_will_iterate_cycle_total: u64,
    pub index_fetch_did_validate_memoized_total: u64,
    pub index_fetch_did_validate_memoized_type_index_total: u64,
    pub index_fetch_did_validate_memoized_parse_result_total: u64,
    pub index_fetch_did_validate_memoized_other_total: u64,
    pub index_fetch_will_check_cancellation_total: u64,
    pub index_fetch_did_set_cancellation_flag_total: u64,
    pub index_fetch_global_did_set_cancellation_flag_total: u64,
    pub index_fetch_did_discard_total: u64,
    pub index_fetch_did_discard_accumulated_total: u64,
    pub index_fetch_revision_start: u64,
    pub index_fetch_revision_end: u64,
    pub index_fetch_revision_delta: u64,
    pub index_query_total_ms: u128,
    pub index_query_inputs_ms: u128,
    pub index_query_parse_result_query_ms: u128,
    pub index_query_build_ms: u128,
    pub index_parse_result_ms: u128,
    pub index_build_total_ms: u128,
    pub index_build_seed_module_context_ms: u128,
    pub index_build_local_function_summaries_ms: u128,
    pub index_build_visit_statements_ms: u128,
    pub index_scan_ms: u128,
    pub total_ms: u128,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAtByteOffsetProfiledResult {
    pub resolution: Option<TypeResolution>,
    pub profile: TypeAtByteOffsetProfile,
    pub serve_reason_code: TypeIndexServeReasonCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIndexServeReasonCode {
    TypeIndexExactHit,
    TypeIndexStaleServed,
    TypeIndexDegradedIncomplete,
    TypeIndexFallbackUnavailable,
}

impl TypeIndexServeReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeIndexExactHit => "type_index_exact_hit",
            Self::TypeIndexStaleServed => "type_index_stale_served",
            Self::TypeIndexDegradedIncomplete => "type_index_degraded_incomplete",
            Self::TypeIndexFallbackUnavailable => "type_index_fallback_unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIndexPrecomputeReasonCode {
    TypeIndexPrecomputeExactStored,
    TypeIndexPrecomputeSuperseded,
    TypeIndexPrecomputeCancelled,
    TypeIndexPrecomputeMissingFile,
    TypeIndexPrecomputeQueueSaturated,
}

impl TypeIndexPrecomputeReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeIndexPrecomputeExactStored => "type_index_precompute_exact_stored",
            Self::TypeIndexPrecomputeSuperseded => "type_index_precompute_superseded",
            Self::TypeIndexPrecomputeCancelled => "type_index_precompute_cancelled",
            Self::TypeIndexPrecomputeMissingFile => "type_index_precompute_missing_file",
            Self::TypeIndexPrecomputeQueueSaturated => "type_index_precompute_queue_saturated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIndexArtifactReasonCode {
    TypeIndexArtifactInvalidatedDeps,
    TypeIndexArtifactInvalidatedSettings,
    TypeIndexArtifactEvictedGlobalGuard,
    TypeIndexArtifactEvictedPerFileWindow,
}

impl TypeIndexArtifactReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeIndexArtifactInvalidatedDeps => "type_index_artifact_invalidated_deps",
            Self::TypeIndexArtifactInvalidatedSettings => {
                "type_index_artifact_invalidated_settings"
            }
            Self::TypeIndexArtifactEvictedGlobalGuard => "type_index_artifact_evicted_global_guard",
            Self::TypeIndexArtifactEvictedPerFileWindow => {
                "type_index_artifact_evicted_per_file_window"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TypeIndexCacheChangeEffects {
    pub invalidated_deps_total: u64,
    pub invalidated_settings_total: u64,
    pub evicted_per_file_window_total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeIndexPrecomputeStats {
    pub queue_wait_ms: u128,
    pub exec_ms: u128,
    pub build_ms: u128,
    pub evicted_per_file_window_total: u64,
    pub evicted_global_guard_total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeIndexPrecomputeResult {
    pub reason_code: TypeIndexPrecomputeReasonCode,
    pub file_version: Option<i32>,
    pub stats: TypeIndexPrecomputeStats,
}

impl TypeIndexPrecomputeResult {
    fn with_reason(reason_code: TypeIndexPrecomputeReasonCode) -> Self {
        Self {
            reason_code,
            file_version: None,
            stats: TypeIndexPrecomputeStats::default(),
        }
    }
}

fn cancellable<T>(op: impl FnOnce() -> T) -> Cancellable<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(op)) {
        Ok(value) => Ok(value),
        Err(payload) => match payload.downcast::<salsa::Cancelled>() {
            Ok(cancelled) => Err(*cancelled),
            Err(payload) => std::panic::resume_unwind(payload),
        },
    }
}

#[inline(always)]
fn cancellation_checkpoint(db: &dyn salsa::Database) {
    db.unwind_if_revision_cancelled();
}

include!("lib/salsa_events.rs");
include!("lib/snapshots.rs");
include!("lib/host_runtime.rs");
include!("lib/analysis_api.rs");

#[cfg(test)]
#[path = "lib/tests.rs"]
mod tests;
