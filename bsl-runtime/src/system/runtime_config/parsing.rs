use super::*;

pub(super) fn collect_requires_restart_keys(
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

pub(super) fn apply_one_override(
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

pub(super) fn read_env_value(spec: KeySpec) -> LayerValue {
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

pub(super) fn parse_bool_truthy(raw: &str, default: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        "" => default,
        _ => default,
    }
}

pub(super) fn parse_env_string(spec: KeySpec, raw: &str) -> Result<ConfigValue, ()> {
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

pub(super) fn parse_json_value(spec: KeySpec, value: &JsonValue) -> Result<ConfigValue, ()> {
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
