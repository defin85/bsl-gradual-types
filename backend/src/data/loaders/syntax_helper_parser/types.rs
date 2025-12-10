//! Типы для парсера синтакс-помощника

use serde::{Deserialize, Serialize};

/// Статистика парсинга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsingStats {
    pub total_files: usize,
    pub processed_files: usize,
    pub error_count: usize,
    pub total_nodes: usize,
    pub types_count: usize,
    pub methods_count: usize,
    pub properties_count: usize,
    pub categories_count: usize,
    pub index_size: usize,
}
