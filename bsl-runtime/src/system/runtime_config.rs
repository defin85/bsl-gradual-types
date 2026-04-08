//! Runtime configuration registry and store for `BSL_*` keys.
//!
//! Goals:
//! - Centralize parsing/defaults for all runtime `BSL_*` keys.
//! - Support runtime overrides (stable + dev-only) applied without restarting a process.
//! - Keep dev-only keys isolated so they can be removed later with minimal impact.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[path = "runtime_config/key_methods.rs"]
mod key_methods;
#[path = "runtime_config/parsing.rs"]
mod parsing;
#[path = "runtime_config/store.rs"]
mod store;

use self::parsing::{apply_one_override, collect_requires_restart_keys, read_env_value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigTier {
    Stable,
    DevOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyMutability {
    Runtime,
    StartupOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoolMode {
    /// `env.is_ok()` legacy behavior: any value => true, missing => false.
    Presence,
    /// `"1|true|yes|on"` => true; `"0|false|no|off"` => false; missing => default.
    Truthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueKind {
    Bool {
        mode: BoolMode,
    },
    U16,
    U64 {
        positive_only: bool,
        /// If true, value `0` is treated as "unset" (None), matching legacy semantics for some
        /// `BSL_*_MS` thresholds where `0` disables the feature.
        zero_means_none: bool,
    },
    Usize {
        positive_only: bool,
    },
    String,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    U16(u16),
    U64(u64),
    Usize(usize),
    String(String),
    Path(String),
}

impl ConfigValue {
    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    fn as_u16(&self) -> Option<u16> {
        match self {
            Self::U16(v) => Some(*v),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(v) => Some(*v),
            _ => None,
        }
    }

    fn as_usize(&self) -> Option<usize> {
        match self {
            Self::Usize(v) => Some(*v),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v.as_str()),
            Self::Path(v) => Some(v.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
enum LayerValue {
    Unset,
    Null,
    Value(ConfigValue),
}

impl LayerValue {
    fn is_unset(&self) -> bool {
        matches!(self, LayerValue::Unset)
    }

    fn effective_value(&self) -> Option<ConfigValue> {
        match self {
            LayerValue::Unset => None,
            LayerValue::Null => None,
            LayerValue::Value(v) => Some(v.clone()),
        }
    }
}

#[derive(Debug, Clone)]
struct KeySpec {
    env: &'static str,
    kind: ValueKind,
    tier: ConfigTier,
    default: Option<ConfigValue>,
    mutability: KeyMutability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueSource {
    Default,
    EnvBootstrap,
    StableOverride,
    DevOverride,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeConfigSnapshot {
    #[serde(rename = "effective")]
    pub effective: HashMap<&'static str, Option<ConfigValue>>,
    #[serde(rename = "sources")]
    pub sources: HashMap<&'static str, ValueSource>,
    #[serde(rename = "tiers")]
    pub tiers: HashMap<&'static str, ConfigTier>,
    #[serde(rename = "mutability")]
    pub mutability: HashMap<&'static str, KeyMutability>,
}

#[derive(Debug, Clone)]
struct RuntimeConfigState {
    env_bootstrap: Vec<LayerValue>,
    stable_overrides: Vec<LayerValue>,
    dev_overrides: Vec<LayerValue>,
}

impl RuntimeConfigState {
    fn new() -> Self {
        Self {
            env_bootstrap: vec![LayerValue::Unset; RuntimeKey::ALL.len()],
            stable_overrides: vec![LayerValue::Unset; RuntimeKey::ALL.len()],
            dev_overrides: vec![LayerValue::Unset; RuntimeKey::ALL.len()],
        }
    }

    fn compute_snapshot(&self) -> RuntimeConfigSnapshot {
        let mut effective = HashMap::new();
        let mut sources = HashMap::new();
        let mut tiers = HashMap::new();
        let mut mutability = HashMap::new();

        for (idx, key) in RuntimeKey::ALL.iter().enumerate() {
            let spec = key.spec();
            tiers.insert(spec.env, spec.tier);
            mutability.insert(spec.env, spec.mutability);

            let (value, source) = if !self.dev_overrides[idx].is_unset() {
                (
                    self.dev_overrides[idx].effective_value(),
                    ValueSource::DevOverride,
                )
            } else if !self.stable_overrides[idx].is_unset() {
                (
                    self.stable_overrides[idx].effective_value(),
                    ValueSource::StableOverride,
                )
            } else if !self.env_bootstrap[idx].is_unset() {
                (
                    self.env_bootstrap[idx].effective_value(),
                    ValueSource::EnvBootstrap,
                )
            } else {
                (spec.default.clone(), ValueSource::Default)
            };

            effective.insert(spec.env, value);
            sources.insert(spec.env, source);
        }

        RuntimeConfigSnapshot {
            effective,
            sources,
            tiers,
            mutability,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOverridesReport {
    pub ignored_unknown_keys: Vec<String>,
    pub ignored_invalid_values: Vec<String>,
    pub ignored_wrong_tier_keys: Vec<String>,
    pub dev_overrides_ignored: bool,
    pub requires_restart_keys: Vec<String>,
}

impl ApplyOverridesReport {
    fn empty() -> Self {
        Self {
            ignored_unknown_keys: Vec::new(),
            ignored_invalid_values: Vec::new(),
            ignored_wrong_tier_keys: Vec::new(),
            dev_overrides_ignored: false,
            requires_restart_keys: Vec::new(),
        }
    }
}

/// Thread-safe runtime config store.
///
/// Reads are snapshot-based. Updates rebuild the snapshot so the hot path does not parse strings.
#[derive(Clone)]
pub struct RuntimeConfigStore {
    state: Arc<RwLock<RuntimeConfigState>>,
    snapshot: Arc<RwLock<RuntimeConfigSnapshot>>,
}

// === Registry ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeKey {
    CacheDir,
    CacheDisable,
    CacheTtlSecs,
    CacheTtlMode,
    CacheMaxBytes,
    CacheCleanupIntervalSecs,
    CacheTouchIntervalSecs,
    CacheSwr,
    CacheStrictFingerprint,
    AstCacheCapacity,
    IndexWarmup,
    LspDiagnosticsDebounceMs,
    IntellisenseV2SlowClientLogMs,
    IntellisenseV2SlowWaitWarnMs,
    IntellisenseV2SlowSnapshotWarnMs,
    IntellisenseV2SlowQueryWarnMs,
    IntellisenseV2InteractiveWaitBudgetMs,
    IntellisenseV2ScaleAwarePolicyEnabled,
    IntellisenseV2ScaleAwareLargeDocBytes,
    IntellisenseV2ScaleAwareLargeDocLines,
    IntellisenseV2ScaleAwareChurnWindowMs,
    IntellisenseV2ScaleAwareChurnMinChanges,
    IntellisenseV2CompletionMode,
    IntellisenseV2CompletionCanaryPercent,
    IntellisenseV2CompletionQueueCapacity,
    IntellisenseV2DidSaveFollowupLaneQuota,
    AgentHttpAddr,
    AgentHttpStaticDir,
    AgentStateTtlSecs,
    WebHost,
    WebPort,
    StaticPath,
    ProjectPath,
    PlatformVersion,
    EnableCors,
    LogLevel,
    SyntaxHelperPath,
    CompletionTrace,
    CompletionQuality,
    IntellisenseV2P3Smoke,
    IntellisenseV2P4Smoke,
    SlowModuleThresholdMs,
    SlowModuleTopN,
    ModuleParseLogEach,
    RunWebApiTests,
    WebApiBaseUrl,
}

impl RuntimeKey {
    pub const ALL: &'static [RuntimeKey] = &[
        RuntimeKey::CacheDir,
        RuntimeKey::CacheDisable,
        RuntimeKey::CacheTtlSecs,
        RuntimeKey::CacheTtlMode,
        RuntimeKey::CacheMaxBytes,
        RuntimeKey::CacheCleanupIntervalSecs,
        RuntimeKey::CacheTouchIntervalSecs,
        RuntimeKey::CacheSwr,
        RuntimeKey::CacheStrictFingerprint,
        RuntimeKey::AstCacheCapacity,
        RuntimeKey::IndexWarmup,
        RuntimeKey::LspDiagnosticsDebounceMs,
        RuntimeKey::IntellisenseV2SlowClientLogMs,
        RuntimeKey::IntellisenseV2SlowWaitWarnMs,
        RuntimeKey::IntellisenseV2SlowSnapshotWarnMs,
        RuntimeKey::IntellisenseV2SlowQueryWarnMs,
        RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs,
        RuntimeKey::IntellisenseV2ScaleAwarePolicyEnabled,
        RuntimeKey::IntellisenseV2ScaleAwareLargeDocBytes,
        RuntimeKey::IntellisenseV2ScaleAwareLargeDocLines,
        RuntimeKey::IntellisenseV2ScaleAwareChurnWindowMs,
        RuntimeKey::IntellisenseV2ScaleAwareChurnMinChanges,
        RuntimeKey::IntellisenseV2CompletionMode,
        RuntimeKey::IntellisenseV2CompletionCanaryPercent,
        RuntimeKey::IntellisenseV2CompletionQueueCapacity,
        RuntimeKey::IntellisenseV2DidSaveFollowupLaneQuota,
        RuntimeKey::AgentHttpAddr,
        RuntimeKey::AgentHttpStaticDir,
        RuntimeKey::AgentStateTtlSecs,
        RuntimeKey::WebHost,
        RuntimeKey::WebPort,
        RuntimeKey::StaticPath,
        RuntimeKey::ProjectPath,
        RuntimeKey::PlatformVersion,
        RuntimeKey::EnableCors,
        RuntimeKey::LogLevel,
        RuntimeKey::SyntaxHelperPath,
        RuntimeKey::CompletionTrace,
        RuntimeKey::CompletionQuality,
        RuntimeKey::IntellisenseV2P3Smoke,
        RuntimeKey::IntellisenseV2P4Smoke,
        RuntimeKey::SlowModuleThresholdMs,
        RuntimeKey::SlowModuleTopN,
        RuntimeKey::ModuleParseLogEach,
        RuntimeKey::RunWebApiTests,
        RuntimeKey::WebApiBaseUrl,
    ];

    fn spec(self) -> KeySpec {
        match self {
            RuntimeKey::CacheDir => KeySpec {
                env: "BSL_CACHE_DIR",
                kind: ValueKind::Path,
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::CacheDisable => KeySpec {
                env: "BSL_CACHE_DISABLE",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::CacheTtlSecs => KeySpec {
                env: "BSL_CACHE_TTL_SECS",
                kind: ValueKind::U64 {
                    positive_only: true,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::CacheTtlMode => KeySpec {
                env: "BSL_CACHE_TTL_MODE",
                kind: ValueKind::String,
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::String("created".to_string())),
                mutability: self.mutability(),
            },
            RuntimeKey::CacheMaxBytes => KeySpec {
                env: "BSL_CACHE_MAX_BYTES",
                kind: ValueKind::U64 {
                    positive_only: true,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::CacheCleanupIntervalSecs => KeySpec {
                env: "BSL_CACHE_CLEANUP_INTERVAL_SECS",
                kind: ValueKind::U64 {
                    positive_only: true,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(300)),
                mutability: self.mutability(),
            },
            RuntimeKey::CacheTouchIntervalSecs => KeySpec {
                env: "BSL_CACHE_TOUCH_INTERVAL_SECS",
                kind: ValueKind::U64 {
                    positive_only: true,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(60)),
                mutability: self.mutability(),
            },
            RuntimeKey::CacheSwr => KeySpec {
                env: "BSL_CACHE_SWR",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Bool(true)),
                mutability: self.mutability(),
            },
            RuntimeKey::CacheStrictFingerprint => KeySpec {
                env: "BSL_CACHE_STRICT_FINGERPRINT",
                kind: ValueKind::Bool {
                    mode: BoolMode::Presence,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::AstCacheCapacity => KeySpec {
                env: "BSL_AST_CACHE_CAPACITY",
                kind: ValueKind::Usize {
                    positive_only: true,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Usize(64)),
                mutability: self.mutability(),
            },
            RuntimeKey::IndexWarmup => KeySpec {
                env: "BSL_INDEX_WARMUP",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Bool(true)),
                mutability: self.mutability(),
            },
            RuntimeKey::LspDiagnosticsDebounceMs => KeySpec {
                env: "BSL_LSP_DIAGNOSTICS_DEBOUNCE_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(250)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2SlowClientLogMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_SLOW_CLIENT_LOG_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: true,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(2000)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2SlowWaitWarnMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_SLOW_WAIT_WARN_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: true,
                },
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2SlowSnapshotWarnMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_SLOW_SNAPSHOT_WARN_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: true,
                },
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2SlowQueryWarnMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_SLOW_QUERY_WARN_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: true,
                },
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2InteractiveWaitBudgetMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_INTERACTIVE_WAIT_BUDGET_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(120)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2ScaleAwarePolicyEnabled => KeySpec {
                env: "BSL_INTELLISENSE_V2_SCALE_AWARE_POLICY_ENABLED",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Bool(true)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2ScaleAwareLargeDocBytes => KeySpec {
                env: "BSL_INTELLISENSE_V2_SCALE_AWARE_LARGE_DOC_BYTES",
                kind: ValueKind::Usize {
                    positive_only: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Usize(64 * 1024)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2ScaleAwareLargeDocLines => KeySpec {
                env: "BSL_INTELLISENSE_V2_SCALE_AWARE_LARGE_DOC_LINES",
                kind: ValueKind::Usize {
                    positive_only: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Usize(2_000)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2ScaleAwareChurnWindowMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_SCALE_AWARE_CHURN_WINDOW_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(1_500)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2ScaleAwareChurnMinChanges => KeySpec {
                env: "BSL_INTELLISENSE_V2_SCALE_AWARE_CHURN_MIN_CHANGES",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(6)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2CompletionMode => KeySpec {
                env: "BSL_INTELLISENSE_V2_COMPLETION_MODE",
                kind: ValueKind::String,
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::String("on".to_string())),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2CompletionCanaryPercent => KeySpec {
                env: "BSL_INTELLISENSE_V2_COMPLETION_CANARY_PERCENT",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(0)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2CompletionQueueCapacity => KeySpec {
                env: "BSL_INTELLISENSE_V2_COMPLETION_QUEUE_CAPACITY",
                kind: ValueKind::Usize {
                    positive_only: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Usize(256)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2DidSaveFollowupLaneQuota => KeySpec {
                env: "BSL_INTELLISENSE_V2_DID_SAVE_FOLLOWUP_LANE_QUOTA",
                kind: ValueKind::Usize {
                    positive_only: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Usize(1)),
                mutability: self.mutability(),
            },
            RuntimeKey::AgentHttpAddr => KeySpec {
                env: "BSL_AGENT_HTTP_ADDR",
                kind: ValueKind::String,
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::AgentHttpStaticDir => KeySpec {
                env: "BSL_AGENT_HTTP_STATIC_DIR",
                kind: ValueKind::Path,
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::AgentStateTtlSecs => KeySpec {
                env: "BSL_AGENT_STATE_TTL_SECS",
                kind: ValueKind::U64 {
                    positive_only: true,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(60 * 60 * 24 * 7)),
                mutability: self.mutability(),
            },
            RuntimeKey::WebHost => KeySpec {
                env: "BSL_WEB_HOST",
                kind: ValueKind::String,
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::String("127.0.0.1".to_string())),
                mutability: self.mutability(),
            },
            RuntimeKey::WebPort => KeySpec {
                env: "BSL_WEB_PORT",
                kind: ValueKind::U16,
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U16(8080)),
                mutability: self.mutability(),
            },
            RuntimeKey::StaticPath => KeySpec {
                env: "BSL_STATIC_PATH",
                kind: ValueKind::Path,
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::ProjectPath => KeySpec {
                env: "BSL_PROJECT_PATH",
                kind: ValueKind::Path,
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::PlatformVersion => KeySpec {
                env: "BSL_PLATFORM_VERSION",
                kind: ValueKind::String,
                tier: ConfigTier::Stable,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::EnableCors => KeySpec {
                env: "BSL_ENABLE_CORS",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::Bool(true)),
                mutability: self.mutability(),
            },
            RuntimeKey::LogLevel => KeySpec {
                env: "BSL_LOG_LEVEL",
                kind: ValueKind::String,
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::String("info".to_string())),
                mutability: self.mutability(),
            },
            RuntimeKey::SyntaxHelperPath => KeySpec {
                env: "BSL_SYNTAX_HELPER_PATH",
                kind: ValueKind::Path,
                tier: ConfigTier::DevOnly,
                default: None,
                mutability: self.mutability(),
            },
            RuntimeKey::CompletionTrace => KeySpec {
                env: "BSL_COMPLETION_TRACE",
                kind: ValueKind::Bool {
                    mode: BoolMode::Presence,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::CompletionQuality => KeySpec {
                env: "BSL_COMPLETION_QUALITY",
                kind: ValueKind::Bool {
                    mode: BoolMode::Presence,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2P3Smoke => KeySpec {
                env: "BSL_INTELLISENSE_V2_P3_SMOKE",
                kind: ValueKind::Bool {
                    mode: BoolMode::Presence,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2P4Smoke => KeySpec {
                env: "BSL_INTELLISENSE_V2_P4_SMOKE",
                kind: ValueKind::Bool {
                    mode: BoolMode::Presence,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::SlowModuleThresholdMs => KeySpec {
                env: "BSL_SLOW_MODULE_THRESHOLD_MS",
                kind: ValueKind::U64 {
                    positive_only: true,
                    zero_means_none: false,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::U64(3000)),
                mutability: self.mutability(),
            },
            RuntimeKey::SlowModuleTopN => KeySpec {
                env: "BSL_SLOW_MODULE_TOP_N",
                kind: ValueKind::Usize {
                    positive_only: true,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Usize(5)),
                mutability: self.mutability(),
            },
            RuntimeKey::ModuleParseLogEach => KeySpec {
                env: "BSL_MODULE_PARSE_LOG_EACH",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::RunWebApiTests => KeySpec {
                env: "BSL_RUN_WEB_API_TESTS",
                kind: ValueKind::Bool {
                    mode: BoolMode::Truthy,
                },
                tier: ConfigTier::DevOnly,
                default: Some(ConfigValue::Bool(false)),
                mutability: self.mutability(),
            },
            RuntimeKey::WebApiBaseUrl => KeySpec {
                env: "BSL_WEB_API_BASE_URL",
                kind: ValueKind::String,
                tier: ConfigTier::DevOnly,
                default: None,
                mutability: self.mutability(),
            },
        }
    }
}

static NAME_TO_KEY: LazyLock<HashMap<&'static str, RuntimeKey>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for key in RuntimeKey::ALL {
        map.insert(key.spec().env, *key);
    }
    map
});

static GLOBAL: OnceLock<RuntimeConfigStore> = OnceLock::new();

pub fn global_runtime_config() -> &'static RuntimeConfigStore {
    GLOBAL.get_or_init(RuntimeConfigStore::new_from_env_bootstrap)
}

#[cfg(test)]
#[path = "runtime_config/tests.rs"]
mod tests;
