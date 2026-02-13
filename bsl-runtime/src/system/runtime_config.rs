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

    fn into_effective(&self) -> Option<ConfigValue> {
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
                    self.dev_overrides[idx].into_effective(),
                    ValueSource::DevOverride,
                )
            } else if !self.stable_overrides[idx].is_unset() {
                (
                    self.stable_overrides[idx].into_effective(),
                    ValueSource::StableOverride,
                )
            } else if !self.env_bootstrap[idx].is_unset() {
                (
                    self.env_bootstrap[idx].into_effective(),
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

impl RuntimeConfigStore {
    pub fn new_from_env_bootstrap() -> Self {
        let mut state = RuntimeConfigState::new();
        for (idx, key) in RuntimeKey::ALL.iter().enumerate() {
            let spec = key.spec();
            state.env_bootstrap[idx] = read_env_value(spec);
        }
        let snapshot = state.compute_snapshot();
        Self {
            state: Arc::new(RwLock::new(state)),
            snapshot: Arc::new(RwLock::new(snapshot)),
        }
    }

    /// Re-read environment variables into the bootstrap layer.
    ///
    /// This is primarily used by tests that temporarily mutate `std::env` and expect the changes
    /// to affect behavior without restarting the process.
    pub fn reload_env_bootstrap_from_env(&self) {
        let mut guard = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for (idx, key) in RuntimeKey::ALL.iter().enumerate() {
            guard.env_bootstrap[idx] = read_env_value(key.spec());
        }
        *self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = guard.compute_snapshot();
    }

    pub fn snapshot(&self) -> RuntimeConfigSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn get_bool(&self, key: RuntimeKey) -> Option<bool> {
        self.get_value(key).and_then(|v| v.as_bool())
    }

    pub fn get_u16(&self, key: RuntimeKey) -> Option<u16> {
        self.get_value(key).and_then(|v| v.as_u16())
    }

    pub fn get_u64(&self, key: RuntimeKey) -> Option<u64> {
        self.get_value(key).and_then(|v| v.as_u64())
    }

    pub fn get_usize(&self, key: RuntimeKey) -> Option<usize> {
        self.get_value(key).and_then(|v| v.as_usize())
    }

    pub fn get_string(&self, key: RuntimeKey) -> Option<String> {
        self.get_value(key)
            .and_then(|v| v.as_string().map(|s| s.to_string()))
    }

    pub fn get_pathbuf(&self, key: RuntimeKey) -> Option<PathBuf> {
        self.get_value(key)
            .and_then(|v| v.as_string().map(PathBuf::from))
    }

    fn get_value(&self, key: RuntimeKey) -> Option<ConfigValue> {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let env = key.spec().env;
        snapshot.effective.get(env).and_then(|v| v.clone())
    }

    /// Replace stable overrides entirely (keys not present become unset).
    pub fn replace_stable_overrides(
        &self,
        overrides: &HashMap<String, JsonValue>,
    ) -> ApplyOverridesReport {
        let mut report = ApplyOverridesReport::empty();
        let mut guard = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = guard.compute_snapshot();

        guard.stable_overrides.fill(LayerValue::Unset);
        for (k, v) in overrides {
            apply_one_override(
                &mut guard.stable_overrides,
                ConfigTier::Stable,
                k,
                v,
                &mut report,
            );
        }

        let after = guard.compute_snapshot();
        report.requires_restart_keys = collect_requires_restart_keys(&before, &after);
        *self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = after;
        report
    }

    /// Replace dev-only overrides entirely (keys not present become unset).
    pub fn replace_dev_overrides(
        &self,
        overrides: &HashMap<String, JsonValue>,
        allow_dev_overrides: bool,
    ) -> ApplyOverridesReport {
        let mut report = ApplyOverridesReport::empty();
        let mut guard = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = guard.compute_snapshot();
        guard.dev_overrides.fill(LayerValue::Unset);
        if !allow_dev_overrides {
            if !overrides.is_empty() {
                report.dev_overrides_ignored = true;
            }
        } else {
            for (k, v) in overrides {
                apply_one_override(
                    &mut guard.dev_overrides,
                    ConfigTier::DevOnly,
                    k,
                    v,
                    &mut report,
                );
            }
        }
        let after = guard.compute_snapshot();
        report.requires_restart_keys = collect_requires_restart_keys(&before, &after);
        *self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = after;
        report
    }

    /// Patch overrides (null removes a key). Used by bsl-agent runtime update.
    pub fn patch_overrides(
        &self,
        stable: Option<&HashMap<String, JsonValue>>,
        dev: Option<&HashMap<String, JsonValue>>,
        allow_dev_overrides: bool,
    ) -> ApplyOverridesReport {
        let mut report = ApplyOverridesReport::empty();
        let mut guard = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = guard.compute_snapshot();

        if let Some(stable) = stable {
            for (k, v) in stable {
                apply_one_override(
                    &mut guard.stable_overrides,
                    ConfigTier::Stable,
                    k,
                    v,
                    &mut report,
                );
            }
        }

        if !allow_dev_overrides {
            // Dev-only layer is inactive unless explicitly allowed. Clear it to avoid stale state.
            guard.dev_overrides.fill(LayerValue::Unset);
            if dev.is_some_and(|m| !m.is_empty()) {
                report.dev_overrides_ignored = true;
            }
        } else if let Some(dev) = dev {
            for (k, v) in dev {
                apply_one_override(
                    &mut guard.dev_overrides,
                    ConfigTier::DevOnly,
                    k,
                    v,
                    &mut report,
                );
            }
        }

        let after = guard.compute_snapshot();
        report.requires_restart_keys = collect_requires_restart_keys(&before, &after);
        *self
            .snapshot
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = after;
        report
    }
}

fn collect_requires_restart_keys(
    before: &RuntimeConfigSnapshot,
    after: &RuntimeConfigSnapshot,
) -> Vec<String> {
    let mut changed = Vec::new();
    for key in RuntimeKey::ALL {
        let spec = key.spec();
        if spec.mutability != KeyMutability::StartupOnly {
            continue;
        }
        let env = spec.env;
        let before_value = before.effective.get(env);
        let after_value = after.effective.get(env);
        if before_value != after_value {
            changed.push(env.to_string());
        }
    }
    changed
}

fn apply_one_override(
    layer: &mut [LayerValue],
    expected_tier: ConfigTier,
    key: &str,
    value: &JsonValue,
    report: &mut ApplyOverridesReport,
) {
    let Some(runtime_key) = NAME_TO_KEY.get(key).copied() else {
        report.ignored_unknown_keys.push(key.to_string());
        return;
    };

    if value.is_null() {
        layer[runtime_key.index()] = LayerValue::Unset;
        return;
    }

    let spec = runtime_key.spec();
    if spec.tier != expected_tier {
        report.ignored_wrong_tier_keys.push(key.to_string());
        return;
    }

    if let ValueKind::U64 {
        zero_means_none: true,
        ..
    } = spec.kind
    {
        let is_zero = value.as_u64() == Some(0)
            || value
                .as_str()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .is_some_and(|v| v == 0);
        if is_zero {
            layer[runtime_key.index()] = LayerValue::Null;
            return;
        }
    }
    match parse_json_value(spec, value) {
        Ok(parsed) => {
            layer[runtime_key.index()] = LayerValue::Value(parsed);
        }
        Err(_) => {
            report.ignored_invalid_values.push(key.to_string());
        }
    }
}

fn read_env_value(spec: KeySpec) -> LayerValue {
    let Ok(raw) = std::env::var(spec.env) else {
        return LayerValue::Unset;
    };

    match spec.kind {
        ValueKind::Bool { mode } => match mode {
            BoolMode::Presence => LayerValue::Value(ConfigValue::Bool(true)),
            BoolMode::Truthy => LayerValue::Value(ConfigValue::Bool(parse_bool_truthy(
                &raw,
                spec.default
                    .as_ref()
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false),
            ))),
        },
        ValueKind::U64 {
            zero_means_none: true,
            ..
        } => {
            if raw.trim().parse::<u64>().ok() == Some(0) {
                return LayerValue::Null;
            }
            parse_env_string(spec, &raw)
                .map(LayerValue::Value)
                .unwrap_or(LayerValue::Unset)
        }
        _ => parse_env_string(spec, &raw)
            .map(LayerValue::Value)
            .unwrap_or(LayerValue::Unset),
    }
}

fn parse_bool_truthy(raw: &str, default: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        "" => default,
        _ => default,
    }
}

fn parse_env_string(spec: KeySpec, raw: &str) -> Result<ConfigValue, ()> {
    let trimmed = raw.trim();
    match spec.kind {
        ValueKind::U16 => trimmed.parse::<u16>().map(ConfigValue::U16).map_err(|_| ()),
        ValueKind::U64 {
            positive_only,
            zero_means_none,
        } => trimmed
            .parse::<u64>()
            .ok()
            .filter(|v| !positive_only || *v > 0)
            .filter(|v| !(zero_means_none && *v == 0))
            .map(ConfigValue::U64)
            .ok_or(()),
        ValueKind::Usize { positive_only } => trimmed
            .parse::<usize>()
            .ok()
            .filter(|v| !positive_only || *v > 0)
            .map(ConfigValue::Usize)
            .ok_or(()),
        ValueKind::String => Ok(ConfigValue::String(trimmed.to_string())),
        ValueKind::Path => Ok(ConfigValue::Path(trimmed.to_string())),
        ValueKind::Bool { .. } => Err(()),
    }
}

fn parse_json_value(spec: KeySpec, value: &JsonValue) -> Result<ConfigValue, ()> {
    match spec.kind {
        ValueKind::Bool { mode } => {
            if let Some(b) = value.as_bool() {
                return Ok(ConfigValue::Bool(b));
            }
            if let Some(s) = value.as_str() {
                return Ok(ConfigValue::Bool(match mode {
                    // JSON overrides are explicit, so accept truthy/falsey strings too.
                    BoolMode::Presence => parse_bool_truthy(s, false),
                    BoolMode::Truthy => parse_bool_truthy(
                        s,
                        spec.default
                            .as_ref()
                            .and_then(|d| d.as_bool())
                            .unwrap_or(false),
                    ),
                }));
            }
            Err(())
        }
        ValueKind::U16 => value
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .map(ConfigValue::U16)
            .ok_or(()),
        ValueKind::U64 {
            positive_only,
            zero_means_none,
        } => value
            .as_u64()
            .filter(|v| !positive_only || *v > 0)
            .filter(|v| !(zero_means_none && *v == 0))
            .map(ConfigValue::U64)
            .ok_or(()),
        ValueKind::Usize { positive_only } => value
            .as_u64()
            .and_then(|v| usize::try_from(v).ok())
            .filter(|v| !positive_only || *v > 0)
            .map(ConfigValue::Usize)
            .ok_or(()),
        ValueKind::String => value
            .as_str()
            .map(|v| ConfigValue::String(v.to_string()))
            .ok_or(()),
        ValueKind::Path => value
            .as_str()
            .map(|v| ConfigValue::Path(v.to_string()))
            .ok_or(()),
    }
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
    IntellisenseV2InteractiveMaxStaleVersionGap,
    IntellisenseV2InteractiveMaxStaleAgeMs,
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
        RuntimeKey::IntellisenseV2InteractiveMaxStaleVersionGap,
        RuntimeKey::IntellisenseV2InteractiveMaxStaleAgeMs,
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

    pub fn env(self) -> &'static str {
        self.spec().env
    }

    fn index(self) -> usize {
        RuntimeKey::ALL
            .iter()
            .position(|k| *k == self)
            .expect("RuntimeKey present in ALL")
    }

    fn mutability(self) -> KeyMutability {
        match self {
            RuntimeKey::CacheDir
            | RuntimeKey::AgentHttpAddr
            | RuntimeKey::AgentHttpStaticDir
            | RuntimeKey::WebHost
            | RuntimeKey::WebPort
            | RuntimeKey::StaticPath
            | RuntimeKey::ProjectPath
            | RuntimeKey::PlatformVersion
            | RuntimeKey::SyntaxHelperPath => KeyMutability::StartupOnly,
            _ => KeyMutability::Runtime,
        }
    }

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
            RuntimeKey::IntellisenseV2InteractiveMaxStaleVersionGap => KeySpec {
                env: "BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_VERSION_GAP",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(1)),
                mutability: self.mutability(),
            },
            RuntimeKey::IntellisenseV2InteractiveMaxStaleAgeMs => KeySpec {
                env: "BSL_INTELLISENSE_V2_INTERACTIVE_MAX_STALE_AGE_MS",
                kind: ValueKind::U64 {
                    positive_only: false,
                    zero_means_none: false,
                },
                tier: ConfigTier::Stable,
                default: Some(ConfigValue::U64(1000)),
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
mod tests {
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
}
