//! API types

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiMetrics {
    pub total_types: usize,
    pub known_types: usize,
    pub inferred_types: usize,
    pub unknown_types: usize,
}
