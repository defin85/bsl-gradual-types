use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils::hash::hash_content;

pub const GLOBAL_CONTEXT_SOURCE_NOTE: &str = "global_context_source:syntax_helper";
pub const GLOBAL_CONTEXT_SOURCE_KEY_NOTE_PREFIX: &str = "global_context_source_key:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GlobalContextAvailability {
    Unavailable,
    Loaded,
    Degraded { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalContextPropertyData {
    pub name: String,
    pub english_name: Option<String>,
    pub prop_type: Option<String>,
    pub is_readonly: bool,
    pub description: Option<String>,
    pub contexts: Vec<String>,
    pub source_key: String,
    pub source_path: Option<String>,
    pub normalized_key: String,
    pub english_normalized_key: Option<String>,
    pub collection_item_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalContextIndex {
    availability: GlobalContextAvailability,
    by_key: HashMap<String, GlobalContextPropertyData>,
    by_source_key: HashMap<String, GlobalContextPropertyData>,
    fingerprint: String,
}

impl GlobalContextIndex {
    pub fn unavailable() -> Self {
        Self::new(GlobalContextAvailability::Unavailable, Vec::new())
    }

    pub fn degraded(reason: impl Into<String>) -> Self {
        Self::new(
            GlobalContextAvailability::Degraded {
                reason: reason.into(),
            },
            Vec::new(),
        )
    }

    pub fn loaded(properties: Vec<GlobalContextPropertyData>) -> Self {
        Self::new(GlobalContextAvailability::Loaded, properties)
    }

    pub fn availability(&self) -> &GlobalContextAvailability {
        &self.availability
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self.availability, GlobalContextAvailability::Loaded)
    }

    pub fn len(&self) -> usize {
        self.by_source_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_source_key.is_empty()
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn get(&self, name: &str) -> Option<&GlobalContextPropertyData> {
        let key = normalize_global_context_property_key(name);
        self.by_key.get(&key)
    }

    pub fn get_by_source_key(&self, source_key: &str) -> Option<&GlobalContextPropertyData> {
        self.by_source_key.get(source_key)
    }

    pub fn properties(&self) -> impl Iterator<Item = &GlobalContextPropertyData> {
        self.by_source_key.values()
    }

    fn new(
        availability: GlobalContextAvailability,
        mut properties: Vec<GlobalContextPropertyData>,
    ) -> Self {
        properties.sort_by(|left, right| {
            left.source_key
                .cmp(&right.source_key)
                .then_with(|| left.normalized_key.cmp(&right.normalized_key))
        });

        let fingerprint = fingerprint_properties(&availability, &properties);
        let mut by_key = HashMap::with_capacity(properties.len() * 2);
        let mut by_source_key = HashMap::with_capacity(properties.len());

        for property in properties {
            if !property.normalized_key.is_empty() {
                by_key.insert(property.normalized_key.clone(), property.clone());
            }
            if let Some(english_key) = property.english_normalized_key.as_ref() {
                if !english_key.is_empty() {
                    by_key.insert(english_key.clone(), property.clone());
                }
            }
            by_source_key.insert(property.source_key.clone(), property);
        }

        Self {
            availability,
            by_key,
            by_source_key,
            fingerprint,
        }
    }
}

impl Default for GlobalContextIndex {
    fn default() -> Self {
        Self::unavailable()
    }
}

pub fn normalize_global_context_property_key(name: &str) -> String {
    strip_global_context_property_owner(name)
        .trim()
        .to_lowercase()
}

pub fn strip_global_context_property_owner(name: &str) -> &str {
    let trimmed = name.trim();
    const RU_PREFIX: &str = "Глобальный контекст.";
    const EN_PREFIX: &str = "Global context.";

    if let Some(stripped) = trimmed.strip_prefix(RU_PREFIX) {
        return stripped;
    }

    if trimmed
        .get(..EN_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(EN_PREFIX))
    {
        return &trimmed[EN_PREFIX.len()..];
    }

    trimmed
}

fn fingerprint_properties(
    availability: &GlobalContextAvailability,
    properties: &[GlobalContextPropertyData],
) -> String {
    let mut parts = vec![format!("{:?}", availability)];
    for property in properties {
        parts.push(format!(
            "{}:{}:{}:{}",
            property.source_key,
            property.normalized_key,
            property.english_normalized_key.as_deref().unwrap_or(""),
            property.prop_type.as_deref().unwrap_or("")
        ));
    }
    format!("{:016x}", hash_content(&parts.join("|")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_context_key_normalization_strips_owner_prefixes() {
        assert_eq!(
            normalize_global_context_property_key("Глобальный контекст.Метаданные"),
            "метаданные"
        );
        assert_eq!(
            normalize_global_context_property_key("global context.Metadata"),
            "metadata"
        );
    }
}
