//! Platform documentation adapter

use anyhow::Result;

pub struct PlatformDocsAdapter;

impl PlatformDocsAdapter {
    pub fn new() -> Self {
        Self
    }
    
    pub fn load_from_cache(&self, _version: &str) -> Result<Vec<crate::core::types::PlatformType>> {
        // TODO: Implement loading from cache
        Ok(Vec::new())
    }
}