//! Реализации источников данных для SignatureIndex
//!
//! Единственный источник типов: syntax_helper (документация 1С).
//! GenericInfo применяется отдельно через apply_generic_info_to_repository().

use bsl_shared::domain::signature_registry::SignatureDataSource;
use bsl_shared::domain::types::RawTypeData;

/// Источник: типы из syntax_helper (загруженные из Syntax Helper файлов)
///
/// Это единственный источник данных о методах, свойствах и типах платформы.
/// Все данные (параметры, описания, return_type) берутся из документации 1С.
///
/// # Приоритет: 10 (единственный источник)
pub struct SyntaxHelperSource {
    types: Vec<RawTypeData>,
}

impl SyntaxHelperSource {
    /// Создать источник из уже загруженных типов
    pub fn new(types: Vec<RawTypeData>) -> Self {
        Self { types }
    }
}

impl SignatureDataSource for SyntaxHelperSource {
    fn name(&self) -> &str {
        "SyntaxHelper"
    }

    fn priority(&self) -> u32 {
        10
    }

    fn load(&self) -> Vec<RawTypeData> {
        self.types.clone()
    }
}

#[cfg(test)]
#[path = "signature_sources/tests.rs"]
mod tests;
