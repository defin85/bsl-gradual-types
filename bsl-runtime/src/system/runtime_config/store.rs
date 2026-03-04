use super::*;

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
